//! Stateful parser for Claude Code CLI stream-json output.

use serde_json::Value;
use std::collections::HashSet;
use std::path::PathBuf;

use crate::types::{DoneStatus, Event, FileOp};

/// Stateful parser that transforms lines of Claude stream-json output into normalized `Event`s.
#[derive(Debug, Default)]
pub struct ClaudeParser {
    session_id: Option<String>,
    seen_tool_ids: HashSet<String>,
    /// Thinking text already arrived as stream deltas for the current message.
    streamed_thinking: bool,
}

impl ClaudeParser {
    /// Creates a new, blank ClaudeParser instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds a single line of NDJSON from Claude's stdout and returns zero or more parsed `Event`s.
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

        // Check if session_id is present at top-level
        if self.session_id.is_none() {
            if let Some(sid) = value.get("session_id").and_then(|v| v.as_str()) {
                self.session_id = Some(sid.to_string());
            }
        }

        let event_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");

        match event_type {
            "system" => {
                let subtype = value.get("subtype").and_then(|v| v.as_str()).unwrap_or("");
                if subtype == "init" {
                    if let Some(sid) = value.get("session_id").and_then(|v| v.as_str()) {
                        self.session_id = Some(sid.to_string());
                        events.push(Event::Session {
                            id: sid.to_string(),
                        });
                    }
                }
            }
            "stream_event" => {
                if let Some(inner) = value.get("event") {
                    let inner_type = inner.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    match inner_type {
                        "message_start" => self.streamed_thinking = false,
                        "content_block_start" => {
                            if let Some(cb) = inner.get("content_block") {
                                if cb.get("type").and_then(|v| v.as_str()) == Some("thinking") {
                                    events.push(Event::Thinking(String::new()));
                                }
                            }
                        }
                        "content_block_delta" => {
                            if let Some(delta) = inner.get("delta") {
                                let delta_type =
                                    delta.get("type").and_then(|v| v.as_str()).unwrap_or("");
                                if delta_type == "text_delta" {
                                    if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
                                        events.push(Event::TextDelta(text.to_string()));
                                    }
                                } else if delta_type == "thinking_delta" {
                                    let text = delta
                                        .get("thinking")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("");
                                    if !text.is_empty() {
                                        self.streamed_thinking = true;
                                        events.push(Event::Thinking(text.to_string()));
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            "assistant" => {
                if let Some(msg) = value.get("message") {
                    if let Some(content) = msg.get("content").and_then(|v| v.as_array()) {
                        for block in content {
                            let block_type =
                                block.get("type").and_then(|v| v.as_str()).unwrap_or("");
                            match block_type {
                                "thinking" => {
                                    // Without partial messages the full block is the only source.
                                    let text = block
                                        .get("thinking")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("");
                                    if !self.streamed_thinking {
                                        events.push(Event::Thinking(text.to_string()));
                                    }
                                }
                                "text" => {
                                    if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                                        events.push(Event::Text(text.to_string()));
                                    }
                                }
                                "tool_use" => {
                                    let id = block
                                        .get("id")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let name = block
                                        .get("name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let input = block.get("input").cloned().unwrap_or(Value::Null);

                                    // Deduplicate if needed
                                    self.seen_tool_ids.insert(id.clone());

                                    // Check for file modification tools
                                    if let Some(op) = file_op_for_tool(&name) {
                                        if let Some(file_path) =
                                            input.get("file_path").and_then(|v| v.as_str())
                                        {
                                            events.push(Event::FileChange {
                                                path: PathBuf::from(file_path),
                                                op,
                                            });
                                        }
                                    }

                                    events.push(Event::ToolCall { id, name, input });
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            "user" => {
                if let Some(msg) = value.get("message") {
                    if let Some(content) = msg.get("content").and_then(|v| v.as_array()) {
                        for block in content {
                            if block.get("type").and_then(|v| v.as_str()) == Some("tool_result") {
                                let id = block
                                    .get("tool_use_id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let is_error = block
                                    .get("is_error")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false);

                                let raw_content = block.get("content");
                                let preview = extract_tool_result_preview(raw_content);

                                events.push(Event::ToolResult {
                                    id,
                                    output_preview: preview,
                                    is_error,
                                });
                            }
                        }
                    }
                }
            }
            "result" => {
                // Usage & cost
                let cost_usd = value.get("total_cost_usd").and_then(|v| v.as_f64());
                let usage = value.get("usage");
                let tok = |key: &str| {
                    usage
                        .and_then(|u| u.get(key))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                };
                let reasoning_tokens = usage
                    .and_then(|u| u.pointer("/output_tokens_details/thinking_tokens"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                events.push(Event::Usage {
                    input_tokens: tok("input_tokens"),
                    output_tokens: tok("output_tokens"),
                    cache_read_tokens: tok("cache_read_input_tokens"),
                    cache_write_tokens: tok("cache_creation_input_tokens"),
                    reasoning_tokens,
                    cost_usd,
                });

                // Permission denials
                if let Some(denials) = value.get("permission_denials").and_then(|v| v.as_array()) {
                    for denial in denials {
                        let tool = denial
                            .get("tool_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let input = denial.get("tool_input").cloned().unwrap_or(Value::Null);
                        events.push(Event::PermissionDenied { tool, input });
                    }
                }

                // Done
                let is_error = value
                    .get("is_error")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let status = if is_error {
                    DoneStatus::Error
                } else {
                    DoneStatus::Success
                };
                let text = value
                    .get("result")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);

                events.push(Event::Done { status, text });
            }
            _ => {
                // Ignored or unrecognized top-level event type
            }
        }

        events
    }
}

fn file_op_for_tool(tool_name: &str) -> Option<FileOp> {
    match tool_name {
        "Write" => Some(FileOp::Write),
        "Edit" | "NotebookEdit" => Some(FileOp::Edit),
        _ => None,
    }
}

fn extract_tool_result_preview(content: Option<&Value>) -> String {
    let raw = match content {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(arr)) => {
            let mut out = String::new();
            for item in arr {
                if let Some(t) = item.get("text").and_then(|v| v.as_str()) {
                    out.push_str(t);
                } else if let Some(tr) = item.get("tool_name").and_then(|v| v.as_str()) {
                    out.push_str(tr);
                } else {
                    out.push_str(&item.to_string());
                }
            }
            out
        }
        Some(other) => other.to_string(),
        None => String::new(),
    };

    if raw.chars().count() > 500 {
        let truncated: String = raw.chars().take(500).collect();
        format!("{}...", truncated)
    } else {
        raw
    }
}
