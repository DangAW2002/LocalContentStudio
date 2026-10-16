use crate::error::{Result, WorkspaceError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Organization representation.
#[derive(Debug, Clone)]
pub struct Org {
    root: PathBuf,
    name: String,
    is_implicit: bool,
}

/// Metadata stored in `org.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgMeta {
    /// Organization name
    pub name: String,
    /// Schema version
    #[serde(default = "default_schema_version")]
    pub version: u32,
    /// Optional monthly budget
    #[serde(default)]
    pub monthly_budget_usd: Option<f64>,
}

fn default_schema_version() -> u32 {
    1
}

impl Org {
    /// Create layout: `org.yaml` + `shared/{brands,channels,playbooks,skills,tools,capabilities,library}`.
    pub fn create(dir: impl AsRef<Path>, name: impl Into<String>) -> Result<Self> {
        let root = dir.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;

        let name = name.into();
        let meta = OrgMeta {
            name: name.clone(),
            version: 1,
            monthly_budget_usd: None,
        };

        let yaml = serde_yaml::to_string(&meta)
            .map_err(|e| WorkspaceError::Serialization(e.to_string()))?;
        fs::write(root.join("org.yaml"), yaml)?;

        let shared = root.join("shared");
        for sub in &[
            "brands",
            "channels",
            "playbooks",
            "skills",
            "tools",
            "capabilities",
            "library",
        ] {
            fs::create_dir_all(shared.join(sub))?;
        }

        Ok(Self {
            root,
            name,
            is_implicit: false,
        })
    }

    /// Open an existing org directory containing `org.yaml`.
    pub fn open(dir: impl AsRef<Path>) -> Result<Self> {
        let root = dir.as_ref().to_path_buf();
        let org_yaml = root.join("org.yaml");
        if !org_yaml.exists() {
            return Err(WorkspaceError::OrgNotFound(root));
        }

        let content = fs::read_to_string(&org_yaml)?;
        let meta: OrgMeta = serde_yaml::from_str(&content)
            .map_err(|e| WorkspaceError::Serialization(e.to_string()))?;

        Ok(Self {
            root,
            name: meta.name,
            is_implicit: false,
        })
    }

    /// Implicit personal org for workspaces without an `org.yaml` above them.
    pub fn implicit_personal(ws_root: impl AsRef<Path>) -> Self {
        Self {
            root: ws_root.as_ref().to_path_buf(),
            name: "Cá nhân (Mặc định)".into(),
            is_implicit: true,
        }
    }

    /// Find org by walking parent directories looking for `org.yaml`.
    /// If none found, returns an implicit personal org.
    pub fn find_for_workspace(ws_dir: impl AsRef<Path>) -> Self {
        let mut cur = ws_dir.as_ref().to_path_buf();
        // Check parents
        while let Some(parent) = cur.parent() {
            let candidate = parent.join("org.yaml");
            if candidate.exists() {
                if let Ok(org) = Self::open(parent) {
                    return org;
                }
            }
            cur = parent.to_path_buf();
        }

        Self::implicit_personal(ws_dir)
    }

    /// Root directory of the organization.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Organization name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// True if this is an implicit personal org without `org.yaml`.
    pub fn is_implicit(&self) -> bool {
        self.is_implicit
    }

    /// Path to org's `shared/` directory.
    pub fn shared_dir(&self) -> PathBuf {
        self.root.join("shared")
    }
}
