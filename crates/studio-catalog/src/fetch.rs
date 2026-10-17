//! Git fetching and updating of source repositories.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::error::CatalogError;
use crate::inventory::{frontmatter, SCRIPT_EXTENSIONS};
use crate::model::Catalog;

/// Summary of changes when checking for upstream updates on a source.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceDiff {
    pub repo: String,
    pub old_commit: String,
    pub new_commit: String,
    pub changed_skills: Vec<String>,
    pub changed_descriptions: BTreeMap<String, (String, String)>,
    pub new_scripts: BTreeMap<String, Vec<String>>,
    pub needs_recuration: bool,
}

/// Clones or checkouts missing sources according to pinned commits in `catalog`.
///
/// For missing sources:
/// - `git clone --depth 1 [--filter=blob:none --sparse] https://github.com/<repo>.git <target_dir>`
/// - `git fetch --depth 1 origin <commit>`
/// - `git checkout <commit>`
/// - `git sparse-checkout set <sparse_dirs>` if sparse is configured.
pub fn fetch<P: AsRef<Path>>(catalog: &Catalog, sources_dir: P) -> Result<(), CatalogError> {
    let sdir = sources_dir.as_ref();
    fs::create_dir_all(sdir).map_err(|e| CatalogError::Io {
        path: sdir.to_path_buf(),
        source: e,
    })?;

    for (repo_id, src) in &catalog.sources {
        // Bundled with the app (installed by the shell), nothing to clone.
        if src.kind.as_deref() == Some("local") {
            continue;
        }
        let repo_folder_name = repo_id.replace('/', "__");
        let target_repo_path = sdir.join(&repo_folder_name);

        if !target_repo_path.exists() {
            let url = format!("https://github.com/{}.git", repo_id);

            let mut clone_args = vec!["clone", "--depth", "1"];
            if !src.sparse.is_empty() {
                clone_args.push("--filter=blob:none");
                clone_args.push("--sparse");
            }
            let target_str = target_repo_path.to_string_lossy().to_string();
            clone_args.push(&url);
            clone_args.push(&target_str);

            run_git(&clone_args, None)?;

            if !src.sparse.is_empty() {
                let mut sparse_args = vec!["sparse-checkout", "set"];
                for sp in &src.sparse {
                    sparse_args.push(sp);
                }
                run_git(&sparse_args, Some(&target_repo_path))?;
            }

            // Fetch pinned commit and checkout
            let fetch_args = vec!["fetch", "--depth", "1", "origin", &src.commit];
            let _ = run_git(&fetch_args, Some(&target_repo_path));

            let checkout_args = vec!["checkout", &src.commit];
            run_git(&checkout_args, Some(&target_repo_path))?;
        }
    }

    Ok(())
}

/// Fetches latest upstream for a repository and calculates a diff summary.
///
/// Note: Marks changed skills as needing re-curation; does not auto-activate.
pub fn update<P: AsRef<Path>>(repo_id: &str, repo_path: P) -> Result<SourceDiff, CatalogError> {
    let rpath = repo_path.as_ref();
    if !rpath.exists() {
        return Err(CatalogError::Git(format!(
            "Repository path does not exist: {:?}",
            rpath
        )));
    }

    // Get current HEAD
    let old_commit = git_output(&["rev-parse", "HEAD"], Some(rpath))?;

    // Pre-scan skills to record current descriptions and scripts
    let old_skills = scan_skills_map(rpath)?;

    // Fetch latest upstream
    run_git(&["fetch", "origin"], Some(rpath))?;

    // Determine default branch or origin/HEAD
    let remote_head = git_output(&["rev-parse", "origin/HEAD"], Some(rpath))
        .or_else(|_| git_output(&["rev-parse", "origin/main"], Some(rpath)))
        .or_else(|_| git_output(&["rev-parse", "origin/master"], Some(rpath)))?;

    if remote_head == old_commit {
        return Ok(SourceDiff {
            repo: repo_id.to_string(),
            old_commit: old_commit.clone(),
            new_commit: old_commit,
            changed_skills: Vec::new(),
            changed_descriptions: BTreeMap::new(),
            new_scripts: BTreeMap::new(),
            needs_recuration: false,
        });
    }

    // Checkout new commit
    run_git(&["checkout", &remote_head], Some(rpath))?;

    let new_skills = scan_skills_map(rpath)?;

    let mut changed_skills = BTreeSet::new();
    let mut changed_descriptions = BTreeMap::new();
    let mut new_scripts = BTreeMap::new();

    // Compare
    for (name, new_data) in &new_skills {
        match old_skills.get(name) {
            None => {
                // Completely new skill
                changed_skills.insert(name.clone());
                changed_descriptions
                    .insert(name.clone(), (String::new(), new_data.description.clone()));
                if !new_data.scripts.is_empty() {
                    new_scripts.insert(name.clone(), new_data.scripts.clone());
                }
            }
            Some(old_data) => {
                let mut skill_changed = false;
                if old_data.description != new_data.description {
                    changed_descriptions.insert(
                        name.clone(),
                        (old_data.description.clone(), new_data.description.clone()),
                    );
                    skill_changed = true;
                }
                let old_script_set: BTreeSet<&str> =
                    old_data.scripts.iter().map(String::as_str).collect();
                let added_scripts: Vec<String> = new_data
                    .scripts
                    .iter()
                    .filter(|s| !old_script_set.contains(s.as_str()))
                    .cloned()
                    .collect();
                if !added_scripts.is_empty() {
                    new_scripts.insert(name.clone(), added_scripts);
                    skill_changed = true;
                }
                if skill_changed {
                    changed_skills.insert(name.clone());
                }
            }
        }
    }

    for name in old_skills.keys() {
        if !new_skills.contains_key(name) {
            changed_skills.insert(name.clone());
        }
    }

    let needs_recuration = !changed_skills.is_empty();

    // Revert back to old commit so we don't accidentally leave working tree modified
    let _ = run_git(&["checkout", &old_commit], Some(rpath));

    Ok(SourceDiff {
        repo: repo_id.to_string(),
        old_commit,
        new_commit: remote_head,
        changed_skills: changed_skills.into_iter().collect(),
        changed_descriptions,
        new_scripts,
        needs_recuration,
    })
}

struct SkillQuickData {
    description: String,
    scripts: Vec<String>,
}

fn scan_skills_map(repo_path: &Path) -> Result<BTreeMap<String, SkillQuickData>, CatalogError> {
    use walkdir::WalkDir;

    let mut out = BTreeMap::new();
    for entry in WalkDir::new(repo_path)
        .follow_links(false)
        .into_iter()
        .flatten()
    {
        if entry.file_type().is_file() {
            let p = entry.path();
            if let Some(file_name) = p.file_name() {
                if file_name.to_string_lossy().eq_ignore_ascii_case("SKILL.md") {
                    let in_git = p.components().any(|c| c.as_os_str() == ".git");
                    if in_git {
                        continue;
                    }
                    let text = fs::read_to_string(p).unwrap_or_default();
                    let fm = frontmatter(&text);
                    let sdir = p.parent().unwrap_or(repo_path);
                    let default_name = sdir
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or_default()
                        .to_string();
                    let name = fm.get("name").cloned().unwrap_or(default_name);
                    let description = fm.get("description").cloned().unwrap_or_default();

                    let mut scripts = Vec::new();
                    for f in WalkDir::new(sdir).into_iter().flatten() {
                        if f.file_type().is_file() {
                            let fp = f.path();
                            if let Some(ext) = fp.extension().and_then(|e| e.to_str()) {
                                if SCRIPT_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                                    let rel = fp
                                        .strip_prefix(sdir)
                                        .unwrap_or(fp)
                                        .to_string_lossy()
                                        .replace('\\', "/");
                                    scripts.push(rel);
                                }
                            }
                        }
                    }
                    scripts.sort();
                    out.insert(
                        name,
                        SkillQuickData {
                            description,
                            scripts,
                        },
                    );
                }
            }
        }
    }
    Ok(out)
}

fn run_git(args: &[&str], cwd: Option<&Path>) -> Result<(), CatalogError> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    if let Some(c) = cwd {
        cmd.current_dir(c);
    }
    let status = cmd
        .status()
        .map_err(|e| CatalogError::Git(format!("Failed to execute git: {}", e)))?;
    if !status.success() {
        return Err(CatalogError::Git(format!(
            "git command failed with status {}: git {}",
            status,
            args.join(" ")
        )));
    }
    Ok(())
}

fn git_output(args: &[&str], cwd: Option<&Path>) -> Result<String, CatalogError> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    if let Some(c) = cwd {
        cmd.current_dir(c);
    }
    let out = cmd
        .output()
        .map_err(|e| CatalogError::Git(format!("Failed to execute git: {}", e)))?;
    if !out.status.success() {
        return Err(CatalogError::Git(format!(
            "git command failed: git {}",
            args.join(" ")
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
