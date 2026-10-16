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

fn rel(ws: &Workspace, p: &Path) -> String {
    p.strip_prefix(ws.root())
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

fn modified(p: &Path) -> Option<String> {
    let t = std::fs::metadata(p).ok()?.modified().ok()?;
    Some(chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
}

// ───────────────────────── Workspace ─────────────────────────

fn ws_summary(ws: &Workspace, current: bool) -> Value {
    json!({ "path": ws.root(), "name": ws.name(), "lastOpenedAt": now(), "isCurrent": current })
}

/// Make `path` the open workspace: record it, allow asset access, watch files.
fn activate(app: &AppHandle, state: &AppState, ws: &Workspace) -> CmdResult<()> {
    if let Ok(db) = state.app_paths.open_app_db() {
        let _ = db.record_workspace_open(&ws.root().to_string_lossy());
    }
    let _ = app.asset_protocol_scope().allow_directory(ws.root(), true);
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
    let db = state.app_paths.open_app_db()?;
    Ok(db
        .list_recent_workspaces()?
        .into_iter()
        .filter_map(|r| {
            let path = PathBuf::from(&r.path);
            let ws = Workspace::open(&path).ok()?;
            Some(json!({
                "path": r.path, "name": ws.name(), "lastOpenedAt": r.last_opened,
                "isCurrent": current.as_deref() == Some(path.as_path()),
            }))
        })
        .collect())
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
        "org": if org.is_implicit() { Value::Null } else { json!({ "id": org.name(), "name": org.name(), "path": org.root() }) },
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
    for e in entries {
        let p = e.path();
        let is_dir = p.is_dir();
        let (kind, _) = kind_of(&p);
        out.push(json!({
            "path": p.strip_prefix(root).unwrap_or(&p).to_string_lossy().replace('\\', "/"),
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
            json!({ "id": item.id, "name": name, "kind": kind, "layer": layer, "path": item.path, "summary": summary })
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
    let p = Project::open(&ws, &project)?;
    let artifact =
        studio_artifacts::Artifact::load(&p, &id).map_err(|e| CmdError::msg(e.to_string()))?;
    let outcome = studio_artifacts::render(&ws, &p, &artifact, &state.scheduler)
        .await
        .map_err(|e| CmdError::msg(format!("Render lỗi: {e}")))?;
    Ok(json!({
        "cached": outcome.cached,
        "outputs": outcome.outputs.iter().map(|o| rel(&ws, o)).collect::<Vec<_>>(),
        "jobId": outcome.job_id,
        "probes": outcome.probe_report.iter().map(|r| json!({ "name": r.name, "available": r.available, "note": r.note })).collect::<Vec<_>>(),
    }))
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
    let renders: Vec<String> = std::fs::read_dir(p.renders_dir().join(&a.id))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|d| std::fs::read_dir(d.path()).ok())
        .flatten()
        .flatten()
        .map(|f| rel(&ws, &f.path()))
        .collect();
    let args = json!({
        "artifact_dir": rel(&ws, &a.dir(&p)),
        "brand": a.brand.clone().unwrap_or_else(|| "example".into()),
        "channels": channels.unwrap_or_else(|| a.channels.clone()),
        "renders": renders,
    });
    studio_mcp::tools::check_run(&ws, &args).map_err(CmdError::msg)
}
