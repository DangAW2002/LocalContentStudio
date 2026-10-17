//! Execution pipeline for running tools via `studio-jobs` scheduler.

use crate::error::{Result, ToolError};
use crate::manifest::{SideEffect, ToolRunType};
use crate::registry::Registry;
use crate::secret::SecretStore;
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use studio_jobs::{CommandSpec, FailedReason, JobEvent, JobId, JobSpec, Priority, Scheduler};
use studio_workspace::{Workspace, WorkspaceDb};
use uuid::Uuid;

/// Max size in bytes before output is written to a file in `.studio/cache/tool-out/`.
pub const MAX_INLINE_RESULT_BYTES: usize = 32 * 1024; // 32 KB

/// Context supplied to `run_tool`.
pub struct ToolRunContext<'a> {
    /// Active workspace.
    pub workspace: &'a Workspace,
    /// Active turn identifier if invoked during an agent turn.
    pub turn_id: Option<String>,
    /// Budget remaining for this turn in USD.
    pub budget: Option<f64>,
    /// Whether user/broker has explicitly approved execution.
    pub approved: bool,
    /// Central jobs scheduler instance.
    pub scheduler: &'a Arc<Scheduler>,
    /// Secrets store.
    pub secret_store: &'a dyn SecretStore,
    /// Host-set API endpoint overrides by tool id (tests, local gateways). Never taken from tool input.
    pub endpoints: std::collections::HashMap<String, String>,
    /// Host-set pricing configuration per provider (+ optional per model). Never taken from tool input.
    pub pricing: std::collections::HashMap<String, crate::pricing::ProviderPricing>,
}

/// Outcome of invoking a tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolRun {
    /// Tool completed successfully with inline JSON payload.
    Success {
        /// Tool JSON result.
        output: serde_json::Value,
        /// Cost in USD incurred.
        cost_usd: f64,
    },
    /// Tool execution requires explicit approval from the user before proceeding.
    NeedsApproval {
        /// Explanation why approval is needed.
        reason: String,
        /// Associated side effect.
        side_effect: SideEffect,
    },
    /// Output payload exceeded 32 KB threshold and was saved to a workspace file.
    LargeOutput {
        /// Path to the written JSON file.
        path: PathBuf,
        /// Total payload size in bytes.
        size_bytes: usize,
        /// Cost in USD incurred.
        cost_usd: f64,
    },
}

impl ToolRun {
    /// Return the output JSON or an object describing the file path.
    pub fn to_json_output(&self) -> serde_json::Value {
        match self {
            Self::Success { output, .. } => output.clone(),
            Self::NeedsApproval {
                reason,
                side_effect,
            } => {
                serde_json::json!({
                    "status": "needs_approval",
                    "reason": reason,
                    "side_effect": side_effect.to_vietnamese()
                })
            }
            Self::LargeOutput {
                path,
                size_bytes,
                cost_usd,
            } => {
                serde_json::json!({
                    "result_path": path.to_string_lossy(),
                    "size_bytes": size_bytes,
                    "cost_usd": cost_usd,
                    "note": "Kết quả vượt quá 32 KB nên đã được ghi vào tệp"
                })
            }
        }
    }
}

/// Execute a tool end-to-end through the jobs scheduler.
pub async fn run_tool(
    registry: &Registry,
    tool_id: &str,
    input: serde_json::Value,
    ctx: &ToolRunContext<'_>,
) -> Result<ToolRun> {
    let tool = registry
        .get_tool(tool_id)
        .ok_or_else(|| ToolError::ToolNotFound(tool_id.to_string()))?;

    // 1. Calculate cost estimate
    let estimated_cost = match tool.cost.as_ref().and_then(|c| c.estimate.as_f64()) {
        Some(c) => c,
        None if tool.side_effect == SideEffect::Paid => 0.05,
        _ => 0.0,
    };

    // 2. Enforce side-effects and budgets
    if tool.side_effect == SideEffect::ExternalPublish
        || tool.side_effect == SideEffect::Destructive
    {
        if !ctx.approved {
            return Ok(ToolRun::NeedsApproval {
                reason: format!(
                    "Công cụ '{}' có tác dụng phụ '{}' cần được bạn xác nhận trước khi chạy.",
                    tool.id,
                    tool.side_effect.to_vietnamese()
                ),
                side_effect: tool.side_effect,
            });
        }
    } else if tool.side_effect == SideEffect::Paid {
        if let Some(budget) = ctx.budget {
            if estimated_cost > budget && !ctx.approved {
                return Ok(ToolRun::NeedsApproval {
                    reason: format!(
                        "Chi phí dự tính (${:.3}) vượt quá ngân sách còn lại của lượt (${:.3}). Cần xác nhận trước khi tiếp tục.",
                        estimated_cost, budget
                    ),
                    side_effect: SideEffect::Paid,
                });
            }
        }
    }

    let org_name = ctx.workspace.org().name();
    let ws_name = ctx.workspace.name();

    // 3. Builtin in-process tools
    if tool.run.run_type == ToolRunType::Builtin {
        let mut api_key = String::new();
        for need in &tool.needs {
            if let Some(key) = need.strip_prefix("secret:") {
                if let Some(val) = ctx.secret_store.get(org_name, Some(ws_name), key) {
                    api_key = val;
                    break;
                } else {
                    return Err(ToolError::ExecutionFailed {
                        tool_id: tool.id.clone(),
                        reason: format!("Chưa có khoá API {key} — thêm trong Cài đặt → Khoá API"),
                    });
                }
            }
        }

        let job_uuid = Uuid::new_v4().to_string();
        let turn_out_dir = match &ctx.turn_id {
            Some(tid) => ctx.workspace.cache_dir().join("turns").join(tid),
            None => ctx.workspace.cache_dir().join("tool-out"),
        };
        fs::create_dir_all(&turn_out_dir).map_err(|e| ToolError::Io {
            path: turn_out_dir.clone(),
            source: e,
        })?;

        let endpoint_override = ctx.endpoints.get(&tool.id).map(String::as_str);
        let pricing_override = ctx.pricing.get(&tool.id);
        let output_json = crate::providers::run_builtin(
            &tool.id,
            ctx.workspace,
            &turn_out_dir,
            &input,
            &api_key,
            endpoint_override,
            pricing_override,
        )
        .await?;

        let job_id: JobId = format!("tool-{}", job_uuid);
        let final_cost = output_json
            .get("provider_metadata")
            .and_then(|m| m.get("estimated_cost_usd"))
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(estimated_cost);
        record_cost_ledger(
            ctx.workspace,
            ctx.turn_id.as_deref(),
            &job_id,
            &tool.id,
            final_cost,
        )?;

        let output_bytes =
            serde_json::to_vec(&output_json).map_err(|e| ToolError::ParseError(e.to_string()))?;
        let output_bytes_len = output_bytes.len();

        if output_bytes_len > MAX_INLINE_RESULT_BYTES {
            let out_file_path = turn_out_dir.join(format!("{}.json", job_uuid));
            fs::write(&out_file_path, output_bytes).map_err(|e| ToolError::Io {
                path: out_file_path.clone(),
                source: e,
            })?;

            return Ok(ToolRun::LargeOutput {
                path: out_file_path,
                size_bytes: output_bytes_len,
                cost_usd: final_cost,
            });
        } else {
            return Ok(ToolRun::Success {
                output: output_json,
                cost_usd: final_cost,
            });
        }
    }

    // 4. Collect declared secrets ONLY from needs
    let mut job_env = BTreeMap::new();

    for need in &tool.needs {
        if let Some(key) = need.strip_prefix("secret:") {
            if let Some(val) = ctx.secret_store.get(org_name, Some(ws_name), key) {
                job_env.insert(key.to_string(), val);
            }
        }
    }

    // 4. Setup output path and pass $STUDIO_OUT in environment
    let job_uuid = Uuid::new_v4().to_string();
    let tool_out_dir = ctx.workspace.cache_dir().join("tool-out");
    fs::create_dir_all(&tool_out_dir).map_err(|e| ToolError::Io {
        path: tool_out_dir.clone(),
        source: e,
    })?;

    let out_file_path = tool_out_dir.join(format!("{}.json", job_uuid));
    job_env.insert(
        "STUDIO_OUT".to_string(),
        out_file_path.to_string_lossy().to_string(),
    );

    // 5. Serialize input to stdin
    let stdin_bytes =
        serde_json::to_vec(&input).map_err(|e| ToolError::ParseError(e.to_string()))?;

    // 6. Build CommandSpec and JobSpec
    let mut command_spec = CommandSpec::new(
        tool.run.command.clone(),
        tool.run.args.clone(),
        ctx.workspace.root(),
    );
    command_spec.env = job_env;
    command_spec.stdin = Some(stdin_bytes);

    let job_id: JobId = format!("tool-{}", job_uuid);
    let job_spec = JobSpec {
        id: job_id.clone(),
        workspace_id: ctx.workspace.name().to_string(),
        tool_id: tool.id.clone(),
        command: command_spec,
        resources: tool.resources.clone(),
        priority: Priority::TurnBlocking,
        timeout: Duration::from_secs(120),
        memory_limit_mb: None,
        lifecycle: tool.lifecycle,
    };

    // 7. Submit to scheduler
    let mut handle = ctx.scheduler.submit(job_spec).await;
    let mut stdout_lines = Vec::new();
    let mut stderr_lines = Vec::new();
    let mut execution_failed = None;

    while let Some(event) = handle.next_event().await {
        match event {
            JobEvent::Stdout(line) => stdout_lines.push(line),
            JobEvent::Stderr(line) => stderr_lines.push(line),
            JobEvent::Failed { reason } => {
                let err_str = match reason {
                    FailedReason::OutOfMemory => "Tiến trình vượt quá giới hạn bộ nhớ RAM".into(),
                    FailedReason::Timeout => "Tiến trình vượt quá thời gian chờ (timeout)".into(),
                    FailedReason::Cancelled => "Tiến trình đã bị hủy".into(),
                    FailedReason::SpawnError(s) => format!("Lỗi khởi tạo tiến trình: {}", s),
                    FailedReason::Exit(code) => format!("Tiến trình kết thúc với mã lỗi {}", code),
                };
                execution_failed = Some(err_str);
            }
            _ => {}
        }
    }

    if let Some(err) = execution_failed {
        let full_err = if !stderr_lines.is_empty() {
            format!("{}: {}", err, stderr_lines.join("\n"))
        } else {
            err
        };
        return Err(ToolError::ExecutionFailed {
            tool_id: tool.id.clone(),
            reason: full_err,
        });
    }

    // 8. Retrieve output JSON from $STUDIO_OUT or stdout
    let (output_json, output_bytes_len) = if out_file_path.is_file() {
        let content = fs::read(&out_file_path).map_err(|e| ToolError::Io {
            path: out_file_path.clone(),
            source: e,
        })?;
        let len = content.len();
        let slice = content
            .strip_prefix(&[0xEF, 0xBB, 0xBF])
            .unwrap_or(&content);
        let json: serde_json::Value = serde_json::from_slice(slice).unwrap_or_else(
            |_| serde_json::json!({ "raw": String::from_utf8_lossy(slice).to_string() }),
        );
        (json, len)
    } else {
        let combined = stdout_lines.join("\n");
        let len = combined.len();
        let trimmed = combined.trim_start_matches('\u{feff}').trim();
        let json: serde_json::Value = serde_json::from_str(trimmed).unwrap_or_else(|_| {
            if trimmed.is_empty() {
                serde_json::json!({})
            } else {
                serde_json::json!({ "output": combined })
            }
        });
        (json, len)
    };

    // 9. Record cost into cost_ledger in studio.db
    let final_cost = estimated_cost;
    record_cost_ledger(
        ctx.workspace,
        ctx.turn_id.as_deref(),
        &job_id,
        &tool.id,
        final_cost,
    )?;

    // 10. Check if output > 32 KB threshold
    if output_bytes_len > MAX_INLINE_RESULT_BYTES {
        // Ensure content is written to output file if not already
        if !out_file_path.is_file() {
            let json_bytes = serde_json::to_vec_pretty(&output_json)
                .map_err(|e| ToolError::ParseError(e.to_string()))?;
            fs::write(&out_file_path, json_bytes).map_err(|e| ToolError::Io {
                path: out_file_path.clone(),
                source: e,
            })?;
        }

        Ok(ToolRun::LargeOutput {
            path: out_file_path,
            size_bytes: output_bytes_len,
            cost_usd: final_cost,
        })
    } else {
        Ok(ToolRun::Success {
            output: output_json,
            cost_usd: final_cost,
        })
    }
}

fn record_cost_ledger(
    ws: &Workspace,
    turn_id: Option<&str>,
    job_id: &str,
    provider: &str,
    cost_usd: f64,
) -> Result<()> {
    if let Ok(mut db) = WorkspaceDb::open(ws) {
        let cost_id = format!("cost-{}", Uuid::new_v4());
        let now = Utc::now().to_rfc3339();
        let _ = db.conn_mut().execute(
            "INSERT INTO cost_ledger (id, turn_id, job_id, provider, model, cost_usd, details_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![cost_id, turn_id, job_id, provider, "", cost_usd, "{}", now],
        );
    }
    Ok(())
}
