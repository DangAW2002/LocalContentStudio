//! Studio MCP server: the single tool gateway for agent engines (plan 0009 §5, ADR-008, ADR-009).

pub mod permission;
pub mod runtime;
pub mod server;
pub mod shim;
pub mod tokens;
pub mod tools;

pub use permission::{BoxFuture, Decision, DefaultBroker, PermissionBroker};
pub use runtime::RuntimeInfo;
pub use server::{start, RunningServer, ServerState};
pub use tokens::{EngineKind, TokenRegistry, TurnContext};
