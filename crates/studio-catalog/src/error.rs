//! Error types for studio-catalog.

use std::path::PathBuf;
use thiserror::Error;

/// Catalog error kinds.
#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("I/O error at {path:?}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Git error: {0}")]
    Git(String),

    #[error("Catalog validation failed: {0}")]
    Validation(String),

    #[error("Materialize error: {0}")]
    Materialize(String),
}
