//! Inventory scanner for third-party skill/MCP sources.
//!
//! Mirrors `scripts/catalog/inventory.py`:
//! Reads files only, never executes anything inside sources.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::error::CatalogError;

/// Known script file extensions.
pub const SCRIPT_EXTENSIONS: &[&str] = &[
    "py", "js", "mjs", "cjs", "ts", "sh", "ps1", "bat", "jsx", "tsx",
];

/// Known license file basenames.
pub const LICENSE_NAMES: &[&str] = &[
    "LICENSE",
    "LICENSE.md",
    "LICENSE.txt",
    "LICENCE",
    "license",
    "LICENSE.TXT",
];

/// Full inventory of scanned repositories.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Inventory {
    pub repositories: Vec<RepoInventory>,
}

/// Inventory for a single cloned repository.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RepoInventory {
    pub repo: String,
    pub commit: String,
    pub license: Option<String>,
    pub markers: RepoMarkers,
    pub skill_count: usize,
    pub desc_tokens_total: usize,
    pub skills: Vec<SkillInventory>,
}

/// Markers indicating repository ecosystem.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RepoMarkers {
    pub claude_plugin: bool,
    pub package_json: bool,
    pub pyproject: bool,
    pub agents_dir: bool,
}

/// Metadata for a single skill discovered in a repository.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SkillInventory {
    pub path: String,
    pub name: String,
    pub description: String,
    pub desc_tokens: usize,
    pub body_tokens: usize,
    pub bundle_tokens: usize,
    pub scripts: Vec<String>,
    pub script_count: usize,
    pub license: Option<String>,
}

/// Estimate token count from character count (chars / 4, minimum 1).
pub fn estimate_tokens(chars: usize) -> usize {
    if chars == 0 {
        1
    } else {
        (chars / 4).max(1)
    }
}

/// Parse YAML-like frontmatter from markdown text.
///
/// Handles folded scalar indicators like `>` and `|-`, and multiline indentation.
pub fn frontmatter(text: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let trimmed = text.trim_start();
    if !trimmed.starts_with("---") {
        return out;
    }

    // Find the end of the frontmatter block
    let after_start = &trimmed[3..];
    let Some(first_newline) = after_start.find('\n') else {
        return out;
    };
    let fm_body = &after_start[first_newline + 1..];

    // Find closing ---
    let mut closing_pos = None;
    let mut search_idx = 0;
    while let Some(pos) = fm_body[search_idx..].find("---") {
        let abs_pos = search_idx + pos;
        // Verify this '---' is at the start of a line
        let is_line_start = abs_pos == 0 || fm_body.as_bytes()[abs_pos - 1] == b'\n';
        if is_line_start {
            closing_pos = Some(abs_pos);
            break;
        }
        search_idx = abs_pos + 3;
    }

    let Some(end_idx) = closing_pos else {
        return out;
    };
    let block = &fm_body[..end_idx];

    let mut current_key: Option<String> = None;
    let mut current_val = String::new();

    for line in block.lines() {
        let is_indented = line.starts_with(' ') || line.starts_with('\t');
        if !is_indented {
            // Check for key: value
            if let Some((k, v)) = line.split_once(':') {
                let k_trimmed = k.trim();
                let valid_key = !k_trimmed.is_empty()
                    && k_trimmed
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
                if valid_key {
                    if let Some(prev_k) = current_key.take() {
                        out.insert(prev_k, sanitize_fm_val(&current_val));
                    }
                    current_key = Some(k_trimmed.to_string());
                    current_val = v.trim().to_string();
                    continue;
                }
            }
        }

        // Indented line continuation
        if current_key.is_some() {
            let l_trimmed = line.trim();
            if !l_trimmed.is_empty() {
                if !current_val.is_empty() {
                    current_val.push(' ');
                }
                current_val.push_str(l_trimmed);
            }
        }
    }

    if let Some(k) = current_key {
        out.insert(k, sanitize_fm_val(&current_val));
    }

    out
}

fn sanitize_fm_val(val: &str) -> String {
    let mut s = val.trim();
    // Strip YAML block indicators like >-, >, |- or |
    if let Some(rest) = s.strip_prefix(">-") {
        s = rest.trim();
    } else if let Some(rest) = s.strip_prefix(">") {
        s = rest.trim();
    } else if let Some(rest) = s.strip_prefix("|-") {
        s = rest.trim();
    } else if let Some(rest) = s.strip_prefix("|") {
        s = rest.trim();
    }

    // Strip surrounding quotes
    if (s.starts_with('"') && s.ends_with('"') && s.len() >= 2)
        || (s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2)
    {
        s = &s[1..s.len() - 1];
    }
    s.trim().to_string()
}

/// Extract license header from a directory.
pub fn license_of<P: AsRef<Path>>(dir: P) -> Option<String> {
    let dir = dir.as_ref();
    for name in LICENSE_NAMES {
        let f = dir.join(name);
        if f.is_file() {
            if let Ok(text) = fs::read_to_string(&f) {
                let lines: Vec<&str> = text
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .take(3)
                    .collect();
                if !lines.is_empty() {
                    let joined = lines.join(" ");
                    let char_count = joined.chars().count();
                    let truncated: String = if char_count > 120 {
                        joined.chars().take(120).collect()
                    } else {
                        joined
                    };
                    return Some(truncated);
                }
            }
        }
    }
    None
}

impl Inventory {
    /// Scan `sources_dir` exactly like `inventory.py`.
    pub fn scan<P: AsRef<Path>>(sources_dir: P) -> Result<Self, CatalogError> {
        let dir = sources_dir.as_ref();
        if !dir.exists() {
            return Ok(Self::default());
        }

        let mut repos = Vec::new();
        let mut entries: Vec<PathBuf> = fs::read_dir(dir)
            .map_err(|e| CatalogError::Io {
                path: dir.to_path_buf(),
                source: e,
            })?
            .filter_map(|r| r.ok().map(|e| e.path()))
            .filter(|p| p.is_dir() && p.join(".git").exists())
            .collect();

        entries.sort();

        for repo_dir in entries {
            let entry = scan_repo(&repo_dir)?;
            repos.push(entry);
        }

        Ok(Self {
            repositories: repos,
        })
    }
}

fn scan_repo(repo_path: &Path) -> Result<RepoInventory, CatalogError> {
    let dir_name = repo_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let repo_id = dir_name.replacen("__", "/", 1);

    // Git commit via git rev-parse HEAD
    let output = Command::new("git")
        .args(["-C", &repo_path.to_string_lossy(), "rev-parse", "HEAD"])
        .output();

    let commit = match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        _ => String::new(),
    };

    let repo_license = license_of(repo_path);

    let markers = RepoMarkers {
        claude_plugin: repo_path.join(".claude-plugin").exists(),
        package_json: repo_path.join("package.json").exists(),
        pyproject: repo_path.join("pyproject.toml").exists(),
        agents_dir: repo_path.join(".agents").exists() || repo_path.join("agents").exists(),
    };

    // Find all SKILL.md files
    let mut skill_files = Vec::new();
    for entry in WalkDir::new(repo_path)
        .follow_links(false)
        .into_iter()
        .flatten()
    {
        if entry.file_type().is_file() {
            let p = entry.path();
            if let Some(file_name) = p.file_name() {
                if file_name.to_string_lossy().eq_ignore_ascii_case("SKILL.md") {
                    // Check if .git is in parts
                    let in_git = p.components().any(|c| c.as_os_str() == ".git");
                    if !in_git {
                        skill_files.push(p.to_path_buf());
                    }
                }
            }
        }
    }

    skill_files.sort();

    let mut skills = Vec::new();

    for md in skill_files {
        let text = fs::read_to_string(&md).unwrap_or_default();
        let fm = frontmatter(&text);

        let sdir = md.parent().unwrap_or(repo_path);
        let sdir_rel = sdir
            .strip_prefix(repo_path)
            .unwrap_or(sdir)
            .to_string_lossy()
            .replace('\\', "/");

        let default_name = sdir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();

        let name = fm.get("name").cloned().unwrap_or(default_name);
        let description = fm.get("description").cloned().unwrap_or_default();
        let desc_tokens = estimate_tokens(description.chars().count());
        let body_tokens = estimate_tokens(text.chars().count());

        // Find scripts and bundle tokens in skill directory
        let mut script_paths = Vec::new();
        let mut bundle_bytes: u64 = 0;

        for f in WalkDir::new(sdir).into_iter().flatten() {
            if f.file_type().is_file() {
                let fp = f.path();
                if let Some(ext) = fp.extension().and_then(|e| e.to_str()) {
                    let ext_lower = ext.to_lowercase();
                    if SCRIPT_EXTENSIONS.contains(&ext_lower.as_str()) {
                        let rel = fp
                            .strip_prefix(sdir)
                            .unwrap_or(fp)
                            .to_string_lossy()
                            .replace('/', "\\"); // inventory.py uses OS format for scripts
                        script_paths.push(rel);
                    }
                    if ext_lower == "md" || ext_lower == "txt" {
                        if let Ok(meta) = fp.metadata() {
                            bundle_bytes += meta.len();
                        }
                    }
                }
            }
        }

        script_paths.sort();
        let script_count = script_paths.len();
        let scripts_sample = script_paths.into_iter().take(20).collect();
        let bundle_tokens = estimate_tokens(bundle_bytes as usize);

        let skill_license = license_of(sdir).or_else(|| fm.get("license").cloned());

        skills.push(SkillInventory {
            path: sdir_rel,
            name,
            description,
            desc_tokens,
            body_tokens,
            bundle_tokens,
            scripts: scripts_sample,
            script_count,
            license: skill_license,
        });
    }

    let desc_tokens_total = skills.iter().map(|s| s.desc_tokens).sum();
    let skill_count = skills.len();

    Ok(RepoInventory {
        repo: repo_id,
        commit,
        license: repo_license,
        markers,
        skill_count,
        desc_tokens_total,
        skills,
    })
}
