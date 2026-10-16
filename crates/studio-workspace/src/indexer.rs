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

        // 2. Scan library: library/** (+ .meta.yaml sidecars)
        let library_dir = ws.root().join("library");
        if library_dir.exists() {
            for entry in WalkDir::new(&library_dir)
                .into_iter()
                .filter_map(|e| e.ok())
            {
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

                    tx.execute(
                        "INSERT INTO assets_index (id, path, name, mime_type, size_bytes, sha256, metadata_json, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                        params![
                            id,
                            rel_path,
                            title,
                            mime,
                            size_bytes,
                            sha256,
                            meta_json,
                            now,
                        ],
                    )?;

                    let fts_content = sidecar.description.unwrap_or_default();
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
