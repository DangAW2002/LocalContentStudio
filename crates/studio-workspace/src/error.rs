use std::path::PathBuf;
use thiserror::Error;

/// Workspace errors.
#[derive(Debug, Error)]
pub enum WorkspaceError {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// SQLite error
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Scoped path escape error
    #[error("Path escape forbidden: {path} is outside workspace root {root}")]
    PathEscape { path: PathBuf, root: PathBuf },

    /// Symlink or junction target outside workspace
    #[error("Symlink or junction target outside workspace: {path}")]
    SymlinkEscape { path: PathBuf },

    /// Org not found
    #[error("Org not found at {0}")]
    OrgNotFound(PathBuf),

    /// Workspace not found
    #[error("Workspace not found at {0}")]
    WorkspaceNotFound(PathBuf),

    /// Project already exists
    #[error("Project already exists: {0}")]
    ProjectAlreadyExists(String),

    /// Project not found
    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    /// Checkpoint not found
    #[error("Checkpoint not found: {0}")]
    CheckpointNotFound(String),

    /// Turn already undone
    #[error("Lượt này đã được hoàn tác trước đó.")]
    TurnAlreadyUndone,

    /// Generic internal error
    #[error("{0}")]
    Internal(String),
}

/// Result alias for workspace operations.
pub type Result<T> = std::result::Result<T, WorkspaceError>;
