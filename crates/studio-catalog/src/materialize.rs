//! Materialize an active skill set into a Claude Code plugin directory.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;
use walkdir::WalkDir;

use crate::error::CatalogError;
use crate::inventory::frontmatter;
use crate::model::Catalog;
use crate::resolve::ActiveSet;

/// Options for plugin materialization.
#[derive(Debug, Clone)]
pub struct MaterializeOptions {
    pub plugin_name: String,
    pub plugin_version: String,
}

impl Default for MaterializeOptions {
    fn default() -> Self {
        Self {
            plugin_name: "studio-workspace".to_string(),
            plugin_version: "0.1.0".to_string(),
        }
    }
}

/// Materialize the active set into a Claude Code plugin directory.
///
/// Layout:
/// - `<out_dir>/.claude-plugin/plugin.json`:
///   `{"name":"studio-workspace","version":"0.1.0","description":"Studio workspace active skills"}`
/// - `<out_dir>/skills/<name>/`:
///   Copies of skill directories (real copies, never symlinks).
///
/// Routers:
/// - `router: generated`:
///   Emits ONE `<out_dir>/skills/<module-id-with-dashes>/SKILL.md` router whose description
///   names the module and lists sub-skills. Sub-skills are placed under
///   `<out_dir>/skills/<router>/refs/<name>/` with their `SKILL.md` renamed to `GUIDE.md` so
///   Claude Code does not auto-load them as root skills.
///   The router body instructs Claude to read the matching GUIDE.md.
/// - `router: <repo>/<name>` (upstream router):
///   The router skill is copied as a top-level skill.
///   Sibling skills in the module are placed under `<out_dir>/skills/<router-skill-name>/refs/<sibling-name>/`
///   with their `SKILL.md` renamed to `GUIDE.md`.
/// - `adapt: <repo>/<name>`:
///   If a skill matches an entry in `catalog.adapt`, emit a wrapper `SKILL.md` whose body starts
///   with the Studio adaptation note from `catalog.yaml`, followed by the original body.
pub fn materialize_plugin<P: AsRef<Path>, Q: AsRef<Path>>(
    active_set: &ActiveSet,
    catalog: &Catalog,
    sources_dir: P,
    out_dir: Q,
) -> Result<(), CatalogError> {
    let sdir = sources_dir.as_ref();
    let odir = out_dir.as_ref();

    // 1. Create .claude-plugin/plugin.json
    let meta_dir = odir.join(".claude-plugin");
    fs::create_dir_all(&meta_dir).map_err(|e| CatalogError::Io {
        path: meta_dir.clone(),
        source: e,
    })?;

    let plugin_json = json!({
        "name": "studio-workspace",
        "version": "0.1.0",
        "description": "Studio workspace active skills"
    });

    let plugin_json_path = meta_dir.join("plugin.json");
    fs::write(
        &plugin_json_path,
        serde_json::to_string_pretty(&plugin_json)?,
    )
    .map_err(|e| CatalogError::Io {
        path: plugin_json_path,
        source: e,
    })?;

    let out_skills_dir = odir.join("skills");
    fs::create_dir_all(&out_skills_dir).map_err(|e| CatalogError::Io {
        path: out_skills_dir.clone(),
        source: e,
    })?;

    // Group active skills by module
    let mut module_skills: BTreeMap<String, Vec<crate::resolve::ResolvedSkill>> = BTreeMap::new();
    for s in &active_set.skills {
        module_skills
            .entry(s.module.clone())
            .or_default()
            .push(s.clone());
    }

    for (mid, skills) in module_skills {
        let mod_def = catalog.modules.get(&mid);
        let router_kind = mod_def.and_then(|m| m.router.as_deref());

        match router_kind {
            Some("generated") if skills.len() > 1 => {
                // Generated router
                materialize_generated_router(&mid, &skills, catalog, sdir, &out_skills_dir)?;
            }
            Some(upstream_router_str) if skills.len() > 1 => {
                // Upstream router (repo/name)
                materialize_upstream_router(
                    &mid,
                    upstream_router_str,
                    &skills,
                    catalog,
                    sdir,
                    &out_skills_dir,
                )?;
            }
            _ => {
                // Standalone skills or module with 1 skill
                for s in &skills {
                    materialize_standalone_skill(s, catalog, sdir, &out_skills_dir)?;
                }
            }
        }
    }

    Ok(())
}

fn locate_skill_dir(sources_dir: &Path, repo: &str, skill_name: &str) -> Option<PathBuf> {
    let repo_dir_name = repo.replace('/', "__");
    let repo_path = sources_dir.join(&repo_dir_name);
    if !repo_path.exists() {
        return None;
    }

    // Direct search skills/<name>
    let direct = repo_path.join("skills").join(skill_name);
    if direct.join("SKILL.md").exists() {
        return Some(direct);
    }

    // Walk repo to find matching SKILL.md
    for entry in WalkDir::new(&repo_path)
        .follow_links(false)
        .into_iter()
        .flatten()
    {
        if entry.file_type().is_file() {
            let p = entry.path();
            if let Some(fname) = p.file_name() {
                if fname.to_string_lossy().eq_ignore_ascii_case("SKILL.md") {
                    let parent = p.parent()?;
                    let text = fs::read_to_string(p).unwrap_or_default();
                    let fm = frontmatter(&text);
                    let name = fm.get("name").map(String::as_str).unwrap_or_else(|| {
                        parent.file_name().and_then(|s| s.to_str()).unwrap_or("")
                    });
                    if name == skill_name {
                        return Some(parent.to_path_buf());
                    }
                }
            }
        }
    }

    None
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), CatalogError> {
    fs::create_dir_all(dst).map_err(|e| CatalogError::Io {
        path: dst.to_path_buf(),
        source: e,
    })?;

    for entry in WalkDir::new(src).follow_links(false).into_iter().flatten() {
        let entry_path = entry.path();
        if entry_path == src {
            continue;
        }

        let rel = entry_path
            .strip_prefix(src)
            .map_err(|e| CatalogError::Materialize(e.to_string()))?;
        let target = dst.join(rel);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).map_err(|e| CatalogError::Io {
                path: target,
                source: e,
            })?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|e| CatalogError::Io {
                    path: parent.to_path_buf(),
                    source: e,
                })?;
            }
            fs::copy(entry_path, &target).map_err(|e| CatalogError::Io {
                path: target,
                source: e,
            })?;
        }
    }
    Ok(())
}

fn apply_adapt_note_if_any(text: &str, repo: &str, skill_name: &str, catalog: &Catalog) -> String {
    let adapt_key = format!("{}/{}", repo, skill_name);
    if let Some(note) = catalog.adapt.get(&adapt_key) {
        // Strip frontmatter from text, prepend note to body, reassemble
        let trimmed = text.trim_start();
        if let Some(after_start) = trimmed.strip_prefix("---") {
            if let Some(first_newline) = after_start.find('\n') {
                let rest = &after_start[first_newline + 1..];
                if let Some(end_idx) = rest.find("---") {
                    let fm_block = &rest[..end_idx];
                    let body = &rest[end_idx + 3..];
                    return format!(
                        "---\n{}---\n\n> [!NOTE] **Studio Adaptation**\n> {}\n\n{}",
                        fm_block,
                        note,
                        body.trim_start()
                    );
                }
            }
        }

        // No frontmatter found
        format!("> [!NOTE] **Studio Adaptation**\n> {}\n\n{}", note, text)
    } else {
        text.to_string()
    }
}

fn materialize_standalone_skill(
    skill: &crate::resolve::ResolvedSkill,
    catalog: &Catalog,
    sources_dir: &Path,
    out_skills_dir: &Path,
) -> Result<(), CatalogError> {
    let Some(src_dir) = locate_skill_dir(sources_dir, &skill.repo, &skill.name) else {
        return Ok(());
    };

    let target_dir = out_skills_dir.join(&skill.name);
    copy_dir_all(&src_dir, &target_dir)?;

    // Adapt wrapper if needed
    let skill_md_path = target_dir.join("SKILL.md");
    if skill_md_path.exists() {
        let content = fs::read_to_string(&skill_md_path).unwrap_or_default();
        let adapted = apply_adapt_note_if_any(&content, &skill.repo, &skill.name, catalog);
        fs::write(&skill_md_path, adapted).map_err(|e| CatalogError::Io {
            path: skill_md_path,
            source: e,
        })?;
    }

    Ok(())
}

fn materialize_generated_router(
    module_id: &str,
    skills: &[crate::resolve::ResolvedSkill],
    catalog: &Catalog,
    sources_dir: &Path,
    out_skills_dir: &Path,
) -> Result<(), CatalogError> {
    let router_dir_name = module_id.replace('.', "-");
    let router_dir = out_skills_dir.join(&router_dir_name);
    let refs_dir = router_dir.join("refs");

    fs::create_dir_all(&refs_dir).map_err(|e| CatalogError::Io {
        path: refs_dir.clone(),
        source: e,
    })?;

    let mut subskill_names = Vec::new();

    for s in skills {
        subskill_names.push(s.name.clone());
        let Some(src_dir) = locate_skill_dir(sources_dir, &s.repo, &s.name) else {
            continue;
        };

        let sub_target_dir = refs_dir.join(&s.name);
        copy_dir_all(&src_dir, &sub_target_dir)?;

        // Rename SKILL.md -> GUIDE.md
        let orig_md = sub_target_dir.join("SKILL.md");
        if orig_md.exists() {
            let guide_md = sub_target_dir.join("GUIDE.md");
            let content = fs::read_to_string(&orig_md).unwrap_or_default();
            let adapted = apply_adapt_note_if_any(&content, &s.repo, &s.name, catalog);
            fs::write(&guide_md, adapted).map_err(|e| CatalogError::Io {
                path: guide_md,
                source: e,
            })?;
            let _ = fs::remove_file(&orig_md);
        }
    }

    // Emit router SKILL.md
    let router_skill_md = router_dir.join("SKILL.md");
    let desc_list = subskill_names.join(", ");
    let content = format!(
        "---\nname: {name}\ndescription: Studio router for module {mid}. Sub-skills: {list}.\n---\n\n# Module Router: {mid}\n\nThis router coordinates the following sub-skills:\n{items}\n\nWhen you need details for any of these capabilities, read the matching guide file:\n`refs/<skill-name>/GUIDE.md`.\n",
        name = router_dir_name,
        mid = module_id,
        list = desc_list,
        items = subskill_names.iter().map(|s| format!("- **{}**: read `refs/{}/GUIDE.md`", s, s)).collect::<Vec<_>>().join("\n")
    );

    fs::write(&router_skill_md, content).map_err(|e| CatalogError::Io {
        path: router_skill_md,
        source: e,
    })?;

    Ok(())
}

fn materialize_upstream_router(
    _module_id: &str,
    upstream_router_str: &str,
    skills: &[crate::resolve::ResolvedSkill],
    catalog: &Catalog,
    sources_dir: &Path,
    out_skills_dir: &Path,
) -> Result<(), CatalogError> {
    let (_router_repo, router_skill_name) = upstream_router_str
        .rsplit_once('/')
        .unwrap_or(("", upstream_router_str));

    // Router skill target dir
    let router_dir = out_skills_dir.join(router_skill_name);
    let refs_dir = router_dir.join("refs");

    // Copy router skill itself
    for s in skills {
        if s.name == router_skill_name {
            if let Some(src_dir) = locate_skill_dir(sources_dir, &s.repo, &s.name) {
                copy_dir_all(&src_dir, &router_dir)?;
                let orig_md = router_dir.join("SKILL.md");
                if orig_md.exists() {
                    let content = fs::read_to_string(&orig_md).unwrap_or_default();
                    let adapted = apply_adapt_note_if_any(&content, &s.repo, &s.name, catalog);
                    let _ = fs::write(&orig_md, adapted);
                }
            }
            break;
        }
    }

    // Copy sibling skills into refs/<name>/GUIDE.md
    for s in skills {
        if s.name == router_skill_name {
            continue;
        }

        if let Some(src_dir) = locate_skill_dir(sources_dir, &s.repo, &s.name) {
            let sub_target_dir = refs_dir.join(&s.name);
            copy_dir_all(&src_dir, &sub_target_dir)?;

            let orig_md = sub_target_dir.join("SKILL.md");
            if orig_md.exists() {
                let guide_md = sub_target_dir.join("GUIDE.md");
                let content = fs::read_to_string(&orig_md).unwrap_or_default();
                let adapted = apply_adapt_note_if_any(&content, &s.repo, &s.name, catalog);
                fs::write(&guide_md, adapted).map_err(|e| CatalogError::Io {
                    path: guide_md,
                    source: e,
                })?;
                let _ = fs::remove_file(&orig_md);
            }
        }
    }

    Ok(())
}
