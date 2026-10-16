//! Job events emitted during execution.

use serde::{Deserialize, Serialize};

/// Terminal failure reason for a job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailedReason {
    /// The process exceeded its allocated memory limit.
    OutOfMemory,
    /// Execution time exceeded the specified timeout.
    Timeout,
    /// Explicitly cancelled by user or caller.
    Cancelled,
    /// Child process failed to spawn.
    SpawnError(String),
    /// Process completed with a non-zero exit code.
    Exit(i32),
}

/// Dynamic events yielded by a `JobHandle`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobEvent {
    /// Job is waiting in admission queue.
    Queued {
        /// 1-based queue position.
        position: usize,
        /// Human-readable explanation in Vietnamese for why the job is waiting.
        reason: String,
    },
    /// Job process has started.
    Started {
        /// Operating system process ID.
        pid: u32,
    },
    /// A single line captured from standard output.
    Stdout(String),
    /// A single line captured from standard error.
    Stderr(String),
    /// Job completed successfully.
    Finished {
        /// Process exit code.
        exit_code: i32,
        /// Peak memory consumption observed in megabytes.
        peak_memory_mb: u64,
        /// Wall-clock duration in milliseconds.
        duration_ms: u64,
    },
    /// Job terminated with a failure.
    Failed {
        /// Reason for failure.
        reason: FailedReason,
    },
}
