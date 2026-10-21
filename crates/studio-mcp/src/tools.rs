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
            "timeline_build",
            "Dựng timeline video tự động từ kịch bản, âm thanh TTS và hình ảnh b-roll. Tạo artifact.yaml và timeline.json với căn thời gian chuẩn xác, chia phụ đề theo câu, hỗ trợ lớp phủ (logo) và chuyển cảnh.",
            json!({
                "project": s("Slug của project (vd ca-phe-phin)"),
                "artifact": s("ID của artifact timeline cần tạo/ghi đè (vd video)"),
                "segments": json!({
                    "type": "array",
                    "description": "Danh sách các phân cảnh: audio (đường dẫn WAV/MP3), text (lời thoại kịch bản), visual (đường dẫn ảnh/video hoặc 'color:#RRGGBB')",
                    "items": {
                        "type": "object",
                        "properties": {
                            "audio": s("Đường dẫn tệp âm thanh tương đối trong workspace"),
                            "text": s("Lời thoại kịch bản của phân cảnh để tạo phụ đề"),
                            "visual": s("Đường dẫn ảnh/video tương đối trong workspace, hoặc mã màu 'color:#RRGGBB'")
                        },
                        "required": ["audio", "text", "visual"]
                    }
                }),
                "lead_s": json!({ "type": "number", "description": "Thời gian đệm đầu video (mặc định 0.5 giây)" }),
                "gap_s": json!({ "type": "number", "description": "Khoảng cách giữa các phân cảnh âm thanh (mặc định 0.4 giây)" }),
                "tail_s": json!({ "type": "number", "description": "Thời gian đệm đuôi video (mặc định 1.5 giây)" }),
                "transition": json!({ "type": "string", "enum": ["fade", "none"], "description": "Hiệu ứng chuyển cảnh giữa các clip hình ảnh (mặc định 'fade' 0.5s)" }),
                "captions": json!({ "type": "string", "enum": ["sentences", "none"], "description": "Chế độ tạo phụ đề: 'sentences' (mặc định) hoặc 'none'" }),
                "overlays": json!({
                    "type": "array",
                    "description": "Các lớp phủ (vd logo PNG): source, x, y, width, height, opacity, start?, duration?",
                    "items": {
                        "type": "object",
                        "properties": {
                            "source": s("Đường dẫn tệp lớp phủ PNG/ảnh tương đối trong workspace"),
                            "x": json!({ "type": "integer", "description": "Tọa độ X tính từ góc trên-trái (px)" }),
                            "y": json!({ "type": "integer", "description": "Tọa độ Y tính từ góc trên-trái (px)" }),
                            "width": json!({ "type": "integer", "description": "Chiều rộng co giãn (px)" }),
                            "height": json!({ "type": "integer", "description": "Chiều cao co giãn (px)" }),
                            "opacity": json!({ "type": "number", "description": "Độ mờ từ 0.0 đến 1.0" }),
                            "start": json!({ "type": "number", "description": "Thời điểm bắt đầu xuất hiện (giây, mặc định 0)" }),
                            "duration": json!({ "type": "number", "description": "Thời lượng xuất hiện (giây, mặc định toàn bộ video)" })
                        },
                        "required": ["source"]
                    }
                }),
                "brand": s("Slug của brand (tùy chọn)"),
                "channels": json!({ "type": "array", "items": { "type": "string" }, "description": "Danh sách kênh đích (vd ['youtube'])" }),
                "width": json!({ "type": "integer", "description": "Chiều rộng khung hình (mặc định lấy theo kênh, hoặc 1920)" }),
                "height": json!({ "type": "integer", "description": "Chiều cao khung hình (mặc định lấy theo kênh, hoặc 1080)" }),
                "fps": json!({ "type": "number", "description": "Tốc độ khung hình (mặc định lấy theo kênh, hoặc 30.0)" }),
                "min_duration_s": json!({ "type": "number", "description": "Thời lượng tối thiểu của video (kéo dài phần đệm đuôi nếu chưa đạt)" })
            }),
            &["project", "artifact", "segments"]
        ),
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
        tool(
            "graph_get",
            "Lấy đồ thị nội dung (các node artifact, trạng thái mới/lỗi thời/chưa render và các cạnh phụ thuộc) của một project.",
            json!({
                "project": s("Slug của project (tùy chọn nếu đang trong ngữ cảnh project)"),
                "focus": s("ID của artifact trọng tâm cần lấy đồ thị con (tùy chọn)"),
                "depth": json!({ "type": "integer", "description": "Độ sâu lân cận khi dùng focus (mặc định toàn bộ)" })
            }),
            &[]
        ),
        tool(
            "graph_impact",
            "Phân tích các sản phẩm/artifact phía dưới bị ảnh hưởng nếu một artifact thay đổi, theo thứ tự topo và kèm chi phí ước tính.",
            json!({
                "project": s("Slug của project (tùy chọn nếu đang trong ngữ cảnh project)"),
                "ref": s("ID artifact hoặc tham chiếu đầu vào cần kiểm tra ảnh hưởng (ví dụ: thumb-01 hoặc artifact:script-01)")
            }),
            &["ref"]
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

/// Strip Windows verbatim prefixes (`\\?\` or `\\?\UNC\`) and convert backslashes to forward slashes.
pub fn clean_to_forward_slashes(s: &str) -> String {
    let s_norm = s.replace('\\', "/");
    if let Some(rest) = s_norm.strip_prefix("//?/") {
        if let Some(unc_rest) = rest
            .strip_prefix("UNC/")
            .or_else(|| rest.strip_prefix("unc/"))
        {
            return format!("//{unc_rest}");
        }
        return rest.to_string();
    }
    s_norm
}

/// Clean workspace root path: strip verbatim prefix, convert slashes, trim trailing slash.
pub fn clean_workspace_root(ws_root: &Path) -> String {
    let clean = clean_to_forward_slashes(&ws_root.to_string_lossy());
    clean.trim_end_matches('/').to_string()
}

/// If a string points inside the workspace root (with or without `\\?\`, either slash style,
/// case-insensitive drive), return the workspace-relative forward-slash path.
/// If the string is outside the workspace or not a path pointing inside, return None.
pub fn normalise_path_str(ws_root: &Path, s: &str) -> Option<String> {
    let ws_clean = clean_workspace_root(ws_root);
    if ws_clean.is_empty() {
        return None;
    }
    let s_clean = clean_to_forward_slashes(s);
    if s_clean.len() < ws_clean.len() {
        return None;
    }

    // `get` (not indexing): the cut may fall inside a multi-byte char (Vietnamese folder names).
    let prefix = s_clean.get(..ws_clean.len())?;
    if !prefix.eq_ignore_ascii_case(&ws_clean) {
        return None;
    }

    if s_clean.len() == ws_clean.len() {
        return Some(String::new());
    }

    if s_clean.as_bytes()[ws_clean.len()] == b'/' {
        let rel = &s_clean[ws_clean.len() + 1..];
        return Some(rel.trim_start_matches('/').to_string());
    }

    None
}

/// Recursively normalise every string value in a JSON Value that points inside the workspace
/// to a workspace-relative forward-slash path. Leaves strings outside the workspace untouched.
pub fn normalise_workspace_paths(ws_root: &Path, value: &mut Value) {
    match value {
        Value::String(s) => {
            if let Some(rel) = normalise_path_str(ws_root, s) {
                *s = rel;
            }
        }
        Value::Array(arr) => {
            for v in arr {
                normalise_workspace_paths(ws_root, v);
            }
        }
        Value::Object(map) => {
            let old_map = std::mem::take(map);
            for (k, mut v) in old_map {
                normalise_workspace_paths(ws_root, &mut v);
                let new_k = normalise_path_str(ws_root, &k).unwrap_or(k);
                map.insert(new_k, v);
            }
        }
        _ => {}
    }
}

fn rel(ws: &Workspace, p: &Path) -> String {
    normalise_path_str(ws.root(), &p.to_string_lossy())
        .unwrap_or_else(|| p.to_string_lossy().replace('\\', "/"))
}

/// Write a large result to the cache and return a pointer value instead.
fn inline_or_file(ws: &Workspace, mut value: Value) -> Value {
    normalise_workspace_paths(ws.root(), &mut value);
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
        "timeline_build" => {
            studio_artifacts::timeline_build::build_timeline_tool(&ws, args).await?
        }
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
        "graph_get" => graph_get(&ws, ctx, args)?,
        "graph_impact" => graph_impact(&ws, ctx, args)?,
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

fn graph_get(ws: &Workspace, ctx: &TurnContext, args: &Value) -> Result<Value, String> {
    let project_slug = arg(args, "project")
        .map(String::from)
        .or_else(|| ctx.project.clone())
        .ok_or_else(|| "Cần chỉ định slug của project.".to_string())?;

    let proj = Project::open(ws, &project_slug).map_err(|e| e.to_string())?;
    let artifacts = studio_artifacts::Artifact::list(&proj).map_err(|e| e.to_string())?;
    let graph =
        studio_artifacts::build_project_graph(ws, &proj, &artifacts).map_err(|e| e.to_string())?;

    // Optional focus / depth filtering
    let focus = arg(args, "focus").map(studio_artifacts::normalize_ref);
    let depth = args
        .get("depth")
        .and_then(Value::as_u64)
        .unwrap_or(u64::MAX) as usize;

    if let Some(focus_id) = focus {
        // Collect subgraph around focus_id within depth
        let mut included: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        included.insert(focus_id.clone());

        for _ in 0..depth {
            let mut newly_added = Vec::new();
            for edge in &graph.edges {
                let from_norm = studio_artifacts::normalize_ref(&edge.from);
                let to_norm = studio_artifacts::normalize_ref(&edge.to);
                if included.contains(&from_norm) || included.contains(&edge.from) {
                    newly_added.push(to_norm.clone());
                    newly_added.push(edge.to.clone());
                }
                if included.contains(&to_norm) || included.contains(&edge.to) {
                    newly_added.push(from_norm);
                    newly_added.push(edge.from.clone());
                }
            }
            if newly_added.is_empty() {
                break;
            }
            for item in newly_added {
                included.insert(item);
            }
        }

        let filtered_nodes: Vec<_> = graph
            .nodes
            .into_iter()
            .filter(|n| {
                included.contains(&n.id)
                    || included.contains(&studio_artifacts::normalize_ref(&n.id))
            })
            .collect();

        let filtered_edges: Vec<_> = graph
            .edges
            .into_iter()
            .filter(|e| {
                (included.contains(&e.from)
                    || included.contains(&studio_artifacts::normalize_ref(&e.from)))
                    && (included.contains(&e.to)
                        || included.contains(&studio_artifacts::normalize_ref(&e.to)))
            })
            .collect();

        Ok(json!({
            "project": project_slug,
            "nodes": filtered_nodes,
            "edges": filtered_edges,
        }))
    } else {
        Ok(json!({
            "project": project_slug,
            "nodes": graph.nodes,
            "edges": graph.edges,
        }))
    }
}

fn graph_impact(ws: &Workspace, ctx: &TurnContext, args: &Value) -> Result<Value, String> {
    let target_ref = required(args, "ref")?;
    let project_slug = arg(args, "project")
        .map(String::from)
        .or_else(|| ctx.project.clone())
        .ok_or_else(|| "Cần chỉ định slug của project.".to_string())?;

    let proj = Project::open(ws, &project_slug).map_err(|e| e.to_string())?;
    let artifacts = studio_artifacts::Artifact::list(&proj).map_err(|e| e.to_string())?;
    let impact = studio_artifacts::calculate_impact(ws, &proj, &artifacts, target_ref);

    Ok(json!({
        "project": project_slug,
        "ref": target_ref,
        "impact": impact,
    }))
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

/// Adds `duration_seconds` next to a WAV `audio_path` in a tool result, so agents can plan a
/// timeline without probing every clip themselves.
fn annotate_audio_duration(ws: &Workspace, output: &mut Value) {
    let Some(rel) = output.get("audio_path").and_then(Value::as_str) else {
        return;
    };
    let path = std::path::Path::new(rel);
    let full = if path.is_absolute() {
        path.to_path_buf()
    } else {
        ws.root().join(path)
    };
    if !full.starts_with(ws.root()) {
        return;
    }
    if let Some(secs) = wav_duration_seconds(&full) {
        output["duration_seconds"] = json!((secs * 1000.0).round() / 1000.0);
    }
}

/// Duration of a PCM/float WAV file from its RIFF header (`data` bytes / byte rate).
fn wav_duration_seconds(path: &std::path::Path) -> Option<f64> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let mut pos = 12;
    let mut byte_rate: Option<u32> = None;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().ok()?) as usize;
        let body = pos + 8;
        if id == b"fmt " && body + 12 <= bytes.len() {
            byte_rate = Some(u32::from_le_bytes(
                bytes[body + 8..body + 12].try_into().ok()?,
            ));
        } else if id == b"data" {
            let rate = byte_rate.filter(|r| *r > 0)?;
            let data_len = size.min(bytes.len().saturating_sub(body));
            return Some(data_len as f64 / rate as f64);
        }
        pos = body + size + (size & 1);
    }
    None
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
                annotate_audio_duration(ws, &mut output);
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
                                ..Default::default()
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

    // Timeline audio is normalized to the loudness target of the first channel that sets one.
    let loudness_lufs = BrandSet::load(ws).ok().and_then(|set| {
        artifact
            .channels
            .iter()
            .find_map(|c| set.get_channel(c).and_then(|ch| ch.loudness_lufs))
            .map(f64::from)
    });
    let options = studio_artifacts::RenderOptions { loudness_lufs };
    let outcome = studio_artifacts::render_with(ws, &p, &artifact, scheduler, &options)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_normalise_path_str_verbatim() {
        let ws = Path::new(r"\\?\D:\Studio\workspace");
        let input = r"\\?\D:\Studio\workspace\projects\p1\artifact.yaml";
        let norm = normalise_path_str(ws, input);
        assert_eq!(norm, Some("projects/p1/artifact.yaml".to_string()));
    }

    #[test]
    fn test_normalise_path_str_normal() {
        let ws = Path::new(r"D:\Studio\workspace");
        let input = "D:/Studio/workspace/projects/p1/artifact.yaml";
        let norm = normalise_path_str(ws, input);
        assert_eq!(norm, Some("projects/p1/artifact.yaml".to_string()));
    }

    #[test]
    fn test_normalise_path_str_mixed_slashes() {
        let ws = Path::new(r"D:/Studio/workspace");
        let input = r"D:\Studio/workspace\projects/p1\artifacts\thumb.png";
        let norm = normalise_path_str(ws, input);
        assert_eq!(norm, Some("projects/p1/artifacts/thumb.png".to_string()));
    }

    #[test]
    fn test_normalise_path_str_case_insensitive_drive() {
        let ws = Path::new(r"D:\Studio\workspace");
        let input = r"\\?\d:\Studio\workspace\projects\p1\file.txt";
        let norm = normalise_path_str(ws, input);
        assert_eq!(norm, Some("projects/p1/file.txt".to_string()));

        let ws_lower = Path::new(r"d:\Studio\workspace");
        let input_upper = r"D:\Studio\workspace\projects\p1\file.txt";
        let norm2 = normalise_path_str(ws_lower, input_upper);
        assert_eq!(norm2, Some("projects/p1/file.txt".to_string()));
    }

    #[test]
    fn test_normalise_path_str_multibyte_no_panic() {
        let ws = Path::new(r"D:\Studio\ws1");
        assert_eq!(normalise_path_str(ws, "D:/Studio/wsđường dẫn/x.png"), None);
        assert_eq!(
            normalise_path_str(ws, "D:/Studio/ws1/dự án/ảnh.png").as_deref(),
            Some("dự án/ảnh.png")
        );
    }

    #[test]
    fn test_normalise_path_str_outside_path() {
        let ws = Path::new(r"D:\Studio\workspace");
        let outside1 = r"\\?\C:\Windows\System32\cmd.exe";
        assert_eq!(normalise_path_str(ws, outside1), None);

        let outside2 = r"D:\Other\Folder\file.txt";
        assert_eq!(normalise_path_str(ws, outside2), None);

        let outside3 = r"D:\Studio\workspace_sibling\file.txt";
        assert_eq!(normalise_path_str(ws, outside3), None);

        let non_path = "hello world";
        assert_eq!(normalise_path_str(ws, non_path), None);
    }

    #[test]
    fn test_normalise_workspace_paths_nested() {
        let ws = Path::new(r"\\?\D:\Studio\ws1");
        let mut json = json!({
            "status": "ok",
            "artifact": {
                "source": r"\\?\D:\Studio\ws1\projects\p1\timeline.json",
                "renders": [
                    r"\\?\d:\Studio\ws1\projects\p1\renders\out.mp4",
                    "D:/Studio/ws1/projects/p1/renders/thumb.png",
                    r"D:\Studio/ws1\projects\p1/renders\thumb.webp",
                    r"C:\Windows\temp\outside.log"
                ]
            },
            "unrelated": "text string",
            "number": 42
        });

        normalise_workspace_paths(ws, &mut json);

        assert_eq!(json["artifact"]["source"], "projects/p1/timeline.json");
        assert_eq!(
            json["artifact"]["renders"][0],
            "projects/p1/renders/out.mp4"
        );
        assert_eq!(
            json["artifact"]["renders"][1],
            "projects/p1/renders/thumb.png"
        );
        assert_eq!(
            json["artifact"]["renders"][2],
            "projects/p1/renders/thumb.webp"
        );
        assert_eq!(
            json["artifact"]["renders"][3],
            r"C:\Windows\temp\outside.log"
        );
        assert_eq!(json["status"], "ok");
        assert_eq!(json["unrelated"], "text string");
        assert_eq!(json["number"], 42);
    }
}

#[cfg(test)]
mod audio_duration_tests {
    use super::wav_duration_seconds;

    fn wav(sample_rate: u32, channels: u16, bits: u16, samples: u32) -> Vec<u8> {
        let block = channels * bits / 8;
        let data_len = samples * block as u32;
        let mut v = Vec::new();
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&(36 + data_len).to_le_bytes());
        v.extend_from_slice(b"WAVEfmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&channels.to_le_bytes());
        v.extend_from_slice(&sample_rate.to_le_bytes());
        v.extend_from_slice(&(sample_rate * block as u32).to_le_bytes());
        v.extend_from_slice(&block.to_le_bytes());
        v.extend_from_slice(&bits.to_le_bytes());
        v.extend_from_slice(b"data");
        v.extend_from_slice(&data_len.to_le_bytes());
        v.resize(v.len() + data_len as usize, 0);
        v
    }

    #[test]
    fn reads_duration_from_wav_header() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.wav");
        std::fs::write(&p, wav(22050, 1, 16, 22050 * 3 / 2)).unwrap();
        let d = wav_duration_seconds(&p).unwrap();
        assert!((d - 1.5).abs() < 1e-6, "{d}");
        std::fs::write(&p, b"not a wav").unwrap();
        assert!(wav_duration_seconds(&p).is_none());
    }
}
