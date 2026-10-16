//! Active skill set resolution honoring kits, modules, requires, skill_requires, and always modules.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::check::ROUTER_DESC_TOKENS;
use crate::error::CatalogError;
use crate::model::Catalog;

/// A resolved skill reference ready for activation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResolvedSkill {
    pub repo: String,
    pub name: String,
    pub path: String,
    pub module: String,
}

/// The set of active skills for a project/turn along with noise budget metrics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ActiveSet {
    pub skills: Vec<ResolvedSkill>,
    pub visible: usize,
    pub desc_tokens: usize,
}

/// Resolve the active set of skills for a kit and optional extra modules.
///
/// Honours:
/// - Always modules (e.g. `studio.core`)
/// - Selected kit modules (narrowed or "all")
/// - Extra modules enabled for the project/turn
/// - Module `requires` dependencies
/// - Skill-level `skill_requires` dependencies
/// - Excluded skills (never loaded)
pub fn resolve(
    catalog: &Catalog,
    kit: Option<&str>,
    extra_modules: &[&str],
) -> Result<ActiveSet, CatalogError> {
    // 1. Collect modules to activate
    // Mapping of module_id -> optional set of chosen skill names (None means all)
    let mut module_selections: BTreeMap<String, Option<BTreeSet<String>>> = BTreeMap::new();

    // Include always modules
    for (mid, mod_def) in &catalog.modules {
        if mod_def.always {
            module_selections.entry(mid.clone()).or_insert(None);
        }
    }

    // Include kit modules
    if let Some(kid) = kit {
        let kit_def = catalog
            .kits
            .get(kid)
            .ok_or_else(|| CatalogError::Validation(format!("Unknown kit: {}", kid)))?;

        for (mid, sel) in &kit_def.use_modules {
            if sel.is_all() {
                module_selections.insert(mid.clone(), None);
            } else if let Some(skills) = sel.as_slice() {
                let entry = module_selections
                    .entry(mid.clone())
                    .or_insert_with(|| Some(BTreeSet::new()));
                if let Some(set) = entry {
                    for s in skills {
                        set.insert(s.clone());
                    }
                }
            }
        }
    }

    // Include extra modules
    for &extra in extra_modules {
        if !catalog.modules.contains_key(extra) {
            return Err(CatalogError::Validation(format!(
                "Unknown extra module: {}",
                extra
            )));
        }
        module_selections.insert(extra.to_string(), None);
    }

    // 2. Transitive resolution loop for requires & skill_requires
    let mut resolved_modules_skills: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();

    loop {
        let mut changed = false;

        // Ensure module `requires` are present
        let current_modules: Vec<String> = module_selections.keys().cloned().collect();
        for mid in current_modules {
            if let Some(mod_def) = catalog.modules.get(&mid) {
                for req in &mod_def.requires {
                    if !module_selections.contains_key(req) {
                        module_selections.insert(req.clone(), None);
                        changed = true;
                    }
                }
            }
        }

        // Determine actual skills for each selected module and collect required skill additions
        let mut skill_require_additions: Vec<(String, Option<String>)> = Vec::new();

        for (mid, maybe_chosen) in &module_selections {
            let Some(mod_def) = catalog.modules.get(mid) else {
                continue;
            };

            let mut refs = Vec::new();
            for (repo, names) in &mod_def.skills {
                for name in names {
                    // Check if excluded in catalog
                    if let Some(ex_repo) = catalog.excluded.get(repo) {
                        if ex_repo.contains_key(name) {
                            continue;
                        }
                    }

                    let include = match maybe_chosen {
                        None => true,
                        Some(set) => set.contains(name),
                    };

                    if include {
                        refs.push((repo.clone(), name.clone()));
                    }
                }
            }

            // Check skill_requires for the chosen skills in this module
            for (_repo, skill_name) in &refs {
                if let Some(deps) = mod_def.skill_requires.get(skill_name) {
                    for dep in deps {
                        let (dmid, dname) = match dep.split_once('/') {
                            Some((m, s)) => (m.to_string(), Some(s.to_string())),
                            None => (dep.clone(), None),
                        };
                        skill_require_additions.push((dmid, dname));
                    }
                }
            }

            resolved_modules_skills.insert(mid.clone(), refs);
        }

        // Apply skill_require additions to module_selections
        for (dmid, dname) in skill_require_additions {
            if let Some(target_skill) = dname {
                let entry = module_selections
                    .entry(dmid)
                    .or_insert_with(|| Some(BTreeSet::new()));
                if let Some(set) = entry {
                    if set.insert(target_skill) {
                        changed = true;
                    }
                }
            } else if let std::collections::btree_map::Entry::Vacant(e) =
                module_selections.entry(dmid)
            {
                e.insert(None);
                changed = true;
            }
        }

        if !changed {
            break;
        }
    }

    // 3. Flatten resolved skills and compute visible / desc_tokens
    let mut skills = Vec::new();
    let mut visible = 0;
    let mut desc_tokens = 0;

    for (mid, refs) in &resolved_modules_skills {
        let mut unique_refs = Vec::new();
        for r in refs {
            if !unique_refs.contains(r) {
                unique_refs.push(r.clone());
            }
        }

        for (repo, name) in &unique_refs {
            let path = format!("skills/{}", name);
            skills.push(ResolvedSkill {
                repo: repo.clone(),
                name: name.clone(),
                path,
                module: mid.clone(),
            });
        }

        let router = catalog.modules.get(mid).and_then(|m| m.router.as_deref());

        if router.is_some() && unique_refs.len() > 1 {
            visible += 1;
            if router == Some("generated") {
                desc_tokens += ROUTER_DESC_TOKENS;
            } else {
                // Approximate 80 tokens if router description not looked up from inventory
                desc_tokens += 80;
            }
        } else {
            visible += unique_refs.len();
            for _ in &unique_refs {
                desc_tokens += 60; // baseline estimate
            }
        }
    }

    skills.sort();

    Ok(ActiveSet {
        skills,
        visible,
        desc_tokens,
    })
}
