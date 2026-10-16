//! Error types for studio-jobs.

use thiserror::Error;

/// Error type for studio-jobs operations.
#[derive(Debug, Error)]
pub enum JobError {
    #[error("Job '{0}' not found")]
    NotFound(String),

    #[error("Job '{0}' is already running or completed")]
    AlreadyActive(String),

    #[error("Job cancelled")]
    Cancelled,

    #[error("Timed out after {0:?}")]
    Timeout(std::time::Duration),

    #[error("Out of memory limit exceeded ({0} MB)")]
    OutOfMemory(u64),

    #[error("Spawn error: {0}")]
    SpawnError(String),

    #[error("Process exited with code: {0}")]
    Exit(i32),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Worker error: {0}")]
    Worker(String),
}
