//! Asset management commands for Local Content Studio.
//! Merges `library/` and `projects/*/assets/**`, handles sidecar metadata,
//! drag-and-drop import with safety checks and checkpoints, delete with undo,
//! and cached thumbnail generation.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use studio_workspace::{AssetSidecarMeta, CheckpointService, Indexer, Workspace, WorkspaceDb};
use tauri::State;

use crate::error::{CmdError, CmdResult};
use crate::state::AppState;

/// Filter options for asset listing.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AssetFilter {
    /// Asset kind (image, video, audio, document, font, other, all)
    pub kind: Option<String>,
    /// Asset source (generated, imported, uploaded, all)
    pub source: Option<String>,
    /// Project slug
    pub project: Option<String>,
    /// Brand slug
    pub brand: Option<String>,
    /// Text search query
    pub search: Option<String>,
    /// Max items to return
    pub limit: Option<usize>,
}

/// AI generation and sidecar metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetMetaDto {
    pub prompt: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub cost_usd: Option<f64>,
    pub created_at: Option<String>,
    pub turn_id: Option<String>,
    pub description: Option<String>,
}

/// Asset item returned to UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetItemDto {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub mime: String,
    pub size: u64,
    pub path: String,
    pub preview_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub source: String,
    pub project: Option<String>,
    pub brand: Option<String>,
    pub meta: AssetMetaDto,
    pub created_at: String,
    pub updated_at: String,
}

/// Import asset request payload.
#[derive(Debug, Clone, Deserialize)]
pub struct AssetImportRequest {
    pub paths: Vec<String>,
}

/// Convert local filesystem path to Tauri asset protocol URL.
pub fn asset_url(path: &Path) -> String {
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

/// Determine asset kind and mime type from path.
pub fn classify_asset(path: &Path) -> (&'static str, &'static str) {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "png" => ("image", "image/png"),
        "jpg" | "jpeg" => ("image", "image/jpeg"),
        "gif" => ("image", "image/gif"),
        "webp" => ("image", "image/webp"),
        "svg" => ("image", "image/svg+xml"),
        "bmp" => ("image", "image/bmp"),
        "ico" => ("image", "image/x-icon"),
        "tif" | "tiff" => ("image", "image/tiff"),

        "mp4" | "m4v" => ("video", "video/mp4"),
        "webm" => ("video", "video/webm"),
        "mov" => ("video", "video/quicktime"),
        "mkv" => ("video", "video/x-matroska"),
        "avi" => ("video", "video/x-msvideo"),

        "mp3" => ("audio", "audio/mpeg"),
        "wav" => ("audio", "audio/wav"),
        "ogg" | "opus" => ("audio", "audio/ogg"),
        "m4a" | "aac" => ("audio", "audio/aac"),
        "flac" => ("audio", "audio/flac"),

        "ttf" => ("font", "font/ttf"),
        "otf" => ("font", "font/otf"),
        "woff" => ("font", "font/woff"),
        "woff2" => ("font", "font/woff2"),

        "pdf" => ("document", "application/pdf"),
        "md" | "markdown" => ("document", "text/markdown"),
        "txt" | "srt" | "vtt" => ("document", "text/plain"),
        "html" | "htm" => ("document", "text/html"),
        "json" => ("document", "application/json"),
        "yaml" | "yml" | "toml" | "csv" => ("document", "text/plain"),
        "css" | "js" | "ts" | "tsx" | "jsx" | "py" | "rs" | "sh" | "ps1" => {
            ("document", "text/plain")
        }

        _ => ("other", "application/octet-stream"),
    }
}

/// Format bytes as human-readable string.
fn human_bytes(bytes: u64) -> String {
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

/// Generate or retrieve cached thumbnail for an image file.
fn get_or_generate_thumbnail(ws: &Workspace, full_path: &Path, sha256: &str) -> Option<String> {
    if !full_path.is_file() {
        return None;
    }

    let ext = full_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // SVGs are vector and can be rendered directly as preview without raster thumbnail
    if ext == "svg" {
        return Some(asset_url(full_path));
    }

    let cache_dir = ws.cache_dir().join("thumbnails");
    let thumb_path = cache_dir.join(format!("{sha256}.png"));

    if thumb_path.is_file() {
        return Some(asset_url(&thumb_path));
    }

    // Attempt to generate thumbnail with max dimensions 320x240
    let _ = fs::create_dir_all(&cache_dir);
    if let Ok(img) = image::open(full_path) {
        let thumb = img.thumbnail(320, 240);
        if thumb.save(&thumb_path).is_ok() {
            return Some(asset_url(&thumb_path));
        }
    }

    // Fallback to original file url
    Some(asset_url(full_path))
}

/// Normalize path string relative to workspace root with forward slashes.
fn normalize_rel_path(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches('/').to_string()
}

/// Implementation of `asset_list`:
/// Scans `assets_index` in `studio.db` (rebuilding if empty),
/// filters by kind, source, project, brand, search.
pub fn asset_list_impl(state: &AppState, filter: Option<AssetFilter>) -> CmdResult<Vec<Value>> {
    let ws = state.workspace()?;

    // Check if assets_index has any rows; if 0, run rebuild
    {
        let db = WorkspaceDb::open(&ws)?;
        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM assets_index", [], |row| row.get(0))
            .unwrap_or(0);
        if count == 0 {
            let _ = Indexer::rebuild(&ws);
        }
    }

    let db = WorkspaceDb::open(&ws)?;
    let filter = filter.unwrap_or_default();

    let mut sql = "SELECT id, path, name, mime_type, size_bytes, sha256, metadata_json, updated_at, source, project, brand FROM assets_index WHERE 1=1".to_string();
    let mut params_vec: Vec<rusqlite::types::Value> = Vec::new();

    if let Some(ref src) = filter.source {
        if src != "all" && !src.is_empty() {
            sql.push_str(" AND source = ?");
            params_vec.push(rusqlite::types::Value::Text(src.clone()));
        }
    }

    if let Some(ref proj) = filter.project {
        if proj != "all" && !proj.is_empty() {
            sql.push_str(" AND project = ?");
            params_vec.push(rusqlite::types::Value::Text(proj.clone()));
        }
    }

    if let Some(ref br) = filter.brand {
        if br != "all" && !br.is_empty() {
            sql.push_str(" AND brand = ?");
            params_vec.push(rusqlite::types::Value::Text(br.clone()));
        }
    }

    if let Some(ref search) = filter.search {
        let q = search.trim();
        if !q.is_empty() {
            let pattern = format!("%{q}%");
            sql.push_str(" AND (name LIKE ? OR path LIKE ? OR metadata_json LIKE ?)");
            params_vec.push(rusqlite::types::Value::Text(pattern.clone()));
            params_vec.push(rusqlite::types::Value::Text(pattern.clone()));
            params_vec.push(rusqlite::types::Value::Text(pattern));
        }
    }

    sql.push_str(" ORDER BY updated_at DESC");

    let limit = filter.limit.unwrap_or(500);
    sql.push_str(&format!(" LIMIT {limit}"));

    let mut stmt = db.conn().prepare(&sql)?;
    let rusqlite_params = rusqlite::params_from_iter(params_vec.iter());

    let want_kind = filter
        .kind
        .as_deref()
        .filter(|k| *k != "all" && !k.is_empty());

    let rows = stmt.query_map(rusqlite_params, |row| {
        let id: String = row.get(0)?;
        let rel_path: String = row.get(1)?;
        let name: String = row.get(2)?;
        let mime_type: String = row.get(3)?;
        let size_bytes: u64 = row.get(4)?;
        let sha256: String = row.get(5)?;
        let meta_json: String = row.get::<_, Option<String>>(6)?.unwrap_or_default();
        let updated_at: String = row.get(7)?;
        let source: String = row
            .get::<_, Option<String>>(8)?
            .unwrap_or_else(|| "uploaded".into());
        let project: Option<String> = row.get(9)?;
        let brand: Option<String> = row.get(10)?;

        Ok((
            id, rel_path, name, mime_type, size_bytes, sha256, meta_json, updated_at, source,
            project, brand,
        ))
    })?;

    let mut items = Vec::new();

    for row in rows.flatten() {
        let (
            id,
            rel_path,
            name,
            mime_type,
            size_bytes,
            sha256,
            meta_json,
            updated_at,
            source,
            project,
            brand,
        ) = row;

        let full_path = ws.root().join(&rel_path);
        let (kind, calculated_mime) = classify_asset(&full_path);
        let mime = if mime_type.is_empty() || mime_type == "application/octet-stream" {
            calculated_mime.to_string()
        } else {
            mime_type
        };

        if let Some(wk) = want_kind {
            if wk != kind {
                continue;
            }
        }

        let mut sidecar: AssetSidecarMeta = serde_json::from_str(&meta_json).unwrap_or_default();
        if sidecar.prompt.is_none() {
            let sidecar_disk =
                std::path::PathBuf::from(format!("{}.meta.yaml", full_path.display()));
            if sidecar_disk.exists() {
                if let Ok(content) = std::fs::read_to_string(&sidecar_disk) {
                    if let Ok(parsed) = serde_yaml::from_str::<AssetSidecarMeta>(&content) {
                        sidecar = parsed;
                    }
                }
            }
        }

        let preview_url = if full_path.is_file() {
            Some(asset_url(&full_path))
        } else {
            None
        };

        let thumbnail_url = if kind == "image" {
            get_or_generate_thumbnail(&ws, &full_path, &sha256)
        } else if kind == "video" {
            preview_url.clone()
        } else {
            None
        };

        let created_at = sidecar
            .created_at
            .clone()
            .unwrap_or_else(|| updated_at.clone());

        let meta_dto = AssetMetaDto {
            prompt: sidecar.prompt,
            provider: sidecar.provider,
            model: sidecar.model,
            cost_usd: sidecar.cost_usd,
            created_at: Some(created_at.clone()),
            turn_id: sidecar.turn_id,
            description: sidecar.description,
        };

        let dto = AssetItemDto {
            id,
            name,
            kind: kind.to_string(),
            mime,
            size: size_bytes,
            path: normalize_rel_path(&rel_path),
            preview_url,
            thumbnail_url,
            source,
            project,
            brand,
            meta: meta_dto,
            created_at,
            updated_at,
        };

        items.push(serde_json::to_value(dto)?);
    }

    Ok(items)
}

/// Tauri command `asset_list`.
#[tauri::command]
pub fn asset_list(
    state: State<'_, AppState>,
    filter: Option<AssetFilter>,
) -> CmdResult<Vec<Value>> {
    asset_list_impl(&state, filter)
}

/// Implementation of `asset_import`:
/// Validates input files (no UNC, no symlinks/junctions, <= 200MB, allowed types),
/// creates checkpoint before import, copies files to `library/imports/<YYYY-MM-DD>/`,
/// indexes assets and generates cached thumbnails.
pub fn asset_import_impl(state: &AppState, paths: Vec<String>) -> CmdResult<Value> {
    if paths.is_empty() {
        return Err(CmdError::msg("Không có tệp nào được chọn để nhập."));
    }

    let ws = state.workspace()?;

    // 1. Validation pass
    for p_str in &paths {
        // Block UNC paths
        if p_str.starts_with(r"\\")
            || p_str.starts_with("//")
            || p_str.starts_with(r"\\?\UNC\")
            || p_str.to_lowercase().contains(r"\unc\")
        {
            return Err(CmdError::msg(
                "Đường dẫn mạng UNC không được phép vì lý do an toàn.",
            ));
        }

        let p = PathBuf::from(p_str);
        if !p.exists() || !p.is_file() {
            return Err(CmdError::msg(format!("Tệp không tồn tại: {p_str}")));
        }

        // Check symlinks / junctions
        let meta = fs::symlink_metadata(&p)
            .map_err(|e| CmdError::msg(format!("Không thể đọc thông tin tệp {p_str}: {e}")))?;

        if meta.file_type().is_symlink() {
            return Err(CmdError::msg(
                "Không hỗ trợ tệp liên kết symlink hoặc junction.",
            ));
        }

        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
            if (meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT) != 0 {
                return Err(CmdError::msg(
                    "Không hỗ trợ tệp liên kết symlink hoặc junction.",
                ));
            }
        }

        // Check size limit: 200 MB
        const MAX_SIZE: u64 = 200 * 1024 * 1024;
        if meta.len() > MAX_SIZE {
            return Err(CmdError::msg(format!(
                "Tệp \"{}\" vượt quá giới hạn 200 MB (dung lượng: {}).",
                p.file_name().unwrap_or_default().to_string_lossy(),
                human_bytes(meta.len())
            )));
        }

        // Check file extension / supported types
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Disallow dangerous executables / scripts
        match ext.as_str() {
            "exe" | "bat" | "cmd" | "ps1" | "vbs" | "sh" | "bin" | "msi" | "dll" | "scr"
            | "com" => {
                return Err(CmdError::msg(format!(
                    "Loại tệp không an toàn và không được hỗ trợ: .{ext}"
                )));
            }
            _ => {}
        }

        let (kind, _) = classify_asset(&p);
        if kind == "other" && ext != "zip" && ext != "tar" && ext != "gz" {
            return Err(CmdError::msg(format!(
                "Loại tệp không được hỗ trợ: {}.",
                if ext.is_empty() {
                    "không có phần mở rộng"
                } else {
                    &ext
                }
            )));
        }
    }

    // 2. Checkpoint before import
    let cp_label = if paths.len() == 1 {
        let name = Path::new(&paths[0])
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        format!("Trước khi nhập tệp {name}")
    } else {
        format!("Trước khi nhập {} tệp tài nguyên", paths.len())
    };
    let cp_id = CheckpointService::create(&ws, &cp_label, None)?;

    // 3. Copy files to library/imports/<YYYY-MM-DD>/
    let date_str = chrono::Local::now().format("%Y-%m-%d").to_string();
    let dest_dir = ws.root().join("library").join("imports").join(&date_str);
    fs::create_dir_all(&dest_dir)?;

    let mut imported_items = Vec::new();

    for p_str in &paths {
        let src_path = PathBuf::from(p_str);
        let orig_name = src_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let stem = Path::new(&orig_name)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();
        let ext = Path::new(&orig_name)
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();

        let mut target_name = orig_name.clone();
        let mut target_path = dest_dir.join(&target_name);
        let mut counter = 1;

        while target_path.exists() {
            target_name = format!("{stem}_{counter}{ext}");
            target_path = dest_dir.join(&target_name);
            counter += 1;
        }

        // Copy (never move) original file
        fs::copy(&src_path, &target_path)?;

        let rel_path = format!("library/imports/{date_str}/{target_name}");

        // Index in assets_index
        Indexer::index_single_asset(&ws, &rel_path)?;

        // Query newly indexed asset
        let items = asset_list_impl(
            state,
            Some(AssetFilter {
                search: Some(target_name.clone()),
                limit: Some(1),
                ..Default::default()
            }),
        )?;

        if let Some(item) = items.into_iter().next() {
            imported_items.push(item);
        }
    }

    Ok(json!({
        "success": true,
        "checkpointId": cp_id,
        "imported": imported_items,
        "count": imported_items.len(),
    }))
}

/// Tauri command `asset_import`.
#[tauri::command]
pub fn asset_import(state: State<'_, AppState>, paths: Vec<String>) -> CmdResult<Value> {
    asset_import_impl(&state, paths)
}

/// Implementation of `asset_delete`:
/// Validates path is inside workspace (library/ or projects/*/assets/),
/// creates checkpoint before deletion, removes file and sidecar,
/// and updates index.
pub fn asset_delete_impl(state: &AppState, path: String) -> CmdResult<Value> {
    let ws = state.workspace()?;
    let rel_path = normalize_rel_path(&path);

    // Validate path is strictly inside allowed asset directories
    let is_allowed_dir = rel_path.starts_with("library/")
        || (rel_path.starts_with("projects/") && rel_path.contains("/assets/"));
    if !is_allowed_dir {
        return Err(CmdError::msg(
            "Chỉ được xoá tệp tài nguyên bên trong thư mục library/ hoặc projects/*/assets/.",
        ));
    }

    if rel_path.contains("..") || rel_path.starts_with('/') {
        return Err(CmdError::msg(
            "Đường dẫn không hợp lệ hoặc vượt ra ngoài workspace.",
        ));
    }

    let full_path = ws.root().join(&rel_path);
    if !full_path.exists() || !full_path.is_file() {
        return Err(CmdError::msg(format!("Tệp không tồn tại: {rel_path}")));
    }

    let file_name = full_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // Checkpoint before delete
    let cp_id = CheckpointService::create(&ws, &format!("Trước khi xoá asset {file_name}"), None)?;

    // Delete file
    fs::remove_file(&full_path)?;

    // Delete sidecar .meta.yaml if exists
    let sidecar_path = PathBuf::from(format!("{}.meta.yaml", full_path.display()));
    if sidecar_path.is_file() {
        let _ = fs::remove_file(sidecar_path);
    }

    // Remove from index
    Indexer::remove_single_asset(&ws, &rel_path)?;

    Ok(json!({
        "success": true,
        "checkpointId": cp_id,
        "deletedPath": rel_path,
    }))
}

/// Tauri command `asset_delete`.
#[tauri::command]
pub fn asset_delete(state: State<'_, AppState>, path: String) -> CmdResult<Value> {
    asset_delete_impl(&state, path)
}

/// Implementation of `asset_open_folder`:
/// Reveals the folder containing the asset in the system file explorer.
pub fn asset_open_folder_impl(state: &AppState, path: String) -> CmdResult<Value> {
    let ws = state.workspace()?;
    let rel_path = normalize_rel_path(&path);

    if rel_path.contains("..") || rel_path.starts_with('/') {
        return Err(CmdError::msg(
            "Đường dẫn không hợp lệ hoặc vượt ra ngoài workspace.",
        ));
    }

    let full_path = ws.root().join(&rel_path);
    let parent_dir = if full_path.is_file() {
        full_path.parent().unwrap_or(ws.root()).to_path_buf()
    } else if full_path.is_dir() {
        full_path
    } else {
        ws.root().to_path_buf()
    };

    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer")
            .arg(&parent_dir)
            .spawn();
    }

    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("xdg-open")
            .arg(&parent_dir)
            .spawn();
    }

    Ok(json!({ "success": true }))
}

/// Tauri command `asset_open_folder`.
#[tauri::command]
pub fn asset_open_folder(state: State<'_, AppState>, path: String) -> CmdResult<Value> {
    asset_open_folder_impl(&state, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_workspace() -> (tempfile::TempDir, Workspace, AppState) {
        let lock = crate::commands::tests::ENV_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let app_dir = temp.path().join("app_data");
        std::env::set_var("STUDIO_APP_DIR", &app_dir);
        let ws_dir = temp.path().join("test_ws");
        let ws = Workspace::create(&ws_dir, "Test Workspace").unwrap();
        let state = AppState::new();
        *state.current.lock().unwrap() = Some(ws_dir.clone());
        drop(lock);
        (temp, ws, state)
    }

    #[tokio::test]
    async fn test_asset_list_merged_sources_and_meta() {
        let (_temp, ws, state) = setup_test_workspace();

        // 1. Uploaded asset in library/
        let lib_dir = ws.root().join("library");
        fs::create_dir_all(&lib_dir).unwrap();
        let sample_png = lib_dir.join("sample.png");
        let img = image::RgbImage::new(64, 64);
        img.save(&sample_png).unwrap();

        // 2. AI Generated asset in projects/my-proj/assets/generated/2026-10-20/
        let gen_dir = ws
            .root()
            .join("projects")
            .join("my-proj")
            .join("assets")
            .join("generated")
            .join("2026-10-20");
        fs::create_dir_all(&gen_dir).unwrap();
        let gen_png = gen_dir.join("gen.png");
        let img2 = image::RgbImage::new(100, 100);
        img2.save(&gen_png).unwrap();

        // Write sidecar metadata
        let sidecar_yaml = r#"
title: "Hero Art"
provider: "google.imagen"
model: "imagen-3"
prompt: "a majestic golden eagle over mountains"
cost_usd: 0.045
brand: "main"
"#;
        fs::write(gen_dir.join("gen.png.meta.yaml"), sidecar_yaml).unwrap();

        // 3. Rebuild index and test list
        Indexer::rebuild(&ws).unwrap();

        let all_assets = asset_list_impl(&state, None).unwrap();
        assert_eq!(all_assets.len(), 2);

        // Find items
        let sample_item = all_assets
            .iter()
            .find(|v| v["name"] == "sample.png")
            .expect("sample.png must be present");
        assert_eq!(sample_item["source"], "uploaded");
        assert!(sample_item["project"].is_null());
        assert_eq!(sample_item["kind"], "image");

        let gen_item = all_assets
            .iter()
            .find(|v| v["name"] == "Hero Art")
            .expect("gen.png (Hero Art) must be present");
        assert_eq!(gen_item["source"], "generated");
        assert_eq!(gen_item["project"], "my-proj");
        assert_eq!(gen_item["brand"], "main");
        assert_eq!(
            gen_item["meta"]["prompt"],
            "a majestic golden eagle over mountains"
        );
        assert_eq!(gen_item["meta"]["provider"], "google.imagen");
        assert_eq!(gen_item["meta"]["model"], "imagen-3");
        assert_eq!(gen_item["meta"]["costUsd"], 0.045);

        // Filter tests
        let gen_only = asset_list_impl(
            &state,
            Some(AssetFilter {
                source: Some("generated".into()),
                ..Default::default()
            }),
        )
        .unwrap();
        assert_eq!(gen_only.len(), 1);
        assert_eq!(gen_only[0]["name"], "Hero Art");

        let proj_only = asset_list_impl(
            &state,
            Some(AssetFilter {
                project: Some("my-proj".into()),
                ..Default::default()
            }),
        )
        .unwrap();
        assert_eq!(proj_only.len(), 1);
        assert_eq!(proj_only[0]["project"], "my-proj");

        let search_res = asset_list_impl(
            &state,
            Some(AssetFilter {
                search: Some("eagle".into()),
                ..Default::default()
            }),
        )
        .unwrap();
        assert_eq!(search_res.len(), 1);
        assert_eq!(search_res[0]["name"], "Hero Art");
    }

    #[tokio::test]
    async fn test_asset_import_checkpoint_and_undo() {
        let (temp, ws, state) = setup_test_workspace();

        // Create an external image to import
        let external_file = temp.path().join("external_photo.png");
        let img = image::RgbImage::new(120, 120);
        img.save(&external_file).unwrap();

        let ext_str = external_file.to_string_lossy().to_string();

        let cp_count_before = CheckpointService::list(&ws).unwrap().len();

        // Run import
        let res = asset_import_impl(&state, vec![ext_str]).expect("import succeeds");
        assert_eq!(res["success"], true);
        assert_eq!(res["count"], 1);

        let cp_id = res["checkpointId"].as_str().unwrap().to_string();

        // Verify checkpoint was created
        let cp_count_after = CheckpointService::list(&ws).unwrap().len();
        assert!(cp_count_after > cp_count_before);

        // Verify asset exists in library/imports/
        let list_after = asset_list_impl(&state, None).unwrap();
        assert_eq!(list_after.len(), 1);
        assert_eq!(list_after[0]["source"], "imported");
        let imported_path = list_after[0]["path"].as_str().unwrap().to_string();
        assert!(imported_path.starts_with("library/imports/"));

        // Verify physical file exists
        assert!(ws.root().join(&imported_path).is_file());

        // Now test undo by restoring checkpoint
        CheckpointService::restore(&ws, &cp_id).unwrap();

        // Verify imported file was removed on restore
        assert!(!ws.root().join(&imported_path).exists());
    }

    #[tokio::test]
    async fn test_asset_import_validations() {
        let (temp, _ws, state) = setup_test_workspace();

        // 1. Reject UNC path
        let unc_err = asset_import_impl(&state, vec![r"\\server\share\file.png".into()]);
        assert!(unc_err.is_err());
        assert!(unc_err.unwrap_err().to_string().contains("UNC"));

        // 2. Reject dangerous executable
        let dangerous = temp.path().join("script.bat");
        fs::write(&dangerous, "echo hello").unwrap();
        let bat_err = asset_import_impl(&state, vec![dangerous.to_string_lossy().to_string()]);
        assert!(bat_err.is_err());
        assert!(bat_err.unwrap_err().to_string().contains("không an toàn"));

        // 3. Reject non-existent file
        let not_found = temp.path().join("does_not_exist.png");
        let nf_err = asset_import_impl(&state, vec![not_found.to_string_lossy().to_string()]);
        assert!(nf_err.is_err());
        assert!(nf_err.unwrap_err().to_string().contains("không tồn tại"));
    }

    #[tokio::test]
    async fn test_asset_delete_with_checkpoint_and_undo() {
        let (_temp, ws, state) = setup_test_workspace();

        // Create asset in library/
        let lib_dir = ws.root().join("library");
        fs::create_dir_all(&lib_dir).unwrap();
        let test_file = lib_dir.join("to_delete.png");
        let img = image::RgbImage::new(50, 50);
        img.save(&test_file).unwrap();
        let sidecar_file = lib_dir.join("to_delete.png.meta.yaml");
        fs::write(&sidecar_file, "title: To Delete").unwrap();

        Indexer::index_single_asset(&ws, "library/to_delete.png").unwrap();

        let list_before = asset_list_impl(&state, None).unwrap();
        assert_eq!(list_before.len(), 1);

        // Delete asset
        let del_res =
            asset_delete_impl(&state, "library/to_delete.png".into()).expect("delete succeeds");
        assert_eq!(del_res["success"], true);
        let cp_id = del_res["checkpointId"].as_str().unwrap().to_string();

        // Verify physical file and sidecar are gone
        assert!(!test_file.exists());
        assert!(!sidecar_file.exists());

        // Verify removed from list
        let list_deleted = asset_list_impl(&state, None).unwrap();
        assert_eq!(list_deleted.len(), 0);

        // Restore checkpoint to undo deletion
        CheckpointService::restore(&ws, &cp_id).unwrap();

        // Verify file is restored
        assert!(test_file.is_file());
    }

    #[tokio::test]
    async fn test_asset_delete_rejects_outside_path() {
        let (_temp, _ws, state) = setup_test_workspace();

        // Attempt to delete path outside library/ or projects/*/assets/
        let err = asset_delete_impl(&state, "studio.yaml".into());
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("Chỉ được xoá"));

        let escape_err = asset_delete_impl(&state, "library/../../escape.txt".into());
        assert!(escape_err.is_err());
    }
}
