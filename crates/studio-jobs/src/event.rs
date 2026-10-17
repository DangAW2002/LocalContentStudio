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

/// Dynamic events yielded during execution by a `JobHandle`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Progress update during job execution.
    Progress {
        /// Normalized progress between 0.0 and 1.0.
        progress: Option<f32>,
        /// Optional descriptive message.
        message: Option<String>,
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

/// Type alias for raw execution events.
pub type TaskEvent = JobEvent;

/// State of a job in the scheduler lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    /// Job is waiting in admission queue.
    Queued,
    /// Job passed admission criteria and is being dispatched.
    Admitted,
    /// Job process is currently executing.
    Running,
    /// Job finished execution with success (exit code 0).
    Succeeded,
    /// Job terminated with error, timeout, or OOM.
    Failed,
    /// Job was cancelled before or during execution.
    Cancelled,
}

/// Broadcast event emitted on every job state transition and progress update.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobBroadcastEvent {
    /// Unique identifier of the job.
    pub job_id: String,
    /// Kind or label of the job (e.g. tool_id).
    pub kind: String,
    /// Resource pool category the job belongs to.
    pub pool: String,
    /// Current state in the job lifecycle.
    pub state: JobState,
    /// Progress percentage from 0.0 to 1.0 if reported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<f32>,
    /// Optional status or failure message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Peak resident memory in bytes if observed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peak_memory_bytes: Option<u64>,
}
