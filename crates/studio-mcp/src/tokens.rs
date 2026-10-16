//! Per-turn bearer tokens. Every MCP request is scoped to the workspace of its token (ADR-009 §3).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

/// Which engine runs the turn that owns a token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineKind {
    /// Claude Code CLI.
    Claude,
    /// Antigravity CLI.
    Agy,
}

/// Context bound to a token for the lifetime of one agent turn.
#[derive(Debug, Clone)]
pub struct TurnContext {
    /// Workspace root directory.
    pub workspace_dir: PathBuf,
    /// Turn identifier.
    pub turn_id: String,
    /// Project slug the turn works on, if any.
    pub project: Option<String>,
    /// Engine that runs the turn.
    pub engine: EngineKind,
    /// Capabilities allowed for this turn; `None` means all.
    pub allowed_capabilities: Option<Vec<String>>,
    /// Remaining USD budget for paid tools in this turn.
    pub budget_usd: Option<f64>,
}

/// Registry of active turn tokens.
#[derive(Default)]
pub struct TokenRegistry {
    tokens: RwLock<HashMap<String, TurnContext>>,
}

impl TokenRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Issue a new random token for a turn.
    pub fn issue(&self, ctx: TurnContext) -> String {
        let token = format!(
            "lcs_{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        if let Ok(mut map) = self.tokens.write() {
            map.insert(token.clone(), ctx);
        }
        token
    }

    /// Revoke a token (turn finished or cancelled).
    pub fn revoke(&self, token: &str) {
        if let Ok(mut map) = self.tokens.write() {
            map.remove(token);
        }
    }

    /// Look up the turn context of a token.
    pub fn get(&self, token: &str) -> Option<TurnContext> {
        self.tokens.read().ok()?.get(token).cloned()
    }

    /// Find the token of the active agy turn whose workspace contains `dir` (used by the stdio shim).
    pub fn active_agy_token_for(&self, dir: &std::path::Path) -> Option<String> {
        let dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
        let map = self.tokens.read().ok()?;
        map.iter()
            .filter(|(_, c)| c.engine == EngineKind::Agy)
            .find(|(_, c)| {
                let ws = std::fs::canonicalize(&c.workspace_dir)
                    .unwrap_or_else(|_| c.workspace_dir.clone());
                dir.starts_with(&ws)
            })
            .map(|(t, _)| t.clone())
    }
}
