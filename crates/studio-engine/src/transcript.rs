//! Transcript accumulator collecting normalized events from a turn into an aggregated summary.

use serde_json::Value;
use std::collections::HashSet;
use std::path::PathBuf;

use crate::types::{DoneStatus, Event, FileOp};

/// Aggregated summary of events recorded during a turn.
#[derive(Debug, Default, Clone)]
pub struct Transcript {
    /// Active session ID if detected.
    pub session_id: Option<String>,
    /// Accumulated final text response.
    pub text: String,
    /// Set of unique file changes recorded during the turn.
    pub file_changes: Vec<(PathBuf, FileOp)>,
    /// Latest token usage metrics.
    pub input_tokens: u64,
    /// Latest token usage metrics.
    pub output_tokens: u64,
    /// Latest cache read token metrics.
    pub cache_read_tokens: u64,
    /// Total cost in USD, if provided.
    pub cost_usd: Option<f64>,
    /// Permission denials recorded during the turn.
    pub permission_denials: Vec<(String, Value)>,
    /// Final turn completion status, if received.
    pub done_status: Option<DoneStatus>,
    /// Raw unparsed lines encountered.
    pub unparsed_lines: Vec<String>,
    /// Errors encountered during the turn.
    pub errors: Vec<String>,
}

impl Transcript {
    /// Creates an empty transcript.
    pub fn new() -> Self {
        Self::default()
    }

    /// Ingests a slice of events into the transcript.
    pub fn ingest(&mut self, events: &[Event]) {
        for event in events {
            self.ingest_one(event);
        }
    }

    /// Ingests a single event into the transcript.
    pub fn ingest_one(&mut self, event: &Event) {
        match event {
            Event::Session { id } => {
                self.session_id = Some(id.clone());
            }
            Event::TextDelta(delta) => {
                self.text.push_str(delta);
            }
            Event::Text(full_chunk) => {
                // If text is already populated by streaming deltas, do not duplicate
                if self.text.is_empty() {
                    self.text.push_str(full_chunk);
                }
            }
            Event::Thinking => {}
            Event::ToolCall { .. } => {}
            Event::ToolResult { .. } => {}
            Event::FileChange { path, op } => {
                if !self.file_changes.iter().any(|(p, o)| p == path && o == op) {
                    self.file_changes.push((path.clone(), *op));
                }
            }
            Event::PermissionRequest { .. } => {}
            Event::PermissionDenied { tool, input } => {
                self.permission_denials.push((tool.clone(), input.clone()));
            }
            Event::Usage {
                input_tokens,
                output_tokens,
                cache_read_tokens,
                cost_usd,
            } => {
                self.input_tokens = *input_tokens;
                self.output_tokens = *output_tokens;
                self.cache_read_tokens = *cache_read_tokens;
                if cost_usd.is_some() {
                    self.cost_usd = *cost_usd;
                }
            }
            Event::Done { status, text } => {
                self.done_status = Some(*status);
                if let Some(t) = text {
                    if self.text.is_empty() {
                        self.text = t.clone();
                    }
                }
            }
            Event::Error(err) => {
                self.errors.push(err.clone());
            }
            Event::Unparsed(line) => {
                self.unparsed_lines.push(line.clone());
            }
        }
    }

    /// Collects unique touched paths.
    pub fn affected_paths(&self) -> Vec<PathBuf> {
        let mut set = HashSet::new();
        let mut out = Vec::new();
        for (p, _) in &self.file_changes {
            if set.insert(p.clone()) {
                out.push(p.clone());
            }
        }
        out
    }
}
