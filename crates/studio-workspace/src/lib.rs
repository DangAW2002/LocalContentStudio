//! `studio-workspace` crate — manages org/workspace folders, scope chain,
//! path scoping, SQLite (`.studio/studio.db`, `app.db`), migrations, file index,
//! checkpoints (CAS blobs), and disk usage statistics.

pub mod app;
pub mod checkpoint;
pub mod db;
pub mod env_path;
pub mod error;
pub mod indexer;
pub mod org;
pub mod scope;
pub mod series;
pub mod usage;
pub mod workspace;

pub use app::{normalize_workspace_path, AppDb, AppOrgRecord, AppPaths, AppRoot, RecentWorkspace};
pub use checkpoint::{CheckpointDiff, CheckpointId, CheckpointRecord, CheckpointService};
pub use db::WorkspaceDb;
pub use env_path::get_fresh_paths;
pub use error::{Result, WorkspaceError};
pub use indexer::{ArtifactYaml, AssetSidecarMeta, Indexer};
pub use org::{Org, OrgMeta};
pub use scope::{ResolutionLayer, ResolvedItem, ScopeChain};
pub use series::{SeriesMeta, SeriesRecipe, SeriesService};
pub use usage::{disk_usage, DiskUsage};
pub use workspace::{Project, ProjectMeta, Workspace, WorkspaceMeta};
