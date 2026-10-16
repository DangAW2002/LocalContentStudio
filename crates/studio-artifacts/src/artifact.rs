use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use studio_workspace::Project;

use crate::error::{ArtifactError, Result};
use crate::kind::KindRegistry;
use crate::starter::*;

/// Artifact model matching `artifact.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Artifact {
    /// Unique artifact id within project, e.g. "thumb-01".
    pub id: String,
    /// Artifact kind: "design", "motion", "timeline", "text", "generation", "audio", etc.
    pub kind: String,
    /// Human-readable title.
    pub title: String,
    /// Optional brand slug, e.g. "main".
    #[serde(default)]
    pub brand: Option<String>,
    /// Channels spec for target platforms, e.g. ["youtube", "tiktok"].
    #[serde(default)]
    pub channels: Vec<String>,
    /// Main source file name, e.g. "index.html", "timeline.json".
    pub source: String,
    /// Renderer tool id, e.g. "render.html_png@1".
    #[serde(default)]
    pub renderer: Option<String>,
    /// Input references: asset paths or artifact ids (e.g. "asset:lib/abc123", "artifact:script-01").
    #[serde(default)]
    pub inputs: Vec<String>,
    /// Additional parameters or configuration.
    #[serde(default)]
    pub params: serde_json::Value,
}

impl Artifact {
    /// Create a new artifact directory and `artifact.yaml` inside `project.artifacts_dir() / id`,
    /// including starter source files tailored to the chosen kind.
    pub fn create(
        project: &Project,
        id: impl Into<String>,
        kind: impl Into<String>,
        title: impl Into<String>,
        brand: Option<String>,
        channels: Vec<String>,
        params: Option<serde_json::Value>,
    ) -> Result<Self> {
        let id = id.into();
        let kind = kind.into();
        let title = title.into();

        let art_dir = project.artifacts_dir().join(&id);
        if art_dir.exists() {
            return Err(ArtifactError::AlreadyExists(format!(
                "Artifact directory already exists: {}",
                art_dir.display()
            )));
        }

        fs::create_dir_all(&art_dir)?;

        let spec = KindRegistry::get(&kind);
        let default_source = spec
            .as_ref()
            .map(|s| s.default_source.clone())
            .unwrap_or_else(|| "source.txt".into());
        let default_renderer = spec.as_ref().and_then(|s| s.renderer.clone());

        // Write starter files based on kind
        match kind.as_str() {
            "design" => {
                fs::write(art_dir.join(&default_source), DESIGN_STARTER_HTML)?;
                fs::write(art_dir.join("brand.css"), DEFAULT_BRAND_CSS)?;
            }
            "motion" => {
                fs::write(art_dir.join(&default_source), MOTION_STARTER_HTML)?;
            }
            "timeline" => {
                fs::write(art_dir.join(&default_source), TIMELINE_STARTER_JSON)?;
            }
            "text" => {
                fs::write(art_dir.join(&default_source), TEXT_STARTER_MD)?;
            }
            "generation" => {
                fs::write(art_dir.join(&default_source), GENERATION_STARTER_JSON)?;
            }
            "audio" => {
                fs::write(art_dir.join(&default_source), AUDIO_STARTER_MD)?;
            }
            _ => {
                // generic fallback
                fs::write(art_dir.join(&default_source), "")?;
            }
        }

        let artifact = Self {
            id: id.clone(),
            kind,
            title,
            brand,
            channels,
            source: default_source,
            renderer: default_renderer,
            inputs: Vec::new(),
            params: params.unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
        };

        let yaml = serde_yaml::to_string(&artifact)?;
        fs::write(art_dir.join("artifact.yaml"), yaml)?;

        Ok(artifact)
    }

    /// Load an artifact from its directory inside the given project.
    pub fn load(project: &Project, id: &str) -> Result<Self> {
        let art_dir = project.artifacts_dir().join(id);
        Self::load_from_dir(&art_dir)
    }

    /// Load an artifact from a specific directory path containing `artifact.yaml`.
    pub fn load_from_dir(dir: &Path) -> Result<Self> {
        let yaml_file = dir.join("artifact.yaml");
        if !yaml_file.exists() {
            return Err(ArtifactError::NotFound(format!(
                "artifact.yaml not found in {}",
                dir.display()
            )));
        }

        let content = fs::read_to_string(&yaml_file)?;
        let artifact: Artifact = serde_yaml::from_str(&content)?;
        Ok(artifact)
    }

    /// Save current artifact metadata back to `artifact.yaml`.
    pub fn save(&self, project: &Project) -> Result<()> {
        let art_dir = project.artifacts_dir().join(&self.id);
        fs::create_dir_all(&art_dir)?;
        let yaml = serde_yaml::to_string(self)?;
        fs::write(art_dir.join("artifact.yaml"), yaml)?;
        Ok(())
    }

    /// List all artifacts in a project by scanning `project.artifacts_dir()`.
    pub fn list(project: &Project) -> Result<Vec<Self>> {
        let artifacts_dir = project.artifacts_dir();
        if !artifacts_dir.exists() {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        for entry in fs::read_dir(artifacts_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() && path.join("artifact.yaml").exists() {
                if let Ok(art) = Self::load_from_dir(&path) {
                    results.push(art);
                }
            }
        }

        // Sort by id for deterministic order
        results.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(results)
    }

    /// Return absolute path to the artifact directory within the project.
    pub fn dir(&self, project: &Project) -> PathBuf {
        project.artifacts_dir().join(&self.id)
    }

    /// Return absolute path to the main source file within the project.
    pub fn source_path(&self, project: &Project) -> PathBuf {
        self.dir(project).join(&self.source)
    }
}
