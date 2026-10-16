//! `studio-tools` — Capability contracts, tool registry, dependency probes,
//! scope-based provider routing, Credential Manager secrets, and supervised execution.

pub mod error;
pub mod manifest;
pub mod probe;
pub mod registry;
pub mod router;
pub mod runner;
pub mod secret;
pub mod setup;

pub use error::{Result, ToolError};
pub use manifest::{
    CapabilitySchema, SideEffect, ToolCost, ToolKind, ToolManifest, ToolRunSpec, ToolRunType,
};
pub use probe::{find_in_path, probe, probe_application, probe_binary, ProbeResult};
pub use registry::{ManifestProblem, Registry};
pub use router::route;
pub use runner::{run_tool, ToolRun, ToolRunContext, MAX_INLINE_RESULT_BYTES};
pub use secret::{format_target_name, CredentialManagerStore, MemoryStore, SecretStore};
pub use setup::{default_agy_settings_path, remove_agy, remove_agy_at, setup_agy, setup_agy_at};
