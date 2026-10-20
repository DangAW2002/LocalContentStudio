use crate::db::WorkspaceDb;
use crate::error::{Result, WorkspaceError};
use crate::workspace::Workspace;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use walkdir::WalkDir;

/// Artifact metadata from `artifact.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactYaml {
    /// Artifact identifier
    pub id: String,
    /// Artifact kind (design, motion, timeline, text, generation, audio, ...)
    pub kind: String,
    /// Artifact title
    pub title: String,
    /// Brand slug
    #[serde(default)]
    pub brand: Option<String>,
    /// Channels list
    #[serde(default)]
    pub channels: Vec<String>,
    /// Main source file
    pub source: String,
    /// Renderer tool id
    #[serde(default)]
    pub renderer: Option<String>,
    /// Inputs list
    #[serde(default)]
    pub inputs: Vec<String>,
}

/// Asset sidecar metadata from `.meta.yaml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssetSidecarMeta {
    /// Optional asset display title
    #[serde(default)]
    pub title: Option<String>,
    /// Optional description or tags
    #[serde(default)]
    pub description: Option<String>,
    /// Optional MIME type override
    #[serde(default)]
    pub mime: Option<String>,
    /// AI provider (e.g. google.imagen, openai.images)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// AI model used
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Generation prompt
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// Generation cost in USD
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    /// Associated turn ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    /// Associated brand slug
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brand: Option<String>,
    /// Asset source override (generated | imported | uploaded)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Creation ISO timestamp
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

/// Indexer responsible for scanning and rebuilding search/artifact/asset indexes in `studio.db`.
pub struct Indexer;

impl Indexer {
    /// Rebuild index from scratch: scans `projects/*/artifacts/*/artifact.yaml` and `library/**`.
    /// Deleting db and rebuilding gives the same rows.
    pub fn rebuild(ws: &Workspace) -> Result<()> {
        let mut db = WorkspaceDb::open(ws)?;
        let tx = db.conn_mut().transaction()?;

        // Clear existing indexes
        tx.execute("DELETE FROM artifacts_index", [])?;
        tx.execute("DELETE FROM assets_index", [])?;
        tx.execute("DELETE FROM search_fts", [])?;

        // 1. Scan artifacts: projects/*/artifacts/*/artifact.yaml
        let projects_dir = ws.root().join("projects");
        if projects_dir.exists() {
            for proj_entry in fs::read_dir(&projects_dir)? {
                let proj_entry = proj_entry?;
                let proj_path = proj_entry.path();
                if !proj_path.is_dir() {
                    continue;
                }
                let project_slug = proj_entry.file_name().to_string_lossy().to_string();
                let artifacts_dir = proj_path.join("artifacts");
                if !artifacts_dir.exists() {
                    continue;
                }

                for art_entry in fs::read_dir(&artifacts_dir)? {
                    let art_entry = art_entry?;
                    let art_path = art_entry.path();
                    if !art_path.is_dir() {
                        continue;
                    }

                    let yaml_path = art_path.join("artifact.yaml");
                    if yaml_path.exists() {
                        if let Ok(content) = fs::read_to_string(&yaml_path) {
                            if let Ok(art) = serde_yaml::from_str::<ArtifactYaml>(&content) {
                                let channels_json = serde_json::to_string(&art.channels)
                                    .unwrap_or_else(|_| "[]".into());
                                let now = chrono::Utc::now().to_rfc3339();
                                let rel_path = pathdiff(ws.root(), &art_path);

                                tx.execute(
                                    "INSERT INTO artifacts_index (id, project_slug, kind, title, path, brand, channels_json, source_file, renderer, updated_at)
                                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                                    params![
                                        art.id,
                                        project_slug,
                                        art.kind,
                                        art.title,
                                        rel_path,
                                        art.brand,
                                        channels_json,
                                        art.source,
                                        art.renderer,
                                        now,
                                    ],
                                )?;

                                // Read source text for FTS if it's text-based
                                let source_path = art_path.join(&art.source);
                                let source_content = if source_path.exists() {
                                    fs::read_to_string(&source_path).unwrap_or_default()
                                } else {
                                    String::new()
                                };

                                tx.execute_batch(
                                    "CREATE VIRTUAL TABLE IF NOT EXISTS search_fts USING fts5(
                                        target_type, target_id, title, content
                                    );",
                                )?;
                                tx.execute(
                                    "INSERT INTO search_fts (target_type, target_id, title, content) VALUES ('artifact', ?1, ?2, ?3)",
                                    params![art.id, art.title, source_content],
                                )?;
                            }
                        }
                    }
                }
            }
        }

        // 2. Scan asset directories: library/** and projects/*/assets/** (+ .meta.yaml sidecars)
        let mut asset_roots = vec![ws.root().join("library")];
        if projects_dir.exists() {
            if let Ok(rd) = fs::read_dir(&projects_dir) {
                for entry in rd.flatten() {
                    let assets_dir = entry.path().join("assets");
                    if assets_dir.exists() {
                        asset_roots.push(assets_dir);
                    }
                }
            }
        }

        for root_dir in asset_roots {
            if !root_dir.exists() {
                continue;
            }
            for entry in WalkDir::new(&root_dir).into_iter().filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_file() {
                    let file_name = entry.file_name().to_string_lossy();
                    // Skip sidecar files themselves
                    if file_name.ends_with(".meta.yaml") || file_name.starts_with('.') {
                        continue;
                    }

                    let rel_path = pathdiff(ws.root(), path);
                    let id = rel_path.clone();

                    let metadata = fs::metadata(path)?;
                    let size_bytes = metadata.len();

                    // Read sidecar .meta.yaml if exists
                    let sidecar_path = PathBuf::from(format!("{}.meta.yaml", path.display()));
                    let sidecar: AssetSidecarMeta = if sidecar_path.exists() {
                        fs::read_to_string(&sidecar_path)
                            .ok()
                            .and_then(|s| serde_yaml::from_str(&s).ok())
                            .unwrap_or_default()
                    } else {
                        AssetSidecarMeta::default()
                    };

                    let title = sidecar
                        .title
                        .clone()
                        .unwrap_or_else(|| file_name.to_string());
                    let mime = sidecar.mime.clone().unwrap_or_else(|| guess_mime(path));
                    let meta_json = serde_json::to_string(&sidecar).unwrap_or_else(|_| "{}".into());

                    // Compute SHA256
                    let sha256 = hash_file(path)?;
                    let now = chrono::Utc::now().to_rfc3339();
                    let (source, project, brand) = determine_asset_origin(&rel_path, &sidecar);

                    tx.execute(
                        "INSERT INTO assets_index (id, path, name, mime_type, size_bytes, sha256, metadata_json, updated_at, source, project, brand)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                        params![
                            id,
                            rel_path,
                            title,
                            mime,
                            size_bytes,
                            sha256,
                            meta_json,
                            now,
                            source,
                            project,
                            brand,
                        ],
                    )?;

                    let fts_content = sidecar.description.unwrap_or_default();
                    tx.execute_batch(
                        "CREATE VIRTUAL TABLE IF NOT EXISTS search_fts USING fts5(
                            target_type, target_id, title, content
                        );",
                    )?;
                    tx.execute(
                        "INSERT INTO search_fts (target_type, target_id, title, content) VALUES ('asset', ?1, ?2, ?3)",
                        params![id, title, fts_content],
                    )?;
                }
            }
        }

        tx.commit()?;
        Ok(())
    }

    /// Watch workspace files and update the index dynamically.
    /// Returns a channel of raw notify events and keeps watching in the background.
    pub fn watch(ws: &Workspace) -> Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)> {
        let (tx, rx) = channel();
        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            notify::Config::default(),
        )
        .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        watcher
            .watch(ws.root(), RecursiveMode::Recursive)
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        Ok((watcher, rx))
    }

    /// Index or update a single asset in `assets_index`.
    pub fn index_single_asset(ws: &Workspace, rel_path: &str) -> Result<()> {
        let normalized = rel_path.replace('\\', "/");
        let abs_path = ws.root().join(&normalized);
        if !abs_path.is_file() {
            return Ok(());
        }
        let file_name = abs_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if file_name.ends_with(".meta.yaml") || file_name.starts_with('.') {
            return Ok(());
        }

        let metadata = fs::metadata(&abs_path)?;
        let size_bytes = metadata.len();
        let sidecar_path = PathBuf::from(format!("{}.meta.yaml", abs_path.display()));
        let sidecar: AssetSidecarMeta = if sidecar_path.exists() {
            fs::read_to_string(&sidecar_path)
                .ok()
                .and_then(|s| serde_yaml::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            AssetSidecarMeta::default()
        };

        let title = sidecar.title.clone().unwrap_or_else(|| file_name.clone());
        let mime = sidecar
            .mime
            .clone()
            .unwrap_or_else(|| guess_mime(&abs_path));
        let meta_json = serde_json::to_string(&sidecar).unwrap_or_else(|_| "{}".into());
        let sha256 = hash_file(&abs_path)?;
        let now = chrono::Utc::now().to_rfc3339();
        let (source, project, brand) = determine_asset_origin(&normalized, &sidecar);

        let mut db = WorkspaceDb::open(ws)?;
        let tx = db.conn_mut().transaction()?;
        tx.execute(
            "INSERT INTO assets_index (id, path, name, mime_type, size_bytes, sha256, metadata_json, updated_at, source, project, brand)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(id) DO UPDATE SET
                path = excluded.path,
                name = excluded.name,
                mime_type = excluded.mime_type,
                size_bytes = excluded.size_bytes,
                sha256 = excluded.sha256,
                metadata_json = excluded.metadata_json,
                updated_at = excluded.updated_at,
                source = excluded.source,
                project = excluded.project,
                brand = excluded.brand",
            params![
                normalized,
                normalized,
                title,
                mime,
                size_bytes,
                sha256,
                meta_json,
                now,
                source,
                project,
                brand,
            ],
        )?;

        let fts_content = sidecar.description.unwrap_or_default();
        tx.execute_batch(
            "CREATE VIRTUAL TABLE IF NOT EXISTS search_fts USING fts5(
                target_type, target_id, title, content
            );",
        )?;
        tx.execute(
            "DELETE FROM search_fts WHERE target_type = 'asset' AND target_id = ?1",
            params![normalized],
        )?;
        tx.execute(
            "INSERT INTO search_fts (target_type, target_id, title, content) VALUES ('asset', ?1, ?2, ?3)",
            params![normalized, title, fts_content],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Remove a single asset from `assets_index` and `search_fts`.
    pub fn remove_single_asset(ws: &Workspace, rel_path: &str) -> Result<()> {
        let normalized = rel_path.replace('\\', "/");
        let mut db = WorkspaceDb::open(ws)?;
        let tx = db.conn_mut().transaction()?;
        tx.execute(
            "DELETE FROM assets_index WHERE path = ?1 OR id = ?1",
            params![normalized],
        )?;
        tx.execute_batch(
            "CREATE VIRTUAL TABLE IF NOT EXISTS search_fts USING fts5(
                target_type, target_id, title, content
            );",
        )?;
        tx.execute(
            "DELETE FROM search_fts WHERE target_type = 'asset' AND target_id = ?1",
            params![normalized],
        )?;
        tx.commit()?;
        Ok(())
    }
}

/// Helper to determine source, project slug, and brand for an asset based on path and sidecar metadata.
pub fn determine_asset_origin(
    rel_path: &str,
    sidecar: &AssetSidecarMeta,
) -> (String, Option<String>, Option<String>) {
    let normalized = rel_path.replace('\\', "/");
    let mut project = None;
    if let Some(rest) = normalized.strip_prefix("projects/") {
        if let Some((proj, _)) = rest.split_once('/') {
            project = Some(proj.to_string());
        }
    }

    let source = if let Some(ref s) = sidecar.source {
        s.clone()
    } else if normalized.contains("/assets/generated/")
        || sidecar.provider.is_some()
        || sidecar.prompt.is_some()
    {
        "generated".to_string()
    } else if normalized.starts_with("library/imports/") {
        "imported".to_string()
    } else {
        "uploaded".to_string()
    };

    let brand = sidecar.brand.clone();

    (source, project, brand)
}

fn pathdiff(base: &Path, path: &Path) -> String {
    if let Ok(rel) = path.strip_prefix(base) {
        rel.to_string_lossy().replace('\\', "/")
    } else {
        path.to_string_lossy().replace('\\', "/")
    }
}

fn guess_mime(path: &Path) -> String {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "json" => "application/json",
        "yaml" | "yml" => "text/yaml",
        "md" => "text/markdown",
        "txt" => "text/plain",
        _ => "application/octet-stream",
    }
    .to_string()
}

fn hash_file(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(hex::encode(hasher.finalize()))
}
