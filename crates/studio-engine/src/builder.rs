//! Pure command builders for Claude and Agy CLI execution.

use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

use crate::types::{EngineError, SessionRef, TurnRequest};

/// Specification of a CLI command ready to be spawned.
#[derive(Debug, Clone)]
pub struct CommandSpec {
    /// Executable program name or path.
    pub program: PathBuf,
    /// Command line arguments.
    pub args: Vec<String>,
    /// Single JSON line sent via stdin.
    pub stdin_line: String,
    /// Temporary files created for this invocation that should be cleaned up after turn finishes.
    pub temp_files: Vec<PathBuf>,
}

/// Builds the CLI command and arguments for Claude Code.
///
/// Generates temporary MCP config file if `req.mcp` is present.
/// Token is never placed on the command line.
pub fn claude_command(
    req: &TurnRequest,
    tmp_dir: &Path,
    program_override: Option<&Path>,
) -> Result<CommandSpec, EngineError> {
    let program = program_override
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("claude"));

    let mut args = vec![
        "-p".to_string(),
        "--input-format".to_string(),
        "stream-json".to_string(),
        "--output-format".to_string(),
        "stream-json".to_string(),
        "--verbose".to_string(),
        "--include-partial-messages".to_string(),
    ];

    match &req.session {
        SessionRef::New(uuid) => {
            args.push("--session-id".to_string());
            args.push(uuid.to_string());
        }
        SessionRef::Resume(id) => {
            args.push("--resume".to_string());
            args.push(id.clone());
        }
    }

    let mut temp_files = Vec::new();

    if let Some(mcp) = &req.mcp {
        let mcp_config = json!({
            "mcpServers": {
                "studio": {
                    "type": "http",
                    "url": mcp.url,
                    "headers": {
                        "Authorization": format!("Bearer {}", mcp.token)
                    }
                }
            }
        });

        let file_name = format!("mcp-config-{}.json", uuid::Uuid::new_v4());
        let mcp_path = tmp_dir.join(file_name);
        fs::write(&mcp_path, serde_json::to_string(&mcp_config)?)?;
        args.push("--mcp-config".to_string());
        args.push(mcp_path.to_string_lossy().to_string());
        args.push("--strict-mcp-config".to_string());
        temp_files.push(mcp_path);
    }

    args.push("--setting-sources".to_string());
    args.push("project".to_string());

    if let Some(plugin_dir) = &req.plugin_dir {
        args.push("--plugin-dir".to_string());
        args.push(plugin_dir.to_string_lossy().to_string());
    }

    args.push("--permission-mode".to_string());
    args.push("acceptEdits".to_string());

    if let Some(prompt_tool) = &req.permission_prompt_tool {
        args.push("--permission-prompt-tool".to_string());
        args.push(prompt_tool.clone());
    }

    if let Some(append_sys) = &req.append_system_prompt {
        args.push("--append-system-prompt".to_string());
        args.push(append_sys.clone());
    }

    if let Some(budget) = req.max_budget_usd {
        args.push("--max-budget-usd".to_string());
        args.push(budget.to_string());
    }

    if let Some(model) = &req.model {
        args.push("--model".to_string());
        args.push(model.clone());
    }

    let stdin_payload = json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": req.prompt
        }
    });
    let stdin_line = serde_json::to_string(&stdin_payload)?;

    Ok(CommandSpec {
        program,
        args,
        stdin_line,
        temp_files,
    })
}

/// Builds the CLI command and arguments for Antigravity (agy).
///
/// agy requires `-p=` form when prompt is passed via stream-json stdin.
/// agy has no per-session MCP or plugin-dir: those are ignored, and
/// `append_system_prompt` is prepended to the prompt.
pub fn agy_command(
    req: &TurnRequest,
    _tmp_dir: &Path,
    program_override: Option<&Path>,
) -> Result<CommandSpec, EngineError> {
    let program = program_override
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("agy"));

    let mut args = vec![
        "--input-format".to_string(),
        "stream-json".to_string(),
        "--output-format".to_string(),
        "stream-json".to_string(),
        "--mode".to_string(),
        "accept-edits".to_string(),
    ];

    match &req.session {
        SessionRef::New(_) => {
            // agy generates conversation_id on init; no CLI session-id flag for new
        }
        SessionRef::Resume(id) => {
            args.push("--conversation".to_string());
            args.push(id.clone());
        }
    }

    if let Some(model) = &req.model {
        args.push("--model".to_string());
        args.push(model.clone());
    }

    // agy requires -p= form when prompt is streamed through stdin
    args.push("-p=".to_string());

    let effective_prompt = if let Some(sys) = &req.append_system_prompt {
        format!("{}\n\n{}", sys, req.prompt)
    } else {
        req.prompt.clone()
    };

    let stdin_payload = json!({
        "event": "user",
        "message": {
            "role": "user",
            "content": effective_prompt
        }
    });
    let stdin_line = serde_json::to_string(&stdin_payload)?;

    Ok(CommandSpec {
        program,
        args,
        stdin_line,
        temp_files: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_claude_command_builder_new_session() {
        let tmp = tempfile::tempdir().unwrap();
        let session_id = Uuid::new_v4();
        let req = TurnRequest {
            workspace_dir: PathBuf::from("D:/workspace"),
            prompt: "Test hello".to_string(),
            session: SessionRef::New(session_id),
            model: Some("claude-haiku-5-5".to_string()),
            mcp: Some(crate::types::McpHttp {
                url: "http://127.0.0.1:4000/mcp".to_string(),
                token: "secret-token-123".to_string(),
            }),
            plugin_dir: Some(PathBuf::from("D:/plugins")),
            permission_prompt_tool: Some("mcp__studio__approve".to_string()),
            append_system_prompt: Some("You are studio".to_string()),
            max_budget_usd: Some(1.5),
        };

        let spec = claude_command(&req, tmp.path(), None).unwrap();
        assert_eq!(spec.program, PathBuf::from("claude"));
        assert!(spec.args.contains(&"-p".to_string()));
        assert!(spec.args.contains(&"--session-id".to_string()));
        assert!(spec.args.contains(&session_id.to_string()));
        assert!(spec.args.contains(&"--model".to_string()));
        assert!(spec.args.contains(&"claude-haiku-5-5".to_string()));
        assert!(spec.args.contains(&"--strict-mcp-config".to_string()));
        assert!(spec.args.contains(&"--mcp-config".to_string()));
        assert_eq!(spec.temp_files.len(), 1);

        // Verify token is NOT in CLI args
        for arg in &spec.args {
            assert!(!arg.contains("secret-token-123"));
        }

        // Verify token is in temp file
        let mcp_content = fs::read_to_string(&spec.temp_files[0]).unwrap();
        assert!(mcp_content.contains("Bearer secret-token-123"));
        assert!(mcp_content.contains("http://127.0.0.1:4000/mcp"));

        // Verify stdin
        assert!(spec.stdin_line.contains("\"type\":\"user\""));
        assert!(spec.stdin_line.contains("Test hello"));
    }

    #[test]
    fn test_claude_command_builder_resume_session() {
        let tmp = tempfile::tempdir().unwrap();
        let req = TurnRequest {
            workspace_dir: PathBuf::from("D:/workspace"),
            prompt: "Resume turn".to_string(),
            session: SessionRef::Resume("session-xyz".to_string()),
            model: None,
            mcp: None,
            plugin_dir: None,
            permission_prompt_tool: None,
            append_system_prompt: None,
            max_budget_usd: None,
        };

        let spec = claude_command(&req, tmp.path(), None).unwrap();
        assert!(spec.args.contains(&"--resume".to_string()));
        assert!(spec.args.contains(&"session-xyz".to_string()));
        assert!(!spec.args.contains(&"--session-id".to_string()));
        assert!(spec.temp_files.is_empty());
    }

    #[test]
    fn test_agy_command_builder_new_session() {
        let tmp = tempfile::tempdir().unwrap();
        let req = TurnRequest {
            workspace_dir: PathBuf::from("D:/workspace"),
            prompt: "Agy hello".to_string(),
            session: SessionRef::New(Uuid::new_v4()),
            model: Some("gemini-3.8-flash".to_string()),
            mcp: Some(crate::types::McpHttp {
                url: "http://127.0.0.1:4000/mcp".to_string(),
                token: "token".to_string(),
            }),
            plugin_dir: Some(PathBuf::from("D:/plugins")),
            permission_prompt_tool: None,
            append_system_prompt: Some("System instructions".to_string()),
            max_budget_usd: None,
        };

        let spec = agy_command(&req, tmp.path(), None).unwrap();
        assert_eq!(spec.program, PathBuf::from("agy"));
        assert!(spec.args.contains(&"-p=".to_string()));
        assert!(spec.args.contains(&"--mode".to_string()));
        assert!(spec.args.contains(&"accept-edits".to_string()));
        assert!(spec.args.contains(&"--model".to_string()));
        assert!(spec.args.contains(&"gemini-3.8-flash".to_string()));
        assert!(!spec.args.contains(&"--conversation".to_string()));

        // agy ignores mcp and plugin_dir
        assert!(!spec.args.contains(&"--mcp-config".to_string()));
        assert!(spec.temp_files.is_empty());

        // System prompt prepended to prompt in stdin
        assert!(spec.stdin_line.contains("\"event\":\"user\""));
        assert!(spec
            .stdin_line
            .contains("System instructions\\n\\nAgy hello"));
    }

    #[test]
    fn test_agy_command_builder_resume_session() {
        let tmp = tempfile::tempdir().unwrap();
        let req = TurnRequest {
            workspace_dir: PathBuf::from("D:/workspace"),
            prompt: "Resume turn".to_string(),
            session: SessionRef::Resume("conv-123".to_string()),
            model: None,
            mcp: None,
            plugin_dir: None,
            permission_prompt_tool: None,
            append_system_prompt: None,
            max_budget_usd: None,
        };

        let spec = agy_command(&req, tmp.path(), None).unwrap();
        assert!(spec.args.contains(&"--conversation".to_string()));
        assert!(spec.args.contains(&"conv-123".to_string()));
    }
}
