//! Stateful parser for Antigravity (agy) CLI stream-json output.

use serde_json::Value;
use std::path::PathBuf;

use crate::types::{DoneStatus, Event, FileOp};

/// Stateful parser that transforms lines of Agy stream-json output into normalized `Event`s.
#[derive(Debug, Default)]
pub struct AgyParser {
    conversation_id: Option<String>,
    accumulated_input_tokens: u64,
    accumulated_output_tokens: u64,
    accumulated_cache_read_tokens: u64,
    accumulated_reasoning_tokens: u64,
}

impl AgyParser {
    /// Creates a new, blank AgyParser instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds a single line of NDJSON from agy's stdout and returns zero or more parsed `Event`s.
    ///
    /// Never panics on malformed or unexpected lines; emits `Event::Unparsed` instead.
    pub fn feed(&mut self, line: &str) -> Vec<Event> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }

        let value: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => return vec![Event::Unparsed(trimmed.to_string())],
        };

        let mut events = Vec::new();

        // Top-level event name
        let event_type = value.get("event").and_then(|v| v.as_str()).unwrap_or("");

        match event_type {
            "init" => {
                if let Some(cid) = value.get("conversation_id").and_then(|v| v.as_str()) {
                    self.conversation_id = Some(cid.to_string());
                    events.push(Event::Session {
                        id: cid.to_string(),
                    });
                }
            }
            "step_update" => {
                if let Some(step) = value.get("step_update") {
                    let step_index = step.get("step_index").and_then(|v| v.as_u64()).unwrap_or(0);
                    let state = step.get("state").and_then(|v| v.as_str()).unwrap_or("");
                    let step_type = step.get("step_type").and_then(|v| v.as_str()).unwrap_or("");

                    // Text streaming delta
                    if let Some(text_delta) = step.get("text_delta").and_then(|v| v.as_str()) {
                        events.push(Event::TextDelta(text_delta.to_string()));
                    }

                    // Track token usage across steps
                    if let Some(usage) = step.get("usage") {
                        let in_tok = usage
                            .get("input_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let out_tok = usage
                            .get("output_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let cache_tok = usage
                            .get("cache_read_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let think_tok = usage
                            .get("thinking_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);

                        self.accumulated_input_tokens += in_tok;
                        self.accumulated_output_tokens += out_tok;
                        self.accumulated_cache_read_tokens += cache_tok;
                        self.accumulated_reasoning_tokens += think_tok;

                        events.push(Event::Usage {
                            input_tokens: self.accumulated_input_tokens,
                            output_tokens: self.accumulated_output_tokens,
                            cache_read_tokens: self.accumulated_cache_read_tokens,
                            // Gemini caches implicitly; agy reports no cache writes.
                            cache_write_tokens: 0,
                            reasoning_tokens: self.accumulated_reasoning_tokens,
                            cost_usd: None,
                        });
                    }

                    // Handle tool calls and completions
                    if step_type == "tool" {
                        let tool_name =
                            step.get("tool_name").and_then(|v| v.as_str()).unwrap_or("");
                        let tool_info = step.get("tool_info");
                        let parameters = tool_info
                            .and_then(|ti| ti.get("parameters"))
                            .cloned()
                            .unwrap_or(Value::Null);

                        let step_id = format!("step-{}", step_index);

                        if state == "ACTIVE" {
                            // File change detection
                            if let Some(op) = file_op_for_agy_tool(tool_name) {
                                if let Some(target) = extract_agy_target_file(&parameters) {
                                    let clean = target.replace('\\', "/");
                                    events.push(Event::FileChange {
                                        path: PathBuf::from(clean),
                                        op,
                                    });
                                }
                            }

                            events.push(Event::ToolCall {
                                id: step_id,
                                name: friendly_agy_tool_name(tool_name).to_string(),
                                input: parameters,
                            });
                        } else if state == "DONE" || state == "ERROR" {
                            let is_error = state == "ERROR"
                                || tool_info.and_then(|ti| ti.get("error")).is_some();

                            let raw_output = tool_info
                                .and_then(|ti| ti.get("output").or_else(|| ti.get("error")))
                                .map(|v| match v {
                                    Value::String(s) => s.clone(),
                                    other => other.to_string(),
                                })
                                .unwrap_or_default();

                            let preview = if raw_output.chars().count() > 500 {
                                let truncated: String = raw_output.chars().take(500).collect();
                                format!("{}...", truncated)
                            } else {
                                raw_output
                            };

                            events.push(Event::ToolResult {
                                id: step_id,
                                output_preview: preview,
                                is_error,
                            });
                        }
                    }
                }
            }
            "result" => {
                if let Some(res) = value.get("result") {
                    // Update final usage if provided
                    if let Some(usage) = res.get("usage") {
                        let in_tok = usage
                            .get("input_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(self.accumulated_input_tokens);
                        let out_tok = usage
                            .get("output_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(self.accumulated_output_tokens);
                        let cache_tok = usage
                            .get("cache_read_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(self.accumulated_cache_read_tokens);
                        let think_tok = usage
                            .get("thinking_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(self.accumulated_reasoning_tokens);

                        events.push(Event::Usage {
                            input_tokens: in_tok,
                            output_tokens: out_tok,
                            cache_read_tokens: cache_tok,
                            cache_write_tokens: 0,
                            reasoning_tokens: think_tok,
                            cost_usd: None,
                        });
                    }

                    // Surface the engine's error (quota, auth, ...) instead of ending silently.
                    if let Some(err) = res.get("error").and_then(|v| v.as_str()) {
                        if !err.trim().is_empty() {
                            events.push(Event::Error(err.to_string()));
                        }
                    }

                    // Denied actions
                    if let Some(denied) = res.get("denied_actions").and_then(|v| v.as_array()) {
                        for action_obj in denied {
                            let tool = action_obj
                                .get("action")
                                .or_else(|| action_obj.get("display_name"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();
                            events.push(Event::PermissionDenied {
                                tool,
                                input: action_obj.clone(),
                            });
                        }
                    }

                    // Done event
                    let status_str = res.get("status").and_then(|v| v.as_str()).unwrap_or("");
                    let status = if status_str == "SUCCESS" {
                        DoneStatus::Success
                    } else {
                        DoneStatus::Error
                    };
                    let text = res
                        .get("response")
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string);

                    events.push(Event::Done { status, text });
                }
            }
            _ => {
                // Ignore unknown top-level event
            }
        }

        events
    }
}

fn file_op_for_agy_tool(tool_name: &str) -> Option<FileOp> {
    match tool_name {
        "write_to_file" => Some(FileOp::Write),
        "replace_file_content" | "multi_replace_file_content" | "sed_file" | "notebook_edit" => {
            Some(FileOp::Edit)
        }
        _ => None,
    }
}

fn extract_agy_target_file(params: &Value) -> Option<&str> {
    params
        .get("TargetFile")
        .or_else(|| params.get("target_file"))
        .or_else(|| params.get("AbsolutePath"))
        .or_else(|| params.get("path"))
        .and_then(|v| v.as_str())
}

/// Map internal agy tool names to friendly Vietnamese readable step names.
fn friendly_agy_tool_name(tool_name: &str) -> &'static str {
    match tool_name {
        "view_file" => "Đọc tệp",
        "write_to_file" => "Tạo tệp",
        "replace_file_content" | "multi_replace_file_content" | "sed_file" => "Sửa tệp",
        "run_command" | "send_command_input" => "Chạy lệnh shell",
        "read_url_content" | "search_web" | "read_browser_page" | "open_browser_url" => {
            "Tìm kiếm & duyệt web"
        }
        "call_mcp_tool" => "Gọi công cụ Studio",
        "invoke_subagent" | "send_message" => "Điều phối tác vụ con",
        "list_dir" | "find_by_name" | "grep_search" => "Tìm kiếm tệp",
        _ => "Công cụ",
    }
}
