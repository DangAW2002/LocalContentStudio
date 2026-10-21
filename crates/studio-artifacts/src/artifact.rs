use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use studio_workspace::Project;

use crate::error::{ArtifactError, Result};
use crate::kind::KindRegistry;
use crate::starter::*;

/// Edge role in the content graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InputRole {
    /// Direct dependency (affects staleness, propagates downstream).
    #[default]
    Uses,
    /// Reference for consistency (character, style, voice). Does not propagate staleness.
    Reference,
    /// Variant of another artifact (e.g. 9:16 cut, different language). Propagates staleness.
    VariantOf,
}

impl InputRole {
    /// String representation matching YAML/SQLite.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Uses => "uses",
            Self::Reference => "reference",
            Self::VariantOf => "variant_of",
        }
    }

    /// Whether staleness propagates downstream along this role.
    pub fn propagates_staleness(&self) -> bool {
        match self {
            Self::Uses | Self::VariantOf => true,
            Self::Reference => false,
        }
    }
}

impl fmt::Display for InputRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// An entry in `artifact.inputs`.
/// Can be deserialized from either a plain string (role = `uses`) or `{ ref: "...", role: "..." }`.
/// Re-serializes back to plain string if created from string or role is uses with `was_string == true`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactInput {
    /// Reference string, e.g. "artifact:script-01" or "asset:lib/abc123" or a path.
    pub reference: String,
    /// Role in the dependency graph.
    pub role: InputRole,
    /// Internal flag to preserve verbatim string representation on serialization.
    pub was_string: bool,
}

impl ArtifactInput {
    /// Create a new input reference with a specific role.
    pub fn new(reference: impl Into<String>, role: InputRole) -> Self {
        Self {
            reference: reference.into(),
            role,
            was_string: false,
        }
    }

    /// Create a new input reference with default `uses` role (as a plain string).
    pub fn from_str_ref(reference: impl Into<String>) -> Self {
        Self {
            reference: reference.into(),
            role: InputRole::Uses,
            was_string: true,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct ObjectInputHelper {
    #[serde(rename = "ref")]
    reference: String,
    #[serde(default)]
    role: InputRole,
}

impl Serialize for ArtifactInput {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.was_string && self.role == InputRole::Uses {
            serializer.serialize_str(&self.reference)
        } else {
            let helper = ObjectInputHelper {
                reference: self.reference.clone(),
                role: self.role,
            };
            helper.serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for ArtifactInput {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum StringOrObject {
            String(String),
            Object {
                #[serde(rename = "ref")]
                reference: String,
                #[serde(default)]
                role: Option<InputRole>,
            },
        }

        match StringOrObject::deserialize(deserializer)? {
            StringOrObject::String(s) => Ok(ArtifactInput {
                reference: s,
                role: InputRole::Uses,
                was_string: true,
            }),
            StringOrObject::Object { reference, role } => Ok(ArtifactInput {
                reference,
                role: role.unwrap_or(InputRole::Uses),
                was_string: false,
            }),
        }
    }
}

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
    /// Input references: asset paths or artifact ids (e.g. "asset:lib/abc123", "artifact:script-01") or objects with role.
    #[serde(default)]
    pub inputs: Vec<ArtifactInput>,
    /// Additional parameters or configuration.
    #[serde(default)]
    pub params: serde_json::Value,
    /// Artboard width in pixels (same as `params.width`; agents often write it at the top level).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// Artboard height in pixels (same as `params.height`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// Pinned flag: user locked, agent must not rewrite or regenerate.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pinned: bool,
    /// User notes attached to this artifact.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
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
                fs::write(art_dir.join(&default_source), TIMELINE_STARTER)?;
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
            "reference" => {
                fs::write(art_dir.join(&default_source), REFERENCE_STARTER_MD)?;
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
            width: None,
            height: None,
            pinned: false,
            notes: Vec::new(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_level_size_is_read_and_not_written_when_absent() {
        let a: Artifact = serde_yaml::from_str(
            "id: thumb
kind: design
title: T
source: index.html
width: 1280
height: 720
",
        )
        .unwrap();
        assert_eq!((a.width, a.height), (Some(1280), Some(720)));
        let b: Artifact = serde_yaml::from_str(
            "id: x
kind: design
title: T
source: index.html
",
        )
        .unwrap();
        assert!(!serde_yaml::to_string(&b).unwrap().contains("width"));
    }

    #[test]
    fn backwards_compat_string_inputs_round_trip() {
        let original_yaml = "id: test-01
kind: design
title: Test Title
brand: null
channels: []
source: index.html
renderer: null
inputs:
- artifact:script-01
- asset:lib/bg.png
params: null
";
        let art: Artifact = serde_yaml::from_str(original_yaml).unwrap();
        assert_eq!(art.inputs.len(), 2);
        assert_eq!(art.inputs[0].reference, "artifact:script-01");
        assert_eq!(art.inputs[0].role, InputRole::Uses);
        assert!(art.inputs[0].was_string);
        assert_eq!(art.inputs[1].reference, "asset:lib/bg.png");
        assert_eq!(art.inputs[1].role, InputRole::Uses);

        let reserialized = serde_yaml::to_string(&art).unwrap();
        assert_eq!(reserialized.trim(), original_yaml.trim());
    }

    #[test]
    fn mixed_inputs_deserialize_and_serialize() {
        let yaml_str = "id: test-02
kind: timeline
title: Episode 1
source: timeline.json
inputs:
- artifact:script-01
- ref: artifact:nv-an
  role: reference
- ref: artifact:ep01-thumb
  role: variant_of
pinned: true
notes:
- Giữ nguyên câu mở đầu
";
        let art: Artifact = serde_yaml::from_str(yaml_str).unwrap();
        assert!(art.pinned);
        assert_eq!(art.notes, vec!["Giữ nguyên câu mở đầu".to_string()]);
        assert_eq!(art.inputs.len(), 3);
        assert_eq!(art.inputs[0].reference, "artifact:script-01");
        assert_eq!(art.inputs[0].role, InputRole::Uses);
        assert_eq!(art.inputs[1].reference, "artifact:nv-an");
        assert_eq!(art.inputs[1].role, InputRole::Reference);
        assert_eq!(art.inputs[2].reference, "artifact:ep01-thumb");
        assert_eq!(art.inputs[2].role, InputRole::VariantOf);

        let out = serde_yaml::to_string(&art).unwrap();
        assert!(out.contains("- artifact:script-01"));
        assert!(out.contains("ref: artifact:nv-an"));
        assert!(out.contains("role: reference"));
        assert!(out.contains("pinned: true"));
        assert!(out.contains("Giữ nguyên câu mở đầu"));
    }
}
