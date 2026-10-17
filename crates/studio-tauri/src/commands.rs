//! IPC commands implementing `packages/desktop-ui/IPC.md`.

use std::path::{Path, PathBuf};

use rusqlite::params;
use serde::Deserialize;
use serde_json::{json, Value};
use studio_engine::Engine;
use studio_workspace::{CheckpointService, Project, ScopeChain, Workspace, WorkspaceDb};
use tauri::{AppHandle, Emitter, Manager, State};

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
fn activate(app: &AppHandle, state: &AppState, ws: &Workspace) -> CmdResult<()> {
    let norm = normalize_workspace_path(ws.root());
    if let Ok(db) = state.app_paths.open_app_db() {
        if let Ok(conn) = rusqlite::Connection::open(state.app_paths.app_db_path()) {
            if cfg!(windows) {
                let _ = conn.execute(
                    "DELETE FROM recent_workspaces WHERE lower(path) = lower(?1) OR lower(path) = lower(?2)",
                    params![norm, format!(r"\\?\{norm}")],
                );
            } else {
                let _ = conn.execute(
                    "DELETE FROM recent_workspaces WHERE path = ?1",
                    params![norm],
                );
            }
        }
        let _ = db.record_workspace_open(&norm);
    }
    let _ = app.asset_protocol_scope().allow_directory(ws.root(), true);
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
    let handle = app.clone();
    let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(ev) = res {
            let paths: Vec<String> = ev
                .paths
                .iter()
                .filter(|p| !p.components().any(|c| c.as_os_str() == ".studio"))
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            if !paths.is_empty() {
                let _ = handle.emit("fs-event", json!({ "paths": paths }));
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

#[tauri::command]
pub fn app_recent_workspaces(state: State<'_, AppState>) -> CmdResult<Vec<Value>> {
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
pub fn workspace_create(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    name: String,
) -> CmdResult<Value> {
    let ws = Workspace::create(&path, name)?;
    activate(&app, &state, &ws)?;
    Ok(ws_summary(&ws, true))
}

#[tauri::command]
pub fn workspace_open(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> CmdResult<Value> {
    let ws = Workspace::open(&path)?;
    activate(&app, &state, &ws)?;
    Ok(ws_summary(&ws, true))
}

#[tauri::command]
pub fn workspace_current(state: State<'_, AppState>) -> CmdResult<Value> {
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

#[tauri::command]
pub fn project_list(state: State<'_, AppState>) -> CmdResult<Vec<Value>> {
    let ws = state.workspace()?;
    Ok(ws.list_projects()?.iter().map(project_summary).collect())
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

#[tauri::command]
pub fn project_create(
    state: State<'_, AppState>,
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
            mods = kit_modules(&state, k);
        }
    }
    project.set_modules(kit, &mods)?;
    Ok(project_summary(&project))
}

#[tauri::command]
pub fn project_files(state: State<'_, AppState>, project: String) -> CmdResult<Vec<Value>> {
    let ws = state.workspace()?;
    let p = Project::open(&ws, &project)?;
    let mut out = Vec::new();
    walk(p.root(), p.root(), &mut out);
    Ok(out)
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
fn resolve_path(ws: &Workspace, project: Option<&str>, path: &str) -> CmdResult<PathBuf> {
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

#[tauri::command]
pub fn file_read(
    state: State<'_, AppState>,
    project: Option<String>,
    path: String,
) -> CmdResult<Value> {
    let ws = state.workspace()?;
    let full = resolve_path(&ws, project.as_deref(), &path)?;
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

#[tauri::command]
pub fn thread_list(state: State<'_, AppState>, project: String) -> CmdResult<Vec<Value>> {
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
pub fn thread_create(
    state: State<'_, AppState>,
    project: String,
    engine: String,
    model: Option<String>,
) -> CmdResult<Value> {
    if engine != "claude" && engine != "agy" {
        return Err(CmdError::msg("Engine không hợp lệ (claude hoặc agy)."));
    }
    let ws = state.workspace()?;
    Project::open(&ws, &project)?;
    let db = WorkspaceDb::open(&ws)?;
    let id = uuid::Uuid::new_v4().to_string();
    db.conn().execute(
        "INSERT INTO threads (id, project_slug, title, engine, model, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![id, project, "Hội thoại mới", engine, model, now()],
    )?;
    thread_json(&db, &id)
}

#[tauri::command]
pub fn thread_events(
    state: State<'_, AppState>,
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
pub async fn turn_start(
    app: AppHandle,
    state: State<'_, AppState>,
    thread_id: String,
    prompt: String,
) -> CmdResult<Value> {
    let turn_id = turns::start_turn(app.clone(), &state, thread_id, prompt).await?;
    Ok(json!({ "turnId": turn_id }))
}

#[tauri::command]
pub fn turn_cancel(state: State<'_, AppState>, turn_id: String) -> CmdResult<Value> {
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
pub fn turn_undo(state: State<'_, AppState>, turn_id: String) -> CmdResult<Value> {
    turns::undo_turn(&state, &turn_id)?;
    Ok(json!({ "success": true, "restoredTurnId": turn_id }))
}

#[tauri::command]
pub fn permission_respond(
    state: State<'_, AppState>,
    request_id: String,
    allow: bool,
) -> CmdResult<Value> {
    Ok(json!({ "acknowledged": state.broker.respond(&request_id, allow) }))
}

// ───────────────────────── Checkpoints ─────────────────────────

#[tauri::command]
pub fn checkpoint_list(state: State<'_, AppState>) -> CmdResult<Vec<Value>> {
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
pub fn checkpoint_restore(state: State<'_, AppState>, id: String) -> CmdResult<Value> {
    let ws = state.workspace()?;
    CheckpointService::restore(&ws, &id)?;
    Ok(json!({ "success": true, "restoredCheckpointId": id }))
}

// ───────────────────────── Engines & catalog ─────────────────────────

#[tauri::command]
pub async fn engines_probe(state: State<'_, AppState>) -> CmdResult<Vec<Value>> {
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

#[tauri::command]
pub fn catalog_kits(state: State<'_, AppState>) -> CmdResult<Vec<Value>> {
    let cat = catalog(&state)?;
    Ok(cat.kits.keys().filter_map(|k| kit_json(&cat, k)).collect())
}

#[tauri::command]
pub fn catalog_modules(state: State<'_, AppState>) -> CmdResult<Vec<Value>> {
    let cat = catalog(&state)?;
    Ok(cat
        .modules
        .keys()
        .filter_map(|m| module_json(&cat, m))
        .collect())
}

#[tauri::command]
pub fn catalog_suggest(state: State<'_, AppState>, text: String) -> CmdResult<Value> {
    let cat = catalog(&state)?;
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
pub fn project_modules_set(
    state: State<'_, AppState>,
    project: String,
    modules: Vec<String>,
) -> CmdResult<Value> {
    let ws = state.workspace()?;
    let p = Project::open(&ws, &project)?;
    let kit = p.meta()?.kit;
    let meta = p.set_modules(kit.clone(), &modules)?;
    let cat = catalog(&state)?;
    let extra: Vec<&str> = meta.modules.iter().map(String::as_str).collect();
    let budget = studio_catalog::resolve(&cat, kit.as_deref(), &extra)
        .map(|a| a.desc_tokens)
        .unwrap_or(0);
    Ok(json!({ "project": project, "modules": meta.modules, "totalTokenBudget": budget }))
}

// ───────────────────────── Scope & assets ─────────────────────────

#[derive(Deserialize)]
pub struct AssetFilter {
    kind: Option<String>,
    search: Option<String>,
    limit: Option<usize>,
}

#[tauri::command]
pub fn scope_list(state: State<'_, AppState>, kind: String) -> CmdResult<Vec<Value>> {
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
    let set = studio_brand::BrandSet::load(&ws).ok();
    Ok(ScopeChain::new(&ws)
        .resolve(&kind)
        .into_iter()
        .map(|item| {
            let layer = format!("{:?}", item.layer).to_lowercase();
            let (name, summary) = match (kind.as_str(), &set) {
                ("brands", Some(s)) => s.get_brand(&item.id).map(|b| (b.spec.name.clone(), Some(b.spec.slogan.clone()))).unwrap_or((item.id.clone(), None)),
                ("channels", Some(s)) => s.get_channel(&item.id).map(|c| (c.title.clone(), Some(c.notes.clone()))).unwrap_or((item.id.clone(), None)),
                ("playbooks", Some(s)) => s.get_playbook(&item.id).map(|p| (p.frontmatter.name.clone(), Some(p.frontmatter.description.clone()))).unwrap_or((item.id.clone(), None)),
                _ => (item.id.clone(), None),
            };
            json!({ "id": item.id, "name": name, "kind": kind, "layer": layer, "path": strip_verbatim(&item.path), "summary": summary })
        })
        .collect())
}

#[tauri::command]
pub fn asset_list(
    state: State<'_, AppState>,
    filter: Option<AssetFilter>,
) -> CmdResult<Vec<Value>> {
    let ws = state.workspace()?;
    let mut files = Vec::new();
    collect(&ws.root().join("library"), &mut files);
    let search = filter
        .as_ref()
        .and_then(|f| f.search.clone())
        .map(|s| s.to_lowercase());
    let want = filter.as_ref().and_then(|f| f.kind.clone());
    let limit = filter.as_ref().and_then(|f| f.limit).unwrap_or(500);
    Ok(files
        .into_iter()
        .filter(|p| !p.to_string_lossy().ends_with(".meta.yaml"))
        .filter_map(|p| {
            let (k, mime) = kind_of(&p);
            let kind = match k {
                "image" | "video" | "audio" => k,
                _ if p.extension().is_some_and(|e| e == "ttf" || e == "otf" || e == "woff2") => "font",
                "markdown" | "text" | "html" | "json" | "code" => "document",
                _ if mime == "application/pdf" => "document",
                _ => "other",
            };
            if want.as_deref().is_some_and(|w| w != kind) {
                return None;
            }
            let r = rel(&ws, &p);
            if search.as_ref().is_some_and(|s| !r.to_lowercase().contains(s)) {
                return None;
            }
            let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            Some(json!({
                "id": r, "name": p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                "kind": kind, "mime": mime, "size": size, "path": r,
                "previewUrl": matches!(kind, "image" | "video" | "audio").then(|| asset_url(&p)),
                "createdAt": modified(&p).unwrap_or_default(),
            }))
        })
        .take(limit)
        .collect())
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect(&p, out)
        } else {
            out.push(p)
        }
    }
}

// ───────────────────────── Jobs & disk ─────────────────────────

#[tauri::command]
pub async fn jobs_snapshot(state: State<'_, AppState>) -> CmdResult<Vec<Value>> {
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
pub fn disk_usage(state: State<'_, AppState>) -> CmdResult<Value> {
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
#[tauri::command]
pub fn file_write(
    state: State<'_, AppState>,
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

/// Manual checkpoint (e.g. before a series of direct edits).
#[tauri::command]
pub fn checkpoint_create(state: State<'_, AppState>, label: Option<String>) -> CmdResult<Value> {
    let ws = state.workspace()?;
    let id = CheckpointService::create(&ws, label.as_deref().unwrap_or("Lưu thủ công"), None)?;
    Ok(json!({ "id": id }))
}

#[tauri::command]
pub fn artifact_list(state: State<'_, AppState>, project: String) -> CmdResult<Vec<Value>> {
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
pub async fn artifact_render(
    state: State<'_, AppState>,
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
pub fn artifact_check(
    state: State<'_, AppState>,
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

/// Secret status entry returned to the UI.
#[derive(serde::Serialize)]
pub struct SecretStatusItem {
    pub name: String,
    pub present: bool,
}

/// Retrieve status of all secrets referenced by registered tool manifests.
#[tauri::command]
pub async fn secret_status(state: State<'_, AppState>) -> CmdResult<Vec<SecretStatusItem>> {
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

/// Set a secret in Windows Credential Manager.
#[tauri::command]
pub async fn secret_set(state: State<'_, AppState>, name: String, value: String) -> CmdResult<()> {
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

/// Delete a secret from Windows Credential Manager.
#[tauri::command]
pub async fn secret_delete(state: State<'_, AppState>, name: String) -> CmdResult<()> {
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
#[tauri::command]
pub async fn provider_endpoint_get(
    state: State<'_, AppState>,
) -> CmdResult<std::collections::HashMap<String, String>> {
    let endpoints = state.endpoints.read().await.clone();
    Ok(endpoints)
}

/// Set or remove a custom provider endpoint in `app.db`.
#[tauri::command]
pub async fn provider_endpoint_set(
    state: State<'_, AppState>,
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

/// Get custom provider pricing from `app.db`.
#[tauri::command]
pub async fn provider_pricing_get(
    state: State<'_, AppState>,
) -> CmdResult<std::collections::HashMap<String, studio_tools::ProviderPricing>> {
    let pricing = state.pricing.read().await.clone();
    Ok(pricing)
}

/// Set or remove custom provider pricing in `app.db`.
#[tauri::command]
pub async fn provider_pricing_set(
    state: State<'_, AppState>,
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
#[tauri::command]
pub async fn env_probe(state: State<'_, AppState>) -> CmdResult<Vec<EnvProbeItem>> {
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

    Ok(items)
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
#[tauri::command]
pub async fn tool_install_preview(
    state: State<'_, AppState>,
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

/// Confirms and finalizes an installation preview.
#[tauri::command]
pub async fn tool_install_confirm(
    state: State<'_, AppState>,
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

/// Lists all tools (builtin and installed) with their probe results.
#[tauri::command]
pub async fn tool_list_installed(
    state: State<'_, AppState>,
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

/// Uninstalls an installed tool.
#[tauri::command]
pub async fn tool_uninstall(
    state: State<'_, AppState>,
    req: ToolUninstallRequest,
) -> CmdResult<bool> {
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
}
