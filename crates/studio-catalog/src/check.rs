//! Validation and budget checks for catalog against inventory.
//!
//! Port of `scripts/catalog/check_catalog.py`.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::inventory::Inventory;
use crate::model::Catalog;

/// Maximum number of skills visible to Claude per turn.
pub const MAX_VISIBLE_SKILLS: usize = 15;

/// Maximum tokens allowed for skill descriptions per turn.
pub const MAX_DESC_TOKENS: usize = 2000;

/// Token estimate for a generated router description.
pub const ROUTER_DESC_TOKENS: usize = 60;

/// Per-kit noise budget evaluation report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct KitReport {
    pub kit_id: String,
    pub visible: usize,
    pub desc_tokens: usize,
    pub loaded: usize,
    pub modules: Vec<String>,
    pub over_budget: bool,
}

/// Catalog validation report listing all problems found.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Report {
    pub problems: Vec<String>,
    pub kit_reports: Vec<KitReport>,
    pub total_skills: usize,
    pub total_desc_tokens: usize,
}

impl Report {
    /// Returns true if no problems were found.
    pub fn is_ok(&self) -> bool {
        self.problems.is_empty()
    }
}

/// Validate `catalog` against `inventory` and report per-kit noise budget.
///
/// Port of `scripts/catalog/check_catalog.py`:
/// 1. Pinned commit drift: catalog sources commit vs local repo commit.
/// 2. Curated set: excluded skills, adapt keys, module skill refs.
/// 3. Module skills exist in inventory and module requires exist in catalog.
/// 4. Any inventory skill not in curated set is uncurated.
/// 5. For each kit, resolve active skills, check unknown kit skills, calculate visible/desc_tokens, check budget.
pub fn check(catalog: &Catalog, inventory: &Inventory) -> Report {
    let mut problems = Vec::new();
    let mut kit_reports = Vec::new();

    // Map: (repo, skill_name) -> skill_inventory
    let mut skills_map = HashMap::new();
    for repo in &inventory.repositories {
        for s in &repo.skills {
            skills_map.insert((repo.repo.clone(), s.name.clone()), s);
        }
    }

    // 1. Commit drift
    let pinned: HashMap<String, String> = catalog
        .sources
        .iter()
        // Bundled sources have no git checkout to drift from.
        .filter(|(_, src)| src.kind.as_deref() != Some("local"))
        .map(|(repo, src)| (repo.clone(), src.commit.clone()))
        .collect();

    for repo in &inventory.repositories {
        if let Some(pinned_commit) = pinned.get(&repo.repo) {
            if pinned_commit != &repo.commit {
                let pin_short = if pinned_commit.len() >= 7 {
                    &pinned_commit[..7]
                } else {
                    pinned_commit
                };
                let loc_short = if repo.commit.len() >= 7 {
                    &repo.commit[..7]
                } else {
                    &repo.commit
                };
                problems.push(format!(
                    "commit drift {}: catalog {} != local {}",
                    repo.repo, pin_short, loc_short
                ));
            }
        }
    }

    // 2. Curated set
    let mut curated: BTreeSet<(String, String)> = BTreeSet::new();

    for (repo, names) in &catalog.excluded {
        for skill_name in names.keys() {
            curated.insert((repo.clone(), skill_name.clone()));
        }
    }

    for key in catalog.adapt.keys() {
        if let Some((repo, name)) = key.rsplit_once('/') {
            curated.insert((repo.to_string(), name.to_string()));
        }
    }

    // 3. Module skills
    let mut module_skills: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for (mid, mod_def) in &catalog.modules {
        let mut refs = Vec::new();
        for (repo, names) in &mod_def.skills {
            for n in names {
                refs.push((repo.clone(), n.clone()));
            }
        }

        for r in &refs {
            if !skills_map.contains_key(r) {
                problems.push(format!("module {}: unknown skill {}/{}", mid, r.0, r.1));
            }
            curated.insert(r.clone());
        }

        module_skills.insert(mid.clone(), refs);

        for dep in &mod_def.requires {
            if !catalog.modules.contains_key(dep) {
                problems.push(format!("module {}: unknown required module {}", mid, dep));
            }
        }
    }

    // 4. Uncurated skills
    for repo in &inventory.repositories {
        for s in &repo.skills {
            let key = (repo.repo.clone(), s.name.clone());
            if !curated.contains(&key) {
                problems.push(format!("uncurated skill {}/{}", key.0, key.1));
            }
        }
    }

    // 5. Kit budgets
    for (kid, kit) in &catalog.kits {
        let mut active: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();

        for (mid, sel) in &kit.use_modules {
            let Some(refs) = module_skills.get(mid) else {
                problems.push(format!("kit {}: unknown module {}", kid, mid));
                continue;
            };

            let chosen: Vec<(String, String)> = if sel.is_all() {
                refs.clone()
            } else if let Some(skill_names) = sel.as_slice() {
                let chosen_set: BTreeSet<&str> = skill_names.iter().map(String::as_str).collect();
                let ref_names: BTreeSet<&str> = refs.iter().map(|r| r.1.as_str()).collect();

                for n in &chosen_set {
                    if !ref_names.contains(n) {
                        problems.push(format!("kit {}: {} has no skill {}", kid, mid, n));
                    }
                }

                refs.iter()
                    .filter(|r| chosen_set.contains(r.1.as_str()))
                    .cloned()
                    .collect()
            } else {
                Vec::new()
            };

            active
                .entry(mid.clone())
                .or_default()
                .extend(chosen.clone());

            let Some(mod_def) = catalog.modules.get(mid) else {
                continue;
            };

            // skill_requires
            for (name, deps) in &mod_def.skill_requires {
                if chosen.iter().any(|r| &r.1 == name) {
                    for dep in deps {
                        let (dmid, dname) = match dep.split_once('/') {
                            Some((m, s)) => (m, Some(s)),
                            None => (dep.as_str(), None),
                        };

                        if let Some(dep_refs) = module_skills.get(dmid) {
                            let to_add: Vec<(String, String)> = dep_refs
                                .iter()
                                .filter(|r| dname.is_none() || dname == Some(r.1.as_str()))
                                .cloned()
                                .collect();
                            active.entry(dmid.to_string()).or_default().extend(to_add);
                        }
                    }
                }
            }

            // requires
            for req in &mod_def.requires {
                active.entry(req.clone()).or_default();
            }
        }

        let mut visible = 0;
        let mut desc = 0;
        let mut loaded = 0;

        for (mid, refs) in &active {
            // Deduplicate refs preserving order
            let mut unique_refs = Vec::new();
            for r in refs {
                if !unique_refs.contains(r) {
                    unique_refs.push(r.clone());
                }
            }

            loaded += unique_refs.len();

            let router = catalog.modules.get(mid).and_then(|m| m.router.as_deref());

            if router.is_some() && unique_refs.len() > 1 {
                visible += 1;
                if router == Some("generated") {
                    desc += ROUTER_DESC_TOKENS;
                } else if let Some(r_str) = router {
                    if let Some((r_repo, r_name)) = r_str.rsplit_once('/') {
                        let key = (r_repo.to_string(), r_name.to_string());
                        if let Some(s) = skills_map.get(&key) {
                            desc += s.desc_tokens;
                        }
                    }
                }
            } else {
                visible += unique_refs.len();
                for r in &unique_refs {
                    if let Some(s) = skills_map.get(r) {
                        desc += s.desc_tokens;
                    }
                }
            }
        }

        let over_budget = visible > MAX_VISIBLE_SKILLS || desc > MAX_DESC_TOKENS;
        if over_budget {
            problems.push(format!(
                "kit {} over budget: {} visible, {} tokens",
                kid, visible, desc
            ));
        }

        kit_reports.push(KitReport {
            kit_id: kid.clone(),
            visible,
            desc_tokens: desc,
            loaded,
            modules: active.keys().cloned().collect(),
            over_budget,
        });
    }

    let total_skills = skills_map.len();
    let total_desc_tokens: usize = skills_map.values().map(|s| s.desc_tokens).sum();

    Report {
        problems,
        kit_reports,
        total_skills,
        total_desc_tokens,
    }
}
