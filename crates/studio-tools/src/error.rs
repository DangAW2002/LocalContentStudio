//! Error types for `studio-tools`.

use std::path::PathBuf;
use thiserror::Error;

/// Result alias for studio-tools operations.
pub type Result<T> = std::result::Result<T, ToolError>;

/// Errors encountered in studio-tools.
#[derive(Debug, Error)]
pub enum ToolError {
    /// Tool manifest or capability parsing error.
    #[error("Lỗi cú pháp tệp cấu hình: {0}")]
    ParseError(String),

    /// I/O error.
    #[error("Lỗi I/O ({path}): {source}")]
    Io {
        /// Affected path
        path: PathBuf,
        /// Underlying error
        #[source]
        source: std::io::Error,
    },

    /// Tool not found in registry.
    #[error("Không tìm thấy công cụ '{0}' trong hệ thống")]
    ToolNotFound(String),

    /// Capability not found.
    #[error("Không tìm thấy năng lực '{0}' trong hệ thống")]
    CapabilityNotFound(String),

    /// No provider available for capability.
    #[error("Không tìm thấy nhà cung cấp khả dụng cho năng lực '{0}'")]
    NoProviderAvailable(String),

    /// Secret store error.
    #[error("Lỗi lưu trữ khóa bảo mật: {0}")]
    SecretStoreError(String),

    /// Missing dependency or probe failure.
    #[error("Công cụ '{tool_id}' thiếu phụ thuộc bắt buộc: {missing:?}")]
    MissingDependency {
        /// Tool ID
        tool_id: String,
        /// List of missing items
        missing: Vec<String>,
    },

    /// Job execution failure.
    #[error("Lỗi thực thi công cụ '{tool_id}': {reason}")]
    ExecutionFailed {
        /// Tool ID
        tool_id: String,
        /// Reason description
        reason: String,
    },

    /// Job scheduler error.
    #[error("Lỗi điều phối tiến trình: {0}")]
    JobSchedulerError(String),

    /// Database error.
    #[error("Lỗi cơ sở dữ liệu: {0}")]
    DatabaseError(#[from] rusqlite::Error),

    /// Workspace error.
    #[error("Lỗi không gian làm việc: {0}")]
    WorkspaceError(#[from] studio_workspace::WorkspaceError),

    /// Setup error.
    #[error("Lỗi cài đặt agy: {0}")]
    SetupError(String),

    /// Install error.
    #[error("Lỗi cài đặt công cụ: {0}")]
    InstallError(String),
}
