//! Series management module.
//!
//! Series are groups of projects sharing common reference profiles (characters, visual styles, voice)
//! and a pinned recipe (kit, catalog modules, playbook).
//! Stored in `series/<slug>/series.yaml` and `series/<slug>/references/`.

use crate::checkpoint::CheckpointService;
use crate::error::{Result, WorkspaceError};
use crate::workspace::{Project, Workspace};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Pinned recipe for a series (kit, modules, playbook).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct SeriesRecipe {
    /// Suggested kit id (e.g. "youtube-pipeline")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kit: Option<String>,
    /// Enabled catalog modules
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modules: Vec<String>,
    /// Pinned playbook id (e.g. "youtube-video")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playbook: Option<String>,
    /// Preserved unknown YAML fields
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

/// Metadata stored in `series/<slug>/series.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SeriesMeta {
    /// Slug identifier (e.g. "lich-su-viet-nam")
    pub slug: String,
    /// Display title
    pub title: String,
    /// Detailed description
    #[serde(default)]
    pub description: String,
    /// Pinned recipe
    #[serde(default)]
    pub recipe: SeriesRecipe,
    /// List of reference artifact ids inside `series/<slug>/references/`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<String>,
    /// Preserved unknown YAML fields
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

/// Service for managing series and series references.
pub struct SeriesService;

impl SeriesService {
    /// Validate series slug to ensure it stays within the workspace and is safe.
    pub fn validate_slug(slug: &str) -> Result<()> {
        if slug.is_empty()
            || slug.contains('/')
            || slug.contains('\\')
            || slug.contains("..")
            || slug.contains(':')
        {
            return Err(WorkspaceError::InvalidInput(format!(
                "Slug series không hợp lệ: \"{slug}\""
            )));
        }
        Ok(())
    }

    /// Return path to `series/` folder in workspace.
    pub fn series_dir(ws: &Workspace) -> PathBuf {
        ws.root().join("series")
    }

    /// Return path to `series/<slug>/` in workspace, scoped and validated.
    pub fn series_path(ws: &Workspace, slug: &str) -> Result<PathBuf> {
        Self::validate_slug(slug)?;
        ws.scoped_path(Path::new("series").join(slug))
    }

    /// Return path to `series/<slug>/references/` in workspace.
    pub fn references_dir(ws: &Workspace, slug: &str) -> Result<PathBuf> {
        let p = Self::series_path(ws, slug)?;
        Ok(p.join("references"))
    }

    /// List all series in workspace.
    pub fn list(ws: &Workspace) -> Result<Vec<SeriesMeta>> {
        let root = Self::series_dir(ws);
        if !root.exists() {
            return Ok(Vec::new());
        }

        let mut list = Vec::new();
        for entry in fs::read_dir(&root)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let yaml_path = path.join("series.yaml");
                if yaml_path.is_file() {
                    if let Ok(content) = fs::read_to_string(&yaml_path) {
                        if let Ok(meta) = serde_yaml::from_str::<SeriesMeta>(&content) {
                            list.push(meta);
                        }
                    }
                }
            }
        }
        list.sort_by(|a, b| a.title.cmp(&b.title));
        Ok(list)
    }

    /// Load series metadata by slug.
    pub fn get(ws: &Workspace, slug: &str) -> Result<SeriesMeta> {
        let dir = Self::series_path(ws, slug)?;
        let yaml_path = dir.join("series.yaml");
        if !yaml_path.exists() {
            return Err(WorkspaceError::SeriesNotFound(format!(
                "Không tìm thấy series \"{slug}\""
            )));
        }

        let content = fs::read_to_string(&yaml_path)?;
        let meta: SeriesMeta = serde_yaml::from_str(&content)
            .map_err(|e| WorkspaceError::Serialization(e.to_string()))?;
        Ok(meta)
    }

    /// Create a new series: creates `series/<slug>/series.yaml` and `series/<slug>/references/`.
    /// Checkpoint is created before writing.
    pub fn create(
        ws: &Workspace,
        slug: &str,
        title: &str,
        description: &str,
        recipe: SeriesRecipe,
        references: Vec<String>,
    ) -> Result<SeriesMeta> {
        Self::validate_slug(slug)?;
        let dir = Self::series_path(ws, slug)?;
        if dir.exists() {
            return Err(WorkspaceError::SeriesAlreadyExists(format!(
                "Series \"{slug}\" đã tồn tại"
            )));
        }

        // Checkpoint before writing user content
        let _ = CheckpointService::create(ws, &format!("Tạo series: {}", title), None);

        fs::create_dir_all(&dir)?;
        fs::create_dir_all(dir.join("references"))?;

        let meta = SeriesMeta {
            slug: slug.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            recipe,
            references,
            extra: BTreeMap::new(),
        };

        let yaml = serde_yaml::to_string(&meta)
            .map_err(|e| WorkspaceError::Serialization(e.to_string()))?;
        fs::write(dir.join("series.yaml"), yaml)?;

        Ok(meta)
    }

    /// Save updated series metadata back to `series.yaml`.
    /// Checkpoint is created before writing.
    pub fn save(ws: &Workspace, meta: &SeriesMeta) -> Result<()> {
        Self::validate_slug(&meta.slug)?;
        let dir = Self::series_path(ws, &meta.slug)?;
        fs::create_dir_all(&dir)?;

        let _ = CheckpointService::create(ws, &format!("Cập nhật series: {}", meta.title), None);

        let yaml = serde_yaml::to_string(meta)
            .map_err(|e| WorkspaceError::Serialization(e.to_string()))?;
        fs::write(dir.join("series.yaml"), yaml)?;
        Ok(())
    }

    /// Create a new episode (project) from a series.
    /// Sets project's `series`, `kit`, `modules`, `playbook` from the series recipe,
    /// and initializes starter content with reference edges to series profiles.
    pub fn create_episode(
        ws: &Workspace,
        series_slug: &str,
        episode_slug: &str,
        episode_title: &str,
    ) -> Result<Project> {
        let series = Self::get(ws, series_slug)?;

        // Checkpoint before creating episode
        let _ = CheckpointService::create(
            ws,
            &format!("Tạo tập \"{}\" từ series {}", episode_title, series_slug),
            None,
        );

        let project = Project::create(ws, episode_slug, episode_title)?;

        // Update project.yaml with series and recipe
        let mut meta = project.meta()?;
        meta.series = Some(series_slug.to_string());
        meta.kit = series.recipe.kit.clone();
        meta.modules = series.recipe.modules.clone();
        meta.playbook = series.recipe.playbook.clone();

        let yaml = serde_yaml::to_string(&meta)
            .map_err(|e| WorkspaceError::Serialization(e.to_string()))?;
        fs::write(project.root().join("project.yaml"), yaml)?;

        // Create starter artifact with reference edges to all profiles of this series
        let script_dir = project.artifacts_dir().join("script-01");
        fs::create_dir_all(&script_dir)?;

        let starter_md = format!(
            "---\ntitle: {}\nchannel: youtube\n---\n\n# {}\n\nKịch bản cho tập mới thuộc series {}.\n",
            episode_title, episode_title, series.title
        );
        fs::write(script_dir.join("text.md"), starter_md)?;

        let mut inputs_yaml = Vec::new();
        for ref_id in &series.references {
            inputs_yaml.push(format!(
                "  - ref: \"artifact:{ref_id}\"\n    role: reference"
            ));
        }

        let inputs_block = if inputs_yaml.is_empty() {
            "inputs: []\n".to_string()
        } else {
            format!("inputs:\n{}\n", inputs_yaml.join("\n"))
        };

        let art_yaml = format!(
            "id: script-01\nkind: text\ntitle: Kịch bản - {}\nsource: text.md\nchannels:\n  - youtube\n{}",
            episode_title, inputs_block
        );
        fs::write(script_dir.join("artifact.yaml"), art_yaml)?;

        Ok(project)
    }

    /// List all reference artifacts in a series.
    pub fn list_references(ws: &Workspace, series_slug: &str) -> Result<Vec<String>> {
        let refs_dir = Self::references_dir(ws, series_slug)?;
        if !refs_dir.exists() {
            return Ok(Vec::new());
        }

        let mut ids = Vec::new();
        for entry in fs::read_dir(&refs_dir)? {
            let entry = entry?;
            let p = entry.path();
            if p.is_dir() && p.join("artifact.yaml").is_file() {
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    ids.push(name.to_string());
                }
            }
        }
        ids.sort();
        Ok(ids)
    }
}
