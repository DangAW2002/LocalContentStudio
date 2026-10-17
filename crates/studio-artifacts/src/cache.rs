use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

use crate::artifact::Artifact;
use crate::error::Result;
use studio_workspace::{Project, Workspace};

/// Compute the deterministic render hash for an artifact.
///
/// Input hash = sha256(
///   source tree files (rel_path + content_sha256) +
///   resolved input asset hashes +
///   brand version string +
///   renderer id/version +
///   params
/// )
pub fn compute_input_hash(
    workspace: &Workspace,
    project: &Project,
    artifact: &Artifact,
    brand_version: &str,
    renderer_id_version: &str,
) -> Result<String> {
    let mut hasher = Sha256::new();

    // 1. Source tree files inside artifact directory
    let art_dir = artifact.dir(project);
    let mut file_entries: BTreeMap<String, String> = BTreeMap::new();

    if art_dir.exists() {
        for entry in WalkDir::new(&art_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                // Compute relative path
                if let Ok(rel) = path.strip_prefix(&art_dir) {
                    let rel_str = rel.to_string_lossy().replace('\\', "/");
                    // Read file bytes and hash them
                    let bytes = fs::read(path)?;
                    let mut file_hasher = Sha256::new();
                    file_hasher.update(&bytes);
                    let file_sha = hex::encode(file_hasher.finalize());
                    file_entries.insert(rel_str, file_sha);
                }
            }
        }
    }

    hasher.update(b"SOURCE_FILES:\n");
    for (rel_path, file_sha) in file_entries {
        hasher.update(rel_path.as_bytes());
        hasher.update(b":");
        hasher.update(file_sha.as_bytes());
        hasher.update(b"\n");
    }

    // 2. Resolved input asset hashes
    let mut input_hashes: BTreeMap<String, String> = BTreeMap::new();
    for input_ref in &artifact.inputs {
        let resolved_hash = resolve_input_hash(workspace, project, input_ref)?;
        input_hashes.insert(input_ref.clone(), resolved_hash);
    }

    hasher.update(b"INPUTS:\n");
    for (input_ref, input_sha) in input_hashes {
        hasher.update(input_ref.as_bytes());
        hasher.update(b":");
        hasher.update(input_sha.as_bytes());
        hasher.update(b"\n");
    }

    // 3. Brand version string and resolved brand content
    hasher.update(b"BRAND_VERSION:\n");
    hasher.update(brand_version.as_bytes());
    hasher.update(b"\n");

    let brand_content_hash = resolve_brand_content_hash(workspace, brand_version)?;
    hasher.update(b"BRAND_CONTENT:\n");
    hasher.update(brand_content_hash.as_bytes());
    hasher.update(b"\n");

    // 4. Renderer id / version
    hasher.update(b"RENDERER:\n");
    hasher.update(renderer_id_version.as_bytes());
    hasher.update(b"\n");

    // 5. Params
    hasher.update(b"PARAMS:\n");
    let params_str = serde_json::to_string(&artifact.params)?;
    hasher.update(params_str.as_bytes());
    hasher.update(b"\n");

    let full_hash = hex::encode(hasher.finalize());
    Ok(full_hash)
}

/// Short 12-character prefix of the hash used for directory naming.
pub fn hash12(full_hash: &str) -> String {
    if full_hash.len() > 12 {
        full_hash[..12].to_string()
    } else {
        full_hash.to_string()
    }
}

/// Resolve the content hash of an input reference (e.g. "asset:lib/abc123" or "artifact:thumb-01" or path).
fn resolve_input_hash(workspace: &Workspace, project: &Project, input_ref: &str) -> Result<String> {
    if let Some(asset_path) = input_ref.strip_prefix("asset:") {
        let abs_path = workspace.root().join(asset_path);
        if abs_path.exists() && abs_path.is_file() {
            let bytes = fs::read(&abs_path)?;
            let mut h = Sha256::new();
            h.update(&bytes);
            return Ok(hex::encode(h.finalize()));
        }
    } else if let Some(other_art_id) = input_ref.strip_prefix("artifact:") {
        let other_dir = project.artifacts_dir().join(other_art_id);
        if other_dir.exists() {
            let mut h = Sha256::new();
            for entry in WalkDir::new(&other_dir).into_iter().filter_map(|e| e.ok()) {
                if entry.path().is_file() {
                    let bytes = fs::read(entry.path())?;
                    h.update(&bytes);
                }
            }
            return Ok(hex::encode(h.finalize()));
        }
    }

    // Fallback: check if it's a relative path in workspace or project
    let path_in_proj = project.root().join(input_ref);
    if path_in_proj.exists() && path_in_proj.is_file() {
        let bytes = fs::read(&path_in_proj)?;
        let mut h = Sha256::new();
        h.update(&bytes);
        return Ok(hex::encode(h.finalize()));
    }

    let path_in_ws = workspace.root().join(input_ref);
    if path_in_ws.exists() && path_in_ws.is_file() {
        let bytes = fs::read(&path_in_ws)?;
        let mut h = Sha256::new();
        h.update(&bytes);
        return Ok(hex::encode(h.finalize()));
    }

    // Default hash based on string reference if file does not exist
    let mut h = Sha256::new();
    h.update(input_ref.as_bytes());
    Ok(hex::encode(h.finalize()))
}

/// Resolve the content hash of a brand by slug or version.
/// Hashes the resolved brand.yaml and any referenced logo/font files that exist.
fn resolve_brand_content_hash(workspace: &Workspace, brand_ref: &str) -> Result<String> {
    use studio_workspace::ScopeChain;

    let slug = brand_ref.split('@').next().unwrap_or(brand_ref);
    let scope = ScopeChain::new(workspace);
    let resolved_brands = scope.resolve("brands");

    let brand_dir = resolved_brands
        .into_iter()
        .find(|item| item.id == slug)
        .map(|item| item.path)
        .unwrap_or_else(|| workspace.root().join("brands").join(slug));

    if !brand_dir.exists() || !brand_dir.is_dir() {
        return Ok(String::new());
    }

    let mut hasher = Sha256::new();
    let mut files_to_hash: BTreeMap<String, Vec<u8>> = BTreeMap::new();

    let brand_yaml_path = brand_dir.join("brand.yaml");
    if brand_yaml_path.exists() && brand_yaml_path.is_file() {
        if let Ok(bytes) = fs::read(&brand_yaml_path) {
            files_to_hash.insert("brand.yaml".to_string(), bytes.clone());

            // Parse brand.yaml to find referenced logo and font files
            if let Ok(yaml_val) = serde_yaml::from_slice::<serde_yaml::Value>(&bytes) {
                // Check logos
                if let Some(logos) = yaml_val.get("logos") {
                    for field in &["full", "mark", "light", "dark"] {
                        if let Some(rel) = logos.get(field).and_then(|v| v.as_str()) {
                            let path = brand_dir.join(rel);
                            if path.exists() && path.is_file() {
                                if let Ok(content) = fs::read(&path) {
                                    files_to_hash.insert(rel.to_string(), content);
                                }
                            }
                        }
                    }
                }

                // Check fonts
                if let Some(fonts) = yaml_val.get("fonts") {
                    if let Some(files) = fonts.get("files").and_then(|v| v.as_sequence()) {
                        for item in files {
                            if let Some(rel) = item.as_str() {
                                let path = brand_dir.join(rel);
                                if path.exists() && path.is_file() {
                                    if let Ok(content) = fs::read(&path) {
                                        files_to_hash.insert(rel.to_string(), content);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    for (rel_path, content) in files_to_hash {
        hasher.update(rel_path.as_bytes());
        hasher.update(b":");
        hasher.update(&content);
        hasher.update(b"\n");
    }

    Ok(hex::encode(hasher.finalize()))
}

/// Render cache record in WorkspaceDb `renders` table.
#[derive(Debug, Clone)]
pub struct RenderRecord {
    pub id: String,
    pub artifact_id: String,
    pub hash: String,
    pub output_path: String,
    pub format: String,
    pub size_bytes: i64,
    pub created_at: String,
}

/// Query the WorkspaceDb for cached renders for a given artifact and hash.
pub fn lookup_render_cache(
    workspace: &Workspace,
    artifact_id: &str,
    hash: &str,
) -> Result<Option<Vec<PathBuf>>> {
    let db = studio_workspace::WorkspaceDb::open(workspace)?;
    let mut stmt = db
        .conn()
        .prepare("SELECT output_path FROM renders WHERE artifact_id = ?1 AND hash = ?2")?;

    let paths: std::result::Result<Vec<String>, _> = stmt
        .query_map(rusqlite::params![artifact_id, hash], |row| row.get(0))?
        .collect();

    let paths = paths?;
    if paths.is_empty() {
        return Ok(None);
    }

    let mut existing_files = Vec::new();
    for p_str in paths {
        let p = workspace.root().join(&p_str);
        if p.exists() {
            existing_files.push(p);
        } else {
            // Cache record exists but physical file missing -> treat as cache miss
            return Ok(None);
        }
    }

    if existing_files.is_empty() {
        Ok(None)
    } else {
        Ok(Some(existing_files))
    }
}

/// Record successful render outputs into the `renders` table in WorkspaceDb.
pub fn record_render_outputs(
    workspace: &Workspace,
    artifact_id: &str,
    hash: &str,
    output_files: &[PathBuf],
) -> Result<()> {
    let db = studio_workspace::WorkspaceDb::open(workspace)?;
    let now = chrono::Utc::now().to_rfc3339();

    for file_path in output_files {
        if !file_path.exists() {
            continue;
        }

        let meta = fs::metadata(file_path)?;
        let size_bytes = meta.len() as i64;
        let format = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("unknown")
            .to_lowercase();

        let rel_path = file_path
            .strip_prefix(workspace.root())
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| file_path.to_string_lossy().replace('\\', "/"));

        let render_id = format!("{}-{}-{}", artifact_id, &hash12(hash), format);

        db.conn().execute(
            "INSERT OR REPLACE INTO renders (id, artifact_id, hash, output_path, format, size_bytes, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                render_id,
                artifact_id,
                hash,
                rel_path,
                format,
                size_bytes,
                now
            ],
        )?;
    }

    Ok(())
}
