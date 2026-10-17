//! Command error: serialized to the UI as either a structured Vietnamese error or plain message.

use serde::Serialize;
use studio_tools::publish::YouTubeErrorDetail;

/// Error returned by IPC commands.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum CmdError {
    /// Structured publishing / OAuth error with fix recommendation.
    Structured(YouTubeErrorDetail),
    /// Plain message.
    Plain(String),
}

/// Result alias for IPC commands.
pub type CmdResult<T> = Result<T, CmdError>;

impl CmdError {
    /// Build an error from a plain message.
    pub fn msg(m: impl Into<String>) -> Self {
        Self::Plain(m.into())
    }

    /// Build a structured error with remediation.
    #[allow(dead_code)]
    pub fn structured(detail: YouTubeErrorDetail) -> Self {
        Self::Structured(detail)
    }
}

impl std::fmt::Display for CmdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Structured(s) => write!(f, "{s}"),
            Self::Plain(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for CmdError {}

impl From<studio_tools::ToolError> for CmdError {
    fn from(e: studio_tools::ToolError) -> Self {
        match e {
            studio_tools::ToolError::PublishError(detail) => Self::Structured(detail),
            other => Self::Plain(other.to_string()),
        }
    }
}

macro_rules! from_err {
    ($($t:ty),*) => {$(
        impl From<$t> for CmdError {
            fn from(e: $t) -> Self {
                Self::Plain(e.to_string())
            }
        }
    )*};
}

from_err!(
    std::io::Error,
    rusqlite::Error,
    serde_json::Error,
    serde_yaml::Error,
    studio_workspace::WorkspaceError,
    studio_catalog::CatalogError,
    studio_engine::EngineError,
    studio_brand::BrandError
);
