//! studio-jobs — Job scheduler, resource pools, Windows Job Objects, process supervision, Worker trait.
//!
//! See AGENTS.md and ADR-009 for architectural context.

pub mod command;
pub mod error;
pub mod estimate;
pub mod event;
pub mod handle;
pub mod job;
pub mod probe;
pub mod resource;
pub mod scheduler;
pub mod warm;
pub mod worker;

#[cfg(windows)]
pub mod windows_job;

pub use command::CommandSpec;
pub use error::JobError;
pub use estimate::{EstimateStore, MemoryEstimateStore};
pub use event::{FailedReason, JobEvent};
pub use handle::JobHandle;
pub use job::{JobId, JobSpec, JobStatus, JobWaitReason, Lifecycle, Priority};
pub use probe::{FakeProbe, SysinfoProbe, SystemProbe};
pub use resource::{CpuClass, ResourceSpec, SchedulerConfig};
pub use scheduler::Scheduler;
pub use warm::WarmPool;
pub use worker::{LocalWorker, Worker};
