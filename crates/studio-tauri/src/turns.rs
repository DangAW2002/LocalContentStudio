//! Agent turn orchestration: checkpoint → skills → token → engine → stream/persist events (plan 0009 §7, ADR-008).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
use studio_engine::{DoneStatus, Engine, Event, FileOp, McpHttp, SessionRef, Turn, TurnRequest};
use studio_mcp::{EngineKind as McpEngine, TurnContext};
use studio_workspace::{CheckpointService, Project, StallWatchdogConfig, Workspace, WorkspaceDb};
use tokio::sync::Notify;

use crate::error::{CmdError, CmdResult};
use crate::state::{AppState, RunningTurn};

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
        Event::Thinking(text) => json!({ "type": "Thinking", "text": text }),
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
            cache_write_tokens,
            reasoning_tokens,
            cost_usd,
        } => json!({
            "type": "Usage", "inputTokens": input_tokens, "outputTokens": output_tokens,
            "cacheReadTokens": cache_read_tokens, "cacheWriteTokens": cache_write_tokens,
            "reasoningTokens": reasoning_tokens, "costUsd": cost_usd
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
    agy_full_access: bool,
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

        // Content graph summary (ADR-011 §5 / Task G1 §5)
        if let Ok(artifacts) = studio_artifacts::Artifact::list(p) {
            if let Ok(graph) = studio_artifacts::build_project_graph(ws, p, &artifacts) {
                let pinned_ids: Vec<&str> = graph
                    .nodes
                    .iter()
                    .filter(|n| n.pinned)
                    .map(|n| n.id.as_str())
                    .collect();
                let stale_count = graph
                    .nodes
                    .iter()
                    .filter(|n| n.status == studio_artifacts::NodeStatus::LoiThoi)
                    .count();
                let mut user_notes: Vec<String> = Vec::new();
                for node in &graph.nodes {
                    for n in &node.notes {
                        let truncated = if n.chars().count() > 120 {
                            format!("{}…", n.chars().take(120).collect::<String>().trim_end())
                        } else {
                            n.clone()
                        };
                        user_notes.push(format!("{}: \"{}\"", node.id, truncated));
                        if user_notes.len() >= 5 {
                            break;
                        }
                    }
                    if user_notes.len() >= 5 {
                        break;
                    }
                }

                let mut parts: Vec<String> = Vec::new();
                if !pinned_ids.is_empty() {
                    parts.push(format!("Đã khóa: {}", pinned_ids.join(", ")));
                }
                if stale_count > 0 {
                    parts.push(format!("{stale_count} artifact lỗi thời"));
                }
                if !user_notes.is_empty() {
                    parts.push(format!("Ghi chú: {}", user_notes.join("; ")));
                }

                if !parts.is_empty() {
                    let mut summary = format!("Sơ đồ nội dung: {}.", parts.join(" | "));
                    if summary.chars().count() > 400 {
                        summary = format!(
                            "{}…",
                            summary.chars().take(399).collect::<String>().trim_end()
                        );
                    }
                    lines.push(summary);
                }
            }
        }

        // Series and reference profiles summary (ADR-011 §5 / Task G3 §3)
        if let Ok(meta) = p.meta() {
            if let Some(series_slug) = meta.series {
                if let Ok(series) = studio_workspace::series::SeriesService::get(ws, &series_slug) {
                    let refs_str = if series.references.is_empty() {
                        "không có".to_string()
                    } else {
                        series.references.join(", ")
                    };
                    lines.push(format!(
                        "Series đang áp dụng: {} ({}), hồ sơ: {}.",
                        series.title, series.slug, refs_str
                    ));
                }
            }
        }
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
        lines.push(
            "Trước khi làm, gọi skill_list (MCP \"studio\") để xem skill của kit, rồi đọc skill phù hợp bằng skill_get và làm theo; tệp tham chiếu trong skill đọc bằng skill_get với tham số file."
                .into(),
        );
        lines.push(
            if agy_full_access {
                "Bạn được chạy lệnh shell nhưng chỉ trong thư mục workspace hiện tại, không xoá/sửa tệp ngoài workspace."
            } else {
                "Mặc định lệnh shell bị chặn, dùng công cụ đọc/ghi tệp và MCP \"studio\" thay thế."
            }
                .into(),
        );
    } else {
        lines.push(
            "Dùng các tool MCP của server \"studio\" (brand_get, channel_get, playbook_get, capability_search, capability_run, check_run, artifact_render) để lấy Brand/spec kênh, chạy năng lực và tự kiểm tra sản phẩm trước khi báo xong. Không cài skill bằng npx skills. Chỉ thao tác bên trong thư mục workspace hiện tại."
                .into(),
        );
        // Bash needs the owner's approval each time; the file tools do not.
        lines.push(
            "Tạo/sửa tệp bằng tool Write/Edit và đọc bằng Read (artifact.yaml, index.html, timeline.json…), không dùng Bash (heredoc, sed, python) cho việc đó: mỗi lệnh Bash phải chờ người dùng duyệt."
                .into(),
        );
    }
    lines.push("Trả lời bằng tiếng Việt, ngắn gọn.".into());
    if let Some(n) = note {
        lines.push(n);
    }
    lines.join("\n")
}

/// First line of a prompt, shortened to a thread title.
/// Drops leading `/<playbook>` and `[Áp dụng Brand: …]` tags so titles show the request itself.
fn strip_prompt_prefixes<'a>(prompt: &'a str, playbooks: &[String]) -> &'a str {
    let mut names: Vec<&str> = playbooks.iter().map(String::as_str).collect();
    names.sort_by_key(|n| std::cmp::Reverse(n.len()));
    let mut rest = prompt.trim_start();
    loop {
        if rest.starts_with("[Áp dụng Brand:") {
            if let Some(end) = rest.find(']') {
                rest = rest[end + 1..].trim_start();
                continue;
            }
        }
        let tag = names.iter().find_map(|n| {
            let after = rest.strip_prefix('/')?.strip_prefix(*n)?;
            (after.is_empty() || after.starts_with(char::is_whitespace)).then_some(after)
        });
        match tag {
            Some(after) => rest = after.trim_start(),
            None => break,
        }
    }
    if rest.is_empty() {
        prompt
    } else {
        rest
    }
}

fn thread_title_from_prompt(prompt: &str) -> String {
    const MAX_CHARS: usize = 48;
    let line = prompt
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("Hội thoại mới");
    if line.chars().count() <= MAX_CHARS {
        line.to_string()
    } else {
        format!(
            "{}…",
            line.chars().take(MAX_CHARS).collect::<String>().trim_end()
        )
    }
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

    // Check-and-reserve under one lock so two concurrent sends cannot both start a turn.
    let turn_id = uuid::Uuid::new_v4().to_string();
    let cancel = Arc::new(Notify::new());
    {
        let mut turns = state
            .turns
            .lock()
            .map_err(|_| CmdError::msg("Trạng thái lượt chạy bị khoá hỏng."))?;
        if turns.values().any(|r| r.thread_id == thread_id) {
            return Err(CmdError::msg("Hội thoại này đang có một lượt chạy."));
        }
        turns.insert(
            turn_id.clone(),
            RunningTurn {
                thread_id: thread_id.clone(),
                cancel: cancel.clone(),
            },
        );
    }
    let mut reservation = TurnReservation {
        state: &state,
        turn_id: &turn_id,
        armed: true,
    };

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
    // Name an untitled thread after its first prompt (like chat apps do), so threads are
    // distinguishable in the switcher.
    if index == 0 {
        let playbooks: Vec<String> = crate::commands::scope_list_impl(&state, "playbooks".into())
            .unwrap_or_default()
            .iter()
            .filter_map(|v| v.get("name").and_then(Value::as_str).map(String::from))
            .collect();
        let title = thread_title_from_prompt(strip_prompt_prefixes(&prompt, &playbooks));
        let renamed = db.conn().execute(
            "UPDATE threads SET title = ?2 WHERE id = ?1 AND (title IS NULL OR title = 'Hội thoại mới')",
            params![thread_id, title],
        )?;
        if renamed > 0 {
            state.emit(
                "thread-updated",
                json!({ "threadId": thread_id, "title": title }),
            );
        }
    }
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

    // Skills of the project's kit/modules, materialized per turn (ADR-010 §4). Claude loads them
    // via --plugin-dir; every engine can read them through the MCP skill_list/skill_get tools.
    let plugin_dir = {
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
    };

    let note_key = format!("thread.note.{thread_id}");
    let note = get_setting(&db, &note_key)?;
    del_setting(&db, &note_key)?;
    let agy_setting = state
        .app_paths
        .open_app_db()
        .ok()
        .and_then(|adb| adb.get_setting("engine:agy_full_access").ok().flatten());
    let skip_permissions = !is_claude && agy_setting.as_deref() == Some("true");
    let context = build_context(
        &ws,
        project.as_ref(),
        &modules,
        note,
        is_claude,
        skip_permissions,
    );

    let turn_budget = crate::budget::calculate_turn_budget(&ws);
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
        budget_usd: turn_budget,
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
        plugin_dir: if is_claude { plugin_dir.clone() } else { None },
        permission_prompt_tool: is_claude.then(|| "mcp__studio__approve".to_string()),
        append_system_prompt: Some(context),
        max_budget_usd: None,
        skip_permissions,
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
            if let Some(dir) = &plugin_dir {
                let _ = std::fs::remove_dir_all(dir);
            }
            return Err(CmdError::msg(msg));
        }
    };

    // run_turn owns the slot from here and releases it when the turn ends.
    reservation.armed = false;
    drop(reservation);
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
        project_dir: project.as_ref().map(|p| p.root().to_path_buf()),
    };
    tauri::async_runtime::spawn(run_turn(ctx, turn, cancel));
    Ok(turn_id)
}

/// Releases a reserved turn slot if `start_turn` bails out before the turn task takes over.
struct TurnReservation<'a> {
    state: &'a AppState,
    turn_id: &'a str,
    armed: bool,
}

impl Drop for TurnReservation<'_> {
    fn drop(&mut self) {
        if self.armed {
            if let Ok(mut turns) = self.state.turns.lock() {
                turns.remove(self.turn_id);
            }
        }
    }
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

/// Clock abstraction for testable time and sleep/resume jump detection.
pub trait Clock: Send + Sync {
    /// Monotonic instant for measuring elapsed durations.
    fn now(&self) -> std::time::Instant;
    /// Wall-clock system time for detecting sleep/resume jumps.
    fn wall_now(&self) -> std::time::SystemTime;
}

/// Production clock using system time.
#[derive(Debug, Default, Clone, Copy)]
pub struct RealClock;

impl Clock for RealClock {
    fn now(&self) -> std::time::Instant {
        std::time::Instant::now()
    }

    fn wall_now(&self) -> std::time::SystemTime {
        std::time::SystemTime::now()
    }
}

/// Mock clock for unit tests that allows explicit advancing of monotonic and wall time.
#[cfg(test)]
#[derive(Debug, Clone)]
pub struct MockClock {
    base_instant: std::time::Instant,
    offset: Arc<std::sync::Mutex<std::time::Duration>>,
    base_wall: std::time::SystemTime,
    wall_offset: Arc<std::sync::Mutex<std::time::Duration>>,
}

#[cfg(test)]
impl MockClock {
    /// Creates a new mock clock initialized to the current system time.
    pub fn new() -> Self {
        Self {
            base_instant: std::time::Instant::now(),
            offset: Arc::new(std::sync::Mutex::new(std::time::Duration::ZERO)),
            base_wall: std::time::SystemTime::now(),
            wall_offset: Arc::new(std::sync::Mutex::new(std::time::Duration::ZERO)),
        }
    }

    /// Advances monotonic time by `d`.
    pub fn advance(&self, d: std::time::Duration) {
        if let Ok(mut off) = self.offset.lock() {
            *off += d;
        }
    }

    /// Advances wall-clock time by `d` (simulating sleep/resume wall-clock jump).
    pub fn advance_wall(&self, d: std::time::Duration) {
        if let Ok(mut woff) = self.wall_offset.lock() {
            *woff += d;
        }
    }
}

#[cfg(test)]
impl Default for MockClock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl Clock for MockClock {
    fn now(&self) -> std::time::Instant {
        let off = self.offset.lock().map(|g| *g).unwrap_or_default();
        self.base_instant + off
    }

    fn wall_now(&self) -> std::time::SystemTime {
        let off = self.wall_offset.lock().map(|g| *g).unwrap_or_default();
        self.base_wall + off
    }
}

pub struct TurnTask {
    pub state: AppState,
    pub ws_root: PathBuf,
    pub thread_id: String,
    pub turn_id: String,
    pub engine: String,
    pub model: Option<String>,
    pub token: String,
    pub plugin_dir: Option<PathBuf>,
    pub session_key: String,
    /// Project folder of the thread; receives agy brain artifacts.
    pub project_dir: Option<PathBuf>,
}

pub async fn run_turn(t: TurnTask, turn: Turn, cancel: Arc<Notify>) {
    let ws_meta = Workspace::open(&t.ws_root)
        .ok()
        .and_then(|ws| ws.meta().ok());
    let config = ws_meta
        .as_ref()
        .map(|m| m.stall_watchdog_config())
        .unwrap_or_default();
    run_turn_inner(t, turn, cancel, config, Arc::new(RealClock)).await;
}

pub async fn run_turn_inner<C: Clock + 'static>(
    t: TurnTask,
    mut turn: Turn,
    cancel: Arc<Notify>,
    config: StallWatchdogConfig,
    clock: Arc<C>,
) {
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

    let mut open_tool_calls: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut emitted_file_paths: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    // agy artifacts touched from here on belong to this turn (slack for coarse mtimes).
    let turn_started = std::time::SystemTime::now() - std::time::Duration::from_secs(2);
    let mut conversation_id: Option<String> = None;
    let mut last_event_instant = clock.now();
    let mut last_wall_tick = clock.wall_now();
    let mut resume_grace_deadline: Option<std::time::Instant> = None;

    let mut watchdog_interval = tokio::time::interval(config.tick_interval);
    watchdog_interval.tick().await;

    loop {
        tokio::select! {
            ev = turn.next_event() => {
                let Some(ev) = ev else { break };
                last_event_instant = clock.now();
                resume_grace_deadline = None;

                match &ev {
                    Event::ToolCall { id, .. } => {
                        open_tool_calls.insert(id.clone());
                    }
                    Event::ToolResult { id, .. } => {
                        open_tool_calls.remove(id);
                    }
                    Event::Session { id } => {
                        conversation_id = Some(id.clone());
                        with_db(&|d| {
                            if get_setting(d, &t.session_key).ok().flatten().is_none() {
                                let _ = set_setting(d, &t.session_key, id);
                            }
                        })
                    }
                    // Brain edits surface as the synced project copy after the turn.
                    Event::FileChange { path, .. } if crate::agy_artifacts::is_brain_path(path) => continue,
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
                if let Event::FileChange { .. } = &ev {
                    if let Some(p) = ui.get("path").and_then(|v| v.as_str()) {
                        emitted_file_paths.insert(p.replace('\\', "/"));
                    }
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
            _ = watchdog_interval.tick() => {
                let now = clock.now();
                let wall_now = clock.wall_now();

                // Detect long wall-clock jump (sleep/resume)
                let wall_gap = wall_now.duration_since(last_wall_tick).unwrap_or(std::time::Duration::ZERO);
                last_wall_tick = wall_now;

                if wall_gap > config.wall_jump_threshold && resume_grace_deadline.is_none() {
                    resume_grace_deadline = Some(now + config.resume_grace);
                }

                let has_pending_perm = t.state.broker.list_pending(Some(&t.thread_id)).iter().any(|p| {
                    p.get("turnId").and_then(|v| v.as_str()) == Some(&t.turn_id)
                });

                let has_running_job = {
                    let snap = t.state.scheduler.snapshot().await;
                    snap.iter().any(|j| matches!(j, studio_jobs::JobStatus::Running { .. } | studio_jobs::JobStatus::Queued { .. }))
                };

                let mut is_stalled = false;
                let mut stall_limit_used = config.stall_after;
                let mut after_resume = false;

                // After a sleep/resume, a stream that stays silent is most likely dead. Not while a
                // tool is running though: a render or install keeps going silently after resume.
                if let Some(deadline) = resume_grace_deadline {
                    if now >= deadline
                        && open_tool_calls.is_empty()
                        && !has_pending_perm
                        && !has_running_job
                    {
                        is_stalled = true;
                        after_resume = true;
                    }
                }

                if !is_stalled {
                    let limit = if !open_tool_calls.is_empty() {
                        config.stall_after_tool
                    } else {
                        config.stall_after
                    };
                    stall_limit_used = limit;

                    let silence_duration = now.duration_since(last_event_instant);
                    if silence_duration >= limit {
                        if open_tool_calls.is_empty() {
                            if !has_pending_perm && !has_running_job {
                                is_stalled = true;
                            }
                        } else if !has_pending_perm {
                            is_stalled = true;
                        }
                    }
                }

                if is_stalled {
                    let _ = turn.cancel().await;
                    final_status = Some("error");
                    if !text.is_empty() {
                        save(&json!({ "type": "Text", "text": text }));
                    }

                    let duration_str = if stall_limit_used.as_secs() >= 60 {
                        format!("{} phút", stall_limit_used.as_secs() / 60)
                    } else if stall_limit_used.as_secs() > 0 {
                        format!("{} giây", stall_limit_used.as_secs())
                    } else {
                        format!("{} ms", stall_limit_used.as_millis())
                    };

                    let stall_msg = if after_resume {
                        "Lượt không phản hồi sau khi máy thức dậy (kết nối tới engine đã mất) nên đã dừng. Bấm Thử lại để chạy lại lượt.".to_string()
                    } else {
                        format!(
                            "Lượt không phản hồi trong {duration_str} (có thể do máy ngủ hoặc mất mạng) nên đã dừng. Bấm Thử lại để chạy lại lượt."
                        )
                    };
                    let err_ev = json!({ "type": "Error", "message": stall_msg });
                    emit(&t.state, &t.thread_id, &t.turn_id, &err_ev);
                    save(&err_ev);

                    let done = json!({ "type": "Done", "status": "error" });
                    emit(&t.state, &t.thread_id, &t.turn_id, &done);
                    save(&done);

                    break;
                }
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

    if t.engine == "agy" {
        // A resumed conversation may not repeat its id: fall back to the stored session.
        if conversation_id.is_none() {
            if let Ok(guard) = db.lock() {
                conversation_id = guard
                    .as_ref()
                    .and_then(|d| get_setting(d, &t.session_key).ok().flatten());
            }
        }
        let src = conversation_id
            .as_deref()
            .and_then(crate::agy_artifacts::brain_dir);
        let dest = t
            .project_dir
            .clone()
            .unwrap_or_else(|| t.ws_root.clone())
            .join(crate::agy_artifacts::DEST_DIR);
        if let Some(src) = src {
            for (path, existed) in crate::agy_artifacts::sync(&src, &dest, turn_started) {
                let rel = path
                    .strip_prefix(&t.ws_root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                let op = if existed { "modify" } else { "create" };
                let ev = json!({ "type": "FileChange", "path": rel, "op": op });
                emit(&t.state, &t.thread_id, &t.turn_id, &ev);
                save(&ev);
                emitted_file_paths.insert(rel);
            }
        }
    }

    // Synchronize workspace file changes that were not explicitly emitted as events
    // (e.g. files created via Studio MCP, custom tools, or shell commands).
    let ws = t.state.workspace().or_else(|_| Workspace::open(&t.ws_root));
    if let Ok(ws) = ws {
        if let Ok(cps) = CheckpointService::list(&ws) {
            if let Some(pre_cp) = cps
                .into_iter()
                .find(|c| c.turn_id.as_deref() == Some(&t.turn_id))
            {
                if let Ok(diff) = CheckpointService::changed_since(&ws, &pre_cp.id, turn_started) {
                    for path in diff.added {
                        if !path.starts_with(".studio/")
                            && !path.starts_with(".studio\\")
                            && !emitted_file_paths.contains(&path)
                        {
                            let ev = json!({ "type": "FileChange", "path": path, "op": "create" });
                            emit(&t.state, &t.thread_id, &t.turn_id, &ev);
                            save(&ev);
                            emitted_file_paths.insert(path);
                        }
                    }
                    for path in diff.modified {
                        if !path.starts_with(".studio/")
                            && !path.starts_with(".studio\\")
                            && !emitted_file_paths.contains(&path)
                        {
                            let ev = json!({ "type": "FileChange", "path": path, "op": "modify" });
                            emit(&t.state, &t.thread_id, &t.turn_id, &ev);
                            save(&ev);
                            emitted_file_paths.insert(path);
                        }
                    }
                    for path in diff.deleted {
                        if !path.starts_with(".studio/")
                            && !path.starts_with(".studio\\")
                            && !emitted_file_paths.contains(&path)
                        {
                            let ev = json!({ "type": "FileChange", "path": path, "op": "delete" });
                            emit(&t.state, &t.thread_id, &t.turn_id, &ev);
                            save(&ev);
                            emitted_file_paths.insert(path);
                        }
                    }
                }
            }
        }
    }

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

        // Post-turn lock check (ADR-011 §5 / Task G2 §4)
        if let Ok(ws) = t.state.workspace() {
            if let Ok(cps) = studio_workspace::CheckpointService::list(&ws) {
                if let Some(pre_cp) = cps
                    .into_iter()
                    .find(|c| c.turn_id.as_deref() == Some(&t.turn_id))
                {
                    if let Ok(modified_pinned) =
                        studio_artifacts::check_pinned_modifications_after_turn(&ws, &pre_cp.id)
                    {
                        for art_id in modified_pinned {
                            let warn_ev = json!({
                                "type": "Text",
                                "text": format!("Cảnh báo: Agent đã sửa tệp thuộc artifact khoá '{}' trong lượt này.", art_id),
                                "pinnedArtifactWarning": {
                                    "artifactId": art_id,
                                    "checkpointId": pre_cp.id
                                }
                            });
                            let _ = persist(db, &t.turn_id, &warn_ev);
                            emit(&t.state, &t.thread_id, &t.turn_id, &warn_ev);
                        }
                    }
                }
            }
        }

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
                    "cacheWriteTokens": 0,
                    "reasoningTokens": 0,
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
    fn thread_title_is_first_line_shortened() {
        assert_eq!(
            thread_title_from_prompt("  \n  Viết kịch bản video\nchi tiết"),
            "Viết kịch bản video"
        );
        let long = "á".repeat(60);
        let t = thread_title_from_prompt(&long);
        assert_eq!(t.chars().count(), 49);
        assert!(t.ends_with('…'));
        assert_eq!(thread_title_from_prompt("   "), "Hội thoại mới");
        let pbs = vec![
            "YouTube Video".to_string(),
            "YouTube Video Chuẩn".to_string(),
        ];
        assert_eq!(
            strip_prompt_prefixes(
                "/YouTube Video Chuẩn [Áp dụng Brand: A (id: a)] /YouTube Video Chuẩn làm video",
                &pbs
            ),
            "làm video"
        );
        assert_eq!(
            strip_prompt_prefixes("/YouTube Video Chuẩn", &pbs),
            "/YouTube Video Chuẩn"
        );
        assert_eq!(
            strip_prompt_prefixes("/khác làm video", &pbs),
            "/khác làm video"
        );
    }

    #[test]
    fn test_build_context_for_agy_and_claude() {
        let _lock = crate::commands::tests::ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let app_dir = tmp.path().join("app_data");
        std::env::set_var("STUDIO_APP_DIR", &app_dir);
        let ws = Workspace::create(tmp.path().join("ws"), "Test Workspace").unwrap();

        let agy_ctx = build_context(&ws, None, &[], None, false, false);
        assert!(agy_ctx.contains("lệnh shell bị chặn"));
        assert!(agy_ctx.contains("dùng công cụ đọc/ghi tệp và MCP \"studio\" thay thế"));
        let agy_full_ctx = build_context(&ws, None, &[], None, false, true);
        assert!(agy_full_ctx.contains("được chạy lệnh shell nhưng chỉ trong thư mục workspace hiện tại, không xoá/sửa tệp ngoài workspace"));
        assert!(!agy_full_ctx.contains("lệnh shell bị chặn"));
        assert!(agy_ctx.contains("MCP server \"studio\""));
        assert!(agy_ctx.contains("call_mcp_tool"));
        assert!(agy_ctx.contains("skill_list"));
        assert!(agy_ctx.contains("ServerName \"studio\""));
        assert!(agy_ctx.contains("Chỉ thao tác bên trong thư mục workspace hiện tại."));
        assert!(agy_ctx.contains("Nếu MCP server \"studio\" không tải được hoặc báo lỗi, phải dừng lại và báo lỗi ngay, không đọc tệp ở nơi khác."));
        assert!(agy_ctx.contains("artifact_render"));

        let claude_ctx = build_context(&ws, None, &[], None, true, false);
        assert!(claude_ctx.contains("Dùng các tool MCP của server \"studio\""));
        assert!(claude_ctx.contains("Tạo/sửa tệp bằng tool Write/Edit"));
        assert!(!claude_ctx.contains("call_mcp_tool"));
    }

    #[cfg(windows)]
    fn is_process_running(pid: u32) -> bool {
        let output = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid)])
            .output();
        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.contains(&pid.to_string())
        } else {
            false
        }
    }

    #[cfg(not(windows))]
    fn is_process_running(pid: u32) -> bool {
        let status_path = format!("/proc/{}/status", pid);
        if let Ok(content) = std::fs::read_to_string(&status_path) {
            for line in content.lines() {
                if let Some(rest) = line.strip_prefix("State:") {
                    let state = rest.trim();
                    // Z (zombie) means process has exited and terminated
                    return !state.starts_with('Z');
                }
            }
        }
        let output = std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output();
        if let Ok(out) = output {
            out.status.success()
        } else {
            false
        }
    }

    /// Waits until the turn loop has stored an event of `event_type`. Advancing the mock
    /// clock before the loop has seen the event moves "last activity" forward and makes
    /// the stall assertions flaky.
    async fn wait_for_event(db: &WorkspaceDb, turn_id: &str, event_type: &str) {
        for _ in 0..500 {
            let n: i64 = db
                .conn()
                .query_row(
                    "SELECT COUNT(*) FROM events WHERE turn_id = ?1 AND event_type = ?2",
                    params![turn_id, event_type],
                    |r| r.get(0),
                )
                .unwrap();
            if n > 0 {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("turn {turn_id} never stored a {event_type} event");
    }

    async fn setup_test_turn_env(
        turn_id: &str,
        thread_id: &str,
    ) -> (
        tempfile::TempDir,
        Workspace,
        WorkspaceDb,
        AppState,
        TurnTask,
        Arc<Notify>,
    ) {
        let lock = crate::commands::tests::ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let app_dir = tmp.path().join("app_data");
        std::env::set_var("STUDIO_APP_DIR", &app_dir);
        let ws = Workspace::create(tmp.path().join("ws"), "Test WS").unwrap();
        let db = WorkspaceDb::open(&ws).unwrap();
        let state = AppState::new();
        if let Ok(mut cur) = state.current.lock() {
            *cur = Some(ws.root().to_path_buf());
        }
        drop(lock);

        db.conn()
            .execute(
                "INSERT INTO threads (id, title, engine, created_at, updated_at) VALUES (?1, 'Test', 'claude', '2026-01-01', '2026-01-01')",
                params![thread_id],
            )
            .unwrap();

        db.conn()
            .execute(
                "INSERT INTO turns (id, thread_id, turn_index, user_prompt, status, created_at) VALUES (?1, ?2, 1, 'Hello', 'running', '2026-01-01')",
                params![turn_id, thread_id],
            )
            .unwrap();

        let cancel = Arc::new(Notify::new());
        if let Ok(mut turns) = state.turns.lock() {
            turns.insert(
                turn_id.to_string(),
                RunningTurn {
                    thread_id: thread_id.to_string(),
                    cancel: cancel.clone(),
                },
            );
        }

        let task = TurnTask {
            state: state.clone(),
            ws_root: ws.root().to_path_buf(),
            thread_id: thread_id.to_string(),
            turn_id: turn_id.to_string(),
            engine: "claude".to_string(),
            model: Some("sonnet".to_string()),
            token: "test-token".to_string(),
            plugin_dir: None,
            session_key: format!("thread.session.{thread_id}"),
            project_dir: None,
        };

        (tmp, ws, db, state, task, cancel)
    }

    #[tokio::test]
    async fn test_stall_silence_without_open_tool() {
        let (_tmp, _ws, db, state, task, cancel) = setup_test_turn_env("turn-a", "thread-a").await;
        let (_tx, rx) = tokio::sync::mpsc::channel(10);
        let turn = Turn::mock(rx, None);
        let clock = Arc::new(MockClock::new());
        let config = StallWatchdogConfig {
            stall_after: std::time::Duration::from_secs(600),
            stall_after_tool: std::time::Duration::from_secs(2700),
            tick_interval: std::time::Duration::from_millis(10),
            wall_jump_threshold: std::time::Duration::from_secs(120),
            resume_grace: std::time::Duration::from_secs(90),
        };

        let handle = tokio::spawn(run_turn_inner(task, turn, cancel, config, clock.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        // Within limit, not stalled
        clock.advance(std::time::Duration::from_secs(300));
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        let status: String = db
            .conn()
            .query_row("SELECT status FROM turns WHERE id = 'turn-a'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(status, "running");

        // Advance past limit (600s)
        clock.advance(std::time::Duration::from_secs(350));
        let _ = tokio::time::timeout(std::time::Duration::from_secs(5), handle)
            .await
            .unwrap();

        // Stalled with error
        let final_status: String = db
            .conn()
            .query_row("SELECT status FROM turns WHERE id = 'turn-a'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(final_status, "error");

        let err_payload: String = db
            .conn()
            .query_row(
                "SELECT payload_json FROM events WHERE turn_id = 'turn-a' AND event_type = 'Error' ORDER BY rowid DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(err_payload.contains("Lượt không phản hồi trong 10 phút (có thể do máy ngủ hoặc mất mạng) nên đã dừng. Bấm Thử lại để chạy lại lượt."));

        let has_done: bool = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM events WHERE turn_id = 'turn-a' AND event_type = 'Done'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap()
            > 0;
        assert!(has_done);

        // Turn is removed from state.turns
        assert!(!state.turns.lock().unwrap().contains_key("turn-a"));
    }

    #[tokio::test]
    async fn test_stall_open_tool_silence_below_tool_limit_not_stalled() {
        let (_tmp, _ws, db, _state, task, cancel) = setup_test_turn_env("turn-b", "thread-b").await;
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        let turn = Turn::mock(rx, None);
        let clock = Arc::new(MockClock::new());
        let config = StallWatchdogConfig {
            stall_after: std::time::Duration::from_secs(600),
            stall_after_tool: std::time::Duration::from_secs(2700),
            tick_interval: std::time::Duration::from_millis(10),
            wall_jump_threshold: std::time::Duration::from_secs(120),
            resume_grace: std::time::Duration::from_secs(90),
        };

        let handle = tokio::spawn(run_turn_inner(task, turn, cancel, config, clock.clone()));

        // Tool call starts
        tx.send(Event::ToolCall {
            id: "call-1".to_string(),
            name: "bash".to_string(),
            input: json!({"cmd": "sleep 10"}),
        })
        .await
        .unwrap();

        wait_for_event(&db, "turn-b", "ToolCall").await;

        // Advance 1000s (> 600s stall_after, but < 2700s stall_after_tool)
        clock.advance(std::time::Duration::from_secs(1000));
        tokio::time::sleep(std::time::Duration::from_millis(40)).await;

        let status: String = db
            .conn()
            .query_row("SELECT status FROM turns WHERE id = 'turn-b'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(status, "running");

        // Advance past stall_after_tool (> 2700s)
        clock.advance(std::time::Duration::from_secs(2000));
        let _ = tokio::time::timeout(std::time::Duration::from_secs(5), handle)
            .await
            .unwrap();

        let final_status: String = db
            .conn()
            .query_row("SELECT status FROM turns WHERE id = 'turn-b'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(final_status, "error");

        let err_payload: String = db
            .conn()
            .query_row(
                "SELECT payload_json FROM events WHERE turn_id = 'turn-b' AND event_type = 'Error' ORDER BY rowid DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(err_payload.contains("45 phút"));
    }

    #[tokio::test]
    async fn test_stall_pending_permission_not_stalled() {
        let (_tmp, _ws, db, state, task, cancel) = setup_test_turn_env("turn-c", "thread-c").await;
        let (_tx, rx) = tokio::sync::mpsc::channel(10);
        let turn = Turn::mock(rx, None);
        let clock = Arc::new(MockClock::new());
        let config = StallWatchdogConfig {
            stall_after: std::time::Duration::from_secs(600),
            stall_after_tool: std::time::Duration::from_secs(2700),
            tick_interval: std::time::Duration::from_millis(10),
            wall_jump_threshold: std::time::Duration::from_secs(120),
            resume_grace: std::time::Duration::from_secs(90),
        };

        // Insert pending permission
        state
            .broker
            .insert_pending_for_test("req-1", "thread-c", "turn-c");

        let handle = tokio::spawn(run_turn_inner(
            task,
            turn,
            cancel.clone(),
            config,
            clock.clone(),
        ));

        // Advance past stall_after (700s)
        clock.advance(std::time::Duration::from_secs(700));
        tokio::time::sleep(std::time::Duration::from_millis(40)).await;

        let status: String = db
            .conn()
            .query_row("SELECT status FROM turns WHERE id = 'turn-c'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(status, "running");

        // Now cancel clean
        cancel.notify_one();
        let _ = tokio::time::timeout(std::time::Duration::from_secs(5), handle)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn resume_does_not_stop_a_turn_while_a_tool_runs() {
        let (_tmp, _ws, db, _state, task, cancel) = setup_test_turn_env("turn-r", "thread-r").await;
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        let turn = Turn::mock(rx, None);
        let clock = Arc::new(MockClock::new());
        let config = StallWatchdogConfig {
            stall_after: std::time::Duration::from_secs(600),
            stall_after_tool: std::time::Duration::from_secs(2700),
            tick_interval: std::time::Duration::from_millis(10),
            wall_jump_threshold: std::time::Duration::from_secs(120),
            resume_grace: std::time::Duration::from_secs(90),
        };
        let handle = tokio::spawn(run_turn_inner(
            task,
            turn,
            cancel.clone(),
            config,
            clock.clone(),
        ));
        tx.send(Event::ToolCall {
            id: "render-1".to_string(),
            name: "bash".to_string(),
            input: json!({"cmd": "ffmpeg ..."}),
        })
        .await
        .unwrap();
        wait_for_event(&db, "turn-r", "ToolCall").await;

        // Sleep/resume while the tool runs, then well past the grace window.
        clock.advance_wall(std::time::Duration::from_secs(600));
        clock.advance(std::time::Duration::from_secs(5));
        tokio::time::sleep(std::time::Duration::from_millis(40)).await;
        clock.advance(std::time::Duration::from_secs(200));
        tokio::time::sleep(std::time::Duration::from_millis(60)).await;

        let status: String = db
            .conn()
            .query_row("SELECT status FROM turns WHERE id = 'turn-r'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(status, "running");

        cancel.notify_one();
        let _ = tokio::time::timeout(std::time::Duration::from_secs(5), handle)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_stall_wall_clock_jump_grace_then_stall() {
        let (_tmp, _ws, db, _state, task, cancel) = setup_test_turn_env("turn-d", "thread-d").await;
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        let turn = Turn::mock(rx, None);
        let clock = Arc::new(MockClock::new());
        let config = StallWatchdogConfig {
            stall_after: std::time::Duration::from_secs(600),
            stall_after_tool: std::time::Duration::from_secs(2700),
            tick_interval: std::time::Duration::from_millis(10),
            wall_jump_threshold: std::time::Duration::from_secs(120),
            resume_grace: std::time::Duration::from_secs(90),
        };

        let handle = tokio::spawn(run_turn_inner(task, turn, cancel, config, clock.clone()));
        // The loop must have read its starting wall time before the jump below.
        tx.send(Event::ToolResult {
            id: "warmup".to_string(),
            output_preview: String::new(),
            is_error: false,
        })
        .await
        .unwrap();
        wait_for_event(&db, "turn-d", "ToolResult").await;

        // Machine sleeps and wakes: wall clock jumps by 10 minutes (600s), but monotonic only 5s
        clock.advance_wall(std::time::Duration::from_secs(600));
        clock.advance(std::time::Duration::from_secs(5));
        tokio::time::sleep(std::time::Duration::from_millis(40)).await;

        // Grace period (90s) is now active, turn is still running
        let status: String = db
            .conn()
            .query_row("SELECT status FROM turns WHERE id = 'turn-d'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(status, "running");

        // Advance past grace deadline (e.g. 95s total since jump)
        clock.advance(std::time::Duration::from_secs(95));
        let _ = tokio::time::timeout(std::time::Duration::from_secs(5), handle)
            .await
            .unwrap();

        let final_status: String = db
            .conn()
            .query_row("SELECT status FROM turns WHERE id = 'turn-d'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(final_status, "error");
    }

    #[tokio::test]
    async fn test_stall_events_keep_arriving_never_stalled() {
        let (_tmp, _ws, db, _state, task, cancel) = setup_test_turn_env("turn-e", "thread-e").await;
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        let turn = Turn::mock(rx, None);
        let clock = Arc::new(MockClock::new());
        let config = StallWatchdogConfig {
            stall_after: std::time::Duration::from_secs(600),
            stall_after_tool: std::time::Duration::from_secs(2700),
            tick_interval: std::time::Duration::from_millis(10),
            wall_jump_threshold: std::time::Duration::from_secs(120),
            resume_grace: std::time::Duration::from_secs(90),
        };

        let handle = tokio::spawn(run_turn_inner(task, turn, cancel, config, clock.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        // Send events every 300s (below 600s) for multiple cycles
        for i in 1..=4 {
            clock.advance(std::time::Duration::from_secs(300));
            tx.send(Event::TextDelta(format!("chunk {i}\n")))
                .await
                .unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;

            let status: String = db
                .conn()
                .query_row("SELECT status FROM turns WHERE id = 'turn-e'", [], |r| {
                    r.get(0)
                })
                .unwrap();
            assert_eq!(status, "running");
        }

        // Now finish successfully
        tx.send(Event::Done {
            status: DoneStatus::Success,
            text: None,
        })
        .await
        .unwrap();
        drop(tx);

        let _ = tokio::time::timeout(std::time::Duration::from_secs(5), handle)
            .await
            .unwrap();

        let final_status: String = db
            .conn()
            .query_row("SELECT status FROM turns WHERE id = 'turn-e'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(final_status, "success");
    }

    #[tokio::test]
    async fn test_stall_terminates_child_process_tree() {
        let (_tmp, _ws, db, _state, task, cancel) = setup_test_turn_env("turn-f", "thread-f").await;
        let (_tx, rx) = tokio::sync::mpsc::channel(10);

        #[cfg(windows)]
        let mut child = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", "Start-Sleep -Seconds 60"])
            .spawn()
            .expect("spawn powershell child");

        #[cfg(not(windows))]
        let mut child = std::process::Command::new("sleep")
            .arg("60")
            .spawn()
            .expect("spawn sleep child");

        let pid = child.id();
        assert!(is_process_running(pid), "Process must be alive initially");

        let turn = Turn::mock(rx, Some(pid));
        let clock = Arc::new(MockClock::new());
        let config = StallWatchdogConfig {
            stall_after: std::time::Duration::from_secs(600),
            stall_after_tool: std::time::Duration::from_secs(2700),
            tick_interval: std::time::Duration::from_millis(10),
            wall_jump_threshold: std::time::Duration::from_secs(120),
            resume_grace: std::time::Duration::from_secs(90),
        };

        let handle = tokio::spawn(run_turn_inner(task, turn, cancel, config, clock.clone()));
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        // Advance clock past stall limit
        clock.advance(std::time::Duration::from_secs(650));
        let _ = tokio::time::timeout(std::time::Duration::from_secs(10), handle)
            .await
            .unwrap();

        // Give OS a moment to complete termination
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        assert!(
            !is_process_running(pid),
            "Child process must be killed on stall"
        );

        let status: String = db
            .conn()
            .query_row("SELECT status FROM turns WHERE id = 'turn-f'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(status, "error");
        let _ = child.wait();
    }

    #[tokio::test]
    async fn test_stall_integration_fake_engine_script() {
        let (tmp, _ws, db, _state, task, cancel) = setup_test_turn_env("turn-g", "thread-g").await;

        // Create a fake engine script that outputs one JSON event line then sleeps
        #[cfg(windows)]
        let script_path = tmp.path().join("fake_engine.bat");
        #[cfg(windows)]
        std::fs::write(
            &script_path,
            "@echo off\r\necho {\"type\":\"text\",\"text\":\"batched text event\"}\r\npowershell -NoProfile -Command \"Start-Sleep -Seconds 60\"\r\n",
        )
        .unwrap();

        #[cfg(not(windows))]
        let script_path = tmp.path().join("fake_engine.sh");
        #[cfg(not(windows))]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::write(
                &script_path,
                "#!/bin/sh\necho '{\"type\":\"text\",\"text\":\"batched text event\"}'\nsleep 60\n",
            )
            .unwrap();
            let mut perms = std::fs::metadata(&script_path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&script_path, perms).unwrap();
        }

        // Spawn fake engine process directly using Command
        let mut cmd = tokio::process::Command::new(&script_path);
        cmd.stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        let mut child = cmd.spawn().expect("spawn fake engine");
        let pid = child.id().expect("child id");
        assert!(
            is_process_running(pid),
            "Fake engine process must be running"
        );

        let stdout = child.stdout.take().expect("stdout");
        let (tx, rx) = tokio::sync::mpsc::channel(16);

        // Read stdout line by line and forward Event::Text
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let mut lines = tokio::io::BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Ok(v) = serde_json::from_str::<Value>(&line) {
                    if v.get("type").and_then(|t| t.as_str()) == Some("text") {
                        let text = v
                            .get("text")
                            .and_then(|t| t.as_str())
                            .unwrap_or("")
                            .to_string();
                        let _ = tx.send(Event::Text(text)).await;
                    }
                }
            }
        });

        let turn = Turn::mock(rx, Some(pid));
        // Use RealClock with millisecond limits
        let clock = Arc::new(RealClock);
        let config = StallWatchdogConfig {
            stall_after: std::time::Duration::from_millis(300),
            stall_after_tool: std::time::Duration::from_millis(1500),
            tick_interval: std::time::Duration::from_millis(30),
            wall_jump_threshold: std::time::Duration::from_secs(120),
            resume_grace: std::time::Duration::from_secs(90),
        };

        let handle = tokio::spawn(run_turn_inner(task, turn, cancel, config, clock));
        let _ = tokio::time::timeout(std::time::Duration::from_secs(10), handle)
            .await
            .unwrap();

        // Process should be terminated
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        assert!(
            !is_process_running(pid),
            "Fake engine child process must be gone after stall"
        );

        // Turn must end with error
        let final_status: String = db
            .conn()
            .query_row("SELECT status FROM turns WHERE id = 'turn-g'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(final_status, "error");

        let err_payload: String = db
            .conn()
            .query_row(
                "SELECT payload_json FROM events WHERE turn_id = 'turn-g' AND event_type = 'Error' ORDER BY rowid DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(err_payload.contains("Lượt không phản hồi trong"));
        assert!(err_payload.contains("Bấm Thử lại để chạy lại lượt."));
        let _ = child.wait().await;
    }

    #[test]
    fn test_build_context_with_graph_summary() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = Workspace::create(tmp.path().join("ws"), "Test Workspace").unwrap();
        let proj = Project::create(&ws, "demo-proj", "Demo Project").unwrap();

        // Create an artifact with pinned and notes
        let mut art =
            studio_artifacts::Artifact::create(&proj, "hero", "design", "Hero", None, vec![], None)
                .unwrap();
        art.pinned = true;
        art.notes = vec!["Giữ nguyên font chữ và màu logo".to_string()];
        art.save(&proj).unwrap();

        let ctx = build_context(&ws, Some(&proj), &[], None, false, false);
        assert!(ctx.contains("Sơ đồ nội dung:"));
        assert!(ctx.contains("Đã khóa: hero"));
        assert!(ctx.contains("hero: \"Giữ nguyên font chữ và màu logo\""));
        assert!(ctx.len() < 4000);
    }

    #[tokio::test]
    async fn test_run_turn_syncs_unemitted_file_changes() {
        let (_tmp, ws, db, _state, task, cancel) =
            setup_test_turn_env("turn-sync", "thread-sync").await;

        // 1. Create a checkpoint before the turn (mimicking start_turn)
        CheckpointService::create(&ws, "Trước lượt", Some("turn-sync".to_string())).unwrap();

        // 2. Mock engine that emits one FileChange for `emitted.md`, and Done
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        let turn = Turn::mock(rx, None);
        let clock = Arc::new(RealClock);
        let config = StallWatchdogConfig::default();

        // Also simulate an out-of-band file creation (e.g. from Studio tool or shell)
        // like `dan-y.md` created directly on disk
        let unemitted_path = ws.root().join("dan-y.md");
        std::fs::write(&unemitted_path, "# Dàn ý video").unwrap();

        let emitted_path = ws.root().join("emitted.md");
        std::fs::write(&emitted_path, "# Emitted file").unwrap();

        let handle = tokio::spawn(run_turn_inner(task, turn, cancel, config, clock));

        // Send an emitted FileChange event and Done
        tx.send(Event::FileChange {
            path: std::path::PathBuf::from("emitted.md"),
            op: FileOp::Write,
        })
        .await
        .unwrap();

        tx.send(Event::Done {
            status: DoneStatus::Success,
            text: Some("Hoàn thành".to_string()),
        })
        .await
        .unwrap();
        drop(tx);

        let _ = handle.await.unwrap();

        // Verify that events table has FileChange for both `emitted.md` AND `dan-y.md`
        let mut stmt = db
            .conn()
            .prepare(
                "SELECT payload_json FROM events WHERE turn_id = 'turn-sync' AND event_type = 'FileChange'",
            )
            .unwrap();
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();

        let mut paths: Vec<String> = rows
            .iter()
            .filter_map(|json_str| {
                let v: serde_json::Value = serde_json::from_str(json_str).ok()?;
                v.get("path")
                    .and_then(|p| p.as_str())
                    .map(ToString::to_string)
            })
            .collect();
        paths.sort();

        assert_eq!(paths, vec!["dan-y.md", "emitted.md"]);
    }
}
