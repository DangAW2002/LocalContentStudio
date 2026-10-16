use thiserror::Error;

/// Error types for studio-brand operations.
#[derive(Debug, Error)]
pub enum BrandError {
    /// IO error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// YAML serialization or deserialization error
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// JSON serialization or deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Image decoding or header read error
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),

    /// Brand not found
    #[error("Brand '{0}' not found")]
    BrandNotFound(String),

    /// Channel not found
    #[error("Channel '{0}' not found")]
    ChannelNotFound(String),

    /// Playbook parsing error
    #[error("Playbook parsing error: {0}")]
    PlaybookParse(String),

    /// Invalid format or schema error
    #[error("Validation error: {0}")]
    Validation(String),
}

/// Convenience Result alias.
pub type Result<T> = std::result::Result<T, BrandError>;
