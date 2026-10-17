//! Permission decisions for engine tool calls (`--permission-prompt-tool mcp__studio__approve`, ADR-008 §5).

use std::path::Path;

use serde_json::Value;
use studio_workspace::Workspace;

use crate::tokens::TurnContext;

/// Outcome of a permission request.
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    /// Allow, possibly with a rewritten input.
    Allow(Value),
    /// Deny with a Vietnamese reason shown to the agent and the user.
    Deny(String),
}

impl Decision {
    /// Claude Code permission-prompt-tool JSON.
    pub fn to_claude_json(&self) -> Value {
        match self {
            Decision::Allow(input) => {
                serde_json::json!({ "behavior": "allow", "updatedInput": input })
            }
            Decision::Deny(message) => {
                serde_json::json!({ "behavior": "deny", "message": message })
            }
        }
    }
}

/// Boxed future returned by [`PermissionBroker::decide`].
pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

/// Decides permission requests. The desktop app implements this with a UI dialog.
pub trait PermissionBroker: Send + Sync {
    /// Decide whether `tool_name` may run with `input` in the given turn.
    fn decide<'a>(
        &'a self,
        ctx: &'a TurnContext,
        tool_name: &'a str,
        input: &'a Value,
    ) -> BoxFuture<'a, Decision>;
}

/// Policy checks shared by every broker: denied commands and paths outside the workspace.
pub fn policy_check(ctx: &TurnContext, tool_name: &str, input: &Value) -> Option<String> {
    let denied = studio_catalog::denied_commands();
    let text = input.to_string().to_lowercase();
    if let Some(cmd) = denied.iter().find(|d| text.contains(&d.to_lowercase())) {
        return Some(format!(
            "Studio chặn lệnh \"{cmd}\": skill được quản lý qua catalog, không cài trực tiếp."
        ));
    }
    let ws = Workspace::open(&ctx.workspace_dir).ok()?;
    for key in ["file_path", "path", "notebook_path"] {
        if let Some(p) = input.get(key).and_then(Value::as_str) {
            if !inside_workspace(&ws, Path::new(p)) {
                return Some(format!(
                    "Studio chặn {tool_name}: đường dẫn \"{p}\" nằm ngoài workspace."
                ));
            }
        }
    }
    if tool_name == "artifact_render" || tool_name == "mcp__studio__artifact_render" {
        if let Some(ref turn_proj) = ctx.project {
            if let Some(p) = input.get("project").and_then(Value::as_str) {
                if p != turn_proj {
                    return Some(format!(
                        "Studio chặn {tool_name}: lượt này bị giới hạn trong project \"{turn_proj}\", không được thao tác project \"{p}\"."
                    ));
                }
            }
            if let Some(art) = input.get("artifact").and_then(Value::as_str) {
                if art.starts_with("projects/") {
                    let parts: Vec<&str> = art.split('/').collect();
                    if parts.len() >= 2 && parts[1] != turn_proj {
                        return Some(format!(
                            "Studio chặn {tool_name}: lượt này bị giới hạn trong project \"{turn_proj}\", không được thao tác project \"{}\".",
                            parts[1]
                        ));
                    }
                }
            }
        }
        if let Some(art) = input.get("artifact").and_then(Value::as_str) {
            if art.contains("..")
                || art.starts_with('/')
                || art.starts_with('\\')
                || (art.len() >= 2 && art.chars().nth(1) == Some(':'))
            {
                return Some(format!(
                    "Studio chặn {tool_name}: artifact nằm ngoài workspace."
                ));
            }
            let p_slug = input
                .get("project")
                .and_then(Value::as_str)
                .or(ctx.project.as_deref());
            if let Some(slug) = p_slug {
                let art_path = ws
                    .root()
                    .join("projects")
                    .join(slug)
                    .join("artifacts")
                    .join(art);
                if art_path.exists() && !ws.can_write(&art_path) {
                    return Some(format!(
                        "Studio chặn {tool_name}: artifact thuộc tầng chỉ đọc."
                    ));
                }
            }
        }
    }
    None
}

fn inside_workspace(ws: &Workspace, p: &Path) -> bool {
    if p.is_absolute() {
        let root = std::fs::canonicalize(ws.root()).unwrap_or_else(|_| ws.root().to_path_buf());
        let target = p
            .ancestors()
            .find(|a| a.exists())
            .and_then(|a| std::fs::canonicalize(a).ok())
            .unwrap_or_else(|| p.to_path_buf());
        target.starts_with(root)
    } else {
        ws.scoped_path(p).is_ok()
    }
}

/// Default broker: applies [`policy_check`] and allows everything else.
pub struct DefaultBroker;

impl DefaultBroker {
    /// Decide synchronously.
    pub fn decide_sync(ctx: &TurnContext, tool_name: &str, input: &Value) -> Decision {
        match policy_check(ctx, tool_name, input) {
            Some(reason) => Decision::Deny(reason),
            None => Decision::Allow(input.clone()),
        }
    }
}

impl PermissionBroker for DefaultBroker {
    fn decide<'a>(
        &'a self,
        ctx: &'a TurnContext,
        tool_name: &'a str,
        input: &'a Value,
    ) -> BoxFuture<'a, Decision> {
        Box::pin(async move { Self::decide_sync(ctx, tool_name, input) })
    }
}
