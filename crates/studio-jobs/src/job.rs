//! Job specification, lifecycle, priority, and snapshot status models.

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::command::CommandSpec;
use crate::event::FailedReason;
use crate::resource::ResourceSpec;

/// Unique identifier for a job.
pub type JobId = String;

/// Priority level of a job for admission ordering.
///
/// Order: Interactive (highest) > TurnBlocking > Background (lowest).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Priority {
    /// Background job (e.g. final render, batch exports, cache warmup).
    /// Lower OS priority class and CPU throttling apply.
    Background = 0,
    /// Job that blocks a turn or interactive conversation step.
    TurnBlocking = 1,
    /// User-facing interactive operation requiring immediate feedback (e.g. preview, probe).
    Interactive = 2,
}

/// Lifecycle model for worker execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Lifecycle {
    /// Worker process spawns for a single call and exits immediately.
    #[default]
    PerCall,
    /// Worker process stays warm in memory for reuse until `idle_timeout` elapses.
    Warm {
        /// Inactivity duration before process terminates.
        idle_timeout: Duration,
    },
}

/// Complete specification of a job to be scheduled and run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSpec {
    /// Unique identifier for this job instance.
    pub id: JobId,
    /// Workspace context ID.
    pub workspace_id: String,
    /// Identifier of the tool or engine being invoked (e.g. "tool:ffmpeg", "engine:claude").
    pub tool_id: String,
    /// Command to execute.
    pub command: CommandSpec,
    /// Resource requirements declared by the tool manifest.
    pub resources: ResourceSpec,
    /// Scheduling priority.
    pub priority: Priority,
    /// Wall-clock timeout duration.
    pub timeout: Duration,
    /// Explicit hard memory limit in megabytes (if None, defaults to 1.5× estimated peak).
    pub memory_limit_mb: Option<u64>,
    /// Lifecycle configuration.
    pub lifecycle: Lifecycle,
}

/// Reason why a queued job is waiting for admission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobWaitReason {
    /// Waiting for a slot in the specific resource pool.
    PoolFull(String),
    /// Exclusive resource is currently occupied by another job.
    ExclusiveOccupied(String),
    /// Waiting for per-key network rate limit slot.
    RateLimit(String),
    /// Insufficient available host RAM above headroom.
    InsufficientRam { needed_mb: u64, available_mb: u64 },
    /// Insufficient available host GPU VRAM above headroom.
    InsufficientVram { needed_mb: u64, available_mb: u64 },
}

impl JobWaitReason {
    /// Format as human-readable Vietnamese explanation for UI display.
    pub fn to_vietnamese(&self) -> String {
        match self {
            Self::PoolFull(pool) => format!("Đang chờ vị trí trống trong hàng đợi {}", pool),
            Self::ExclusiveOccupied(name) => format!("Tài nguyên độc quyền '{}' đang bận", name),
            Self::RateLimit(key) => format!("Đang chờ giới hạn tốc độ mạng ({})", key),
            Self::InsufficientRam {
                needed_mb,
                available_mb,
            } => {
                format!(
                    "Chờ bộ nhớ RAM (cần {} MB, hiện có {} MB sau khi trừ dự phòng)",
                    needed_mb, available_mb
                )
            }
            Self::InsufficientVram {
                needed_mb,
                available_mb,
            } => {
                format!(
                    "Chờ bộ nhớ VRAM GPU (cần {} MB, hiện có {} MB sau khi trừ dự phòng)",
                    needed_mb, available_mb
                )
            }
        }
    }
}

/// High-level status snapshot of a job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    /// Job is waiting in queue.
    Queued {
        /// Job ID.
        id: JobId,
        /// 1-based position in queue.
        position: usize,
        /// Vietnamese human-readable explanation of why the job is waiting.
        reason: String,
        /// Priority of the queued job.
        priority: Priority,
    },
    /// Job is actively executing.
    Running {
        /// Job ID.
        id: JobId,
        /// OS Process ID.
        pid: u32,
        /// Wall-clock running duration in milliseconds so far.
        elapsed_ms: u64,
    },
    /// Job completed successfully.
    Finished {
        /// Job ID.
        id: JobId,
        /// Process exit code.
        exit_code: i32,
        /// Peak memory consumption observed in megabytes.
        peak_memory_mb: u64,
        /// Total duration in milliseconds.
        duration_ms: u64,
    },
    /// Job failed.
    Failed {
        /// Job ID.
        id: JobId,
        /// Failure reason.
        reason: FailedReason,
    },
}
