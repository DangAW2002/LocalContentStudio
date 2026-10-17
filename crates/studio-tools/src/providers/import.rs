//! Media importer provider: yt-dlp binary probe/execution and local file import.

use crate::error::{Result, ToolError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;
use studio_workspace::Workspace;

/// Maximum duration allowed for download in seconds (10 minutes = 600s).
pub const MAX_IMPORT_DURATION_SECS: f64 = 600.0;

/// Maximum file size allowed for import in bytes (200 MB).
pub const MAX_IMPORT_FILE_SIZE: u64 = 200 * 1024 * 1024;

/// Output metadata parsed from yt-dlp -J or extracted from local files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportMetadata {
    pub files: Vec<String>,
    pub title: Option<String>,
    pub duration: Option<f64>,
    pub source_url: Option<String>,
    pub uploader: Option<String>,
    pub license: Option<String>,
}

/// Parsed subset of `yt-dlp -J` output.
#[derive(Debug, Clone, Deserialize)]
pub struct YtDlpJson {
    pub title: Option<String>,
    pub duration: Option<f64>,
    pub uploader: Option<String>,
    pub license: Option<String>,
    #[serde(rename = "webpage_url")]
    pub webpage_url: Option<String>,
    pub ext: Option<String>,
    pub filesize: Option<u64>,
    pub filesize_approx: Option<u64>,
}

/// Parse metadata from raw `yt-dlp -J` JSON output.
pub fn parse_ytdlp_json(json_str: &str) -> Result<YtDlpJson> {
    serde_json::from_str::<YtDlpJson>(json_str)
        .map_err(|e| ToolError::ParseError(format!("Không thể đọc JSON từ yt-dlp: {e}")))
}

/// Validate that a URL starts with http:// or https:// and contains no shell characters.
pub fn validate_import_url(url: &str) -> Result<()> {
    let lower = url.trim().to_ascii_lowercase();
    if !lower.starts_with("http://") && !lower.starts_with("https://") {
        return Err(ToolError::ExecutionFailed {
            tool_id: "media.import.yt-dlp".into(),
            reason: "Chỉ hỗ trợ URL giao thức HTTP hoặc HTTPS".into(),
        });
    }

    // Check for private / loopback hosts if needed, and reject shell control chars
    for c in url.chars() {
        if matches!(c, ';' | '&' | '|' | '`' | '$' | '\n' | '\r' | '>' | '<') {
            return Err(ToolError::ExecutionFailed {
                tool_id: "media.import.yt-dlp".into(),
                reason: "URL chứa ký tự không hợp lệ hoặc tiềm ẩn nguy cơ bảo mật".into(),
            });
        }
    }

    Ok(())
}

/// Check if yt-dlp executable is present in PATH and probe its version.
pub fn probe_ytdlp() -> Result<String> {
    let exe = crate::probe::find_in_path("yt-dlp").ok_or_else(|| ToolError::MissingDependency {
        tool_id: "media.import.yt-dlp".into(),
        missing: vec!["bin:yt-dlp (Chưa cài đặt yt-dlp. Vui lòng cài đặt qua `winget install yt-dlp` hoặc thêm yt-dlp vào biến môi trường PATH)".into()],
    })?;

    let mut cmd = Command::new(&exe);
    cmd.arg("--version");
    match cmd.output() {
        Ok(out) if out.status.success() => {
            let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
            Ok(ver)
        }
        _ => Err(ToolError::ExecutionFailed {
            tool_id: "media.import.yt-dlp".into(),
            reason:
                "Không thể thực thi lệnh 'yt-dlp --version'. Vui lòng kiểm tra lại cài đặt yt-dlp."
                    .into(),
        }),
    }
}

/// Run media importer in-process.
pub async fn run_media_import(ws: &Workspace, out_dir: &Path, input: &Value) -> Result<Value> {
    let url_opt = input.get("url").and_then(Value::as_str);
    let path_opt = input.get("path").and_then(Value::as_str);

    if url_opt.is_none() && path_opt.is_none() {
        return Err(ToolError::ParseError(
            "Yêu cầu cung cấp ít nhất một tham số 'url' hoặc 'path'".into(),
        ));
    }

    if let Some(path_str) = path_opt {
        return import_local_file(ws, path_str, out_dir).await;
    }

    if let Some(url_str) = url_opt {
        return import_url_ytdlp(url_str, out_dir).await;
    }

    Err(ToolError::ParseError("Không có nguồn media để nhập".into()))
}

/// Import a local file by copying it into the turn output directory.
///
/// The path must be workspace-relative and stay inside the workspace (no absolute paths, `..`,
/// or junction escapes), so an agent cannot pull arbitrary user files into its context.
async fn import_local_file(ws: &Workspace, path_str: &str, out_dir: &Path) -> Result<Value> {
    let src_path: PathBuf = ws
        .scoped_path(path_str)
        .map_err(|e| ToolError::ExecutionFailed {
            tool_id: "media.import.yt-dlp".into(),
            reason: format!(
                "Chỉ nhập được tệp nằm trong workspace (đường dẫn tương đối): {path_str} ({e})"
            ),
        })?;
    if !src_path.is_file() {
        return Err(ToolError::ExecutionFailed {
            tool_id: "media.import.yt-dlp".into(),
            reason: format!("Tệp cục bộ không tồn tại: {path_str}"),
        });
    }

    let meta = std::fs::metadata(&src_path).map_err(|e| ToolError::Io {
        path: src_path.clone(),
        source: e,
    })?;

    if meta.len() > MAX_IMPORT_FILE_SIZE {
        return Err(ToolError::ExecutionFailed {
            tool_id: "media.import.yt-dlp".into(),
            reason: format!(
                "Kích thước tệp ({:.1} MB) vượt quá giới hạn cho phép ({:.1} MB)",
                meta.len() as f64 / (1024.0 * 1024.0),
                MAX_IMPORT_FILE_SIZE as f64 / (1024.0 * 1024.0)
            ),
        });
    }

    let file_name = src_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "imported_media".to_string());

    let dest_path = out_dir.join(&file_name);
    std::fs::copy(&src_path, &dest_path).map_err(|e| ToolError::Io {
        path: dest_path.clone(),
        source: e,
    })?;

    let title = src_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| file_name.clone());

    Ok(json!({
        "files": [dest_path.to_string_lossy().to_string()],
        "title": title,
        "duration": Value::Null,
        "source_url": path_str,
        "uploader": Value::Null,
        "license": Value::Null
    }))
}

/// Download media from URL using yt-dlp CLI binary with fixed, safe argument list.
async fn import_url_ytdlp(url_str: &str, out_dir: &Path) -> Result<Value> {
    validate_import_url(url_str)?;

    let exe = crate::probe::find_in_path("yt-dlp").ok_or_else(|| ToolError::MissingDependency {
        tool_id: "media.import.yt-dlp".into(),
        missing: vec!["bin:yt-dlp (Chưa cài đặt yt-dlp. Vui lòng cài đặt qua `winget install yt-dlp` hoặc thêm yt-dlp vào PATH)".into()],
    })?;

    // 1. Fetch metadata first using `yt-dlp -J --no-playlist <url>`
    let mut info_cmd = Command::new(&exe);
    info_cmd
        .arg("--ignore-config")
        .arg("--socket-timeout")
        .arg("30")
        .arg("-J")
        .arg("--no-playlist")
        .arg("--no-warnings")
        .arg("--")
        .arg(url_str);

    let info_out = info_cmd.output().map_err(|e| ToolError::Io {
        path: exe.clone(),
        source: e,
    })?;

    if !info_out.status.success() {
        let err_msg = String::from_utf8_lossy(&info_out.stderr);
        return Err(ToolError::ExecutionFailed {
            tool_id: "media.import.yt-dlp".into(),
            reason: format!("yt-dlp không thể lấy thông tin URL: {err_msg}"),
        });
    }

    let info_str = String::from_utf8_lossy(&info_out.stdout);
    let parsed = parse_ytdlp_json(&info_str)?;

    // Enforce duration limits
    if let Some(dur) = parsed.duration {
        if dur > MAX_IMPORT_DURATION_SECS {
            return Err(ToolError::ExecutionFailed {
                tool_id: "media.import.yt-dlp".into(),
                reason: format!(
                    "Thời lượng video ({:.0}s) vượt quá giới hạn tối đa cho phép ({:.0}s)",
                    dur, MAX_IMPORT_DURATION_SECS
                ),
            });
        }
    }

    // Enforce size limits if known
    let sz = parsed.filesize.or(parsed.filesize_approx);
    if let Some(bytes) = sz {
        if bytes > MAX_IMPORT_FILE_SIZE {
            return Err(ToolError::ExecutionFailed {
                tool_id: "media.import.yt-dlp".into(),
                reason: format!(
                    "Kích thước media ước tính ({:.1} MB) vượt quá giới hạn ({:.1} MB)",
                    bytes as f64 / (1024.0 * 1024.0),
                    MAX_IMPORT_FILE_SIZE as f64 / (1024.0 * 1024.0)
                ),
            });
        }
    }

    // 2. Download media into out_dir with template `%(id)s.%(ext)s`
    let out_tmpl = out_dir.join("%(id)s.%(ext)s");
    let mut dl_cmd = Command::new(&exe);
    dl_cmd
        .arg("--ignore-config")
        .arg("--socket-timeout")
        .arg("30")
        .arg("--no-playlist")
        .arg("--max-filesize")
        .arg("200M")
        .arg("-o")
        .arg(&out_tmpl)
        .arg("--")
        .arg(url_str);

    let dl_out = dl_cmd.output().map_err(|e| ToolError::Io {
        path: exe.clone(),
        source: e,
    })?;

    if !dl_out.status.success() {
        let err_msg = String::from_utf8_lossy(&dl_out.stderr);
        return Err(ToolError::ExecutionFailed {
            tool_id: "media.import.yt-dlp".into(),
            reason: format!("Lỗi khi tải media qua yt-dlp: {err_msg}"),
        });
    }

    // Find downloaded file(s) in out_dir
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(out_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                let p_str = p.to_string_lossy().to_string();
                if !p_str.ends_with(".json") {
                    files.push(p_str);
                }
            }
        }
    }

    Ok(json!({
        "files": files,
        "title": parsed.title.unwrap_or_else(|| "Tải về".to_string()),
        "duration": parsed.duration,
        "source_url": parsed.webpage_url.unwrap_or_else(|| url_str.to_string()),
        "uploader": parsed.uploader,
        "license": parsed.license
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_YT_DLP_JSON: &str = r#"{
        "id": "dQw4w9WgXcQ",
        "title": "Rick Astley - Never Gonna Give You Up (Official Music Video)",
        "duration": 213.0,
        "uploader": "Rick Astley",
        "license": "Standard YouTube License",
        "webpage_url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
        "ext": "mp4",
        "filesize_approx": 15420000
    }"#;

    #[test]
    fn test_parse_ytdlp_json_fixture() {
        let parsed = parse_ytdlp_json(SAMPLE_YT_DLP_JSON).expect("should parse fixture");
        assert_eq!(
            parsed.title.as_deref(),
            Some("Rick Astley - Never Gonna Give You Up (Official Music Video)")
        );
        assert_eq!(parsed.duration, Some(213.0));
        assert_eq!(parsed.uploader.as_deref(), Some("Rick Astley"));
        assert_eq!(parsed.license.as_deref(), Some("Standard YouTube License"));
        assert_eq!(parsed.filesize_approx, Some(15420000));
    }

    #[test]
    fn test_validate_import_url() {
        assert!(validate_import_url("https://www.youtube.com/watch?v=123").is_ok());
        assert!(validate_import_url("http://example.com/video.mp4").is_ok());
        assert!(validate_import_url("ftp://example.com/video.mp4").is_err());
        assert!(validate_import_url("https://example.com/video;rm -rf /").is_err());
        assert!(validate_import_url("https://example.com/`echo hi`").is_err());
    }

    #[tokio::test]
    async fn test_import_local_file_success() {
        let temp_dir = tempfile::tempdir().unwrap();
        let ws_dir = temp_dir.path().join("ws");
        let ws = studio_workspace::Workspace::create(&ws_dir, "test_ws").unwrap();
        std::fs::create_dir_all(ws.root().join("library")).unwrap();
        std::fs::write(
            ws.root().join("library/test_clip.mp4"),
            b"fake video content",
        )
        .unwrap();
        // Outside the workspace: must be refused.
        let outside = temp_dir.path().join("secret.mp4");
        std::fs::write(&outside, b"x").unwrap();

        let out_dir = temp_dir.path().join("output");
        std::fs::create_dir_all(&out_dir).unwrap();

        let refused =
            run_media_import(&ws, &out_dir, &json!({ "path": outside.to_str().unwrap() })).await;
        assert!(refused.is_err());
        let refused = run_media_import(&ws, &out_dir, &json!({ "path": "../secret.mp4" })).await;
        assert!(refused.is_err());

        let input = json!({ "path": "library/test_clip.mp4" });
        let result = run_media_import(&ws, &out_dir, &input).await.unwrap();
        let files = result.get("files").unwrap().as_array().unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(result.get("title").unwrap().as_str().unwrap(), "test_clip");
    }

    #[test]
    #[ignore = "requires real yt-dlp binary installed in system"]
    fn test_real_probe_ytdlp() {
        let res = probe_ytdlp();
        println!("yt-dlp probe result: {:?}", res);
    }
}
