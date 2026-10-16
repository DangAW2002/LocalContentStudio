use thiserror::Error;

/// Error type for studio-artifacts operations.
#[derive(Debug, Error)]
pub enum ArtifactError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML serialization error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("SQLite database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Workspace error: {0}")]
    Workspace(#[from] studio_workspace::WorkspaceError),

    #[error("Artifact already exists: {0}")]
    AlreadyExists(String),

    #[error("Artifact not found: {0}")]
    NotFound(String),

    #[error("Invalid artifact configuration: {0}")]
    InvalidConfig(String),

    #[error("Unsupported artifact kind: {0}")]
    UnsupportedKind(String),

    #[error("Renderer error: {0}")]
    RendererError(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Tool or binary not found: {0}")]
    ToolNotFound(String),

    #[error("Execution failed: {message}")]
    ExecutionFailed {
        message: String,
        details: Option<String>,
    },

    #[error("Render job cancelled or failed: {0}")]
    JobFailed(String),
}

/// Convenience result type for studio-artifacts.
pub type Result<T> = std::result::Result<T, ArtifactError>;
