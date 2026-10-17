//! Agent turn orchestration: checkpoint → skills → token → engine → stream/persist events (plan 0009 §7, ADR-008).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
use studio_engine::{DoneStatus, Engine, Event, FileOp, McpHttp, SessionRef, Turn, TurnRequest};
use studio_mcp::{EngineKind as McpEngine, TurnContext};
use studio_workspace::{CheckpointService, Project, Workspace, WorkspaceDb};
use tokio::sync::Notify;

use crate::error::{CmdError, CmdResult};
use crate::state::{AppState, RunningTurn};

/// Default per-turn budget for paid tools when `studio.yaml` sets none.
const DEFAULT_TURN_BUDGET_USD: f64 = 1.0;

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Read a workspace setting.
pub fn get_setting(db: &WorkspaceDb, key: &str) -> CmdResult<Option<String>> {
    Ok(db
        .conn()
        .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
            r.get(0)
        })
        .optional()?)
}

/// Write a workspace setting.
pub fn set_setting(db: &WorkspaceDb, key: &str, value: &str) -> CmdResult<()> {
    db.conn().execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

/// Delete a workspace setting.
pub fn del_setting(db: &WorkspaceDb, key: &str) -> CmdResult<()> {
    db.conn()
        .execute("DELETE FROM settings WHERE key = ?1", [key])?;
    Ok(())
}

/// Map an engine event to the UI `EngineEvent` (IPC.md §2). `None` = not shown.
pub fn to_ui_event(ev: &Event, ws_root: &Path) -> Option<Value> {
    Some(match ev {
        Event::Session { id } => json!({ "type": "Session", "id": id }),
        Event::TextDelta(t) => json!({ "type": "TextDelta", "text": t }),
        Event::Text(t) => json!({ "type": "Text", "text": t }),
        Event::Thinking => json!({ "type": "Thinking", "text": "" }),
        Event::ToolCall { id, name, input } => {
            json!({ "type": "ToolCall", "id": id, "name": name, "input": input })
        }
        Event::ToolResult {
            id,
            output_preview,
            is_error,
        } => {
            json!({ "type": "ToolResult", "id": id, "outputPreview": output_preview, "isError": is_error })
        }
        Event::FileChange { path, op } => {
            let rel = path
                .strip_prefix(ws_root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            let op = match op {
                FileOp::Write => "create",
                FileOp::Delete => "delete",
                _ => "modify",
            };
            json!({ "type": "FileChange", "path": rel, "op": op })
        }
        Event::PermissionRequest { tool, input } => {
            json!({ "type": "PermissionDenied", "tool": tool, "input": input })
        }
        Event::PermissionDenied { tool, input } => {
            json!({ "type": "PermissionDenied", "tool": tool, "input": input })
        }
        Event::Usage {
            input_tokens,
            output_tokens,
            cache_read_tokens,
            cost_usd,
        } => json!({
            "type": "Usage", "inputTokens": input_tokens, "outputTokens": output_tokens,
            "cacheReadTokens": cache_read_tokens, "costUsd": cost_usd
        }),
        Event::Done { status, text } => {
            json!({ "type": "Done", "status": status_str(status), "text": text })
        }
        Event::Error(m) => json!({ "type": "Error", "message": m }),
        Event::Unparsed(_) => return None,
    })
}

fn status_str(s: &DoneStatus) -> &'static str {
    match s {
        DoneStatus::Success => "success",
        DoneStatus::Error => "error",
        DoneStatus::Cancelled => "cancelled",
    }
}

/// Persist one UI event of a turn.
pub fn persist(db: &WorkspaceDb, turn_id: &str, event: &Value) -> CmdResult<()> {
    let kind = event
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("Unknown");
    db.conn().execute(
        "INSERT INTO events (id, turn_id, event_type, payload_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![uuid::Uuid::new_v4().to_string(), turn_id, kind, event.to_string(), now()],
    )?;
    Ok(())
}

fn emit(state: &AppState, thread_id: &str, turn_id: &str, event: &Value) {
    state.emit(
        "turn-event",
        json!({ "threadId": thread_id, "turnId": turn_id, "event": event }),
    );
}

/// Context appended to the engine system prompt.
fn build_context(
    ws: &Workspace,
    project: Option<&Project>,
    modules: &[String],
    note: Option<String>,
    is_claude: bool,
) -> String {
    let mut lines = vec![format!(
        "Bạn đang làm việc trong Local Content Studio, workspace \"{}\" (thư mục hiện tại).",
        ws.name()
    )];
    if let Some(p) = project {
        lines.push(format!(
            "Project: {} ({}). Đặt mỗi sản phẩm trong projects/{}/artifacts/<id>/ kèm artifact.yaml (id, kind, title, brand, channels, source).",
            p.title(),
            p.slug(),
            p.slug()
        ));
    }
    if let Ok(set) = studio_brand::BrandSet::load(ws) {
        let join = |v: Vec<&String>| v.into_iter().cloned().collect::<Vec<_>>().join(", ");
        lines.push(format!(
            "Brand có sẵn: {}.",
            join(set.brands.keys().collect())
        ));
        lines.push(format!("Kênh: {}.", join(set.channels.keys().collect())));
        lines.push(format!(
            "Playbook: {}.",
            join(set.playbooks.keys().collect())
        ));
    }
    if !modules.is_empty() {
        lines.push(format!("Module catalog đang bật: {}.", modules.join(", ")));
    }
    if !is_claude {
        lines.push(
            "Các công cụ Studio nằm trên MCP server \"studio\" (gọi qua call_mcp_tool với ServerName \"studio\"): brand_get, channel_get, playbook_get, capability_search, capability_run, check_run, artifact_render. Chỉ thao tác bên trong thư mục workspace hiện tại. Nếu MCP server \"studio\" không tải được hoặc báo lỗi, phải dừng lại và báo lỗi ngay, không đọc tệp ở nơi khác."
                .into(),
        );
    } else {
        lines.push(
            "Dùng các tool MCP của server \"studio\" (brand_get, channel_get, playbook_get, capability_search, capability_run, check_run, artifact_render) để lấy Brand/spec kênh, chạy năng lực và tự kiểm tra sản phẩm trước khi báo xong. Không cài skill bằng npx skills. Chỉ thao tác bên trong thư mục workspace hiện tại."
                .into(),
        );
    }
    lines.push("Trả lời bằng tiếng Việt, ngắn gọn.".into());
    if let Some(n) = note {
        lines.push(n);
    }
    lines.join("\n")
}

/// Thread row needed to run a turn.
struct ThreadRow {
    project: Option<String>,
    engine: String,
    model: Option<String>,
}

fn load_thread(db: &WorkspaceDb, thread_id: &str) -> CmdResult<ThreadRow> {
    db.conn()
        .query_row(
            "SELECT project_slug, engine, model FROM threads WHERE id = ?1",
            [thread_id],
            |r| {
                Ok(ThreadRow {
                    project: r.get(0)?,
                    engine: r.get(1)?,
                    model: r.get(2)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| CmdError::msg("Không tìm thấy hội thoại."))
}

/// Start a turn and return its id; events stream to the UI as `turn-event`.
pub async fn start_turn(state: AppState, thread_id: String, prompt: String) -> CmdResult<String> {
    let ws = state.workspace()?;
    let db = WorkspaceDb::open(&ws)?;
    let thread = load_thread(&db, &thread_id)?;
    let busy = state
        .turns
        .lock()
        .map(|t| t.values().any(|r| r.thread_id == thread_id))
        .unwrap_or(false);
    if busy {
        return Err(CmdError::msg("Hội thoại này đang có một lượt chạy."));
    }

    let turn_id = uuid::Uuid::new_v4().to_string();
    let index: i64 = db.conn().query_row(
        "SELECT COUNT(*) FROM turns WHERE thread_id = ?1",
        [&thread_id],
        |r| r.get(0),
    )?;
    db.conn().execute(
        "INSERT INTO turns (id, thread_id, turn_index, user_prompt, status, created_at) VALUES (?1, ?2, ?3, ?4, 'running', ?5)",
        params![turn_id, thread_id, index + 1, prompt, now()],
    )?;
    CheckpointService::create(
        &ws,
        &format!("Trước lượt {}", index + 1),
        Some(turn_id.clone()),
    )?;
    let prompt_event = json!({ "type": "UserPrompt", "text": prompt });
    persist(&db, &turn_id, &prompt_event)?;
    emit(&state, &thread_id, &turn_id, &prompt_event);

    let project = thread
        .project
        .as_deref()
        .and_then(|s| Project::open(&ws, s).ok());
    let meta = project.as_ref().and_then(|p| p.meta().ok());
    let modules = meta.as_ref().map(|m| m.modules.clone()).unwrap_or_default();
    let is_claude = thread.engine == "claude";

    // Skills of the project's kit/modules, materialized per turn (ADR-010 §4).
    let plugin_dir = if is_claude {
        let dir = ws.cache_dir().join("plugins").join(&turn_id);
        match materialize_skills(
            &state,
            meta.as_ref().and_then(|m| m.kit.clone()),
            &modules,
            &dir,
        ) {
            Ok(()) => Some(dir),
            Err(e) => {
                let note = json!({ "type": "Text", "text": format!("(Không nạp được skill của kit: {e})") });
                emit(&state, &thread_id, &turn_id, &note);
                None
            }
        }
    } else {
        None
    };

    let note_key = format!("thread.note.{thread_id}");
    let note = get_setting(&db, &note_key)?;
    del_setting(&db, &note_key)?;
    let context = build_context(&ws, project.as_ref(), &modules, note, is_claude);

    let token = state.tokens.issue(TurnContext {
        workspace_dir: ws.root().to_path_buf(),
        turn_id: turn_id.clone(),
        project: thread.project.clone(),
        engine: if is_claude {
            McpEngine::Claude
        } else {
            McpEngine::Agy
        },
        allowed_capabilities: None,
        budget_usd: Some(DEFAULT_TURN_BUDGET_USD),
    });
    state.broker.register_turn(&turn_id, &thread_id);

    let session_key = format!("thread.session.{thread_id}");
    let session = match get_setting(&db, &session_key)? {
        Some(id) => SessionRef::Resume(id),
        None => {
            let id = uuid::Uuid::new_v4();
            if is_claude {
                set_setting(&db, &session_key, &id.to_string())?;
            }
            SessionRef::New(id)
        }
    };
    let mcp = state.mcp_port.get().map(|port| McpHttp {
        url: format!("http://127.0.0.1:{port}/mcp"),
        token: token.clone(),
    });
    let req = TurnRequest {
        workspace_dir: ws.root().to_path_buf(),
        prompt,
        session,
        model: thread.model.clone(),
        mcp: if is_claude { mcp } else { None },
        plugin_dir: plugin_dir.clone(),
        permission_prompt_tool: is_claude.then(|| "mcp__studio__approve".to_string()),
        append_system_prompt: Some(context),
        max_budget_usd: None,
    };

    let started = if is_claude {
        state.claude.start_turn(req).await
    } else {
        state.agy.start_turn(req).await
    };
    let turn = match started {
        Ok(t) => t,
        Err(e) => {
            let msg = format!("Không khởi động được engine {}: {e}", thread.engine);
            for ev in [
                json!({ "type": "Error", "message": msg }),
                json!({ "type": "Done", "status": "error" }),
            ] {
                let _ = persist(&db, &turn_id, &ev);
                emit(&state, &thread_id, &turn_id, &ev);
            }
            db.conn().execute(
                "UPDATE turns SET status = 'error', completed_at = ?2 WHERE id = ?1",
                params![turn_id, now()],
            )?;
            state.tokens.revoke(&token);
            state.broker.unregister_turn(&turn_id);
            return Err(CmdError::msg(msg));
        }
    };

    let cancel = Arc::new(Notify::new());
    if let Ok(mut turns) = state.turns.lock() {
        turns.insert(
            turn_id.clone(),
            RunningTurn {
                thread_id: thread_id.clone(),
                cancel: cancel.clone(),
            },
        );
    }
    let ctx = TurnTask {
        state: state.clone(),
        ws_root: ws.root().to_path_buf(),
        thread_id,
        turn_id: turn_id.clone(),
        engine: thread.engine,
        model: thread.model,
        token,
        plugin_dir,
        session_key,
    };
    tauri::async_runtime::spawn(run_turn(ctx, turn, cancel));
    Ok(turn_id)
}

fn materialize_skills(
    state: &AppState,
    kit: Option<String>,
    modules: &[String],
    dir: &Path,
) -> CmdResult<()> {
    let catalog = studio_catalog::Catalog::load_file(&state.catalog_path)?;
    // project.yaml lists the kit's modules too; only modules the kit does not use are "extra".
    // Passing the kit's own modules as extra would load them whole and bypass the kit's narrowing.
    let kit_modules = kit
        .as_deref()
        .and_then(|k| catalog.kits.get(k))
        .map(|k| k.use_modules.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let extra: Vec<&str> = modules
        .iter()
        .filter(|m| !kit_modules.contains(m))
        .map(String::as_str)
        .collect();
    let active = studio_catalog::resolve(&catalog, kit.as_deref(), &extra)?;
    studio_catalog::materialize_plugin(&active, &catalog, &state.sources_dir, dir)?;
    Ok(())
}

struct TurnTask {
    state: AppState,
    ws_root: PathBuf,
    thread_id: String,
    turn_id: String,
    engine: String,
    model: Option<String>,
    token: String,
    plugin_dir: Option<PathBuf>,
    session_key: String,
}

async fn run_turn(t: TurnTask, mut turn: Turn, cancel: Arc<Notify>) {
    // WorkspaceDb is Send but not Sync: keep it behind a mutex across awaits.
    let db = std::sync::Mutex::new(
        Workspace::open(&t.ws_root)
            .ok()
            .and_then(|ws| WorkspaceDb::open(&ws).ok()),
    );
    let with_db = |f: &dyn Fn(&WorkspaceDb)| {
        if let Ok(guard) = db.lock() {
            if let Some(d) = guard.as_ref() {
                f(d);
            }
        }
    };
    let save = |ev: &Value| {
        with_db(&|d| {
            let _ = persist(d, &t.turn_id, ev);
        })
    };
    let mut text = String::new();
    let mut final_status: Option<&'static str> = None;
    let mut cost: Option<f64> = None;

    loop {
        tokio::select! {
            ev = turn.next_event() => {
                let Some(ev) = ev else { break };
                match &ev {
                    Event::Session { id } => with_db(&|d| {
                        if get_setting(d, &t.session_key).ok().flatten().is_none() {
                            let _ = set_setting(d, &t.session_key, id);
                        }
                    }),
                    Event::TextDelta(d) => text.push_str(d),
                    Event::Text(full) => text = full.clone(),
                    Event::Usage { cost_usd, .. } => cost = *cost_usd,
                    Event::Done { status, .. } => {
                        final_status = Some(status_str(status));
                        if !text.is_empty() {
                            save(&json!({ "type": "Text", "text": text }));
                        }
                    }
                    _ => {}
                }
                let Some(ui) = to_ui_event(&ev, &t.ws_root) else { continue };
                emit(&t.state, &t.thread_id, &t.turn_id, &ui);
                if !matches!(ev, Event::TextDelta(_) | Event::Text(_) | Event::Session { .. }) {
                    save(&ui);
                }
            }
            _ = cancel.notified() => {
                let _ = turn.cancel().await;
                final_status = Some("cancelled");
                if !text.is_empty() {
                    save(&json!({ "type": "Text", "text": text }));
                }
                let done = json!({ "type": "Done", "status": "cancelled" });
                emit(&t.state, &t.thread_id, &t.turn_id, &done);
                save(&done);
                break;
            }
        }
    }

    let status = match final_status {
        Some(s) => s,
        None => {
            // Engine exited without a result line.
            if !text.is_empty() {
                save(&json!({ "type": "Text", "text": text }));
            }
            let done = json!({ "type": "Done", "status": "error", "text": "Engine kết thúc mà không trả kết quả." });
            emit(&t.state, &t.thread_id, &t.turn_id, &done);
            save(&done);
            "error"
        }
    };

    with_db(&|db| {
        let _ = db.conn().execute(
            "UPDATE turns SET status = ?2, completed_at = ?3 WHERE id = ?1",
            params![t.turn_id, status, now()],
        );
        let _ = db.conn().execute(
            "UPDATE threads SET updated_at = ?2 WHERE id = ?1",
            params![t.thread_id, now()],
        );
        if let Some(c) = cost {
            let _ = db.conn().execute(
                "INSERT INTO cost_ledger (id, turn_id, job_id, provider, model, cost_usd, details_json, created_at) VALUES (?1, ?2, NULL, ?3, ?4, ?5, NULL, ?6)",
                params![uuid::Uuid::new_v4().to_string(), t.turn_id, t.engine, t.model, c, now()],
            );
        }

        let cap_cost: f64 = db
            .conn()
            .query_row(
                "SELECT COALESCE(SUM(cost_usd), 0.0) FROM cost_ledger WHERE turn_id = ?1 AND job_id IS NOT NULL",
                [&t.turn_id],
                |r| r.get(0),
            )
            .unwrap_or(0.0);

        if cap_cost > 0.0 {
            let existing_usage: Option<(String, Value)> = db
                .conn()
                .query_row(
                    "SELECT id, payload_json FROM events WHERE turn_id = ?1 AND event_type = 'Usage' ORDER BY rowid DESC LIMIT 1",
                    [&t.turn_id],
                    |r| {
                        Ok((
                            r.get(0)?,
                            serde_json::from_str(&r.get::<_, String>(1)?).unwrap_or(Value::Null),
                        ))
                    },
                )
                .ok();

            if let Some((id, mut payload)) = existing_usage {
                if let Some(obj) = payload.as_object_mut() {
                    obj.insert("estimatedCostUsd".into(), json!(cap_cost));
                    let _ = db.conn().execute(
                        "UPDATE events SET payload_json = ?2 WHERE id = ?1",
                        params![id, payload.to_string()],
                    );
                    emit(&t.state, &t.thread_id, &t.turn_id, &payload);
                }
            } else {
                let usage_ev = json!({
                    "type": "Usage",
                    "inputTokens": 0,
                    "outputTokens": 0,
                    "cacheReadTokens": 0,
                    "costUsd": null,
                    "estimatedCostUsd": cap_cost,
                });
                let _ = persist(db, &t.turn_id, &usage_ev);
                emit(&t.state, &t.thread_id, &t.turn_id, &usage_ev);
            }
        }
    });
    let state = &t.state;
    state.tokens.revoke(&t.token);
    state.broker.unregister_turn(&t.turn_id);
    if let Ok(mut turns) = state.turns.lock() {
        turns.remove(&t.turn_id);
    }
    if let Some(dir) = &t.plugin_dir {
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// Undo a turn: restore the checkpoint taken before it and drop its events.
pub fn undo_turn(state: &AppState, turn_id: &str) -> CmdResult<bool> {
    let ws = state.workspace()?;
    if state
        .turns
        .lock()
        .map(|t| t.contains_key(turn_id))
        .unwrap_or(false)
    {
        return Err(CmdError::msg(
            "Lượt đang chạy; hãy dừng trước khi hoàn tác.",
        ));
    }
    CheckpointService::undo_turn(&ws, turn_id).map_err(|e| CmdError::msg(e.to_string()))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_context_for_agy_and_claude() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = Workspace::create(tmp.path().join("ws"), "Test Workspace").unwrap();

        let agy_ctx = build_context(&ws, None, &[], None, false);
        assert!(agy_ctx.contains("MCP server \"studio\""));
        assert!(agy_ctx.contains("call_mcp_tool"));
        assert!(agy_ctx.contains("ServerName \"studio\""));
        assert!(agy_ctx.contains("Chỉ thao tác bên trong thư mục workspace hiện tại."));
        assert!(agy_ctx.contains("Nếu MCP server \"studio\" không tải được hoặc báo lỗi, phải dừng lại và báo lỗi ngay, không đọc tệp ở nơi khác."));
        assert!(agy_ctx.contains("artifact_render"));

        let claude_ctx = build_context(&ws, None, &[], None, true);
        assert!(claude_ctx.contains("Dùng các tool MCP của server \"studio\""));
        assert!(!claude_ctx.contains("call_mcp_tool"));
    }
}
