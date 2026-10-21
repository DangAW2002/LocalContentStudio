//! IPC commands implementing `packages/desktop-ui/IPC.md`.

use std::path::{Path, PathBuf};

use rusqlite::params;
use serde::Deserialize;
use serde_json::{json, Value};
use studio_engine::Engine;
use studio_workspace::{CheckpointService, Project, ScopeChain, Workspace, WorkspaceDb};
use tauri::{AppHandle, Manager, State};

use crate::error::{CmdError, CmdResult};
use crate::state::AppState;
use crate::turns;

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn kind_of(path: &Path) -> (&'static str, &'static str) {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "md" | "markdown" => ("markdown", "text/markdown"),
        "txt" | "srt" | "vtt" => ("text", "text/plain"),
        "html" | "htm" => ("html", "text/html"),
        "json" => ("json", "application/json"),
        "yaml" | "yml" | "toml" | "css" | "js" | "ts" | "tsx" | "jsx" | "py" | "rs" | "sh"
        | "ps1" => ("code", "text/plain"),
        "png" => ("image", "image/png"),
        "jpg" | "jpeg" => ("image", "image/jpeg"),
        "gif" => ("image", "image/gif"),
        "webp" => ("image", "image/webp"),
        "svg" => ("image", "image/svg+xml"),
        "mp4" | "m4v" => ("video", "video/mp4"),
        "webm" => ("video", "video/webm"),
        "mov" => ("video", "video/quicktime"),
        "mp3" => ("audio", "audio/mpeg"),
        "wav" => ("audio", "audio/wav"),
        "ogg" | "opus" => ("audio", "audio/ogg"),
        "m4a" | "aac" => ("audio", "audio/aac"),
        "pdf" => ("unknown", "application/pdf"),
        _ => ("unknown", "application/octet-stream"),
    }
}

/// Tauri v2 asset URL for a local file (same as `convertFileSrc` on Windows).
fn asset_url(path: &Path) -> String {
    let raw = path.to_string_lossy();
    let mut enc = String::new();
    for b in raw.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                enc.push(*b as char)
            }
            _ => enc.push_str(&format!("%{b:02X}")),
        }
    }
    if cfg!(windows) {
        format!("http://asset.localhost/{enc}")
    } else {
        format!("asset://localhost/{enc}")
    }
}

fn human(bytes: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < units.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{bytes} B")
    } else {
        format!("{v:.1} {}", units[i])
    }
}

/// Strip Windows verbatim prefixes (`\\?\` or `\\?\UNC\`) from a path for user display.
pub fn strip_verbatim(path: &Path) -> String {
    let raw = path.to_string_lossy();
    if let Some(rest) = raw.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = raw.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        raw.into_owned()
    }
}

/// Normalize a workspace path for storage and deduplication:
/// Strips verbatim prefixes (`\\?\`) and trims trailing slashes (case-insensitive on Windows).
pub fn normalize_workspace_path(path: impl AsRef<Path>) -> String {
    let stripped = strip_verbatim(path.as_ref());
    let trimmed = stripped.trim_end_matches(['/', '\\']);
    if trimmed.len() == 2 && trimmed.ends_with(':') {
        format!("{trimmed}\\")
    } else if trimmed.is_empty() {
        stripped
    } else {
        trimmed.to_string()
    }
}

fn rel(ws: &Workspace, p: &Path) -> String {
    let clean_root = PathBuf::from(strip_verbatim(ws.root()));
    let clean_p = PathBuf::from(strip_verbatim(p));
    clean_p
        .strip_prefix(&clean_root)
        .unwrap_or(&clean_p)
        .to_string_lossy()
        .replace('\\', "/")
}

fn modified(p: &Path) -> Option<String> {
    let t = std::fs::metadata(p).ok()?.modified().ok()?;
    Some(chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
}

// ───────────────────────── Workspace ─────────────────────────

fn ws_summary(ws: &Workspace, current: bool) -> Value {
    json!({ "path": normalize_workspace_path(ws.root()), "name": ws.name(), "lastOpenedAt": now(), "isCurrent": current })
}

/// Make `path` the open workspace: record it, allow asset access, watch files.
/// Make `path` the open workspace: record it, allow asset access, watch files.
fn activate(state: &AppState, ws: &Workspace) -> CmdResult<()> {
    let norm = normalize_workspace_path(ws.root());
    if let Ok(db) = state.app_paths.open_app_db() {
        let _ = db.record_workspace_open(&norm);
    }
    if let Some(app) = state.app_handle.get() {
        let _ = app.asset_protocol_scope().allow_directory(ws.root(), true);
    }
    // Turns left "running" by a crash or forced quit can never finish: close them.
    if let Ok(db) = WorkspaceDb::open(ws) {
        let live: Vec<String> = state
            .turns
            .lock()
            .map(|t| t.keys().cloned().collect())
            .unwrap_or_default();
        if let Ok(mut stmt) = db
            .conn()
            .prepare("SELECT id FROM turns WHERE status = 'running'")
        {
            let stale: Vec<String> = stmt
                .query_map([], |r| r.get::<_, String>(0))
                .map(|rows| rows.flatten().filter(|id| !live.contains(id)).collect())
                .unwrap_or_default();
            for id in stale {
                let done = json!({ "type": "Done", "status": "error", "text": "Lượt bị gián đoạn do ứng dụng đóng." });
                let _ = turns::persist(&db, &id, &done);
                let _ = db.conn().execute(
                    "UPDATE turns SET status = 'error', completed_at = ?2 WHERE id = ?1",
                    params![id, now()],
                );
            }
        }
    }
    *state
        .current
        .lock()
        .map_err(|_| CmdError::msg("Trạng thái ứng dụng bị khóa."))? =
        Some(ws.root().to_path_buf());

    use notify::Watcher;
    let state_clone = state.clone();
    let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(ev) = res {
            let paths: Vec<String> = ev
                .paths
                .iter()
                .filter(|p| !p.components().any(|c| c.as_os_str() == ".studio"))
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            if !paths.is_empty() {
                state_clone.emit("fs-event", json!({ "paths": paths }));
            }
        }
    });
    if let Ok(mut w) = watcher {
        let _ = w.watch(ws.root(), notify::RecursiveMode::Recursive);
        if let Ok(mut slot) = state.watcher.lock() {
            *slot = Some(w);
        }
    }
    Ok(())
}

pub fn app_recent_workspaces_impl(state: &AppState) -> CmdResult<Vec<Value>> {
    let current = state.current.lock().ok().and_then(|c| c.clone());
    let current_clean = current.as_deref().map(normalize_workspace_path);
    let db = state.app_paths.open_app_db()?;
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();

    for r in db.list_recent_workspaces()? {
        let clean_path = normalize_workspace_path(Path::new(&r.path));
        let key = if cfg!(windows) {
            clean_path.to_lowercase()
        } else {
            clean_path.clone()
        };

        if !seen.insert(key) {
            continue;
        }

        let path = PathBuf::from(&clean_path);
        let Ok(ws) = Workspace::open(&path) else {
            continue;
        };

        let is_current = current_clean.as_deref().is_some_and(|c| {
            if cfg!(windows) {
                c.eq_ignore_ascii_case(&clean_path)
            } else {
                c == clean_path
            }
        });

        out.push(json!({
            "path": clean_path,
            "name": ws.name(),
            "lastOpenedAt": r.last_opened,
            "isCurrent": is_current,
        }));
    }
    Ok(out)
}

#[tauri::command]
pub fn app_recent_workspaces(state: State<'_, AppState>) -> CmdResult<Vec<Value>> {
    app_recent_workspaces_impl(&state)
}

pub fn workspace_create_impl(state: &AppState, path: String, name: String) -> CmdResult<Value> {
    let ws = Workspace::create(&path, name)?;
    activate(state, &ws)?;
    Ok(ws_summary(&ws, true))
}

#[tauri::command]
pub fn workspace_create(
    _app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    name: String,
) -> CmdResult<Value> {
    workspace_create_impl(&state, path, name)
}

pub fn workspace_open_impl(state: &AppState, path: String) -> CmdResult<Value> {
    let ws = Workspace::open(&path)?;
    activate(state, &ws)?;
    Ok(ws_summary(&ws, true))
}

#[tauri::command]
pub fn workspace_open(
    _app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> CmdResult<Value> {
    workspace_open_impl(&state, path)
}

pub fn workspace_current_impl(state: &AppState) -> CmdResult<Value> {
    let ws = state.workspace()?;
    let db = WorkspaceDb::open(&ws)?;
    let month = chrono::Utc::now().format("%Y-%m").to_string();
    let spent: f64 = db.conn().query_row(
        "SELECT COALESCE(SUM(cost_usd), 0) FROM cost_ledger WHERE substr(created_at, 1, 7) = ?1",
        [month],
        |r| r.get(0),
    )?;
    let cfg: Value = std::fs::read_to_string(ws.root().join("studio.yaml"))
        .ok()
        .and_then(|t| serde_yaml::from_str(&t).ok())
        .unwrap_or(Value::Null);
    let max = cfg
        .get("budget_month_usd")
        .and_then(Value::as_f64)
        .unwrap_or(20.0);
    let org = ws.org();
    Ok(json!({
        "workspace": ws_summary(&ws, true),
        "org": if org.is_implicit() { Value::Null } else { json!({ "id": org.name(), "name": org.name(), "path": strip_verbatim(org.root()) }) },
        "budget": { "currentSpendUsd": spent, "maxSpendUsd": max, "warningThreshold": 0.8 },
    }))
}

#[tauri::command]
pub fn workspace_current(state: State<'_, AppState>) -> CmdResult<Value> {
    workspace_current_impl(&state)
}

// ───────────────────────── Projects & files ─────────────────────────

fn project_summary(p: &Project) -> Value {
    let meta = p.meta().ok();
    let artifacts = std::fs::read_dir(p.artifacts_dir())
        .map(|d| d.count())
        .unwrap_or(0);
    json!({
        "slug": p.slug(), "title": p.title(),
        "kit": meta.as_ref().and_then(|m| m.kit.clone()),
        "modules": meta.as_ref().map(|m| m.modules.clone()).unwrap_or_default(),
        "status": "active",
        "createdAt": meta.as_ref().map(|m| m.created_at.clone()).unwrap_or_default(),
        "updatedAt": modified(p.root()).unwrap_or_default(),
        "artifactCount": artifacts,
    })
}

pub fn project_list_impl(state: &AppState) -> CmdResult<Vec<Value>> {
    let ws = state.workspace()?;
    Ok(ws.list_projects()?.iter().map(project_summary).collect())
}

#[tauri::command]
pub fn project_list(state: State<'_, AppState>) -> CmdResult<Vec<Value>> {
    project_list_impl(&state)
}

fn kit_modules(state: &AppState, kit: &str) -> Vec<String> {
    studio_catalog::Catalog::load_file(&state.catalog_path)
        .ok()
        .and_then(|c| {
            c.kits
                .get(kit)
                .map(|k| k.use_modules.keys().cloned().collect())
        })
        .unwrap_or_default()
}

pub fn project_create_impl(
    state: &AppState,
    slug: String,
    title: String,
    kit: Option<String>,
    modules: Option<Vec<String>>,
) -> CmdResult<Value> {
    let ws = state.workspace()?;
    let project = Project::create(&ws, &slug, &title)?;
    let mut mods = modules.unwrap_or_default();
    if mods.is_empty() {
        if let Some(k) = &kit {
            mods = kit_modules(state, k);
        }
    }
    project.set_modules(kit, &mods)?;
    Ok(project_summary(&project))
}

#[tauri::command]
pub fn project_create(
    state: State<'_, AppState>,
    slug: String,
    title: String,
    kit: Option<String>,
    modules: Option<Vec<String>>,
) -> CmdResult<Value> {
    project_create_impl(&state, slug, title, kit, modules)
}

pub fn project_files_impl(state: &AppState, project: String) -> CmdResult<Vec<Value>> {
    let ws = state.workspace()?;
    let p = Project::open(&ws, &project)?;
    let mut out = Vec::new();
    walk(p.root(), p.root(), &mut out);
    Ok(out)
}

#[tauri::command]
pub fn project_files(state: State<'_, AppState>, project: String) -> CmdResult<Vec<Value>> {
    project_files_impl(&state, project)
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<Value>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = rd.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    let clean_root = PathBuf::from(strip_verbatim(root));
    for e in entries {
        let p = e.path();
        let is_dir = p.is_dir();
        let (kind, _) = kind_of(&p);
        let clean_p = PathBuf::from(strip_verbatim(&p));
        out.push(json!({
            "path": clean_p.strip_prefix(&clean_root).unwrap_or(&clean_p).to_string_lossy().replace('\\', "/"),
            "name": e.file_name().to_string_lossy(),
            "isDir": is_dir,
            "size": if is_dir { 0 } else { e.metadata().map(|m| m.len()).unwrap_or(0) },
            "kind": if is_dir { "unknown" } else { kind },
            "updatedAt": modified(&p),
        }));
        if is_dir {
            walk(root, &p, out);
        }
    }
}

/// Resolve a UI path: absolute inside the workspace, project-relative, or workspace-relative.
pub fn resolve_path(ws: &Workspace, project: Option<&str>, path: &str) -> CmdResult<PathBuf> {
    let p = Path::new(path);
    if p.is_absolute() {
        let root = std::fs::canonicalize(ws.root())?;
        let target = std::fs::canonicalize(p)?;
        return if target.starts_with(&root) {
            Ok(target)
        } else {
            Err(CmdError::msg("Tệp nằm ngoài workspace."))
        };
    }
    if let Some(slug) = project {
        let candidate = ws.scoped_path(Path::new("projects").join(slug).join(p))?;
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Ok(ws.scoped_path(p)?)
}

/// Like [`resolve_path`], but additionally lets absolute paths into the App/Org shared layers
/// through, read-only, so the scope chain's inherited brands/channels/playbooks can be viewed.
fn resolve_read_path(
    state: &AppState,
    ws: &Workspace,
    project: Option<&str>,
    path: &str,
) -> CmdResult<PathBuf> {
    let err = match resolve_path(ws, project, path) {
        Ok(p) => return Ok(p),
        Err(e) => e,
    };
    let p = Path::new(path);
    if p.is_absolute() {
        if let Ok(target) = std::fs::canonicalize(p) {
            let layers = [state.app_paths.shared_dir(), ws.org().shared_dir()];
            if layers
                .iter()
                .filter_map(|dir| std::fs::canonicalize(dir).ok())
                .any(|dir| target.starts_with(dir))
            {
                return Ok(target);
            }
        }
    }
    Err(err)
}

pub fn file_read_impl(state: &AppState, project: Option<String>, path: String) -> CmdResult<Value> {
    let ws = state.workspace()?;
    let full = resolve_read_path(state, &ws, project.as_deref(), &path)?;
    let size = std::fs::metadata(&full)?.len();
    let (kind, mime) = kind_of(&full);
    let textual = matches!(kind, "markdown" | "text" | "html" | "json" | "code");
    let text = if textual && size <= 2 * 1024 * 1024 {
        std::fs::read_to_string(&full).ok()
    } else {
        None
    };
    Ok(
        json!({ "kind": kind, "mime": mime, "text": text, "assetUrl": asset_url(&full), "size": size }),
    )
}

#[tauri::command]
pub fn file_read(
    state: State<'_, AppState>,
    project: Option<String>,
    path: String,
) -> CmdResult<Value> {
    file_read_impl(&state, project, path)
}

// ───────────────────────── Threads & turns ─────────────────────────

fn thread_json(db: &WorkspaceDb, id: &str) -> CmdResult<Value> {
    Ok(db.conn().query_row(
        "SELECT t.id, t.project_slug, t.engine, t.model, t.title, t.created_at, t.updated_at,
                (SELECT COUNT(*) FROM turns u WHERE u.thread_id = t.id AND u.status != 'undone')
         FROM threads t WHERE t.id = ?1",
        [id],
        |r| {
            Ok(json!({
                "id": r.get::<_, String>(0)?, "project": r.get::<_, Option<String>>(1)?,
                "engine": r.get::<_, String>(2)?, "model": r.get::<_, Option<String>>(3)?,
                "title": r.get::<_, String>(4)?, "createdAt": r.get::<_, String>(5)?,
                "updatedAt": r.get::<_, String>(6)?, "turnCount": r.get::<_, i64>(7)?,
            }))
        },
    )?)
}

pub fn thread_list_impl(state: &AppState, project: String) -> CmdResult<Vec<Value>> {
    let ws = state.workspace()?;
    let db = WorkspaceDb::open(&ws)?;
    let ids: Vec<String> = {
        let mut stmt = db
            .conn()
            .prepare("SELECT id FROM threads WHERE project_slug = ?1 ORDER BY updated_at DESC")?;
        let rows = stmt.query_map([project], |r| r.get(0))?;
        rows.collect::<Result<_, _>>()?
    };
    ids.iter().map(|id| thread_json(&db, id)).collect()
}

#[tauri::command]
pub fn thread_list(state: State<'_, AppState>, project: String) -> CmdResult<Vec<Value>> {
    thread_list_impl(&state, project)
}

pub fn thread_create_impl(
    state: &AppState,
    project: String,
    engine: String,
    model: Option<String>,
    title: Option<String>,
) -> CmdResult<Value> {
    if engine != "claude" && engine != "agy" {
        return Err(CmdError::msg("Engine không hợp lệ (claude hoặc agy)."));
    }
    let ws = state.workspace()?;
    Project::open(&ws, &project)?;
    let db = WorkspaceDb::open(&ws)?;
    let id = uuid::Uuid::new_v4().to_string();
    let t = title.unwrap_or_else(|| "Hội thoại mới".into());
    db.conn().execute(
        "INSERT INTO threads (id, project_slug, title, engine, model, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![id, project, t, engine, model, now()],
    )?;
    thread_json(&db, &id)
}

#[tauri::command]
pub fn thread_create(
    state: State<'_, AppState>,
    project: String,
    engine: String,
    model: Option<String>,
) -> CmdResult<Value> {
    thread_create_impl(&state, project, engine, model, None)
}

pub fn thread_events_impl(state: &AppState, thread_id: String) -> CmdResult<Value> {
    thread_events_paged_impl(state, thread_id, None, None)
}

pub fn thread_events_paged_impl(
    state: &AppState,
    thread_id: String,
    cursor: Option<String>,
    limit: Option<u32>,
) -> CmdResult<Value> {
    let ws = state.workspace()?;
    let db = WorkspaceDb::open(&ws)?;
    let offset: i64 = cursor.and_then(|c| c.parse().ok()).unwrap_or(0);
    let limit = i64::from(limit.unwrap_or(500).clamp(1, 2000));
    let mut stmt = db.conn().prepare(
        "SELECT e.turn_id, e.payload_json FROM events e JOIN turns t ON t.id = e.turn_id
         WHERE t.thread_id = ?1 AND t.status != 'undone'
         ORDER BY t.turn_index, e.rowid LIMIT ?2 OFFSET ?3",
    )?;
    let rows = stmt.query_map(params![thread_id, limit + 1, offset], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })?;
    let mut events = Vec::new();
    for row in rows {
        let (turn_id, payload) = row?;
        let event: Value = serde_json::from_str(&payload)?;
        events.push(json!({ "threadId": thread_id, "turnId": turn_id, "event": event }));
    }
    let has_more = events.len() as i64 > limit;
    events.truncate(limit as usize);
    Ok(json!({
        "events": events,
        "hasMore": has_more,
        "nextCursor": if has_more { Some((offset + limit).to_string()) } else { None },
    }))
}

#[tauri::command]
pub fn thread_events(
    state: State<'_, AppState>,
    thread_id: String,
    cursor: Option<String>,
    limit: Option<u32>,
) -> CmdResult<Value> {
    thread_events_paged_impl(&state, thread_id, cursor, limit)
}

#[tauri::command]
pub async fn turn_start(
    _app: AppHandle,
    state: State<'_, AppState>,
    thread_id: String,
    prompt: String,
) -> CmdResult<Value> {
    let turn_id = turns::start_turn((*state).clone(), thread_id, prompt).await?;
    Ok(json!({ "turnId": turn_id }))
}

pub fn turn_cancel_impl(state: &AppState, turn_id: String) -> CmdResult<Value> {
    let found = state
        .turns
        .lock()
        .ok()
        .and_then(|t| t.get(&turn_id).map(|r| r.cancel.clone()));
    if let Some(cancel) = &found {
        cancel.notify_one();
    }
    Ok(json!({ "success": found.is_some() }))
}

#[tauri::command]
pub fn turn_cancel(state: State<'_, AppState>, turn_id: String) -> CmdResult<Value> {
    turn_cancel_impl(&state, turn_id)
}

pub fn turn_undo_impl(state: &AppState, turn_id: String) -> CmdResult<Value> {
    turns::undo_turn(state, &turn_id)?;
    Ok(json!({ "success": true, "restoredTurnId": turn_id }))
}

#[tauri::command]
pub fn turn_undo(state: State<'_, AppState>, turn_id: String) -> CmdResult<Value> {
    turn_undo_impl(&state, turn_id)
}

pub fn permission_respond_impl(
    state: &AppState,
    request_id: String,
    allow: bool,
) -> CmdResult<Value> {
    Ok(json!({ "acknowledged": state.broker.respond(&request_id, allow) }))
}

/// Permission requests still waiting for an answer (optionally for one thread).
pub fn permission_pending_impl(state: &AppState, thread_id: Option<String>) -> CmdResult<Value> {
    Ok(json!(state.broker.list_pending(thread_id.as_deref())))
}

#[tauri::command]
pub fn permission_pending(
    state: State<'_, AppState>,
    thread_id: Option<String>,
) -> CmdResult<Value> {
    permission_pending_impl(&state, thread_id)
}

#[tauri::command]
pub fn permission_respond(
    state: State<'_, AppState>,
    request_id: String,
    allow: bool,
) -> CmdResult<Value> {
    permission_respond_impl(&state, request_id, allow)
}

// ───────────────────────── Checkpoints ─────────────────────────

pub fn checkpoint_list_impl(state: &AppState) -> CmdResult<Vec<Value>> {
    let ws = state.workspace()?;
    let db = WorkspaceDb::open(&ws)?;
    let list = CheckpointService::list(&ws)?;
    Ok(list
        .into_iter()
        .map(|c| {
            let files: i64 = db
                .conn()
                .query_row(
                    "SELECT COUNT(*) FROM checkpoint_files WHERE checkpoint_id = ?1",
                    [&c.id],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            json!({
                "id": c.id, "timestamp": c.created_at, "message": c.label, "filesChanged": files,
                "author": c.turn_id.map(|t| format!("turn:{t}")).unwrap_or_else(|| "user".into()),
            })
        })
        .collect())
}

#[tauri::command]
pub fn checkpoint_list(state: State<'_, AppState>) -> CmdResult<Vec<Value>> {
    checkpoint_list_impl(&state)
}

pub fn checkpoint_restore_impl(state: &AppState, id: String) -> CmdResult<Value> {
    let ws = state.workspace()?;
    CheckpointService::restore(&ws, &id)?;
    Ok(json!({ "success": true, "restoredCheckpointId": id }))
}

#[tauri::command]
pub fn checkpoint_restore(state: State<'_, AppState>, id: String) -> CmdResult<Value> {
    checkpoint_restore_impl(&state, id)
}

// ───────────────────────── Engines & catalog ─────────────────────────

pub async fn engines_probe_impl(state: &AppState) -> CmdResult<Vec<Value>> {
    let mut out = Vec::new();
    for (id, name, probe) in [
        ("claude", "Claude Code", state.claude.probe().await),
        ("agy", "Antigravity (agy)", state.agy.probe().await),
    ] {
        out.push(json!({
            "id": id, "name": name, "installed": probe.installed, "version": probe.version,
            "supported": probe.supported, "notes": probe.note,
        }));
    }
    let (node, npx) = studio_artifacts::probe_node_npx();
    for (id, name, r) in [
        ("ffmpeg", "FFmpeg", studio_artifacts::probe_ffmpeg()),
        (
            "chromium",
            "Chrome / Edge (render HTML)",
            studio_artifacts::probe_chromium(),
        ),
        ("node", "Node.js", node),
        ("npx", "npx (HyperFrames)", npx),
    ] {
        out.push(json!({
            "id": id, "name": name, "installed": r.available, "version": r.version,
            "path": r.path, "supported": r.available, "notes": r.note,
        }));
    }
    Ok(out)
}

#[tauri::command]
pub async fn engines_probe(state: State<'_, AppState>) -> CmdResult<Vec<Value>> {
    engines_probe_impl(&state).await
}

fn catalog(state: &AppState) -> CmdResult<studio_catalog::Catalog> {
    Ok(studio_catalog::Catalog::load_file(&state.catalog_path)?)
}

fn kit_json(cat: &studio_catalog::Catalog, id: &str) -> Option<Value> {
    let kit = cat.kits.get(id)?;
    let modules: Vec<String> = kit.use_modules.keys().cloned().collect();
    let budget = studio_catalog::resolve(cat, Some(id), &[])
        .map(|a| a.desc_tokens)
        .unwrap_or(0);
    Some(json!({
        "id": id, "name": kit.title, "description": modules.join(", "),
        "modules": modules, "tokenBudget": budget, "tags": kit.keywords,
    }))
}

fn module_json(cat: &studio_catalog::Catalog, id: &str) -> Option<Value> {
    let m = cat.modules.get(id)?;
    let skills: usize = m.skills.values().map(Vec::len).sum();
    let caps: Vec<String> = m
        .level
        .as_ref()
        .and_then(|l| serde_json::to_value(l).ok())
        .map(|l| {
            let mut v: Vec<String> = l
                .get("provider_for")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect();
            if let Some(c) = l.get("capability").and_then(Value::as_str) {
                v.push(c.to_string());
            }
            v
        })
        .unwrap_or_default();
    let tokens = studio_catalog::resolve(cat, None, &[id])
        .map(|a| a.desc_tokens)
        .unwrap_or(0);
    Some(json!({
        "id": id, "name": id, "description": m.description.clone().unwrap_or_default(),
        "capabilities": caps, "skillCount": skills, "tokenCount": tokens,
    }))
}

pub fn catalog_kits_impl(state: &AppState) -> CmdResult<Vec<Value>> {
    let cat = catalog(state)?;
    Ok(cat.kits.keys().filter_map(|k| kit_json(&cat, k)).collect())
}

#[tauri::command]
pub fn catalog_kits(state: State<'_, AppState>) -> CmdResult<Vec<Value>> {
    catalog_kits_impl(&state)
}

pub fn catalog_modules_impl(state: &AppState) -> CmdResult<Vec<Value>> {
    let cat = catalog(state)?;
    Ok(cat
        .modules
        .keys()
        .filter_map(|m| module_json(&cat, m))
        .collect())
}

#[tauri::command]
pub fn catalog_modules(state: State<'_, AppState>) -> CmdResult<Vec<Value>> {
    catalog_modules_impl(&state)
}

pub fn catalog_suggest_impl(state: &AppState, text: String) -> CmdResult<Value> {
    let cat = catalog(state)?;
    let ranked = studio_catalog::suggest_kits(&cat, &text);
    let kits: Vec<Value> = ranked
        .iter()
        .filter(|(_, s)| *s > 0)
        .take(3)
        .filter_map(|(k, _)| kit_json(&cat, k))
        .collect();
    let modules: Vec<Value> = ranked
        .first()
        .filter(|(_, s)| *s > 0)
        .and_then(|(k, _)| cat.kits.get(k))
        .map(|kit| {
            kit.use_modules
                .keys()
                .filter_map(|m| module_json(&cat, m))
                .collect()
        })
        .unwrap_or_default();
    Ok(json!({ "suggestedKits": kits, "suggestedModules": modules }))
}

#[tauri::command]
pub fn catalog_suggest(state: State<'_, AppState>, text: String) -> CmdResult<Value> {
    catalog_suggest_impl(&state, text)
}

pub fn project_modules_set_impl(
    state: &AppState,
    project: String,
    modules: Vec<String>,
) -> CmdResult<Value> {
    let ws = state.workspace()?;
    let p = Project::open(&ws, &project)?;
    let kit = p.meta()?.kit;
    let meta = p.set_modules(kit.clone(), &modules)?;
    let cat = catalog(state)?;
    let extra: Vec<&str> = meta.modules.iter().map(String::as_str).collect();
    let budget = studio_catalog::resolve(&cat, kit.as_deref(), &extra)
        .map(|a| a.desc_tokens)
        .unwrap_or(0);
    Ok(json!({ "project": project, "modules": meta.modules, "totalTokenBudget": budget }))
}

#[tauri::command]
pub fn project_modules_set(
    state: State<'_, AppState>,
    project: String,
    modules: Vec<String>,
) -> CmdResult<Value> {
    project_modules_set_impl(&state, project, modules)
}

fn validate_scope_id(id: &str) -> CmdResult<()> {
    // No trimming: the id is used verbatim in paths, so surrounding spaces are rejected too.
    let id_trimmed = id;
    if id_trimmed.len() < 2 || id_trimmed.len() > 40 {
        return Err(CmdError::msg("ID phải từ 2 đến 40 ký tự."));
    }
    if !id_trimmed
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(CmdError::msg(
            "ID chỉ gồm chữ thường (a-z), chữ số (0-9) và dấu gạch ngang (-).",
        ));
    }
    Ok(())
}

fn copy_dir_all_safe(src: &Path, dst: &Path) -> CmdResult<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        if ft.is_symlink() {
            return Err(CmdError::msg(
                "Không cho phép sao chép liên kết tượng trưng (symlink / junction).",
            ));
        }
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ft.is_dir() {
            copy_dir_all_safe(&src_path, &dst_path)?;
        } else if ft.is_file() {
            if let Some(p) = dst_path.parent() {
                std::fs::create_dir_all(p)?;
            }
            let bytes = std::fs::read(&src_path)?;
            std::fs::write(&dst_path, &bytes)?;
        }
    }
    Ok(())
}

pub fn scope_list_impl(state: &AppState, kind: String) -> CmdResult<Vec<Value>> {
    let allowed = [
        "brands",
        "channels",
        "playbooks",
        "skills",
        "tools",
        "capabilities",
    ];
    if !allowed.contains(&kind.as_str()) {
        return Err(CmdError::msg("Loại không hợp lệ."));
    }
    let ws = state.workspace()?;

    let app_shared = state.app_paths.shared_dir().join(&kind);
    let org_shared = ws.org().shared_dir().join(&kind);

    let items = ScopeChain::new(&ws).resolve(&kind);

    let mut result = Vec::new();
    for item in items {
        let layer = format!("{:?}", item.layer).to_lowercase();
        let is_ws = item.layer == studio_workspace::ResolutionLayer::Workspace;

        let is_overriding = if is_ws {
            match kind.as_str() {
                "brands" => {
                    app_shared.join(&item.id).is_dir() || org_shared.join(&item.id).is_dir()
                }
                "channels" => {
                    let fname = format!("{}.yaml", item.id);
                    app_shared.join(&fname).is_file() || org_shared.join(&fname).is_file()
                }
                "playbooks" => {
                    let fname = format!("{}.md", item.id);
                    app_shared.join(&fname).is_file()
                        || org_shared.join(&fname).is_file()
                        || app_shared.join(&item.id).is_dir()
                        || org_shared.join(&item.id).is_dir()
                }
                _ => false,
            }
        } else {
            false
        };

        let mut name = item.id.clone();
        let mut summary: Option<String> = None;
        let mut error_msg: Option<String> = None;
        let mut formats_val: Option<Value> = None;

        match kind.as_str() {
            "brands" => {
                let yaml_path = item.path.join("brand.yaml");
                if yaml_path.exists() {
                    match std::fs::read_to_string(&yaml_path) {
                        Ok(content) => {
                            let val = studio_brand::validate_scope_item("brands", &content);
                            if val.valid {
                                if let Ok(spec) =
                                    serde_yaml::from_str::<studio_brand::BrandSpec>(&content)
                                {
                                    name = spec.name;
                                    summary = Some(spec.slogan);
                                }
                            } else if let Some(err) = val.errors.first() {
                                error_msg = Some(format!(
                                    "Dòng {}, cột {}: {}",
                                    err.line, err.column, err.message
                                ));
                            }
                        }
                        Err(e) => {
                            error_msg = Some(format!("Không thể đọc tệp brand.yaml: {e}"));
                        }
                    }
                } else {
                    error_msg = Some("Thiếu tệp brand.yaml trong thư mục thương hiệu.".to_string());
                }
            }
            "channels" if item.path.is_file() => match std::fs::read_to_string(&item.path) {
                Ok(content) => {
                    let val = studio_brand::validate_scope_item("channels", &content);
                    if val.valid {
                        if let Ok(spec) =
                            serde_yaml::from_str::<studio_brand::ChannelSpec>(&content)
                        {
                            name = spec.title;
                            summary = Some(spec.notes);
                            formats_val = Some(json!(spec.formats));
                        }
                    } else if let Some(err) = val.errors.first() {
                        error_msg = Some(format!(
                            "Dòng {}, cột {}: {}",
                            err.line, err.column, err.message
                        ));
                    }
                }
                Err(e) => {
                    error_msg = Some(format!("Không thể đọc tệp kênh: {e}"));
                }
            },
            "playbooks" if item.path.is_file() => match std::fs::read_to_string(&item.path) {
                Ok(content) => {
                    let val = studio_brand::validate_scope_item("playbooks", &content);
                    if val.valid {
                        if let Ok(pb) = studio_brand::Playbook::parse(&item.id, &content) {
                            name = pb.frontmatter.name;
                            summary = Some(pb.frontmatter.description);
                        }
                    } else if let Some(err) = val.errors.first() {
                        error_msg = Some(format!(
                            "Dòng {}, cột {}: {}",
                            err.line, err.column, err.message
                        ));
                    }
                }
                Err(e) => {
                    error_msg = Some(format!("Không thể đọc tệp quy trình: {e}"));
                }
            },
            _ => {}
        }

        let mut out = json!({
            "id": item.id,
            "name": name,
            "kind": kind,
            "layer": layer,
            "path": strip_verbatim(&item.path),
            "summary": summary,
            "isOverride": is_overriding,
        });
        if let Some(err) = error_msg {
            out["error"] = json!(err);
        }
        if let Some(f) = formats_val {
            out["formats"] = f;
        }
        result.push(out);
    }

    Ok(result)
}

#[tauri::command]
pub fn scope_list(state: State<'_, AppState>, kind: String) -> CmdResult<Vec<Value>> {
    scope_list_impl(&state, kind)
}

pub fn scope_override_impl(state: &AppState, kind: String, id: String) -> CmdResult<Value> {
    let allowed = ["brands", "channels", "playbooks"];
    if !allowed.contains(&kind.as_str()) {
        return Err(CmdError::msg(
            "Chỉ hỗ trợ ghi đè brands, channels hoặc playbooks.",
        ));
    }
    validate_scope_id(&id)?;
    let ws = state.workspace()?;

    let ws_dest_rel = match kind.as_str() {
        "brands" => PathBuf::from("brands").join(&id),
        "channels" => PathBuf::from("channels").join(format!("{id}.yaml")),
        "playbooks" => PathBuf::from("playbooks").join(format!("{id}.md")),
        _ => unreachable!(),
    };
    let ws_dest_full = ws.scoped_path(&ws_dest_rel)?;
    if ws_dest_full.exists() {
        return Err(CmdError::msg(format!(
            "Bản ghi đè '{id}' đã tồn tại trong workspace."
        )));
    }

    let org_shared = ws.org().shared_dir().join(&kind);
    let app_shared = state.app_paths.shared_dir().join(&kind);

    let src_path = match kind.as_str() {
        "brands" => {
            let org_candidate = org_shared.join(&id);
            if org_candidate.is_dir() {
                org_candidate
            } else {
                let app_candidate = app_shared.join(&id);
                if app_candidate.is_dir() {
                    app_candidate
                } else {
                    return Err(CmdError::msg(format!(
                        "Không tìm thấy thương hiệu '{id}' ở tầng kế thừa để ghi đè."
                    )));
                }
            }
        }
        "channels" => {
            let file_name = format!("{id}.yaml");
            let org_candidate = org_shared.join(&file_name);
            if org_candidate.is_file() {
                org_candidate
            } else {
                let app_candidate = app_shared.join(&file_name);
                if app_candidate.is_file() {
                    app_candidate
                } else {
                    return Err(CmdError::msg(format!(
                        "Không tìm thấy kênh '{id}' ở tầng kế thừa để ghi đè."
                    )));
                }
            }
        }
        "playbooks" => {
            let file_name = format!("{id}.md");
            let org_candidate = org_shared.join(&file_name);
            if org_candidate.is_file() {
                org_candidate
            } else if org_shared.join(&id).is_dir() {
                org_shared.join(&id)
            } else {
                let app_candidate = app_shared.join(&file_name);
                if app_candidate.is_file() {
                    app_candidate
                } else if app_shared.join(&id).is_dir() {
                    app_shared.join(&id)
                } else {
                    return Err(CmdError::msg(format!(
                        "Không tìm thấy quy trình '{id}' ở tầng kế thừa để ghi đè."
                    )));
                }
            }
        }
        _ => unreachable!(),
    };

    if src_path.is_dir() {
        for entry in std::fs::read_dir(&src_path)? {
            let entry = entry?;
            if entry.file_type()?.is_symlink() {
                return Err(CmdError::msg(
                    "Không cho phép ghi đè thư mục chứa liên kết tượng trưng (symlink / junction).",
                ));
            }
        }
    } else {
        let meta = std::fs::symlink_metadata(&src_path)?;
        if meta.file_type().is_symlink() {
            return Err(CmdError::msg(
                "Không cho phép ghi đè tệp là liên kết tượng trưng (symlink).",
            ));
        }
    }

    let kind_label = match kind.as_str() {
        "brands" => "thương hiệu",
        "channels" => "kênh",
        _ => "quy trình",
    };
    CheckpointService::create(&ws, &format!("Ghi đè {kind_label} {id}"), None)?;

    if src_path.is_dir() {
        copy_dir_all_safe(&src_path, &ws_dest_full)?;
    } else {
        if let Some(p) = ws_dest_full.parent() {
            std::fs::create_dir_all(p)?;
        }
        let bytes = std::fs::read(&src_path)?;
        std::fs::write(&ws_dest_full, &bytes)?;
    }

    Ok(json!({
        "success": true,
        "id": id,
        "kind": kind,
        "path": rel(&ws, &ws_dest_full),
    }))
}

#[tauri::command]
pub fn scope_override(state: State<'_, AppState>, kind: String, id: String) -> CmdResult<Value> {
    scope_override_impl(&state, kind, id)
}

pub fn scope_create_impl(
    state: &AppState,
    kind: String,
    id: String,
    template: Option<String>,
) -> CmdResult<Value> {
    let allowed = ["brands", "channels", "playbooks"];
    if !allowed.contains(&kind.as_str()) {
        return Err(CmdError::msg(
            "Chỉ hỗ trợ tạo mới brands, channels hoặc playbooks.",
        ));
    }
    validate_scope_id(&id)?;
    let ws = state.workspace()?;

    let ws_dest_rel = match kind.as_str() {
        "brands" => PathBuf::from("brands").join(&id),
        "channels" => PathBuf::from("channels").join(format!("{id}.yaml")),
        "playbooks" => PathBuf::from("playbooks").join(format!("{id}.md")),
        _ => unreachable!(),
    };
    let ws_dest_full = ws.scoped_path(&ws_dest_rel)?;
    if ws_dest_full.exists() {
        return Err(CmdError::msg(format!(
            "Mục '{id}' đã tồn tại trong workspace."
        )));
    }

    let kind_label = match kind.as_str() {
        "brands" => "thương hiệu",
        "channels" => "kênh",
        _ => "quy trình",
    };
    CheckpointService::create(&ws, &format!("Tạo mới {kind_label} {id}"), None)?;

    match kind.as_str() {
        "brands" => {
            std::fs::create_dir_all(&ws_dest_full)?;
            let brand_yaml_path = ws_dest_full.join("brand.yaml");
            let default_brand_yaml = r##"name: "Thương hiệu mới"
slogan: "Sáng tạo nội dung chất lượng"
colors:
  primary: "#1E88E5"
  accent: "#FFB300"
  bg: "#0B0F14"
  text: "#FFFFFF"
  extra: []
fonts:
  heading: "Be Vietnam Pro"
  body: "Inter"
logos:
  full: "logo.svg"
  mark: "mark.svg"
  min_size_px: 32
voice: "voice.md"
voice_words:
  prefer: []
  avoid: []
rules: []
version: "brand@1"
"##
            .to_string();
            let brand_content = template
                .filter(|t| !t.trim().is_empty())
                .unwrap_or(default_brand_yaml);
            std::fs::write(&brand_yaml_path, brand_content.as_bytes())?;

            let voice_path = ws_dest_full.join("voice.md");
            if !voice_path.exists() {
                std::fs::write(
                    &voice_path,
                    "# Hướng dẫn giọng điệu\n\n- Tự nhiên, rõ ràng, ngắn gọn.\n".as_bytes(),
                )?;
            }
            let logo_path = ws_dest_full.join("logo.svg");
            if !logo_path.exists() {
                std::fs::write(
                    &logo_path,
                    br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="40" fill="#1E88E5"/><text x="50" y="55" font-size="14" fill="#FFFFFF" text-anchor="middle" font-family="sans-serif">LOGO</text></svg>"##,
                )?;
            }
            let mark_path = ws_dest_full.join("mark.svg");
            if !mark_path.exists() {
                std::fs::write(
                    &mark_path,
                    br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><rect x="20" y="20" width="60" height="60" rx="12" fill="#FFB300"/></svg>"##,
                )?;
            }
        }
        "channels" => {
            if let Some(parent) = ws_dest_full.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let default_channel_yaml = format!(
                r##"id: "{id}"
platform: "custom"
title: "Kênh mới"
formats:
  - name: "standard"
    width: 1080
    height: 1920
    aspect: "9:16"
    fps: 30.0
    max_duration_s: 60.0
    safe_zone:
      top: 0.05
      right: 0.05
      bottom: 0.15
      left: 0.05
text:
  title_max: 100
  description_max: 2000
captions_required: false
file:
  containers: ["mp4", "mov"]
  max_size_mb: 500
notes: "Cấu hình kênh tùy chỉnh"
"##
            );
            let channel_content = template
                .filter(|t| !t.trim().is_empty())
                .unwrap_or(default_channel_yaml);
            std::fs::write(&ws_dest_full, channel_content.as_bytes())?;
        }
        "playbooks" => {
            if let Some(parent) = ws_dest_full.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let default_playbook_md = r##"---
name: "Quy trình mới"
description: "Mô tả các bước thực hiện quy trình nội dung"
kind: playbook
artifacts: []
steps:
  - id: "step-1"
    name: "Khởi tạo nội dung"
    capabilities: []
checkers: []
ask_user_at: []
recipes: []
---
# Hướng dẫn quy trình

1. Bước 1: Chuẩn bị nội dung và ý tưởng
2. Bước 2: Thực hiện và kiểm tra quy chuẩn
"##
            .to_string();
            let playbook_content = template
                .filter(|t| !t.trim().is_empty())
                .unwrap_or(default_playbook_md);
            std::fs::write(&ws_dest_full, playbook_content.as_bytes())?;
        }
        _ => unreachable!(),
    }

    Ok(json!({
        "success": true,
        "id": id,
        "kind": kind,
        "path": rel(&ws, &ws_dest_full),
    }))
}

#[tauri::command]
pub fn scope_create(
    state: State<'_, AppState>,
    kind: String,
    id: String,
    template: Option<String>,
) -> CmdResult<Value> {
    scope_create_impl(&state, kind, id, template)
}

pub fn scope_validate_impl(_state: &AppState, kind: String, text: String) -> CmdResult<Value> {
    let res = studio_brand::validate_scope_item(&kind, &text);
    Ok(json!(res))
}

#[tauri::command]
pub fn scope_validate(state: State<'_, AppState>, kind: String, text: String) -> CmdResult<Value> {
    scope_validate_impl(&state, kind, text)
}

pub fn scope_remove_override_impl(state: &AppState, kind: String, id: String) -> CmdResult<Value> {
    let allowed = ["brands", "channels", "playbooks"];
    if !allowed.contains(&kind.as_str()) {
        return Err(CmdError::msg(
            "Chỉ hỗ trợ xoá bản ghi đè brands, channels hoặc playbooks.",
        ));
    }
    validate_scope_id(&id)?;
    let ws = state.workspace()?;

    let ws_dest_rel = match kind.as_str() {
        "brands" => PathBuf::from("brands").join(&id),
        "channels" => PathBuf::from("channels").join(format!("{id}.yaml")),
        "playbooks" => {
            let md_path = PathBuf::from("playbooks").join(format!("{id}.md"));
            if ws.root().join(&md_path).exists() {
                md_path
            } else {
                PathBuf::from("playbooks").join(&id)
            }
        }
        _ => unreachable!(),
    };

    let full_target = ws.scoped_path(&ws_dest_rel)?;

    if !full_target.starts_with(ws.root()) {
        return Err(CmdError::msg("Đường dẫn nằm ngoài thư mục workspace."));
    }

    let app_shared = state.app_paths.shared_dir();
    let org_shared = ws.org().shared_dir();
    if full_target.starts_with(&app_shared) || full_target.starts_with(&org_shared) {
        return Err(CmdError::msg(
            "Không được phép xoá dữ liệu tầng App hoặc Tổ chức.",
        ));
    }

    if !full_target.exists() {
        return Err(CmdError::msg(format!(
            "Không tìm thấy bản ghi đè '{id}' trong workspace để xoá."
        )));
    }

    let kind_label = match kind.as_str() {
        "brands" => "thương hiệu",
        "channels" => "kênh",
        _ => "quy trình",
    };
    CheckpointService::create(&ws, &format!("Xoá bản ghi đè {kind_label} {id}"), None)?;

    if full_target.is_dir() {
        std::fs::remove_dir_all(&full_target)?;
    } else {
        std::fs::remove_file(&full_target)?;
    }

    Ok(json!({
        "success": true,
        "id": id,
        "kind": kind,
    }))
}

#[tauri::command]
pub fn scope_remove_override(
    state: State<'_, AppState>,
    kind: String,
    id: String,
) -> CmdResult<Value> {
    scope_remove_override_impl(&state, kind, id)
}

// ───────────────────────── Jobs & disk ─────────────────────────

pub async fn jobs_snapshot_impl(state: &AppState) -> CmdResult<Vec<Value>> {
    let snap = state.scheduler.snapshot().await;
    Ok(snap
        .into_iter()
        .filter_map(|s| serde_json::to_value(s).ok())
        .filter_map(|v| {
            let (variant, body) = v.as_object()?.iter().next().map(|(k, b)| (k.clone(), b.clone()))?;
            let state = match variant.as_str() {
                "Queued" => "PENDING",
                "Running" => "RUNNING",
                "Failed" => "FAILED",
                "Cancelled" => "CANCELLED",
                _ => "DONE",
            };
            Some(json!({
                "id": body.get("id").cloned().unwrap_or(Value::Null),
                "title": body.get("tool_id").or_else(|| body.get("id")).cloned().unwrap_or(Value::Null),
                "kind": body.get("tool_id").cloned().unwrap_or(Value::Null),
                "state": state,
                "stageName": body.get("reason").cloned(),
                "createdAt": now(),
                "error": body.get("error").or_else(|| body.get("reason")).filter(|_| state == "FAILED").cloned(),
            }))
        })
        .collect())
}

#[tauri::command]
pub async fn jobs_snapshot(state: State<'_, AppState>) -> CmdResult<Vec<Value>> {
    jobs_snapshot_impl(&state).await
}

pub fn disk_usage_impl(state: &AppState) -> CmdResult<Value> {
    let ws = state.workspace()?;
    let u = studio_workspace::disk_usage(&ws)?;
    let projects = u.sources_bytes + u.renders_bytes;
    Ok(json!({
        "blobsBytes": u.blobs_bytes, "cacheBytes": u.cache_bytes, "projectsBytes": projects,
        "libraryBytes": u.library_bytes, "totalBytes": u.total_bytes,
        "formatted": {
            "blobs": human(u.blobs_bytes), "cache": human(u.cache_bytes), "projects": human(projects),
            "library": human(u.library_bytes), "total": human(u.total_bytes),
        }
    }))
}

#[tauri::command]
pub fn disk_usage(state: State<'_, AppState>) -> CmdResult<Value> {
    disk_usage_impl(&state)
}

// ───────────────────────── agy integration ─────────────────────────

/// Register the `studio-mcp --shim` server and the `mcp(studio)` allow-rule in agy (ADR-008 §4; user consent 2026-10-16).
#[tauri::command]
pub fn agy_connect() -> CmdResult<Value> {
    let exe = std::env::current_exe()?.with_file_name(if cfg!(windows) {
        "studio-mcp.exe"
    } else {
        "studio-mcp"
    });
    if !exe.exists() {
        return Err(CmdError::msg(format!(
            "Không thấy {} cạnh ứng dụng.",
            exe.display()
        )));
    }
    studio_tools::setup_agy(&exe).map_err(|e| CmdError::msg(e.to_string()))?;
    Ok(json!({ "connected": true, "shim": exe }))
}

/// Undo [`agy_connect`].
#[tauri::command]
pub fn agy_disconnect() -> CmdResult<Value> {
    studio_tools::remove_agy().map_err(|e| CmdError::msg(e.to_string()))?;
    Ok(json!({ "connected": false }))
}

// ───────────────────────── Editing, artifacts ─────────────────────────

/// Write a text file (direct edits on the Stage). Paths outside the workspace or in read-only layers are rejected.
pub fn file_write_impl(
    state: &AppState,
    project: Option<String>,
    path: String,
    text: String,
) -> CmdResult<Value> {
    let ws = state.workspace()?;
    let rel_path = match &project {
        Some(slug) if !Path::new(&path).starts_with("projects") => {
            Path::new("projects").join(slug).join(&path)
        }
        _ => PathBuf::from(&path),
    };
    let full = ws.scoped_path(&rel_path)?;
    if !ws.can_write(&full) {
        return Err(CmdError::msg("Tệp này thuộc tầng chỉ đọc."));
    }
    if let Some(dir) = full.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = full.with_extension("lcs-tmp");
    std::fs::write(&tmp, text.as_bytes())?;
    std::fs::rename(&tmp, &full)?;
    Ok(json!({ "path": rel(&ws, &full), "size": text.len() }))
}

#[tauri::command]
pub fn file_write(
    state: State<'_, AppState>,
    project: Option<String>,
    path: String,
    text: String,
) -> CmdResult<Value> {
    file_write_impl(&state, project, path, text)
}

/// Manual checkpoint (e.g. before a series of direct edits).
pub fn checkpoint_create_impl(state: &AppState, label: Option<String>) -> CmdResult<Value> {
    let ws = state.workspace()?;
    let id = CheckpointService::create(&ws, label.as_deref().unwrap_or("Lưu thủ công"), None)?;
    Ok(json!({ "id": id }))
}

#[tauri::command]
pub fn checkpoint_create(state: State<'_, AppState>, label: Option<String>) -> CmdResult<Value> {
    checkpoint_create_impl(&state, label)
}

pub fn graph_get_impl(state: &AppState, project: String) -> CmdResult<Value> {
    let ws = state.workspace()?;
    let p = Project::open(&ws, &project)?;
    let artifacts =
        studio_artifacts::Artifact::list(&p).map_err(|e| CmdError::msg(e.to_string()))?;
    let graph = studio_artifacts::build_project_graph(&ws, &p, &artifacts)
        .map_err(|e| CmdError::msg(e.to_string()))?;
    Ok(json!({
        "project": project,
        "nodes": graph.nodes,
        "edges": graph.edges,
    }))
}

#[tauri::command]
pub fn graph_get(state: State<'_, AppState>, project: String) -> CmdResult<Value> {
    graph_get_impl(&state, project)
}

pub fn artifact_list_impl(state: &AppState, project: String) -> CmdResult<Vec<Value>> {
    let ws = state.workspace()?;
    let p = Project::open(&ws, &project)?;
    let list = studio_artifacts::Artifact::list(&p).map_err(|e| CmdError::msg(e.to_string()))?;
    Ok(list
        .iter()
        .map(|a| {
            json!({
                "id": a.id, "kind": a.kind, "title": a.title, "brand": a.brand, "channels": a.channels,
                "dir": rel(&ws, &a.dir(&p)), "source": rel(&ws, &a.source_path(&p)),
            })
        })
        .collect())
}

#[tauri::command]
pub fn artifact_list(state: State<'_, AppState>, project: String) -> CmdResult<Vec<Value>> {
    artifact_list_impl(&state, project)
}

pub async fn artifact_render_impl(
    state: &AppState,
    project: String,
    id: String,
) -> CmdResult<Value> {
    let ws = state.workspace()?;
    let out = studio_mcp::tools::render_artifact_shared(&ws, &state.scheduler, &project, &id, None)
        .await
        .map_err(CmdError::msg)?;
    Ok(out)
}

#[tauri::command]
pub async fn artifact_render(
    state: State<'_, AppState>,
    project: String,
    id: String,
) -> CmdResult<Value> {
    artifact_render_impl(&state, project, id).await
}

pub fn artifact_check_impl(
    state: &AppState,
    project: String,
    id: String,
    channels: Option<Vec<String>>,
) -> CmdResult<Value> {
    let ws = state.workspace()?;
    let p = Project::open(&ws, &project)?;
    let a = studio_artifacts::Artifact::load(&p, &id).map_err(|e| CmdError::msg(e.to_string()))?;
    // `check_run` resolves the newest render of the artifact itself (render table, then disk).
    let args = json!({
        "artifact_dir": rel(&ws, &a.dir(&p)),
        "brand": a.brand.clone().unwrap_or_else(|| "example".into()),
        "channels": channels.unwrap_or_else(|| a.channels.clone()),
    });
    studio_mcp::tools::check_run(&ws, &args).map_err(CmdError::msg)
}

#[tauri::command]
pub fn artifact_check(
    state: State<'_, AppState>,
    project: String,
    id: String,
    channels: Option<Vec<String>>,
) -> CmdResult<Value> {
    artifact_check_impl(&state, project, id, channels)
}

/// Channel matrix status item for a single artifact x channel pair.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArtifactChannelMatrixCell {
    pub channel_id: String,
    pub channel_name: String,
    pub status: String, // "passed" | "warning" | "error" | "unrendered"
    pub errors_count: usize,
    pub warnings_count: usize,
    pub render_hash: Option<String>,
    pub findings: Vec<Value>,
}

/// Matrix row for an artifact.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArtifactChannelMatrixRow {
    pub artifact_id: String,
    pub artifact_title: String,
    pub kind: String,
    pub source: String,
    pub channels: Vec<ArtifactChannelMatrixCell>,
}

/// Compute channel matrix for all artifacts in a project, using SQLite cache by (render_hash, channel_version).
pub fn artifact_channel_matrix_impl(
    state: &AppState,
    project: String,
) -> CmdResult<Vec<ArtifactChannelMatrixRow>> {
    let ws = state.workspace()?;
    let p = Project::open(&ws, &project)?;
    let artifacts =
        studio_artifacts::Artifact::list(&p).map_err(|e| CmdError::msg(e.to_string()))?;

    // Resolve available channels for project via ScopeChain
    let scope_chain = ScopeChain::with_app_paths(&ws, state.app_paths.clone());
    let resolved_channels = scope_chain.resolve("channels");
    let mut channel_map = std::collections::BTreeMap::new();
    for item in &resolved_channels {
        if item.path.is_file() {
            if let Ok(content) = std::fs::read_to_string(&item.path) {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(content.as_bytes());
                let ch_version = hex::encode(hasher.finalize());
                let title = serde_yaml::from_str::<studio_brand::ChannelSpec>(&content)
                    .map(|s| s.title)
                    .unwrap_or_else(|_| item.id.clone());
                channel_map.insert(item.id.clone(), (title, ch_version));
            }
        }
    }

    let db = WorkspaceDb::open(&ws)?;

    let mut rows = Vec::new();

    for art in artifacts {
        let art_channels: Vec<String> = if !art.channels.is_empty() {
            art.channels.clone()
        } else {
            channel_map.keys().cloned().collect()
        };

        // Query newest render hash for this artifact
        let newest_render: Option<(String, Vec<PathBuf>)> = {
            let stmt = db.conn().prepare(
                "SELECT output_path, hash FROM renders WHERE artifact_id = ?1 ORDER BY created_at DESC",
            ).ok();
            let mut out_hash = None;
            let mut out_paths = Vec::new();
            if let Some(mut stmt) = stmt {
                if let Ok(rows) = stmt.query_map([&art.id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                }) {
                    for r in rows.flatten() {
                        match &out_hash {
                            None => out_hash = Some(r.1.clone()),
                            Some(h) if *h != r.1 => break,
                            Some(_) => {}
                        }
                        if let Ok(p) = ws.scoped_path(&r.0) {
                            if p.is_file() {
                                out_paths.push(p);
                            }
                        }
                    }
                }
            }
            out_hash.map(|h| (h, out_paths))
        };

        let mut cells = Vec::new();

        for ch_id in art_channels {
            let (ch_name, ch_ver) = channel_map
                .get(&ch_id)
                .cloned()
                .unwrap_or_else(|| (ch_id.clone(), "default".to_string()));

            if let Some((render_hash, render_files)) = &newest_render {
                // Check cache in SQLite artifact_checks
                let cached: Option<(String, i64, i64, String)> = db.conn().query_row(
                    "SELECT status, errors_count, warnings_count, findings_json FROM artifact_checks
                     WHERE project = ?5 AND artifact_id = ?1 AND channel_id = ?2 AND render_hash = ?3 AND channel_version = ?4",
                    rusqlite::params![art.id, ch_id, render_hash, ch_ver, project],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                ).ok();

                if let Some((status, err_cnt, warn_cnt, findings_json)) = cached {
                    let findings: Vec<Value> =
                        serde_json::from_str(&findings_json).unwrap_or_default();
                    cells.push(ArtifactChannelMatrixCell {
                        channel_id: ch_id,
                        channel_name: ch_name,
                        status,
                        errors_count: err_cnt as usize,
                        warnings_count: warn_cnt as usize,
                        render_hash: Some(render_hash.clone()),
                        findings,
                    });
                } else {
                    // Cache miss: execute checker for this specific channel
                    let art_dir = art.dir(&p);
                    let source_file = art.source.clone();
                    let source_content = std::fs::read_to_string(art_dir.join(&source_file)).ok();
                    let brand_set = studio_brand::BrandSet::load(&ws)
                        .map_err(|e| CmdError::msg(e.to_string()))?;
                    let brand_slug = art.brand.as_deref().unwrap_or("example");

                    let report = studio_brand::checkers::run_checks(
                        studio_brand::checkers::CheckSubject {
                            artifact_dir: &art_dir,
                            source_content: source_content.as_deref(),
                            source_file: Some(&source_file),
                            renders: render_files,
                            ffprobe_json: None,
                        },
                        &brand_set,
                        brand_slug,
                        &[&ch_id],
                    );

                    let (status, err_cnt, warn_cnt, findings_val) = if let Some(mut rep) = report {
                        let ch_rep = rep.channels.remove(&ch_id);
                        if let Some(ch_rep) = ch_rep {
                            let mut errs = 0;
                            let mut warns = 0;
                            let mut f_vals = Vec::new();
                            for mut f in ch_rep.findings {
                                if let Some(loc) = f.location.as_mut() {
                                    let loc_p = Path::new(loc.as_str());
                                    if loc_p.is_absolute() {
                                        *loc = rel(&ws, loc_p);
                                    }
                                }
                                match f.severity {
                                    studio_brand::checkers::Severity::Error => errs += 1,
                                    studio_brand::checkers::Severity::Warn => warns += 1,
                                    _ => {}
                                }
                                if let Ok(val) = serde_json::to_value(&f) {
                                    f_vals.push(val);
                                }
                            }
                            let st = if errs > 0 {
                                "error"
                            } else if warns > 0 {
                                "warning"
                            } else {
                                "passed"
                            };
                            (st.to_string(), errs, warns, f_vals)
                        } else {
                            ("passed".to_string(), 0, 0, Vec::new())
                        }
                    } else {
                        (
                            "error".to_string(),
                            1,
                            0,
                            vec![json!({
                                "severity": "error",
                                "code": "BRAND_NOT_FOUND",
                                "message_vi": format!("Không tìm thấy cấu hình thương hiệu: {brand_slug}"),
                            })],
                        )
                    };

                    let findings_json_str =
                        serde_json::to_string(&findings_val).unwrap_or_else(|_| "[]".into());
                    let _ = db.conn().execute(
                        "INSERT OR REPLACE INTO artifact_checks
                         (artifact_id, project, channel_id, render_hash, channel_version, status, errors_count, warnings_count, findings_json, checked_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                        rusqlite::params![
                            art.id,
                            project,
                            ch_id,
                            render_hash,
                            ch_ver,
                            status,
                            err_cnt,
                            warn_cnt,
                            findings_json_str,
                            now()
                        ],
                    );

                    cells.push(ArtifactChannelMatrixCell {
                        channel_id: ch_id,
                        channel_name: ch_name,
                        status,
                        errors_count: err_cnt,
                        warnings_count: warn_cnt,
                        render_hash: Some(render_hash.clone()),
                        findings: findings_val,
                    });
                }
            } else {
                // Not rendered
                cells.push(ArtifactChannelMatrixCell {
                    channel_id: ch_id,
                    channel_name: ch_name,
                    status: "unrendered".to_string(),
                    errors_count: 0,
                    warnings_count: 0,
                    render_hash: None,
                    findings: Vec::new(),
                });
            }
        }

        rows.push(ArtifactChannelMatrixRow {
            artifact_id: art.id,
            artifact_title: art.title,
            kind: art.kind,
            source: art.source,
            channels: cells,
        });
    }

    Ok(rows)
}

#[tauri::command]
pub fn artifact_channel_matrix(
    state: State<'_, AppState>,
    project: String,
) -> CmdResult<Vec<ArtifactChannelMatrixRow>> {
    artifact_channel_matrix_impl(&state, project)
}

/// Secret status entry returned to the UI.
#[derive(Clone, serde::Serialize)]
pub struct SecretStatusItem {
    pub name: String,
    pub present: bool,
}

/// Retrieve status of all secrets referenced by registered tool manifests.
pub async fn secret_status_impl(state: &AppState) -> CmdResult<Vec<SecretStatusItem>> {
    let ws = state.workspace().ok();
    let reg = match ws.as_ref() {
        Some(w) => studio_tools::Registry::load(w),
        None => studio_tools::Registry::with_builtins(),
    };

    let referenced = reg.referenced_secrets();
    let org_name = ws.as_ref().map(|w| w.org().name()).unwrap_or("");
    let ws_name = ws.as_ref().map(|w| w.name());

    let mut out = Vec::new();
    for name in referenced {
        let present = state.secrets.get(org_name, ws_name, &name).is_some();
        out.push(SecretStatusItem { name, present });
    }
    Ok(out)
}

#[tauri::command]
pub async fn secret_status(state: State<'_, AppState>) -> CmdResult<Vec<SecretStatusItem>> {
    secret_status_impl(&state).await
}

/// Set a secret in Windows Credential Manager.
pub async fn secret_set_impl(state: &AppState, name: String, value: String) -> CmdResult<()> {
    let ws = state.workspace().ok();
    let reg = match ws.as_ref() {
        Some(w) => studio_tools::Registry::load(w),
        None => studio_tools::Registry::with_builtins(),
    };

    let referenced = reg.referenced_secrets();
    if !referenced.contains(&name) {
        return Err(CmdError::msg(format!(
            "Tên khoá bí mật không hợp lệ: '{name}'. Khoá phải thuộc danh sách khai báo trong tool manifest."
        )));
    }

    let org_name = ws.as_ref().map(|w| w.org().name()).unwrap_or("");
    let ws_name = ws.as_ref().map(|w| w.name());

    state
        .secrets
        .set(org_name, ws_name, &name, value.trim())
        .map_err(|e| CmdError::msg(format!("Lỗi lưu khoá vào Credential Manager: {e}")))?;
    Ok(())
}

#[tauri::command]
pub async fn secret_set(state: State<'_, AppState>, name: String, value: String) -> CmdResult<()> {
    secret_set_impl(&state, name, value).await
}

/// Delete a secret from Windows Credential Manager.
pub async fn secret_delete_impl(state: &AppState, name: String) -> CmdResult<()> {
    let ws = state.workspace().ok();
    let reg = match ws.as_ref() {
        Some(w) => studio_tools::Registry::load(w),
        None => studio_tools::Registry::with_builtins(),
    };

    let referenced = reg.referenced_secrets();
    if !referenced.contains(&name) {
        return Err(CmdError::msg(format!(
            "Tên khoá bí mật không hợp lệ: '{name}'. Khoá phải thuộc danh sách khai báo trong tool manifest."
        )));
    }

    let org_name = ws.as_ref().map(|w| w.org().name()).unwrap_or("");
    let ws_name = ws.as_ref().map(|w| w.name());

    state
        .secrets
        .delete(org_name, ws_name, &name)
        .map_err(|e| CmdError::msg(format!("Lỗi xoá khoá từ Credential Manager: {e}")))?;
    Ok(())
}

#[tauri::command]
pub async fn secret_delete(state: State<'_, AppState>, name: String) -> CmdResult<()> {
    secret_delete_impl(&state, name).await
}

/// Validate endpoint URL format according to requirements:
/// Must parse as URL, scheme https (allow http only for 127.0.0.1/localhost), no userinfo/query/fragment.
pub fn validate_endpoint_url(raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("URL endpoint không được để trống".into());
    }
    let parsed = url::Url::parse(raw).map_err(|e| format!("URL không hợp lệ: {e}"))?;

    let scheme = parsed.scheme();
    let host = parsed.host_str().unwrap_or("");

    let is_loopback = host == "127.0.0.1" || host == "localhost";
    if scheme == "http" {
        if !is_loopback {
            return Err("Giao thức http chỉ được phép dùng cho 127.0.0.1 hoặc localhost".into());
        }
    } else if scheme != "https" {
        return Err("Chỉ hỗ trợ giao thức https (hoặc http cho localhost/127.0.0.1)".into());
    }

    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("URL endpoint không được chứa thông tin xác thực người dùng (userinfo)".into());
    }

    if parsed.query().is_some() {
        return Err("URL endpoint không được chứa chuỗi truy vấn (query string)".into());
    }

    if parsed.fragment().is_some() {
        return Err("URL endpoint không được chứa fragment (#)".into());
    }

    Ok(raw.trim_end_matches('/').to_string())
}

/// Get custom provider endpoints from `app.db`.
pub async fn provider_endpoint_get_impl(
    state: &AppState,
) -> CmdResult<std::collections::HashMap<String, String>> {
    let endpoints = state.endpoints.read().await.clone();
    Ok(endpoints)
}

#[tauri::command]
pub async fn provider_endpoint_get(
    state: State<'_, AppState>,
) -> CmdResult<std::collections::HashMap<String, String>> {
    provider_endpoint_get_impl(&state).await
}

/// Set or remove a custom provider endpoint in `app.db`.
pub async fn provider_endpoint_set_impl(
    state: &AppState,
    tool_id: String,
    base_url: Option<String>,
) -> CmdResult<()> {
    let tool_id = tool_id.trim().to_string();
    if tool_id.is_empty() {
        return Err(CmdError::msg("Thiếu tool_id"));
    }

    let db = state.app_paths.open_app_db().map_err(CmdError::from)?;
    let setting_key = format!("endpoint:{tool_id}");

    match base_url {
        Some(url_str) if !url_str.trim().is_empty() => {
            let validated = validate_endpoint_url(&url_str).map_err(CmdError::msg)?;
            db.set_setting(&setting_key, &validated)
                .map_err(CmdError::from)?;
            state.endpoints.write().await.insert(tool_id, validated);
        }
        _ => {
            db.delete_setting(&setting_key).map_err(CmdError::from)?;
            state.endpoints.write().await.remove(&tool_id);
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn provider_endpoint_set(
    state: State<'_, AppState>,
    tool_id: String,
    base_url: Option<String>,
) -> CmdResult<()> {
    provider_endpoint_set_impl(&state, tool_id, base_url).await
}

/// Get custom provider pricing from `app.db`.
pub async fn provider_pricing_get_impl(
    state: &AppState,
) -> CmdResult<std::collections::HashMap<String, studio_tools::ProviderPricing>> {
    let pricing = state.pricing.read().await.clone();
    Ok(pricing)
}

#[tauri::command]
pub async fn provider_pricing_get(
    state: State<'_, AppState>,
) -> CmdResult<std::collections::HashMap<String, studio_tools::ProviderPricing>> {
    provider_pricing_get_impl(&state).await
}

/// Set or remove custom provider pricing in `app.db`.
pub async fn provider_pricing_set_impl(
    state: &AppState,
    provider_id: String,
    pricing: Option<studio_tools::ProviderPricing>,
) -> CmdResult<()> {
    let provider_id = provider_id.trim().to_string();
    if provider_id.is_empty() {
        return Err(CmdError::msg("Thiếu provider_id"));
    }

    let db = state.app_paths.open_app_db().map_err(CmdError::from)?;
    let setting_key = format!("pricing:{provider_id}");

    match pricing {
        Some(p) => {
            p.validate().map_err(CmdError::msg)?;
            let json_str = serde_json::to_string(&p).map_err(CmdError::from)?;
            db.set_setting(&setting_key, &json_str)
                .map_err(CmdError::from)?;
            state.pricing.write().await.insert(provider_id, p);
        }
        None => {
            db.delete_setting(&setting_key).map_err(CmdError::from)?;
            state.pricing.write().await.remove(&provider_id);
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn provider_pricing_set(
    state: State<'_, AppState>,
    provider_id: String,
    pricing: Option<studio_tools::ProviderPricing>,
) -> CmdResult<()> {
    provider_pricing_set_impl(&state, provider_id, pricing).await
}

/// Environment probe result for external dependencies.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnvProbeItem {
    pub id: String,
    pub name: String,
    pub found: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub hint: String,
}

/// Probes external tools (Node, npx, FFmpeg, ffprobe, Chrome/Edge, claude, agy) via real binaries.
pub async fn env_probe_impl(state: &AppState) -> CmdResult<Vec<EnvProbeItem>> {
    // "Quét lại" forces a fresh PATH inspection from registry
    studio_workspace::env_path::get_fresh_paths(true);

    let mut items = Vec::new();

    let (node, npx) = studio_artifacts::probe_node_npx();
    items.push(EnvProbeItem {
        id: "node".into(),
        name: "Node.js".into(),
        found: node.available,
        version: node.version,
        path: node.path.as_deref().map(strip_verbatim),
        hint:
            "Cài đặt Node.js từ https://nodejs.org hoặc qua winget: 'winget install OpenJS.NodeJS'"
                .into(),
    });

    items.push(EnvProbeItem {
        id: "npx".into(),
        name: "npx".into(),
        found: npx.available,
        version: npx.version,
        path: npx.path.as_deref().map(strip_verbatim),
        hint: "npx đi kèm với Node.js. Hãy cài đặt Node.js từ https://nodejs.org".into(),
    });

    let ffmpeg = studio_artifacts::probe_ffmpeg();
    items.push(EnvProbeItem {
        id: "ffmpeg".into(),
        name: "FFmpeg".into(),
        found: ffmpeg.available,
        version: ffmpeg.version,
        path: ffmpeg.path.as_deref().map(strip_verbatim),
        hint: "Tải FFmpeg từ https://ffmpeg.org hoặc qua winget: 'winget install Gyan.FFmpeg'"
            .into(),
    });

    let ffprobe = studio_artifacts::probe_ffprobe();
    items.push(EnvProbeItem {
        id: "ffprobe".into(),
        name: "ffprobe".into(),
        found: ffprobe.available,
        version: ffprobe.version,
        path: ffprobe.path.as_deref().map(strip_verbatim),
        hint: "ffprobe đi kèm với FFmpeg. Hãy tải từ https://ffmpeg.org hoặc 'winget install Gyan.FFmpeg'".into(),
    });

    let chromium = studio_artifacts::probe_chromium();
    items.push(EnvProbeItem {
        id: "chromium".into(),
        name: "Chrome / Microsoft Edge".into(),
        found: chromium.available,
        version: chromium.version,
        path: chromium.path.as_deref().map(strip_verbatim),
        hint: "Cài đặt Google Chrome (https://google.com/chrome) hoặc Microsoft Edge để render HTML sang PNG/PDF".into(),
    });

    let ytdlp_path = studio_tools::find_in_path("yt-dlp");
    let ytdlp_ver = studio_tools::probe_ytdlp().ok();
    items.push(EnvProbeItem {
        id: "yt-dlp".into(),
        name: "yt-dlp".into(),
        found: ytdlp_path.is_some(),
        version: ytdlp_ver,
        path: ytdlp_path.as_deref().map(strip_verbatim),
        hint: "Cài đặt yt-dlp qua winget: 'winget install yt-dlp' hoặc tải từ https://github.com/yt-dlp/yt-dlp".into(),
    });

    let claude = state.claude.probe().await;
    items.push(EnvProbeItem {
        id: "claude".into(),
        name: "Claude Code CLI".into(),
        found: claude.installed,
        version: claude.version,
        path: None,
        hint: "Cài đặt Claude Code qua npm: 'npm install -g @anthropic-ai/claude-code'".into(),
    });

    let agy = state.agy.probe().await;
    items.push(EnvProbeItem {
        id: "agy".into(),
        name: "Antigravity CLI (agy)".into(),
        found: agy.installed,
        version: agy.version,
        path: None,
        hint: "Cài đặt Antigravity CLI theo tài liệu Google Antigravity".into(),
    });

    let vieneu_probe = studio_tools::probe_vieneu();
    let python_path =
        studio_tools::find_in_path("python").or_else(|| studio_tools::find_in_path("python3"));
    let vieneu_ver = if vieneu_probe.available {
        let trimmed = vieneu_probe
            .note_vi
            .trim_start_matches("Sẵn sàng (VieNeu-TTS ")
            .trim_end_matches(')')
            .trim();
        Some(trimmed.to_string())
    } else {
        None
    };
    items.push(EnvProbeItem {
        id: "tts-vieneu".into(),
        name: "VieNeu-TTS (Python v3 Turbo)".into(),
        found: vieneu_probe.available,
        version: vieneu_ver,
        path: python_path.as_deref().map(strip_verbatim),
        hint: "Cài đặt thư viện: 'pip install vieneu' (Yêu cầu Python 3.10+ và eSpeak NG)".into(),
    });

    let piper_bin = studio_tools::find_piper_binary();
    let piper_ver = if let Some(ref bin) = piper_bin {
        let mut cmd = std::process::Command::new(bin);
        cmd.arg("--version");
        match cmd.output() {
            Ok(out) if out.status.success() => {
                let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if v.is_empty() {
                    Some(studio_tools::PIPER_RELEASE_VERSION.to_string())
                } else {
                    Some(v)
                }
            }
            _ => Some(studio_tools::PIPER_RELEASE_VERSION.to_string()),
        }
    } else {
        None
    };
    items.push(EnvProbeItem {
        id: "tts-piper".into(),
        name: "Piper TTS".into(),
        found: piper_bin.is_some(),
        version: piper_ver,
        path: piper_bin.as_deref().map(strip_verbatim),
        hint: "Tải Piper từ https://github.com/rhasspy/piper/releases hoặc tải trực tiếp trong Cài đặt → Giọng đọc (TTS)".into(),
    });

    Ok(items)
}

#[tauri::command]
pub async fn env_probe(state: State<'_, AppState>) -> CmdResult<Vec<EnvProbeItem>> {
    env_probe_impl(&state).await
}

/// Request payload for tool install preview.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInstallPreviewRequest {
    pub source: String,
    pub scope: studio_tools::ToolInstallScope,
}

/// Request payload for tool install confirmation.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInstallConfirmRequest {
    pub preview_id: String,
}

/// Request payload for tool uninstallation.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolUninstallRequest {
    pub tool_id: String,
    pub version: Option<String>,
    pub scope: Option<studio_tools::ToolInstallScope>,
}

/// Previews a tool installation from a local folder or an HTTPS zip URL.
pub async fn tool_install_preview_impl(
    state: &AppState,
    req: ToolInstallPreviewRequest,
) -> CmdResult<studio_tools::ToolInstallPreview> {
    let ws = state.workspace().ok();
    let preview =
        studio_tools::preview_install(&req.source, req.scope, ws.as_ref(), &state.app_paths)
            .await
            .map_err(|e| CmdError::msg(e.to_string()))?;

    // Store in pending previews
    if let Ok(mut map) = state.tool_previews.lock() {
        map.insert(preview.preview_id.clone(), preview.clone());
    }

    Ok(preview)
}

#[tauri::command]
pub async fn tool_install_preview(
    state: State<'_, AppState>,
    req: ToolInstallPreviewRequest,
) -> CmdResult<studio_tools::ToolInstallPreview> {
    tool_install_preview_impl(&state, req).await
}

/// Confirms and finalizes an installation preview.
pub async fn tool_install_confirm_impl(
    state: &AppState,
    req: ToolInstallConfirmRequest,
) -> CmdResult<studio_tools::InstalledToolInfo> {
    let preview = {
        let mut map = state
            .tool_previews
            .lock()
            .map_err(|_| CmdError::msg("Không thể khóa bộ nhớ xem trước"))?;
        map.remove(&req.preview_id)
            .ok_or_else(|| CmdError::msg("Phiên xem trước cài đặt không tồn tại hoặc đã hết hạn"))?
    };

    let ws = state.workspace().ok();
    let installed = studio_tools::confirm_install(&preview, state.secrets.as_ref(), ws.as_ref())
        .map_err(|e| CmdError::msg(e.to_string()))?;

    Ok(installed)
}

#[tauri::command]
pub async fn tool_install_confirm(
    state: State<'_, AppState>,
    req: ToolInstallConfirmRequest,
) -> CmdResult<studio_tools::InstalledToolInfo> {
    tool_install_confirm_impl(&state, req).await
}

/// Lists all tools (builtin and installed) with their probe results.
pub async fn tool_list_installed_impl(
    state: &AppState,
) -> CmdResult<Vec<studio_tools::InstalledToolInfo>> {
    let ws = state.workspace().ok();
    let reg = match ws.as_ref() {
        Some(w) => studio_tools::Registry::load(w),
        None => studio_tools::Registry::with_builtins(),
    };

    let list = studio_tools::list_tools_with_probe(
        &reg,
        state.secrets.as_ref(),
        ws.as_ref(),
        &state.app_paths,
    );

    Ok(list)
}

#[tauri::command]
pub async fn tool_list_installed(
    state: State<'_, AppState>,
) -> CmdResult<Vec<studio_tools::InstalledToolInfo>> {
    tool_list_installed_impl(&state).await
}

/// Uninstalls an installed tool.
pub async fn tool_uninstall_impl(state: &AppState, req: ToolUninstallRequest) -> CmdResult<bool> {
    let ws = state.workspace().ok();
    studio_tools::uninstall_tool(
        &req.tool_id,
        req.version.as_deref(),
        req.scope,
        ws.as_ref(),
        &state.app_paths,
    )
    .map_err(|e| CmdError::msg(e.to_string()))
}

#[tauri::command]
pub async fn tool_uninstall(
    state: State<'_, AppState>,
    req: ToolUninstallRequest,
) -> CmdResult<bool> {
    tool_uninstall_impl(&state, req).await
}

// ───────────────────────── Personal Web Access ─────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebConfigureRequest {
    pub enabled: Option<bool>,
    pub lan_allowed: Option<bool>,
    pub port: Option<u16>,
    pub password: Option<String>,
}

pub fn web_status_impl(state: &AppState) -> CmdResult<Value> {
    let db = state.app_paths.open_app_db()?;
    let running = state
        .web_server
        .lock()
        .map(|s| s.is_some())
        .unwrap_or(false);
    let lan_allowed = db
        .get_setting("web:lan_allowed")?
        .map(|v| v == "true")
        .unwrap_or(false);
    let port: u16 = db
        .get_setting("web:port")?
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000);
    let has_password = db.get_setting("web:password_hash")?.is_some();
    let lan_ip = studio_web::lan::detect_lan_ip();
    let lan_url = lan_ip.as_ref().map(|ip| format!("http://{ip}:{port}"));
    let qr_code_svg = if lan_allowed && has_password {
        lan_url
            .as_ref()
            .and_then(|url| studio_web::qr::generate_qr_svg(url).ok())
    } else {
        None
    };

    Ok(json!({
        "enabled": running,
        "lanAllowed": lan_allowed,
        "port": port,
        "hasPassword": has_password,
        "lanIp": lan_ip,
        "lanUrl": lan_url,
        "qrCodeSvg": qr_code_svg,
    }))
}

#[tauri::command]
pub fn web_status(state: State<'_, AppState>) -> CmdResult<Value> {
    web_status_impl(&state)
}

pub async fn web_configure_impl(state: &AppState, req: WebConfigureRequest) -> CmdResult<Value> {
    let db = state.app_paths.open_app_db()?;

    if let Some(pw) = req.password {
        let trimmed = pw.trim();
        if !trimmed.is_empty() {
            let hash = studio_web::auth::hash_password(trimmed)
                .map_err(|e| CmdError::msg(format!("Không thể tạo băm mật khẩu: {e}")))?;
            db.set_setting("web:password_hash", &hash)?;
        }
    }

    let mut needs_restart = false;

    if let Some(lan) = req.lan_allowed {
        db.set_setting("web:lan_allowed", if lan { "true" } else { "false" })?;
        needs_restart = true;
    }

    if let Some(port) = req.port {
        db.set_setting("web:port", &port.to_string())?;
        needs_restart = true;
    }

    let is_currently_running = state
        .web_server
        .lock()
        .map(|s| s.is_some())
        .unwrap_or(false);

    if let Some(enabled) = req.enabled {
        db.set_setting("web:enabled", if enabled { "true" } else { "false" })?;
        if enabled {
            let has_pw = db.get_setting("web:password_hash")?.is_some();
            if !has_pw {
                return Err(CmdError::msg(
                    "Phải thiết lập mật khẩu chủ sở hữu trước khi bật chế độ web.",
                ));
            }
            crate::web_dispatch::start_web_server(state).await?;
        } else {
            crate::web_dispatch::stop_web_server(state);
        }
    } else if needs_restart && is_currently_running {
        crate::web_dispatch::start_web_server(state).await?;
    }

    web_status_impl(state)
}

#[tauri::command]
pub async fn web_configure(
    state: State<'_, AppState>,
    req: Option<WebConfigureRequest>,
    enabled: Option<bool>,
    lan_allowed: Option<bool>,
    port: Option<u16>,
    password: Option<String>,
) -> CmdResult<Value> {
    let final_req = if let Some(r) = req {
        r
    } else {
        WebConfigureRequest {
            enabled,
            lan_allowed,
            port,
            password,
        }
    };
    web_configure_impl(&state, final_req).await
}

/// Individual voice entry for TTS settings.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsVoiceItem {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub quality: Option<String>,
    pub downloaded: bool,
    pub size_bytes: Option<u64>,
    pub size_human: Option<String>,
    pub license_note: Option<String>,
}

/// Provider details for TTS settings.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsProviderInfo {
    pub id: String,
    pub name: String,
    pub available: bool,
    pub note_vi: String,
    pub voices: Vec<TtsVoiceItem>,
    pub binary_installed: bool,
    pub binary_path: Option<String>,
    pub binary_version: Option<String>,
}

/// Overall TTS status response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsStatusResponse {
    pub providers: Vec<TtsProviderInfo>,
    pub default_provider: Option<String>,
    pub default_voice: Option<String>,
}

/// Model or binary download preview.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsModelPreview {
    pub provider: String,
    pub voice: String,
    pub name: String,
    pub size_bytes: u64,
    pub size_human: String,
    pub license: String,
    pub license_note: String,
    pub download_urls: Vec<String>,
}

/// Result of installing a model or binary.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsModelInstallResult {
    pub success: bool,
    pub message: String,
    pub provider: String,
    pub voice: String,
}

/// Request for synthesizing speech preview.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsPreviewRequest {
    pub text: String,
    pub provider: Option<String>,
    pub voice: Option<String>,
    pub speed: Option<f32>,
}

/// Response from synthesizing speech preview.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsPreviewResponse {
    pub audio_url: String,
    pub audio_path: String,
    pub provider: String,
    pub voice: String,
    pub duration_estimate_secs: Option<f64>,
    pub size_bytes: u64,
    pub note: Option<String>,
}

/// TTS configuration response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsConfigResponse {
    pub default_provider: Option<String>,
    pub default_voice: Option<String>,
}

/// TTS configuration update request.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsSetConfigRequest {
    pub default_provider: Option<String>,
    pub default_voice: Option<String>,
}

fn get_windows_sapi_voices() -> Vec<String> {
    #[cfg(windows)]
    {
        let mut cmd = std::process::Command::new("powershell");
        cmd.args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            "Add-Type -AssemblyName System.Speech; $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; ($s.GetInstalledVoices() | Where-Object { $_.Enabled } | ForEach-Object { $_.VoiceInfo.Name }) -join ';;'",
        ]);
        if let Ok(out) = cmd.output() {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let list: Vec<String> = s
                    .split(";;")
                    .map(str::trim)
                    .filter(|n| !n.is_empty())
                    .map(String::from)
                    .collect();
                if !list.is_empty() {
                    return list;
                }
            }
        }
    }
    vec![
        "Microsoft An".into(),
        "Microsoft Mai".into(),
        "SAPI Default".into(),
    ]
}

/// Retrieves TTS status, installed providers, voices, and active configuration.
pub async fn tts_status_impl(state: &AppState) -> CmdResult<TtsStatusResponse> {
    let mut providers = Vec::new();

    // 1. VieNeu-TTS
    let vieneu_probe = studio_tools::probe_vieneu();
    let python_path =
        studio_tools::find_in_path("python").or_else(|| studio_tools::find_in_path("python3"));
    let vieneu_voices = studio_tools::VIENEU_DEFAULT_VOICES
        .iter()
        .map(|v| TtsVoiceItem {
            id: v.to_string(),
            name: format!("VieNeu {}", v),
            provider: "tts.vieneu".into(),
            quality: Some("48kHz".into()),
            downloaded: vieneu_probe.available,
            size_bytes: None,
            size_human: None,
            license_note: Some("Mô hình VieNeu-TTS mã nguồn mở Apache-2.0.".into()),
        })
        .collect();

    providers.push(TtsProviderInfo {
        id: "tts.vieneu".into(),
        name: "VieNeu-TTS (v3 Turbo ONNX)".into(),
        available: vieneu_probe.available,
        note_vi: vieneu_probe.note_vi,
        voices: vieneu_voices,
        binary_installed: python_path.is_some(),
        binary_path: python_path.as_deref().map(strip_verbatim),
        binary_version: if vieneu_probe.available {
            Some("v3-turbo".into())
        } else {
            None
        },
    });

    // 2. Piper TTS
    let piper_probe = studio_tools::probe_piper();
    let piper_bin = studio_tools::find_piper_binary();
    let installed_piper = studio_tools::get_installed_piper_voices();
    let piper_voices = studio_tools::PIPER_VIETNAMESE_VOICES
        .iter()
        .map(|v| {
            let downloaded = installed_piper.contains(&v.id.to_string());
            let total_size = v.onnx_size_bytes + v.json_size_bytes;
            TtsVoiceItem {
                id: v.id.to_string(),
                name: v.name.to_string(),
                provider: "tts.piper".into(),
                quality: Some(v.quality.to_string()),
                downloaded,
                size_bytes: Some(total_size),
                size_human: Some(human(total_size)),
                license_note: Some(studio_tools::VIETNAMESE_VOICE_LICENSE_NOTE.into()),
            }
        })
        .collect();

    providers.push(TtsProviderInfo {
        id: "tts.piper".into(),
        name: "Piper TTS (Offline Fast ONNX)".into(),
        available: piper_probe.available,
        note_vi: piper_probe.note_vi,
        voices: piper_voices,
        binary_installed: piper_bin.is_some(),
        binary_path: piper_bin.as_deref().map(strip_verbatim),
        binary_version: if piper_bin.is_some() {
            Some(studio_tools::PIPER_RELEASE_VERSION.into())
        } else {
            None
        },
    });

    // 3. Windows SAPI
    let sapi_voices_raw = get_windows_sapi_voices();
    let sapi_voices = sapi_voices_raw
        .iter()
        .map(|v| TtsVoiceItem {
            id: v.clone(),
            name: v.clone(),
            provider: "tts.windows-sapi".into(),
            quality: Some("SAPI".into()),
            downloaded: true,
            size_bytes: None,
            size_human: None,
            license_note: Some("Giọng đọc tích hợp sẵn trong hệ điều hành Windows.".into()),
        })
        .collect();

    providers.push(TtsProviderInfo {
        id: "tts.windows-sapi".into(),
        name: "Windows SAPI (Hệ thống)".into(),
        available: cfg!(windows),
        note_vi: if cfg!(windows) {
            "Sẵn sàng (Tích hợp trong Windows)".into()
        } else {
            "Chỉ hỗ trợ trên Windows".into()
        },
        voices: sapi_voices,
        binary_installed: cfg!(windows),
        binary_path: None,
        binary_version: None,
    });

    let (mut default_provider, mut default_voice) = (None, None);
    if let Ok(db) = state.app_paths.open_app_db() {
        default_provider = db.get_setting("tts:default_provider").ok().flatten();
        default_voice = db.get_setting("tts:default_voice").ok().flatten();
    }

    if default_provider.is_none() {
        if vieneu_probe.available {
            default_provider = Some("tts.vieneu".into());
        } else if piper_probe.available {
            default_provider = Some("tts.piper".into());
        } else {
            default_provider = Some("tts.windows-sapi".into());
        }
    }

    if default_voice.is_none() {
        if default_provider.as_deref() == Some("tts.vieneu") {
            default_voice = Some("Bình".into());
        } else if default_provider.as_deref() == Some("tts.piper") {
            default_voice = Some("vi_VN-vais1000-medium".into());
        } else {
            default_voice = sapi_voices_raw.first().cloned();
        }
    }

    Ok(TtsStatusResponse {
        providers,
        default_provider,
        default_voice,
    })
}

/// Retrieves TTS status, installed providers, voices, and active configuration.
#[tauri::command]
pub async fn tts_status(state: State<'_, AppState>) -> CmdResult<TtsStatusResponse> {
    tts_status_impl(&state).await
}

/// Generates download preview info (size, license terms) before downloading a model or binary.
pub async fn tts_model_preview_impl(
    _state: &AppState,
    provider: String,
    voice: String,
) -> CmdResult<TtsModelPreview> {
    let p_lower = provider.to_lowercase();
    if p_lower == "tts.piper" || p_lower == "piper" {
        if voice == "binary" || voice == "piper-bin" {
            let size = studio_tools::PIPER_ZIP_SIZE_BYTES;
            return Ok(TtsModelPreview {
                provider: "tts.piper".into(),
                voice: "binary".into(),
                name: format!("Piper TTS Binary ({})", studio_tools::PIPER_RELEASE_VERSION),
                size_bytes: size,
                size_human: human(size),
                license: "MIT".into(),
                license_note:
                    "Trình biên dịch mã nguồn mở Piper TTS từ Rhasspy (GitHub: rhasspy/piper)."
                        .into(),
                download_urls: vec![studio_tools::PIPER_ZIP_URL.into()],
            });
        }

        if let Some(meta) = studio_tools::PIPER_VIETNAMESE_VOICES
            .iter()
            .find(|v| v.id == voice)
        {
            let total_size = meta.onnx_size_bytes + meta.json_size_bytes;
            return Ok(TtsModelPreview {
                provider: "tts.piper".into(),
                voice: meta.id.into(),
                name: meta.name.into(),
                size_bytes: total_size,
                size_human: human(total_size),
                license: "CC-BY 4.0 (Nghiên cứu / Phi thương mại)".into(),
                license_note: studio_tools::VIETNAMESE_VOICE_LICENSE_NOTE.into(),
                download_urls: vec![
                    format!(
                        "{}/{}",
                        studio_tools::PIPER_HF_BASE_URL,
                        meta.onnx_relative_path
                    ),
                    format!(
                        "{}/{}",
                        studio_tools::PIPER_HF_BASE_URL,
                        meta.json_relative_path
                    ),
                ],
            });
        }
    }

    Err(CmdError::msg(format!(
        "Không tìm thấy thông tin gói tải về cho {provider} / {voice}"
    )))
}

/// Generates download preview info (size, license terms) before downloading a model or binary.
#[tauri::command]
pub async fn tts_model_preview(
    state: State<'_, AppState>,
    provider: String,
    voice: String,
) -> CmdResult<TtsModelPreview> {
    tts_model_preview_impl(&state, provider, voice).await
}

/// Downloads and installs a TTS model or binary with SHA-256 verification after confirmation.
pub async fn tts_model_install_impl(
    _state: &AppState,
    provider: String,
    voice: String,
) -> CmdResult<TtsModelInstallResult> {
    let p_lower = provider.to_lowercase();
    if p_lower == "tts.piper" || p_lower == "piper" {
        if voice == "binary" || voice == "piper-bin" {
            let bin_path = studio_tools::install_piper_binary()
                .await
                .map_err(|e| CmdError::msg(e.to_string()))?;
            return Ok(TtsModelInstallResult {
                success: true,
                message: format!(
                    "Đã cài đặt Piper binary thành công tại {}",
                    strip_verbatim(&bin_path)
                ),
                provider: "tts.piper".into(),
                voice: "binary".into(),
            });
        }

        if studio_tools::PIPER_VIETNAMESE_VOICES
            .iter()
            .any(|v| v.id == voice)
        {
            studio_tools::install_piper_voice(&voice)
                .await
                .map_err(|e| CmdError::msg(e.to_string()))?;
            return Ok(TtsModelInstallResult {
                success: true,
                message: format!("Đã tải và cài đặt thành công mô hình giọng đọc '{voice}'"),
                provider: "tts.piper".into(),
                voice,
            });
        }
    }

    Err(CmdError::msg(format!(
        "Không hỗ trợ cài đặt tự động cho {provider} / {voice}"
    )))
}

/// Downloads and installs a TTS model or binary with SHA-256 verification after confirmation.
#[tauri::command]
pub async fn tts_model_install(
    state: State<'_, AppState>,
    provider: String,
    voice: String,
) -> CmdResult<TtsModelInstallResult> {
    tts_model_install_impl(&state, provider, voice).await
}

/// Synthesize speech for testing/preview in settings ("Nghe thử").
pub async fn tts_synthesize_preview_impl(
    state: &AppState,
    req: TtsPreviewRequest,
) -> CmdResult<TtsPreviewResponse> {
    let text = req.text.trim();
    if text.is_empty() {
        return Err(CmdError::msg("Văn bản nghe thử không được để trống."));
    }

    let preview_dir = state
        .app_paths
        .data_dir()
        .join("cache")
        .join("tts-previews");
    let _ = std::fs::create_dir_all(&preview_dir);

    let target_wav = preview_dir.join(format!("preview-{}.wav", uuid::Uuid::new_v4()));

    // Determine provider
    let chosen_provider = if let Some(p) = req.provider {
        p
    } else {
        let mut def = None;
        if let Ok(db) = state.app_paths.open_app_db() {
            def = db.get_setting("tts:default_provider").ok().flatten();
        }
        def.unwrap_or_else(|| {
            if studio_tools::probe_vieneu().available {
                "tts.vieneu".into()
            } else if studio_tools::probe_piper().available {
                "tts.piper".into()
            } else {
                "tts.windows-sapi".into()
            }
        })
    };

    let actual_voice: String;
    let note: String;

    if chosen_provider == "tts.vieneu" {
        let probe = studio_tools::probe_vieneu();
        if !probe.available {
            return Err(CmdError::msg(probe.note_vi));
        }

        let py_exe = studio_tools::find_in_path("python")
            .or_else(|| studio_tools::find_in_path("python3"))
            .ok_or_else(|| CmdError::msg("Chưa cài đặt Python."))?;

        let script = studio_tools::ensure_helper_script(&state.app_paths.data_dir().join("cache"))
            .map_err(|e| CmdError::msg(e.to_string()))?;

        let mut temp_txt = tempfile::Builder::new()
            .prefix("vieneu-preview-")
            .suffix(".txt")
            .tempfile()
            .map_err(|e| CmdError::msg(format!("Không thể tạo tệp tạm: {e}")))?;

        std::io::Write::write_all(&mut temp_txt, text.as_bytes())
            .map_err(|e| CmdError::msg(format!("Không thể ghi tệp tạm: {e}")))?;
        std::io::Write::flush(&mut temp_txt)
            .map_err(|e| CmdError::msg(format!("Không thể flush tệp tạm: {e}")))?;

        let voice_name = req.voice.as_deref().unwrap_or("Bình");
        actual_voice = voice_name.to_string();

        let cmd_args = studio_tools::build_vieneu_args(
            &script,
            temp_txt.path(),
            &target_wav,
            Some(voice_name),
            req.speed,
            None,
            None,
        );

        let output = std::process::Command::new(&py_exe)
            .args(&cmd_args)
            .output()
            .map_err(|e| CmdError::msg(format!("Lỗi chạy Python VieNeu: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(CmdError::msg(format!("VieNeu-TTS lỗi: {}", stderr.trim())));
        }

        note = "Tổng hợp thành công bằng VieNeu-TTS v3 Turbo".into();
    } else if chosen_provider == "tts.piper" {
        let piper_exe = studio_tools::find_piper_binary().ok_or_else(|| {
            CmdError::msg("Chưa cài đặt Piper TTS. Vui lòng bấm 'Cài đặt binary' trước.")
        })?;

        let installed = studio_tools::get_installed_piper_voices();
        if installed.is_empty() {
            return Err(CmdError::msg(
                "Chưa có mô hình giọng đọc Piper nào. Vui lòng tải một giọng đọc tiếng Việt trước.",
            ));
        }

        let voice_name = if let Some(ref v) = req.voice {
            if installed.contains(v) {
                v.clone()
            } else {
                installed[0].clone()
            }
        } else {
            installed[0].clone()
        };
        actual_voice = voice_name.clone();

        let models_root = studio_tools::piper_models_dir();
        let voice_dir = models_root.join(&voice_name);
        let onnx_path = voice_dir.join(format!("{voice_name}.onnx"));
        let json_path = voice_dir.join(format!("{voice_name}.onnx.json"));

        if !onnx_path.is_file() || !json_path.is_file() {
            return Err(CmdError::msg(format!(
                "Mô hình '{voice_name}' chưa hoàn thiện trên đĩa."
            )));
        }

        let cmd_args =
            studio_tools::build_piper_args(&onnx_path, &json_path, &target_wav, req.speed, None);

        let mut child = std::process::Command::new(&piper_exe)
            .args(&cmd_args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| CmdError::msg(format!("Lỗi khởi chạy Piper: {e}")))?;

        if let Some(mut stdin) = child.stdin.take() {
            let _ = std::io::Write::write_all(&mut stdin, text.as_bytes());
            let _ = std::io::Write::flush(&mut stdin);
        }

        let output = child
            .wait_with_output()
            .map_err(|e| CmdError::msg(format!("Lỗi chờ Piper hoàn tất: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(CmdError::msg(format!("Piper lỗi: {}", stderr.trim())));
        }

        let lic_note = studio_tools::VIETNAMESE_VOICE_LICENSE_NOTE;
        note = format!("Tổng hợp thành công bằng Piper ({voice_name}). {lic_note}");
    } else {
        // Windows SAPI
        let voice_name = req.voice.unwrap_or_else(|| "Microsoft An".into());
        actual_voice = voice_name.clone();

        #[cfg(windows)]
        {
            let ps_script = r#"[Console]::InputEncoding = [System.Text.Encoding]::UTF8;
$json = [Console]::In.ReadToEnd() | ConvertFrom-Json;
Add-Type -AssemblyName System.Speech;
$s = New-Object System.Speech.Synthesis.SpeechSynthesizer;
if ($json.voice) {
    try { $s.SelectVoice($json.voice) } catch {}
}
if ($json.rate) {
    $s.Rate = [int]$json.rate
}
$s.SetOutputToWaveFile($json.output_path);
$s.Speak($json.text);
$s.Dispose();"#
                .to_string();

            let rate: i32 = req
                .speed
                .map(|s| ((s - 1.0) * 10.0).clamp(-10.0, 10.0) as i32)
                .unwrap_or(0);
            let input_obj = serde_json::json!({
                "text": text,
                "voice": voice_name,
                "rate": rate,
                "output_path": target_wav.to_string_lossy().to_string(),
            });

            let mut child = std::process::Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-Command",
                    &ps_script,
                ])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| CmdError::msg(format!("Lỗi khởi chạy PowerShell SAPI: {e}")))?;

            if let Some(mut stdin) = child.stdin.take() {
                let _ = std::io::Write::write_all(&mut stdin, input_obj.to_string().as_bytes());
                let _ = std::io::Write::flush(&mut stdin);
            }

            let output = child
                .wait_with_output()
                .map_err(|e| CmdError::msg(format!("Lỗi chạy Windows SAPI: {e}")))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                return Err(CmdError::msg(format!(
                    "Windows SAPI lỗi: {}",
                    stderr.trim()
                )));
            }
        }
        #[cfg(not(windows))]
        {
            return Err(CmdError::msg(
                "Windows SAPI chỉ hỗ trợ trên hệ điều hành Windows.",
            ));
        }

        note = format!("Tổng hợp thành công bằng Windows SAPI ({actual_voice})");
    }

    if !target_wav.is_file() {
        return Err(CmdError::msg(
            "Tiến trình hoàn tất nhưng không tạo được tệp âm thanh đầu ra.",
        ));
    }

    let meta = std::fs::metadata(&target_wav).map_err(|e| CmdError::msg(e.to_string()))?;
    let size_bytes = meta.len();
    // Rough estimate for 16-bit 22kHz mono PCM WAV: ~44,100 bytes/sec
    let duration_estimate_secs = Some((size_bytes as f64 / 44_100.0).max(0.1));

    Ok(TtsPreviewResponse {
        audio_url: asset_url(&target_wav),
        audio_path: target_wav.to_string_lossy().to_string(),
        provider: chosen_provider,
        voice: actual_voice,
        duration_estimate_secs,
        size_bytes,
        note: Some(note),
    })
}

/// Synthesize speech for testing/preview in settings ("Nghe thử").
#[tauri::command]
pub async fn tts_synthesize_preview(
    app: AppHandle,
    state: State<'_, AppState>,
    req: TtsPreviewRequest,
) -> CmdResult<TtsPreviewResponse> {
    let preview_dir = state
        .app_paths
        .data_dir()
        .join("cache")
        .join("tts-previews");
    let _ = std::fs::create_dir_all(&preview_dir);
    let _ = app
        .asset_protocol_scope()
        .allow_directory(&preview_dir, true);

    tts_synthesize_preview_impl(&state, req).await
}

/// Get stored default TTS provider and voice.
pub fn tts_get_config_impl(state: &AppState) -> CmdResult<TtsConfigResponse> {
    let mut default_provider = None;
    let mut default_voice = None;

    if let Ok(db) = state.app_paths.open_app_db() {
        default_provider = db.get_setting("tts:default_provider").ok().flatten();
        default_voice = db.get_setting("tts:default_voice").ok().flatten();
    }

    Ok(TtsConfigResponse {
        default_provider,
        default_voice,
    })
}

/// Get stored default TTS provider and voice.
#[tauri::command]
pub fn tts_get_config(state: State<'_, AppState>) -> CmdResult<TtsConfigResponse> {
    tts_get_config_impl(&state)
}

/// Save user's preferred default TTS provider and voice.
pub fn tts_set_config_impl(state: &AppState, req: TtsSetConfigRequest) -> CmdResult<()> {
    let db = state.app_paths.open_app_db()?;
    if let Some(p) = req.default_provider {
        db.set_setting("tts:default_provider", &p)?;
    }
    if let Some(v) = req.default_voice {
        db.set_setting("tts:default_voice", &v)?;
    }
    Ok(())
}

/// Save user's preferred default TTS provider and voice.
#[tauri::command]
pub fn tts_set_config(state: State<'_, AppState>, req: TtsSetConfigRequest) -> CmdResult<()> {
    tts_set_config_impl(&state, req)
}

/// Native folder picker dialog. Returns the selected directory path, or None if cancelled.
pub async fn dialog_pick_folder_impl() -> CmdResult<Option<String>> {
    let handle = rfd::AsyncFileDialog::new()
        .set_title("Chọn thư mục Workspace")
        .pick_folder()
        .await;
    Ok(handle.map(|h| strip_verbatim(h.path())))
}

/// Native folder picker dialog command.
#[tauri::command]
pub async fn dialog_pick_folder() -> CmdResult<Option<String>> {
    dialog_pick_folder_impl().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_verbatim() {
        assert_eq!(
            strip_verbatim(Path::new(r"\\?\D:\Studio\workspace")),
            r"D:\Studio\workspace"
        );
        assert_eq!(
            strip_verbatim(Path::new(r"\\?\UNC\server\share\dir")),
            r"\\server\share\dir"
        );
        assert_eq!(
            strip_verbatim(Path::new(r"D:\Normal\Path")),
            r"D:\Normal\Path"
        );
    }

    #[test]
    fn test_validate_endpoint_url_valid() {
        assert_eq!(
            validate_endpoint_url("https://api.openai.com/v1").unwrap(),
            "https://api.openai.com/v1"
        );
        assert_eq!(
            validate_endpoint_url("https://gateway.example.com/v1/").unwrap(),
            "https://gateway.example.com/v1"
        );
        assert_eq!(
            validate_endpoint_url("http://127.0.0.1:8080/v1").unwrap(),
            "http://127.0.0.1:8080/v1"
        );
        assert_eq!(
            validate_endpoint_url("http://localhost:3000/v1").unwrap(),
            "http://localhost:3000/v1"
        );
    }

    #[test]
    fn test_validate_endpoint_url_invalid() {
        // http non-localhost
        assert!(validate_endpoint_url("http://remote.server/v1").is_err());
        // userinfo
        assert!(validate_endpoint_url("https://user:pass@api.openai.com/v1").is_err());
        // query
        assert!(validate_endpoint_url("https://api.openai.com/v1?test=1").is_err());
        // fragment
        assert!(validate_endpoint_url("https://api.openai.com/v1#heading").is_err());
        // invalid URL
        assert!(validate_endpoint_url("not a url").is_err());
        // empty
        assert!(validate_endpoint_url("").is_err());
    }

    #[test]
    fn test_normalize_workspace_path() {
        assert_eq!(
            normalize_workspace_path(Path::new(r"\\?\D:\Studio\workspace\")),
            r"D:\Studio\workspace"
        );
        assert_eq!(
            normalize_workspace_path(Path::new(r"\\?\D:\Studio\workspace")),
            r"D:\Studio\workspace"
        );
        assert_eq!(normalize_workspace_path(Path::new(r"\\?\D:\")), r"D:\");
        assert_eq!(
            normalize_workspace_path(Path::new(r"D:\Normal\Path/")),
            r"D:\Normal\Path"
        );
    }

    #[test]
    fn test_provider_pricing_validation() {
        let valid = studio_tools::ProviderPricing {
            image_usd: Some(0.05),
            video_usd_per_s: Some(0.15),
            models: std::collections::HashMap::new(),
        };
        assert!(valid.validate().is_ok());

        let invalid_negative = studio_tools::ProviderPricing {
            image_usd: Some(-0.01),
            video_usd_per_s: None,
            models: std::collections::HashMap::new(),
        };
        assert!(invalid_negative.validate().is_err());

        let invalid_nan = studio_tools::ProviderPricing {
            image_usd: Some(f64::NAN),
            video_usd_per_s: None,
            models: std::collections::HashMap::new(),
        };
        assert!(invalid_nan.validate().is_err());
    }

    #[tokio::test]
    async fn test_tts_model_preview_binary_and_voices() {
        let state = AppState::new();
        let bin_preview = tts_model_preview_impl(&state, "piper".into(), "binary".into())
            .await
            .expect("piper binary preview failed");
        assert_eq!(bin_preview.provider, "tts.piper");
        assert_eq!(bin_preview.voice, "binary");
        assert!(bin_preview.size_bytes > 20_000_000);
        assert_eq!(bin_preview.license, "MIT");

        let voice_preview =
            tts_model_preview_impl(&state, "tts.piper".into(), "vi_VN-vais1000-medium".into())
                .await
                .expect("voice preview failed");
        assert_eq!(voice_preview.voice, "vi_VN-vais1000-medium");
        assert!(voice_preview.size_bytes > 60_000_000);
        assert!(voice_preview
            .license_note
            .contains("cá nhân/phi thương mại"));

        let invalid =
            tts_model_preview_impl(&state, "piper".into(), "non_existent_voice".into()).await;
        assert!(invalid.is_err());
    }

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_scope_id_validation() {
        assert!(validate_scope_id("my-brand").is_ok());
        assert!(validate_scope_id("tiktok").is_ok());
        assert!(validate_scope_id("pb-1").is_ok());

        // Rejections
        assert!(validate_scope_id("").is_err());
        assert!(validate_scope_id("a").is_err()); // too short (<2)
        assert!(validate_scope_id("ThisHasCaps").is_err());
        assert!(validate_scope_id("../evil").is_err());
        assert!(validate_scope_id("path/traversal").is_err());
        assert!(validate_scope_id("name with spaces").is_err());
        assert!(validate_scope_id(" tiktok").is_err());
    }

    #[tokio::test]
    async fn test_scope_override_copies_binary_and_refuses_duplicate() {
        let _lock = ENV_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let app_dir = temp.path().join("app_data");
        std::env::set_var("STUDIO_APP_DIR", &app_dir);

        let app_brand_dir = app_dir.join("shared").join("brands").join("demo-brand");
        std::fs::create_dir_all(&app_brand_dir).unwrap();
        std::fs::write(
            app_brand_dir.join("brand.yaml"),
            r##"
name: "Demo Brand"
slogan: "Khẩu hiệu"
colors:
  primary: "#1E88E5"
  accent: "#FFB300"
  bg: "#0B0F14"
  text: "#FFFFFF"
fonts:
  heading: "Inter"
  body: "Inter"
logos:
  full: "logo.png"
  mark: "mark.svg"
version: "brand@1"
"##,
        )
        .unwrap();

        // Write a binary file
        let binary_bytes: Vec<u8> = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0xFF, 0xFE,
        ];
        std::fs::write(app_brand_dir.join("logo.png"), &binary_bytes).unwrap();
        std::fs::write(app_brand_dir.join("voice.md"), "# Voice").unwrap();

        let ws_dir = temp.path().join("my_workspace");
        let ws = Workspace::create(&ws_dir, "Test WS").unwrap();

        let state = AppState::new();
        *state.current.lock().unwrap() = Some(ws_dir.clone());

        // Initial checkpoints count
        let cp_before = CheckpointService::list(&ws).unwrap().len();

        // Perform override
        let res = scope_override_impl(&state, "brands".into(), "demo-brand".into())
            .expect("scope_override should succeed");
        assert_eq!(res["success"], true);

        // Verify workspace files
        let ws_brand = ws_dir.join("brands").join("demo-brand");
        assert!(ws_brand.join("brand.yaml").is_file());
        assert!(ws_brand.join("voice.md").is_file());
        let copied_bytes = std::fs::read(ws_brand.join("logo.png")).unwrap();
        assert_eq!(copied_bytes, binary_bytes);

        // Verify checkpoint was created
        let cp_after = CheckpointService::list(&ws).unwrap().len();
        assert!(cp_after > cp_before);

        // Refuse duplicate override
        let dup = scope_override_impl(&state, "brands".into(), "demo-brand".into());
        assert!(dup.is_err());
        assert!(dup.unwrap_err().to_string().contains("đã tồn tại"));

        // Reject path traversal / invalid ID
        let invalid_id = scope_override_impl(&state, "brands".into(), "../escape".into());
        assert!(invalid_id.is_err());
    }

    #[tokio::test]
    async fn test_scope_create_and_checkpoints() {
        let _lock = ENV_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let app_dir = temp.path().join("app_data");
        std::env::set_var("STUDIO_APP_DIR", &app_dir);

        let ws_dir = temp.path().join("my_workspace");
        let ws = Workspace::create(&ws_dir, "Test WS").unwrap();

        let state = AppState::new();
        *state.current.lock().unwrap() = Some(ws_dir.clone());

        // Create new channel
        let res_channel =
            scope_create_impl(&state, "channels".into(), "youtube-shorts".into(), None)
                .expect("create channel");
        assert_eq!(res_channel["success"], true);
        assert!(ws_dir
            .join("channels")
            .join("youtube-shorts.yaml")
            .is_file());

        // Create new playbook
        let res_pb = scope_create_impl(&state, "playbooks".into(), "custom-flow".into(), None)
            .expect("create playbook");
        assert_eq!(res_pb["success"], true);
        assert!(ws_dir.join("playbooks").join("custom-flow.md").is_file());

        // Verify checkpoints were created
        let cp = CheckpointService::list(&ws).unwrap();
        assert!(!cp.is_empty());

        // Duplicate create fails
        let dup = scope_create_impl(&state, "channels".into(), "youtube-shorts".into(), None);
        assert!(dup.is_err());
    }

    #[tokio::test]
    async fn test_scope_remove_override_never_touches_app_org() {
        let _lock = ENV_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let app_dir = temp.path().join("app_data");
        std::env::set_var("STUDIO_APP_DIR", &app_dir);

        let app_channel = app_dir
            .join("shared")
            .join("channels")
            .join("app-chan.yaml");
        std::fs::create_dir_all(app_channel.parent().unwrap()).unwrap();
        std::fs::write(&app_channel, "id: app-chan\n").unwrap();

        let ws_dir = temp.path().join("my_workspace");
        let _ws = Workspace::create(&ws_dir, "Test WS").unwrap();

        let state = AppState::new();
        *state.current.lock().unwrap() = Some(ws_dir.clone());

        // Attempting to remove something that only exists in App layer fails
        let err = scope_remove_override_impl(&state, "channels".into(), "app-chan".into());
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("Không tìm thấy"));
        // App file is completely untouched
        assert!(app_channel.is_file());

        // Now create a workspace override for app-chan
        let ws_channel = ws_dir.join("channels").join("app-chan.yaml");
        std::fs::write(&ws_channel, "id: app-chan\ntitle: Workspace override\n").unwrap();

        // Remove workspace override
        let res = scope_remove_override_impl(&state, "channels".into(), "app-chan".into())
            .expect("remove workspace override");
        assert_eq!(res["success"], true);
        assert!(!ws_channel.exists());
        // App file STILL completely untouched!
        assert!(app_channel.is_file());
    }

    #[tokio::test]
    async fn test_file_write_rejects_absolute_app_path() {
        let _lock = ENV_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let app_dir = temp.path().join("app_data");
        std::env::set_var("STUDIO_APP_DIR", &app_dir);

        let ws_dir = temp.path().join("my_workspace");
        let _ws = Workspace::create(&ws_dir, "Test WS").unwrap();

        let state = AppState::new();
        *state.current.lock().unwrap() = Some(ws_dir.clone());

        // Attempting file_write directly to App layer path fails
        let res = file_write_impl(
            &state,
            None,
            app_dir
                .join("shared")
                .join("brands")
                .to_string_lossy()
                .to_string(),
            "hack".into(),
        );
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_scope_list_independent_loading_and_errors() {
        let _lock = ENV_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let app_dir = temp.path().join("app_data");
        std::env::set_var("STUDIO_APP_DIR", &app_dir);

        let ws_dir = temp.path().join("my_workspace");
        let _ws = Workspace::create(&ws_dir, "Test WS").unwrap();

        let state = AppState::new();
        *state.current.lock().unwrap() = Some(ws_dir.clone());

        // 1. Valid channel
        let ch_valid = ws_dir.join("channels").join("ch-valid.yaml");
        std::fs::write(
            &ch_valid,
            r#"
id: "ch-valid"
platform: "tiktok"
title: "Kênh Hợp Lệ"
formats:
  - name: "reel"
    width: 1080
    height: 1920
    aspect: "9:16"
"#,
        )
        .unwrap();

        // 2. Broken channel (invalid yaml syntax)
        let ch_bad = ws_dir.join("channels").join("ch-bad.yaml");
        std::fs::write(
            &ch_bad,
            r#"
id: "ch-bad"
platform: "tiktok"
title: "Kênh Hỏng"
formats: [unclosed list
"#,
        )
        .unwrap();

        let list = scope_list_impl(&state, "channels".into()).expect("scope_list should succeed");
        assert_eq!(list.len(), 2);

        let valid_item = list.iter().find(|i| i["id"] == "ch-valid").unwrap();
        assert_eq!(valid_item["name"], "Kênh Hợp Lệ");
        assert!(valid_item.get("error").is_none());
        assert!(valid_item.get("formats").is_some());

        let bad_item = list.iter().find(|i| i["id"] == "ch-bad").unwrap();
        assert_eq!(bad_item["name"], "ch-bad");
        assert!(bad_item.get("error").is_some());
        let err_str = bad_item["error"].as_str().unwrap();
        assert!(err_str.contains("Dòng") || err_str.contains("Lỗi"));
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn test_web_dispatch_scope_and_sudo() {
        let _lock = ENV_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let app_dir = temp.path().join("app_data");
        std::env::set_var("STUDIO_APP_DIR", &app_dir);

        let ws_dir = temp.path().join("my_workspace");
        let _ws = Workspace::create(&ws_dir, "Test WS").unwrap();

        let state = AppState::new();
        *state.current.lock().unwrap() = Some(ws_dir.clone());

        // Create via web_dispatch without sudo
        let create_args = serde_json::json!({
            "kind": "channels",
            "id": "web-chan"
        });
        crate::web_dispatch::dispatch_invoke(&state, "scope_create", create_args, false)
            .await
            .expect("scope_create without sudo should work");

        // Validate via web_dispatch
        let val_args = serde_json::json!({
            "kind": "channels",
            "text": "invalid: yaml: ["
        });
        let val_res =
            crate::web_dispatch::dispatch_invoke(&state, "scope_validate", val_args, false)
                .await
                .expect("scope_validate should work");
        assert_eq!(val_res["valid"], false);

        // Remove override WITHOUT sudo -> FAILS
        let rem_args = serde_json::json!({
            "kind": "channels",
            "id": "web-chan"
        });
        let sudo_err = crate::web_dispatch::dispatch_invoke(
            &state,
            "scope_remove_override",
            rem_args.clone(),
            false,
        )
        .await;
        assert!(sudo_err.is_err());
        assert!(sudo_err.unwrap_err().to_string().contains("sudo"));

        // Remove override WITH sudo -> SUCCEEDS
        let sudo_ok =
            crate::web_dispatch::dispatch_invoke(&state, "scope_remove_override", rem_args, true)
                .await;
        assert!(sudo_ok.is_ok());
    }

    #[tokio::test]
    async fn test_artifact_channel_matrix_and_cache() {
        let _lock = ENV_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let ws_dir = temp.path().join("my_ws");
        let ws = Workspace::create(&ws_dir, "Matrix WS").unwrap();

        // 1. Setup channels: youtube (1920x1080) and tiktok (1080x1920)
        let ch_dir = ws_dir.join("channels");
        std::fs::create_dir_all(&ch_dir).unwrap();
        std::fs::write(
            ch_dir.join("youtube.yaml"),
            r#"
id: youtube
platform: youtube
title: "YouTube Video"
formats:
  - name: landscape
    width: 1920
    height: 1080
    aspect: "16:9"
notes: "Kênh YouTube chuẩn 16:9"
"#,
        )
        .unwrap();

        std::fs::write(
            ch_dir.join("tiktok.yaml"),
            r#"
id: tiktok
platform: tiktok
title: "TikTok Video"
formats:
  - name: vertical
    width: 1080
    height: 1920
    aspect: "9:16"
notes: "Kênh TikTok dọc"
"#,
        )
        .unwrap();

        // 2. Setup project and 2 artifacts
        let proj = Project::create(&ws, "demo-proj", "Demo Project").unwrap();

        // Artifact 1: thumbnail (rendered as 1280x720 png)
        let art1_dir = proj.artifacts_dir().join("thumbnail");
        std::fs::create_dir_all(&art1_dir).unwrap();
        std::fs::write(
            art1_dir.join("artifact.yaml"),
            r#"
id: thumbnail
title: "Ảnh bìa video"
kind: design
channels: [youtube, tiktok]
brand: example
source: index.html
"#,
        )
        .unwrap();
        std::fs::write(
            art1_dir.join("index.html"),
            "<html><body>Thumb</body></html>",
        )
        .unwrap();

        // Artifact 2: poster (unrendered)
        let art2_dir = proj.artifacts_dir().join("poster");
        std::fs::create_dir_all(&art2_dir).unwrap();
        std::fs::write(
            art2_dir.join("artifact.yaml"),
            r#"
id: poster
title: "Poster dự án"
kind: design
channels: [youtube, tiktok]
brand: example
source: index.html
"#,
        )
        .unwrap();
        std::fs::write(
            art2_dir.join("index.html"),
            "<html><body>Poster</body></html>",
        )
        .unwrap();

        // Create a 1280x720 PNG file for thumbnail render
        let renders_dir = proj.renders_dir().join("thumbnail");
        std::fs::create_dir_all(&renders_dir).unwrap();
        let render_img_path = renders_dir.join("render.png");

        let img = image::RgbaImage::new(1280, 720);
        img.save(&render_img_path).unwrap();

        // Record in WorkspaceDb renders table
        let db = WorkspaceDb::open(&ws).unwrap();
        let rel_render_path = "projects/demo-proj/renders/thumbnail/render.png".to_string();
        db.conn()
            .execute(
                "INSERT INTO renders (id, artifact_id, hash, output_path, format, size_bytes, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    "rend-1",
                    "thumbnail",
                    "hash-1280x720",
                    rel_render_path,
                    "png",
                    1000,
                    chrono::Utc::now().to_rfc3339()
                ],
            )
            .unwrap();

        let state = AppState::new();
        *state.current.lock().unwrap() = Some(ws_dir.clone());

        // 3. First call: computes matrix and caches results
        let rows = artifact_channel_matrix_impl(&state, "demo-proj".to_string()).unwrap();
        assert_eq!(rows.len(), 2, "Must return 2 artifact rows");

        let thumb_row = rows.iter().find(|r| r.artifact_id == "thumbnail").unwrap();
        assert_eq!(thumb_row.channels.len(), 2, "Must have 2 channel cells");

        // YouTube check on 1280x720 vs 1920x1080 -> error (DIMENSIONS_MISMATCH)
        let yt_cell = thumb_row
            .channels
            .iter()
            .find(|c| c.channel_id == "youtube")
            .unwrap();
        assert_eq!(yt_cell.status, "error");
        assert!(yt_cell.errors_count > 0);
        assert!(
            yt_cell
                .findings
                .iter()
                .any(|f| f["code"] == "DIMENSIONS_MISMATCH" || f["code"] == "BRAND_NOT_FOUND"),
            "Expected dimension mismatch or brand not found finding"
        );

        // Poster is unrendered
        let poster_row = rows.iter().find(|r| r.artifact_id == "poster").unwrap();
        let poster_yt = poster_row
            .channels
            .iter()
            .find(|c| c.channel_id == "youtube")
            .unwrap();
        assert_eq!(poster_yt.status, "unrendered");
        assert_eq!(poster_yt.render_hash, None);

        // 4. Verify SQLite cache was populated
        let count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM artifact_checks WHERE artifact_id = 'thumbnail'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 2, "2 channels checked and cached for thumbnail");

        // 5. Update cached status in DB to a sentinel value to prove second call hits the cache
        db.conn()
            .execute(
                "UPDATE artifact_checks SET status = 'cached_sentinel' WHERE artifact_id = 'thumbnail' AND channel_id = 'youtube'",
                [],
            )
            .unwrap();

        let rows2 = artifact_channel_matrix_impl(&state, "demo-proj".to_string()).unwrap();
        let thumb_row2 = rows2.iter().find(|r| r.artifact_id == "thumbnail").unwrap();
        let yt_cell2 = thumb_row2
            .channels
            .iter()
            .find(|c| c.channel_id == "youtube")
            .unwrap();
        assert_eq!(
            yt_cell2.status, "cached_sentinel",
            "Second call must hit SQLite cache without running checker"
        );
    }
}
