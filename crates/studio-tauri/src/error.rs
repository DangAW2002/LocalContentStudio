//! Command error: serialized to the UI as a plain Vietnamese message.

use serde::Serialize;

/// Error returned by IPC commands.
#[derive(Debug)]
pub struct CmdError(pub String);

/// Result alias for IPC commands.
pub type CmdResult<T> = Result<T, CmdError>;

impl CmdError {
    /// Build an error from a message.
    pub fn msg(m: impl Into<String>) -> Self {
        Self(m.into())
    }
}

impl Serialize for CmdError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}

macro_rules! from_err {
    ($($t:ty),*) => {$(
        impl From<$t> for CmdError {
            fn from(e: $t) -> Self {
                Self(e.to_string())
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
    studio_brand::BrandError,
    studio_tools::ToolError
);
