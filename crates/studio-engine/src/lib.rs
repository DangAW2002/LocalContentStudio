//! `studio-engine` — Agent engine adapters (claude, agy) and normalized event streaming.
//!
//! Provides CLI command builders, stateful stream-json parsers, process supervision,
//! and compatibility probing for Claude Code and Antigravity (agy).

pub mod builder;
pub mod engine;
pub mod parser_agy;
pub mod parser_claude;
pub mod transcript;
pub mod types;

pub use builder::{agy_command, claude_command, CommandSpec};
pub use engine::{
    kill_process_tree, parse_agy_models, AgyEngine, ClaudeEngine, Engine, ModelInfo, Turn,
    SUPPORTED_AGY_VERSION, SUPPORTED_CLAUDE_VERSION,
};
pub use parser_agy::AgyParser;
pub use parser_claude::ClaudeParser;
pub use transcript::Transcript;
pub use types::{
    DoneStatus, EngineError, EngineKind, Event, FileOp, McpHttp, Probe, SessionRef, TurnRequest,
};
