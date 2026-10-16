use crate::app::AppPaths;
use crate::error::{Result, WorkspaceError};
use crate::org::Org;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};

/// Workspace metadata in `studio.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMeta {
    /// Workspace display name
    pub name: String,
    /// Schema version
    #[serde(default = "default_ws_version")]
    pub version: u32,
    /// Default brand slug
    #[serde(default)]
    pub default_brand: Option<String>,
    /// Monthly budget in USD
    #[serde(default)]
    pub monthly_budget_usd: Option<f64>,
}

fn default_ws_version() -> u32 {
    1
}

/// Project metadata in `project.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMeta {
    /// Unique slug of project
    pub slug: String,
    /// Project title
    pub title: String,
    /// Created timestamp in RFC3339 format
    pub created_at: String,
    /// Catalog modules enabled for this project (kit + additions), see ADR-010.
    #[serde(default)]
    pub kit: Option<String>,
    /// Enabled catalog module ids.
    #[serde(default)]
    pub modules: Vec<String>,
}

/// Project handle.
#[derive(Debug, Clone)]
pub struct Project {
    slug: String,
    title: String,
    root: PathBuf,
}

impl Project {
    /// Create layout: `projects/<slug>/{project.yaml, artifacts/, renders/}`.
    pub fn create(ws: &Workspace, slug: &str, title: &str) -> Result<Self> {
        let proj_root = ws.root().join("projects").join(slug);
        if proj_root.exists() {
            return Err(WorkspaceError::ProjectAlreadyExists(slug.to_string()));
        }

        fs::create_dir_all(proj_root.join("artifacts"))?;
        fs::create_dir_all(proj_root.join("renders"))?;

        let meta = ProjectMeta {
            slug: slug.to_string(),
            title: title.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            kit: None,
            modules: Vec::new(),
        };

        let yaml = serde_yaml::to_string(&meta)
            .map_err(|e| WorkspaceError::Serialization(e.to_string()))?;
        fs::write(proj_root.join("project.yaml"), yaml)?;

        Ok(Self {
            slug: slug.to_string(),
            title: title.to_string(),
            root: proj_root,
        })
    }

    /// Open existing project in workspace.
    pub fn open(ws: &Workspace, slug: &str) -> Result<Self> {
        let proj_root = ws.root().join("projects").join(slug);
        let meta_file = proj_root.join("project.yaml");
        if !meta_file.exists() {
            return Err(WorkspaceError::ProjectNotFound(slug.to_string()));
        }

        let content = fs::read_to_string(&meta_file)?;
        let meta: ProjectMeta = serde_yaml::from_str(&content)
            .map_err(|e| WorkspaceError::Serialization(e.to_string()))?;

        Ok(Self {
            slug: meta.slug,
            title: meta.title,
            root: proj_root,
        })
    }

    /// Read the project's metadata file.
    pub fn meta(&self) -> Result<ProjectMeta> {
        let content = fs::read_to_string(self.root.join("project.yaml"))?;
        serde_yaml::from_str(&content).map_err(|e| WorkspaceError::Serialization(e.to_string()))
    }

    /// Set the kit and enabled catalog modules (deduplicated, order kept).
    pub fn set_modules(&self, kit: Option<String>, modules: &[String]) -> Result<ProjectMeta> {
        let mut meta = self.meta()?;
        meta.kit = kit;
        meta.modules.clear();
        for m in modules {
            if !meta.modules.contains(m) {
                meta.modules.push(m.clone());
            }
        }
        let yaml = serde_yaml::to_string(&meta)
            .map_err(|e| WorkspaceError::Serialization(e.to_string()))?;
        fs::write(self.root.join("project.yaml"), yaml)?;
        Ok(meta)
    }

    /// Project slug.
    pub fn slug(&self) -> &str {
        &self.slug
    }

    /// Project title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Project root path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Artifacts directory.
    pub fn artifacts_dir(&self) -> PathBuf {
        self.root.join("artifacts")
    }

    /// Renders directory.
    pub fn renders_dir(&self) -> PathBuf {
        self.root.join("renders")
    }
}

/// Workspace representation.
#[derive(Debug, Clone)]
pub struct Workspace {
    root: PathBuf,
    name: String,
    org: Org,
}

impl Workspace {
    /// Create layout: `studio.yaml`, `brands/`, `channels/`, `playbooks/`, `skills/`, `tools/`,
    /// `capabilities/`, `library/`, `projects/`, `.studio/{blobs,cache,backups}`.
    pub fn create(dir: impl AsRef<Path>, name: impl Into<String>) -> Result<Self> {
        let root = dir.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;

        let name = name.into();
        let meta = WorkspaceMeta {
            name: name.clone(),
            version: 1,
            default_brand: None,
            monthly_budget_usd: None,
        };

        let yaml = serde_yaml::to_string(&meta)
            .map_err(|e| WorkspaceError::Serialization(e.to_string()))?;
        fs::write(root.join("studio.yaml"), yaml)?;

        for dir_name in &[
            "brands",
            "channels",
            "playbooks",
            "skills",
            "tools",
            "capabilities",
            "library",
            "projects",
            ".studio/blobs",
            ".studio/cache",
            ".studio/backups",
        ] {
            fs::create_dir_all(root.join(dir_name))?;
        }

        let org = Org::find_for_workspace(&root);

        // Also record in AppDb if possible
        let app_paths = AppPaths::from_env();
        if let Ok(app_db) = app_paths.open_app_db() {
            let canon = fs::canonicalize(&root).unwrap_or_else(|_| root.clone());
            let _ = app_db.record_workspace_open(&canon.to_string_lossy());
        }

        Ok(Self { root, name, org })
    }

    /// Open an existing workspace directory containing `studio.yaml`.
    pub fn open(dir: impl AsRef<Path>) -> Result<Self> {
        let root = dir.as_ref().to_path_buf();
        let studio_yaml = root.join("studio.yaml");
        if !studio_yaml.exists() {
            return Err(WorkspaceError::WorkspaceNotFound(root));
        }

        let content = fs::read_to_string(&studio_yaml)?;
        let meta: WorkspaceMeta = serde_yaml::from_str(&content)
            .map_err(|e| WorkspaceError::Serialization(e.to_string()))?;

        // Ensure .studio layout exists
        for dir_name in &[".studio/blobs", ".studio/cache", ".studio/backups"] {
            let _ = fs::create_dir_all(root.join(dir_name));
        }

        let org = Org::find_for_workspace(&root);

        // Record in AppDb
        let app_paths = AppPaths::from_env();
        if let Ok(app_db) = app_paths.open_app_db() {
            let canon = fs::canonicalize(&root).unwrap_or_else(|_| root.clone());
            let _ = app_db.record_workspace_open(&canon.to_string_lossy());
        }

        Ok(Self {
            root,
            name: meta.name,
            org,
        })
    }

    /// Workspace root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Workspace name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Organization owning or encompassing this workspace.
    pub fn org(&self) -> &Org {
        &self.org
    }

    /// `.studio` directory.
    pub fn studio_dir(&self) -> PathBuf {
        self.root.join(".studio")
    }

    /// Path to workspace SQLite database (`.studio/studio.db`).
    pub fn db_path(&self) -> PathBuf {
        self.studio_dir().join("studio.db")
    }

    /// Path to `.studio/blobs/`.
    pub fn blobs_dir(&self) -> PathBuf {
        self.studio_dir().join("blobs")
    }

    /// Path to `.studio/cache/`.
    pub fn cache_dir(&self) -> PathBuf {
        self.studio_dir().join("cache")
    }

    /// Path to `.studio/backups/`.
    pub fn backups_dir(&self) -> PathBuf {
        self.studio_dir().join("backups")
    }

    /// Scopes relative path to workspace root.
    /// Rejects absolute paths, `..` escapes, and symlinks/junctions resolving outside the workspace.
    pub fn scoped_path(&self, rel: impl AsRef<Path>) -> Result<PathBuf> {
        let rel_path = rel.as_ref();

        // 1. Rejects absolute paths
        if rel_path.is_absolute() {
            return Err(WorkspaceError::PathEscape {
                path: rel_path.to_path_buf(),
                root: self.root.clone(),
            });
        }

        // 2. Rejects `..`, and root/prefix components (`\foo`, `C:foo` are not `is_absolute()` on Windows)
        for c in rel_path.components() {
            if matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            ) {
                return Err(WorkspaceError::PathEscape {
                    path: rel_path.to_path_buf(),
                    root: self.root.clone(),
                });
            }
        }

        let full_path = self.root.join(rel_path);

        // Canonicalize ws root for prefix checking
        let canonical_ws = fs::canonicalize(&self.root).unwrap_or_else(|_| self.root.clone());

        // 3. If file or parent exists, test canonical link resolution
        if full_path.exists() {
            let canonical_target = fs::canonicalize(&full_path)?;
            if !canonical_target.starts_with(&canonical_ws) {
                return Err(WorkspaceError::SymlinkEscape { path: full_path });
            }
        } else if let Some(ancestor) = full_path.ancestors().skip(1).find(|a| a.exists()) {
            // Nearest existing ancestor: a junction in any intermediate directory must not escape.
            let canonical_ancestor = fs::canonicalize(ancestor)?;
            if !canonical_ancestor.starts_with(&canonical_ws) {
                return Err(WorkspaceError::SymlinkEscape { path: full_path });
            }
        }

        Ok(full_path)
    }

    /// Checks if a path is writable by workspace agent.
    /// Returns false for org `shared/`, app layer, or anything outside the workspace.
    pub fn can_write(&self, path: impl AsRef<Path>) -> bool {
        let p = path.as_ref();

        // If path is within org shared/ -> false
        let org_shared = self.org.shared_dir();
        if let (Ok(cp), Ok(cs)) = (fs::canonicalize(p), fs::canonicalize(&org_shared)) {
            if cp.starts_with(&cs) {
                return false;
            }
        } else if p.starts_with(&org_shared) {
            return false;
        }

        // If path is within app data dir -> false
        let app_paths = AppPaths::from_env();
        let app_dir = app_paths.data_dir();
        if let (Ok(cp), Ok(ca)) = (fs::canonicalize(p), fs::canonicalize(app_dir)) {
            if cp.starts_with(&ca) {
                return false;
            }
        } else if p.starts_with(app_dir) {
            return false;
        }

        // Path must be within workspace
        let canonical_ws = fs::canonicalize(&self.root).unwrap_or_else(|_| self.root.clone());
        if let Ok(cp) = fs::canonicalize(p) {
            cp.starts_with(&canonical_ws)
        } else {
            p.starts_with(&self.root)
        }
    }

    /// Create a project in this workspace.
    pub fn create_project(&self, slug: &str, title: &str) -> Result<Project> {
        Project::create(self, slug, title)
    }

    /// List all projects in this workspace.
    pub fn list_projects(&self) -> Result<Vec<Project>> {
        let projects_dir = self.root.join("projects");
        if !projects_dir.exists() {
            return Ok(Vec::new());
        }

        let mut list = Vec::new();
        for entry in fs::read_dir(projects_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() && path.join("project.yaml").exists() {
                let slug = entry.file_name().to_string_lossy().to_string();
                if let Ok(p) = Project::open(self, &slug) {
                    list.push(p);
                }
            }
        }
        Ok(list)
    }
}
