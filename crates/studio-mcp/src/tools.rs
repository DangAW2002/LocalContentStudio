//! Core Studio MCP tools (plan 0009 §5, ADR-009 §2). Every call is scoped to the token's workspace.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use studio_brand::{run_checks, BrandSet, CheckSubject};
use studio_tools::{route, run_tool, Registry, ToolRunContext};
use studio_workspace::{CheckpointService, Project, Workspace};

use crate::server::ServerState;
use crate::tokens::TurnContext;

/// Results larger than this are written to a file and returned as a path.
pub const MAX_INLINE: usize = 32 * 1024;

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": { "type": "object", "properties": properties, "required": required }
    })
}

/// MCP `tools/list` payload.
pub fn definitions() -> Vec<Value> {
    let s = |d: &str| json!({ "type": "string", "description": d });
    vec![
        tool("workspace_map", "Bản đồ thư mục workspace hiện tại (độ sâu 3, bỏ qua .studio).", json!({}), &[]),
        tool("artifact_list", "Liệt kê artifact (id, kind, title) của một project hoặc mọi project.", json!({ "project": s("slug project, bỏ trống = tất cả") }), &[]),
        tool("artifact_get", "Đọc một tệp trong workspace theo đường dẫn tương đối.", json!({ "path": s("đường dẫn tương đối trong workspace") }), &["path"]),
        tool("brand_get", "Lấy Brand (màu, font, logo, giọng văn, quy tắc). Bỏ trống slug để liệt kê.", json!({ "slug": s("slug brand") }), &[]),
        tool("channel_get", "Lấy spec kênh (kích thước, độ dài, safe zone, giới hạn chữ). Bỏ trống id để liệt kê.", json!({ "id": s("id kênh, vd youtube-shorts") }), &[]),
        tool("playbook_get", "Lấy playbook (quy trình mẫu). Bỏ trống id để liệt kê.", json!({ "id": s("id playbook") }), &[]),
        tool("asset_list", "Liệt kê tài nguyên trong library/.", json!({ "contains": s("lọc theo chuỗi trong đường dẫn") }), &[]),
        tool("checkpoint_list", "Liệt kê checkpoint của workspace.", json!({}), &[]),
        tool("checkpoint_restore", "Khôi phục workspace về một checkpoint.", json!({ "id": s("id checkpoint") }), &["id"]),
        tool("job_status", "Trạng thái các job đang chờ/chạy/xong.", json!({}), &[]),
        tool("catalog_enable", "Bật thêm một module catalog (skill) cho project hiện tại.", json!({ "module": s("id module, vd render.hyperframes") }), &["module"]),
        tool("capability_search", "Tìm capability theo từ khóa (vd tts, render, ảnh).", json!({ "query": s("từ khóa") }), &["query"]),
        tool("capability_describe", "Mô tả một capability: schema vào/ra và các provider.", json!({ "id": s("vd tts.synthesize@1") }), &["id"]),
        tool("capability_run", "Chạy một capability qua provider đã chọn (hoặc mặc định, có dự phòng).", json!({ "capability": s("vd tts.synthesize@1"), "provider": s("id tool, tùy chọn"), "input": { "type": "object" } }), &["capability", "input"]),
        tool("check_run", "Kiểm artifact theo Brand và kênh (palette, tương phản, logo, safe zone, kích thước, thời lượng, giới hạn chữ, phụ đề, từ cấm).", json!({ "artifact_dir": s("thư mục artifact tương đối"), "brand": s("slug brand"), "channels": { "type": "array", "items": { "type": "string" } }, "renders": { "type": "array", "items": { "type": "string" } } }), &["artifact_dir", "brand", "channels"]),
        tool("approve", "Nội bộ: xử lý yêu cầu cấp quyền của engine.", json!({ "tool_name": s("tên tool"), "input": { "type": "object" } }), &["tool_name", "input"]),
    ]
}

fn arg<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
}

fn required<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    arg(args, key).ok_or_else(|| format!("Thiếu tham số \"{key}\"."))
}

fn rel(ws: &Workspace, p: &Path) -> String {
    p.strip_prefix(ws.root())
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Write a large result to the cache and return a pointer value instead.
fn inline_or_file(ws: &Workspace, value: Value) -> Value {
    let text = value.to_string();
    if text.len() <= MAX_INLINE {
        return value;
    }
    let dir = ws.cache_dir().join("tool-out");
    let path = dir.join(format!("{}.json", uuid::Uuid::new_v4().simple()));
    match std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, &text)) {
        Ok(()) => {
            json!({ "result_path": rel(ws, &path), "size_bytes": text.len(), "note": "Kết quả vượt quá 32 KB nên đã được ghi vào tệp" })
        }
        Err(e) => json!({ "error": format!("Không ghi được kết quả lớn: {e}") }),
    }
}

/// Dispatch a `tools/call`. Returns the JSON result or a Vietnamese error message.
pub async fn call(
    state: &ServerState,
    ctx: &TurnContext,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    if name == "approve" {
        let tool_name = required(args, "tool_name")?;
        let input = args.get("input").cloned().unwrap_or(Value::Null);
        let decision = state.broker.decide(ctx, tool_name, &input).await;
        return Ok(decision.to_claude_json());
    }

    let ws =
        Workspace::open(&ctx.workspace_dir).map_err(|e| format!("Không mở được workspace: {e}"))?;
    let out = match name {
        "workspace_map" => json!({ "root": ws.name(), "entries": walk(&ws, ws.root(), 0) }),
        "artifact_list" => artifact_list(&ws, arg(args, "project"))?,
        "artifact_get" => {
            let p = ws
                .scoped_path(required(args, "path")?)
                .map_err(|e| e.to_string())?;
            let bytes = std::fs::read(&p).map_err(|e| format!("Không đọc được tệp: {e}"))?;
            match String::from_utf8(bytes) {
                Ok(text) if text.len() <= MAX_INLINE => {
                    json!({ "path": rel(&ws, &p), "text": text })
                }
                Ok(text) => {
                    json!({ "path": rel(&ws, &p), "size_bytes": text.len(), "note": "Tệp lớn hơn 32 KB, hãy đọc trực tiếp bằng công cụ đọc tệp." })
                }
                Err(e) => {
                    json!({ "path": rel(&ws, &p), "size_bytes": e.as_bytes().len(), "binary": true })
                }
            }
        }
        "brand_get" | "channel_get" | "playbook_get" => {
            let set = BrandSet::load(&ws).map_err(|e| format!("Không nạp được Brand: {e}"))?;
            scope_get(&set, name, args)?
        }
        "asset_list" => {
            let filter = arg(args, "contains").map(str::to_lowercase);
            let mut items = Vec::new();
            collect_files(&ws.root().join("library"), &mut items);
            let items: Vec<Value> = items
                .into_iter()
                .filter(|p| !p.to_string_lossy().ends_with(".meta.yaml"))
                .map(|p| {
                    (
                        rel(&ws, &p),
                        std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0),
                    )
                })
                .filter(|(r, _)| filter.as_ref().is_none_or(|f| r.to_lowercase().contains(f)))
                .map(|(r, size)| json!({ "path": r, "size_bytes": size }))
                .collect();
            json!({ "assets": items })
        }
        "checkpoint_list" => {
            let list = CheckpointService::list(&ws).map_err(|e| e.to_string())?;
            serde_json::to_value(list).map_err(|e| e.to_string())?
        }
        "checkpoint_restore" => {
            let id = required(args, "id")?;
            CheckpointService::restore(&ws, id).map_err(|e| e.to_string())?;
            json!({ "restored": id })
        }
        "job_status" => {
            serde_json::to_value(state.scheduler.snapshot().await).map_err(|e| e.to_string())?
        }
        "catalog_enable" => catalog_enable(state, ctx, &ws, required(args, "module")?)?,
        "capability_search" => {
            let q = required(args, "query")?.to_lowercase();
            let reg = Registry::load(&ws);
            let tools: Vec<_> = reg.list_tools().into_iter().cloned().collect();
            let caps: Vec<Value> = reg
                .list_capabilities()
                .into_iter()
                .filter(|c| allowed(ctx, &c.full_id()))
                .filter(|c| {
                    format!("{} {} {}", c.full_id(), c.title, c.description).to_lowercase().contains(&q)
                })
                .map(|c| json!({ "id": c.full_id(), "title": c.title, "providers": route(&c.full_id(), None, &ws, &tools) }))
                .collect();
            json!({ "capabilities": caps })
        }
        "capability_describe" => {
            let id = required(args, "id")?;
            let reg = Registry::load(&ws);
            let cap = reg
                .get_capability(id)
                .ok_or_else(|| format!("Không có capability \"{id}\"."))?;
            let tools: Vec<_> = reg.list_tools().into_iter().cloned().collect();
            let providers: Vec<Value> = route(id, None, &ws, &tools)
                .iter()
                .filter_map(|t| reg.get_tool(t))
                .map(|t| {
                    let probe = studio_tools::probe(t, state.secrets.as_ref(), Some(&ws));
                    json!({ "id": t.id, "description": t.description, "side_effect": t.side_effect.to_vietnamese(), "probe": probe })
                })
                .collect();
            json!({ "capability": cap, "providers": providers })
        }
        "capability_run" => capability_run(state, ctx, &ws, args).await?,
        "check_run" => check_run(&ws, args)?,
        other => return Err(format!("Không có tool \"{other}\".")),
    };
    Ok(inline_or_file(&ws, out))
}

fn allowed(ctx: &TurnContext, cap: &str) -> bool {
    ctx.allowed_capabilities
        .as_ref()
        .is_none_or(|list| list.iter().any(|c| c == cap))
}

fn walk(ws: &Workspace, dir: &Path, depth: usize) -> Vec<Value> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut entries: Vec<_> = rd.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    entries
        .into_iter()
        .filter(|e| e.file_name() != ".studio")
        .map(|e| {
            let p = e.path();
            if p.is_dir() {
                let children = if depth < 2 {
                    walk(ws, &p, depth + 1)
                } else {
                    Vec::new()
                };
                json!({ "dir": rel(ws, &p), "children": children })
            } else {
                json!({ "file": rel(ws, &p) })
            }
        })
        .collect()
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_files(&p, out);
        } else {
            out.push(p);
        }
    }
}

fn artifact_list(ws: &Workspace, project: Option<&str>) -> Result<Value, String> {
    let projects = match project {
        Some(slug) => vec![Project::open(ws, slug).map_err(|e| e.to_string())?],
        None => ws.list_projects().map_err(|e| e.to_string())?,
    };
    let mut out = Vec::new();
    for p in projects {
        let Ok(rd) = std::fs::read_dir(p.artifacts_dir()) else {
            continue;
        };
        for e in rd.flatten() {
            let yaml = e.path().join("artifact.yaml");
            let meta: Value = std::fs::read_to_string(&yaml)
                .ok()
                .and_then(|t| serde_yaml::from_str(&t).ok())
                .unwrap_or(Value::Null);
            out.push(json!({
                "project": p.slug(),
                "dir": rel(ws, &e.path()),
                "id": meta.get("id"), "kind": meta.get("kind"), "title": meta.get("title"),
            }));
        }
    }
    Ok(json!({ "artifacts": out }))
}

fn scope_get(set: &BrandSet, name: &str, args: &Value) -> Result<Value, String> {
    let key = arg(args, if name == "brand_get" { "slug" } else { "id" });
    let v = match (name, key) {
        ("brand_get", Some(k)) => {
            let b = set
                .get_brand(k)
                .ok_or_else(|| format!("Không có brand \"{k}\"."))?;
            json!({ "slug": b.slug, "dir": b.dir, "version": b.version, "spec": b.spec, "voice": b.voice_text })
        }
        ("brand_get", None) => json!({ "brands": set.brands.keys().collect::<Vec<_>>() }),
        ("channel_get", Some(k)) => serde_json::to_value(
            set.get_channel(k)
                .ok_or_else(|| format!("Không có kênh \"{k}\"."))?,
        )
        .map_err(|e| e.to_string())?,
        ("channel_get", None) => json!({ "channels": set.channels.keys().collect::<Vec<_>>() }),
        ("playbook_get", Some(k)) => serde_json::to_value(
            set.get_playbook(k)
                .ok_or_else(|| format!("Không có playbook \"{k}\"."))?,
        )
        .map_err(|e| e.to_string())?,
        _ => json!({ "playbooks": set.playbooks.keys().collect::<Vec<_>>() }),
    };
    Ok(v)
}

fn catalog_enable(
    state: &ServerState,
    ctx: &TurnContext,
    ws: &Workspace,
    module: &str,
) -> Result<Value, String> {
    let slug = ctx
        .project
        .as_deref()
        .ok_or("Lượt này không gắn với project nào.")?;
    if !state.catalog_paths.is_empty() {
        let catalog = studio_catalog::Catalog::load_layers(&state.catalog_paths)
            .map_err(|e| e.to_string())?;
        if !catalog.modules.contains_key(module) {
            return Err(format!("Catalog không có module \"{module}\"."));
        }
    }
    let project = Project::open(ws, slug).map_err(|e| e.to_string())?;
    let meta = project.meta().map_err(|e| e.to_string())?;
    let mut modules = meta.modules.clone();
    modules.push(module.to_string());
    let meta = project
        .set_modules(meta.kit, &modules)
        .map_err(|e| e.to_string())?;
    Ok(
        json!({ "project": slug, "modules": meta.modules, "note": "Module có hiệu lực từ lượt tiếp theo." }),
    )
}

async fn capability_run(
    state: &ServerState,
    ctx: &TurnContext,
    ws: &Workspace,
    args: &Value,
) -> Result<Value, String> {
    let cap = required(args, "capability")?;
    if !allowed(ctx, cap) {
        return Err(format!("Capability \"{cap}\" không được bật cho lượt này."));
    }
    let input = args.get("input").cloned().unwrap_or_else(|| json!({}));
    let reg = Registry::load(ws);
    let tools: Vec<_> = reg.list_tools().into_iter().cloned().collect();
    let providers = route(cap, arg(args, "provider"), ws, &tools);
    if providers.is_empty() {
        return Err(format!("Chưa có provider nào cho \"{cap}\"."));
    }
    let run_ctx = ToolRunContext {
        workspace: ws,
        turn_id: Some(ctx.turn_id.clone()),
        budget: ctx.budget_usd,
        approved: false,
        scheduler: &state.scheduler,
        secret_store: state.secrets.as_ref(),
    };
    let mut errors = Vec::new();
    for provider in &providers {
        match run_tool(&reg, provider, input.clone(), &run_ctx).await {
            Ok(run) => return Ok(json!({ "provider": provider, "result": run.to_json_output() })),
            Err(e) => errors.push(format!("{provider}: {e}")),
        }
    }
    Err(format!("Mọi provider đều lỗi: {}", errors.join("; ")))
}

fn check_run(ws: &Workspace, args: &Value) -> Result<Value, String> {
    let dir = ws
        .scoped_path(required(args, "artifact_dir")?)
        .map_err(|e| e.to_string())?;
    let brand = required(args, "brand")?;
    let channels: Vec<String> = args
        .get("channels")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let renders: Vec<PathBuf> = args
        .get("renders")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .filter_map(|r| ws.scoped_path(r).ok())
                .collect()
        })
        .unwrap_or_default();
    let meta: Value = std::fs::read_to_string(dir.join("artifact.yaml"))
        .ok()
        .and_then(|t| serde_yaml::from_str(&t).ok())
        .unwrap_or(Value::Null);
    let source_file = meta
        .get("source")
        .and_then(Value::as_str)
        .unwrap_or("index.html")
        .to_string();
    let source = std::fs::read_to_string(dir.join(&source_file)).ok();
    let ffprobe = renders
        .iter()
        .find(|r| {
            matches!(
                r.extension().and_then(|e| e.to_str()),
                Some("mp4" | "mov" | "webm")
            )
        })
        .and_then(|r| ffprobe_json(r));
    let set = BrandSet::load(ws).map_err(|e| format!("Không nạp được Brand: {e}"))?;
    let channel_refs: Vec<&str> = channels.iter().map(String::as_str).collect();
    let report = run_checks(
        CheckSubject {
            artifact_dir: &dir,
            source_content: source.as_deref(),
            source_file: Some(&source_file),
            renders: &renders,
            ffprobe_json: ffprobe.as_ref(),
        },
        &set,
        brand,
        &channel_refs,
    )
    .ok_or_else(|| format!("Không có brand \"{brand}\"."))?;
    serde_json::to_value(report).map_err(|e| e.to_string())
}

fn ffprobe_json(path: &Path) -> Option<Value> {
    let out = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(path)
        .output()
        .ok()?;
    serde_json::from_slice(&out.stdout).ok()
}
