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
        tool("artifact_render", "Render một artifact thành tệp đầu ra (PNG, PDF, MP4...) qua job scheduler. Render an artifact into output files via job scheduler.", json!({ "artifact": s("ID artifact (vd thumb-01) hoặc đường dẫn tương đối (Artifact ID or relative path)"), "project": s("Slug project, tùy chọn nếu artifact chứa đường dẫn hoặc trong ngữ cảnh project (Optional project slug)"), "format": s("Định dạng xuất tùy chọn, vd png, pdf (Optional output format)") }), &["artifact"]),
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
        tool("check_run", "Kiểm artifact theo Brand và kênh (hãy render trước rồi kiểm tra — render first, then check).", json!({ "artifact_dir": s("thư mục artifact tương đối"), "brand": s("slug brand"), "channels": { "type": "array", "items": { "type": "string" } }, "renders": { "type": "array", "items": { "type": "string" } } }), &["artifact_dir", "brand", "channels"]),
        tool(
            "publish_prepare",
            "Chuẩn bị bản nháp đăng tải (YouTube, Facebook, TikTok, Instagram, Export). Kiểm tra giới hạn kênh và bản render mới nhất, tạo bản nháp chờ người dùng xác nhận trên desktop.",
            json!({
                "artifact": s("ID artifact (vd thumb-01) hoặc đường dẫn tương đối"),
                "project": s("Slug project, tùy chọn nếu artifact chứa đường dẫn hoặc trong ngữ cảnh project"),
                "channel": s("ID kênh đích (vd youtube, youtube-shorts, facebook, tiktok, instagram)"),
                "target": json!({
                    "type": "string",
                    "enum": ["youtube", "facebook", "tiktok", "instagram", "export"],
                    "description": "Nền tảng xuất bản đích"
                }),
                "metadata": json!({
                    "type": "object",
                    "properties": {
                        "title": s("Tiêu đề bài đăng hoặc video"),
                        "description": s("Mô tả hoặc nội dung bài đăng"),
                        "tags": json!({ "type": "array", "items": { "type": "string" }, "description": "Danh sách thẻ hoặc hashtag" }),
                        "privacy": json!({ "type": "string", "enum": ["private", "unlisted", "public"], "description": "Chế độ hiển thị (mặc định private)" }),
                        "schedule": s("Thời gian hẹn giờ đăng theo chuẩn ISO 8601 (tùy chọn)"),
                        "thumbnail": s("Đường dẫn tệp ảnh thu nhỏ tùy chọn trong workspace")
                    },
                    "required": ["title"]
                })
            }),
            &["artifact", "channel", "target", "metadata"]
        ),
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
        "artifact_render" => {
            let artifact_raw = required(args, "artifact")?;
            let project_raw = arg(args, "project");
            let format_raw = arg(args, "format");
            let (proj, id) = resolve_artifact_target(&ws, ctx, artifact_raw, project_raw)?;
            render_artifact_shared(&ws, &state.scheduler, &proj, &id, format_raw).await?
        }
        "check_run" => check_run(&ws, args)?,
        "publish_prepare" => {
            let artifact_raw = required(args, "artifact")?;
            let project_raw = arg(args, "project");
            let channel_raw = required(args, "channel")?;
            let target_raw = required(args, "target")?;
            let target = target_raw
                .parse::<studio_tools::publish::PublishTarget>()
                .map_err(|e| e.to_string())?;

            let meta_val = args.get("metadata").cloned().unwrap_or(json!({}));
            let metadata: studio_tools::publish::PublishMetadata = serde_json::from_value(meta_val)
                .map_err(|e| format!("Dữ liệu metadata không hợp lệ: {e}"))?;

            let db = studio_workspace::WorkspaceDb::open(&ws)
                .map_err(|e| format!("Không mở được cơ sở dữ liệu: {e}"))?;

            let outcome = studio_tools::publish::prepare_publish(
                &ws,
                db.conn(),
                artifact_raw,
                project_raw,
                channel_raw,
                target,
                metadata,
            )
            .map_err(|e| e.to_string())?;

            serde_json::to_value(outcome).map_err(|e| e.to_string())?
        }
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
    let endpoints = state.endpoints.read().await.clone();
    let pricing = state.pricing.read().await.clone();
    let run_ctx = ToolRunContext {
        workspace: ws,
        turn_id: Some(ctx.turn_id.clone()),
        budget: ctx.budget_usd,
        approved: false,
        scheduler: &state.scheduler,
        secret_store: state.secrets.as_ref(),
        endpoints,
        pricing,
    };
    let mut errors = Vec::new();
    for provider in &providers {
        match run_tool(&reg, provider, input.clone(), &run_ctx).await {
            Ok(run) => {
                let mut output = run.to_json_output();
                relocate_and_index_generated_files(ws, ctx, provider, &input, &mut output);
                return Ok(json!({ "provider": provider, "result": output }));
            }
            Err(e) => errors.push(format!("{provider}: {e}")),
        }
    }
    Err(format!("Mọi provider đều lỗi: {}", errors.join("; ")))
}

fn relocate_and_index_generated_files(
    ws: &Workspace,
    ctx: &TurnContext,
    provider: &str,
    input: &Value,
    output: &mut Value,
) {
    // Only files the generator wrote into this turn's cache are moved; paths the agent passed in
    // (or anything else in the workspace) are never touched.
    let turn_cache = ws.cache_dir().join("turns").join(&ctx.turn_id);
    let date_str = chrono::Local::now().format("%Y-%m-%d").to_string();
    let target_dir = match &ctx.project {
        Some(proj) => ws
            .root()
            .join("projects")
            .join(proj)
            .join("assets")
            .join("generated")
            .join(&date_str),
        None => ws.root().join("library").join("generated").join(&date_str),
    };

    if std::fs::create_dir_all(&target_dir).is_err() {
        return;
    }

    let model = output
        .get("provider_metadata")
        .and_then(|m| m.get("model"))
        .and_then(Value::as_str)
        .map(String::from);
    let prompt = input
        .get("prompt")
        .and_then(Value::as_str)
        .map(String::from);
    let cost = output
        .get("provider_metadata")
        .and_then(|m| m.get("estimated_cost_usd"))
        .and_then(Value::as_f64);

    let mut path_map = std::collections::HashMap::new();

    // Check files array
    if let Some(files_arr) = output.get("files").and_then(Value::as_array) {
        for f_val in files_arr {
            if let Some(f_str) = f_val.as_str() {
                // Providers may return absolute paths (e.g. the importer); make them workspace-relative
                // first so the same scoping + turn-cache check applies.
                let rel_str = Path::new(f_str)
                    .strip_prefix(ws.root())
                    .map(|p| p.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_else(|_| f_str.to_string());
                if let Ok(src_path) = ws.scoped_path(&rel_str) {
                    if src_path.is_file() && src_path.starts_with(&turn_cache) {
                        let filename = src_path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "generated_asset".into());
                        let file_stem = Path::new(&filename)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("asset");
                        let file_ext = Path::new(&filename)
                            .extension()
                            .and_then(|s| s.to_str())
                            .unwrap_or("");

                        // Unique destination filename
                        let dest_filename = if target_dir.join(&filename).exists() {
                            let uid = &uuid::Uuid::new_v4().to_string()[..8];
                            if file_ext.is_empty() {
                                format!("{file_stem}-{uid}")
                            } else {
                                format!("{file_stem}-{uid}.{file_ext}")
                            }
                        } else {
                            filename
                        };

                        let dest_path = target_dir.join(&dest_filename);

                        // Move (or copy then delete)
                        let moved = std::fs::rename(&src_path, &dest_path).is_ok()
                            || (std::fs::copy(&src_path, &dest_path).is_ok()
                                && std::fs::remove_file(&src_path).is_ok());

                        if moved {
                            let new_rel = rel(ws, &dest_path);
                            path_map.insert(f_str.to_string(), new_rel.clone());

                            // Record in assets_index & create .meta.yaml sidecar
                            let sidecar = studio_workspace::AssetSidecarMeta {
                                title: Some(dest_filename.clone()),
                                description: prompt.clone(),
                                mime: None,
                                provider: Some(provider.to_string()),
                                model: model.clone(),
                                prompt: prompt.clone(),
                                cost_usd: cost,
                                turn_id: Some(ctx.turn_id.clone()),
                            };

                            let sidecar_file = dest_path.with_extension(format!(
                                "{}.meta.yaml",
                                dest_path.extension().and_then(|s| s.to_str()).unwrap_or("")
                            ));
                            if let Ok(sidecar_yaml) = serde_yaml::to_string(&sidecar) {
                                let _ = std::fs::write(sidecar_file, sidecar_yaml);
                            }

                            // Insert into assets_index table
                            if let Ok(db) = studio_workspace::WorkspaceDb::open(ws) {
                                let size_bytes = std::fs::metadata(&dest_path)
                                    .map(|m| m.len() as i64)
                                    .unwrap_or(0);
                                let ext = dest_path
                                    .extension()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("")
                                    .to_lowercase();
                                let mime = match ext.as_str() {
                                    "png" => "image/png",
                                    "jpg" | "jpeg" => "image/jpeg",
                                    "webp" => "image/webp",
                                    "mp4" => "video/mp4",
                                    "webm" => "video/webm",
                                    _ => "application/octet-stream",
                                };
                                let sha256 = hash_file_sha256(&dest_path).unwrap_or_default();
                                let now = chrono::Utc::now().to_rfc3339();
                                let meta_json =
                                    serde_json::to_string(&sidecar).unwrap_or_else(|_| "{}".into());

                                let _ = db.conn().execute(
                                    "INSERT OR REPLACE INTO assets_index (id, path, name, mime_type, size_bytes, sha256, metadata_json, updated_at)
                                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                                    rusqlite::params![
                                        new_rel,
                                        new_rel,
                                        dest_filename,
                                        mime,
                                        size_bytes,
                                        sha256,
                                        meta_json,
                                        now,
                                    ],
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // Rewrite returned paths in output
    if let Some(files_arr) = output.get_mut("files").and_then(Value::as_array_mut) {
        for item in files_arr {
            if let Some(old_str) = item.as_str() {
                if let Some(new_p) = path_map.get(old_str) {
                    *item = json!(new_p);
                }
            }
        }
    }
    if let Some(video_p) = output.get_mut("video_path") {
        if let Some(old_str) = video_p.as_str() {
            if let Some(new_p) = path_map.get(old_str) {
                *video_p = json!(new_p);
            }
        }
    }
}

fn hash_file_sha256(path: &Path) -> Option<String> {
    use sha2::{Digest, Sha256};
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).ok()?;
    Some(hex::encode(hasher.finalize()))
}

/// Run brand/channel checkers on an artifact (shared by the MCP tool and the desktop app).
pub fn check_run(ws: &Workspace, args: &Value) -> Result<Value, String> {
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
    let mut renders: Vec<PathBuf> = args
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

    // No explicit renders → use the newest render of this artifact (one input hash only, so stale
    // renders of an older version never mask a failing check).
    if renders.is_empty() {
        if let Some(artifact_id) = meta.get("id").and_then(Value::as_str) {
            renders = latest_renders(ws, &dir, artifact_id);
        }
    }

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
    let mut report = run_checks(
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
    // Show workspace-relative locations to the user, never absolute verbatim paths.
    for ch in report.channels.values_mut() {
        for f in &mut ch.findings {
            if let Some(loc) = f.location.as_mut() {
                let p = Path::new(loc.as_str());
                if p.is_absolute() {
                    *loc = rel(ws, p);
                }
            }
        }
    }
    serde_json::to_value(report).map_err(|e| e.to_string())
}

/// Output files of the newest render of an artifact at `projects/<p>/artifacts/<id>`.
///
/// Prefers the `renders` table (newest `created_at`, a single input hash, restricted to this
/// project's renders folder); falls back to the newest `renders/<id>/<hash>/` folder on disk.
fn latest_renders(ws: &Workspace, artifact_dir: &Path, artifact_id: &str) -> Vec<PathBuf> {
    let Some(proj_dir) = artifact_dir
        .parent()
        .filter(|p| p.file_name().and_then(|n| n.to_str()) == Some("artifacts"))
        .and_then(Path::parent)
    else {
        return Vec::new();
    };
    let renders_dir = proj_dir.join("renders").join(artifact_id);
    let prefix = format!("{}/", rel(ws, &renders_dir));

    let mut out = Vec::new();
    if let Ok(db) = studio_workspace::WorkspaceDb::open(ws) {
        if let Ok(mut stmt) = db.conn().prepare(
            "SELECT output_path, hash FROM renders WHERE artifact_id = ?1 ORDER BY created_at DESC",
        ) {
            let rows: Vec<(String, String)> = stmt
                .query_map([artifact_id], |row| Ok((row.get(0)?, row.get(1)?)))
                .map(|iter| iter.flatten().collect())
                .unwrap_or_default();
            let mut newest_hash: Option<String> = None;
            for (rel_p, hash) in rows {
                if !rel_p.starts_with(&prefix) {
                    continue;
                }
                match &newest_hash {
                    None => newest_hash = Some(hash),
                    Some(h) if *h != hash => continue,
                    Some(_) => {}
                }
                if let Ok(p) = ws.scoped_path(&rel_p) {
                    if p.is_file() && !out.contains(&p) {
                        out.push(p);
                    }
                }
            }
        }
    }
    if !out.is_empty() {
        return out;
    }

    let mut hash_dirs: Vec<PathBuf> = std::fs::read_dir(&renders_dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    hash_dirs.sort_by_key(|p| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });
    if let Some(latest) = hash_dirs.last() {
        out.extend(
            std::fs::read_dir(latest)
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_file()),
        );
    }
    out
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

/// Shared render execution path used by both MCP tool and desktop Tauri command.
pub async fn render_artifact_shared(
    ws: &Workspace,
    scheduler: &studio_jobs::Scheduler,
    project_slug: &str,
    artifact_id: &str,
    format: Option<&str>,
) -> Result<Value, String> {
    let p = Project::open(ws, project_slug).map_err(|e| e.to_string())?;
    let mut artifact = studio_artifacts::Artifact::load(&p, artifact_id)
        .map_err(|e| format!("Không tải được artifact \"{artifact_id}\": {e}"))?;

    let art_dir = artifact.dir(&p);
    if !ws.can_write(&art_dir) {
        return Err("Artifact này thuộc tầng chỉ đọc, không thể render.".into());
    }

    if let Some(f) = format {
        match f.to_lowercase().as_str() {
            "pdf" => artifact.renderer = Some("render.html_pdf@1".into()),
            "png" => artifact.renderer = Some("render.html_png@1".into()),
            _ => {}
        }
    }

    let outcome = studio_artifacts::render(ws, &p, &artifact, scheduler)
        .await
        .map_err(|e| format!("Render lỗi: {e}"))?;

    let outputs: Vec<String> = outcome.outputs.iter().map(|o| rel(ws, o)).collect();
    let job_id = outcome.job_id.map(|j| j.to_string());

    Ok(json!({
        "cached": outcome.cached,
        "outputs": outputs,
        "job_id": job_id.clone(),
        "jobId": job_id,
        "probes": outcome.probe_report.iter().map(|r| json!({ "name": r.name, "available": r.available, "note": r.note })).collect::<Vec<_>>(),
    }))
}

fn resolve_artifact_target(
    ws: &Workspace,
    ctx: &TurnContext,
    artifact_raw: &str,
    project_raw: Option<&str>,
) -> Result<(String, String), String> {
    let raw = artifact_raw.trim().replace('\\', "/");
    if raw.is_empty() {
        return Err("Tham số \"artifact\" không được để trống.".into());
    }
    if raw.contains("..")
        || raw.starts_with('/')
        || (raw.len() >= 2 && raw.chars().nth(1) == Some(':'))
    {
        return Err("Đường dẫn artifact không hợp lệ (path traversal).".into());
    }

    let parts: Vec<&str> = raw.split('/').filter(|p| !p.is_empty()).collect();
    let (inferred_proj, art_id) = if parts.first() == Some(&"projects") {
        if parts.len() < 3 {
            return Err(format!(
                "Đường dẫn artifact không hợp lệ: \"{artifact_raw}\"."
            ));
        }
        let proj = parts[1].to_string();
        let id = if parts.len() >= 4 && parts[2] == "artifacts" {
            parts[3].trim_end_matches(".yaml").to_string()
        } else {
            parts[2].trim_end_matches(".yaml").to_string()
        };
        (Some(proj), id)
    } else if parts.first() == Some(&"artifacts") {
        if parts.len() < 2 {
            return Err(format!(
                "Đường dẫn artifact không hợp lệ: \"{artifact_raw}\"."
            ));
        }
        let id = parts[1].trim_end_matches(".yaml").to_string();
        (None, id)
    } else {
        let id = parts[0].trim_end_matches(".yaml").to_string();
        (None, id)
    };

    let target_proj = match (project_raw, inferred_proj) {
        (Some(p_arg), Some(p_inf)) => {
            if p_arg != p_inf {
                return Err(format!(
                    "Slug project được truyền (\"{p_arg}\") không khớp với đường dẫn artifact (\"{p_inf}\")."
                ));
            }
            p_arg.to_string()
        }
        (Some(p_arg), None) => p_arg.to_string(),
        (None, Some(p_inf)) => p_inf,
        (None, None) => {
            if let Some(ref cp) = ctx.project {
                cp.clone()
            } else {
                let projects = ws.list_projects().map_err(|e| e.to_string())?;
                let matching: Vec<_> = projects
                    .into_iter()
                    .filter(|p| p.artifacts_dir().join(&art_id).exists())
                    .collect();
                if matching.len() == 1 {
                    matching[0].slug().to_string()
                } else if matching.is_empty() {
                    return Err(format!(
                        "Không tìm thấy artifact \"{art_id}\" trong project nào. Hãy chỉ định tham số \"project\"."
                    ));
                } else {
                    return Err(format!(
                        "Artifact \"{art_id}\" xuất hiện ở nhiều project. Hãy chỉ định rõ tham số \"project\"."
                    ));
                }
            }
        }
    };

    if let Some(ref turn_proj) = ctx.project {
        if *turn_proj != target_proj {
            return Err(format!(
                "Lượt này bị giới hạn trong project \"{turn_proj}\", không được thao tác project \"{target_proj}\"."
            ));
        }
    }

    Ok((target_proj, art_id))
}
