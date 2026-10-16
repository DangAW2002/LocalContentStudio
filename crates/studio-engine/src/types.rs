//! Core domain types and events for engine adapters.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use uuid::Uuid;

/// Target agent engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EngineKind {
    /// Claude Code CLI engine.
    Claude,
    /// Antigravity (agy) CLI engine.
    Agy,
}

/// Operation performed on a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileOp {
    /// File created or written.
    Write,
    /// File edited in-place.
    Edit,
    /// File deleted.
    Delete,
    /// Unknown file operation.
    Unknown,
}

/// Status of a completed turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DoneStatus {
    /// Turn completed successfully.
    Success,
    /// Turn finished with error.
    Error,
    /// Turn was cancelled by user.
    Cancelled,
}

/// Normalized event emitted by an engine adapter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum Event {
    /// Engine session identified or initialized.
    Session {
        /// Session or conversation ID.
        id: String,
    },
    /// Streaming text delta.
    TextDelta(String),
    /// Complete text message chunk.
    Text(String),
    /// Model entered thinking state.
    Thinking,
    /// Model initiated a tool call.
    ToolCall {
        /// Tool call ID.
        id: String,
        /// Name of the tool.
        name: String,
        /// JSON input parameters passed to the tool.
        input: Value,
    },
    /// Output or result of a tool execution.
    ToolResult {
        /// Tool call ID.
        id: String,
        /// Preview of the output (truncated to <= 500 chars).
        output_preview: String,
        /// Whether the tool execution resulted in an error.
        is_error: bool,
    },
    /// A file was modified, created, or deleted.
    FileChange {
        /// Path to the changed file.
        path: PathBuf,
        /// Type of change.
        op: FileOp,
    },
    /// Permission request awaiting user confirmation.
    PermissionRequest {
        /// Tool name requiring permission.
        tool: String,
        /// Input arguments requiring permission.
        input: Value,
    },
    /// Permission was denied for an action or tool.
    PermissionDenied {
        /// Name of tool or action.
        tool: String,
        /// Input arguments or action context.
        input: Value,
    },
    /// Token usage and cost metrics.
    Usage {
        /// Number of input prompt tokens.
        input_tokens: u64,
        /// Number of generated output tokens.
        output_tokens: u64,
        /// Number of tokens read from prompt cache.
        cache_read_tokens: u64,
        /// Cost in USD, if provided by engine.
        cost_usd: Option<f64>,
    },
    /// Turn has completed.
    Done {
        /// Final status of turn.
        status: DoneStatus,
        /// Final response text, if available.
        text: Option<String>,
    },
    /// Unrecoverable engine error.
    Error(String),
    /// Line that could not be parsed into a known event.
    Unparsed(String),
}

/// Session identifier reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionRef {
    /// Start a new session with an explicitly assigned UUID.
    New(Uuid),
    /// Resume an existing session by ID.
    Resume(String),
}

/// Studio MCP HTTP configuration for Claude session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpHttp {
    /// Streamable HTTP loopback URL.
    pub url: String,
    /// Bearer authentication token.
    pub token: String,
}

/// Request parameters for a turn execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnRequest {
    /// Working directory for the workspace.
    pub workspace_dir: PathBuf,
    /// User prompt for this turn.
    pub prompt: String,
    /// Target session reference.
    pub session: SessionRef,
    /// Optional model override.
    pub model: Option<String>,
    /// Optional Studio MCP HTTP loopback configuration.
    pub mcp: Option<McpHttp>,
    /// Optional workspace skills plugin directory.
    pub plugin_dir: Option<PathBuf>,
    /// Optional tool name to prompt for approvals.
    pub permission_prompt_tool: Option<String>,
    /// Optional additional system prompt.
    pub append_system_prompt: Option<String>,
    /// Optional maximum spending budget in USD.
    pub max_budget_usd: Option<f64>,
}

/// Engine installation and compatibility probe result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Probe {
    /// Whether the executable was found and executed.
    pub installed: bool,
    /// Detected version string.
    pub version: Option<String>,
    /// Whether this version has been verified and supported.
    pub supported: bool,
    /// Note on version compatibility in Vietnamese.
    pub note: Option<String>,
}

/// Engine adapter errors.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// I/O error executing engine process.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Serialization or deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// Process failed or exited unexpectedly.
    #[error("Process error: {0}")]
    Process(String),
    /// Unsupported feature or configuration.
    #[error("Unsupported: {0}")]
    Unsupported(String),
}
