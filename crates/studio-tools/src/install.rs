//! Tool installation, preview, and uninstallation routines.
//!
//! Supports installing tools from:
//! 1. Local folder containing `tool.yaml` (and optionally capabilities)
//! 2. HTTPS URL to a `.zip` file (max 100MB, no zip-slip/symlinks/absolute entries)
//!
//! Enforces:
//! - Manifest validation before installation
//! - Scope targeting: App (`%APPDATA%/LocalContentStudio/tools/<id>@<version>`) or Workspace (`<ws>/.studio/tools/<id>@<version>`)
//! - Never executes anything during installation; runs only declared probe after installation.

use crate::error::{Result, ToolError};
use crate::manifest::ToolManifest;
use crate::probe::{probe, ProbeResult};
use crate::registry::Registry;
use crate::secret::SecretStore;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use studio_workspace::{AppPaths, Workspace};
use url::Url;

/// Scope for tool installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolInstallScope {
    /// App level: `%APPDATA%/LocalContentStudio/tools/<id>@<version>`
    App,
    /// Workspace level: `<ws>/.studio/tools/<id>@<version>`
    Workspace,
}

impl ToolInstallScope {
    /// User-facing Vietnamese translation
    pub fn to_vietnamese(&self) -> &'static str {
        match self {
            Self::App => "Toàn ứng dụng (App)",
            Self::Workspace => "Không gian làm việc (Workspace)",
        }
    }
}

/// Installation preview details returned before user confirmation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInstallPreview {
    /// Unique preview identifier (UUID)
    pub preview_id: String,
    /// Tool ID
    pub tool_id: String,
    /// Tool version
    pub version: String,
    /// Tool kind / category
    pub kind: String,
    /// Tool description
    pub description: String,
    /// Command that will be executed
    pub command: String,
    /// Arguments that will be passed
    pub args: Vec<String>,
    /// Capabilities provided by the tool
    pub provides: Vec<String>,
    /// Dependencies and required secrets
    pub needs: Vec<String>,
    /// Network requirement explanation
    pub network: Option<String>,
    /// Side effect level in Vietnamese
    pub side_effect: String,
    /// Target installation scope
    pub scope: ToolInstallScope,
    /// Target folder path
    pub target_dir: String,
    /// Staged directory containing validated tool files
    pub staged_dir: String,
    /// Whether an existing installation with the same id and version will be replaced
    pub is_reinstall: bool,
}

/// Information about an installed tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledToolInfo {
    /// Tool identifier
    pub id: String,
    /// Tool version
    pub version: String,
    /// Scope: app or workspace (or builtin)
    pub scope: String,
    /// Absolute directory of the tool
    pub path: String,
    /// Manifest details
    pub manifest: ToolManifest,
    /// Current probe result
    pub probe: ProbeResult,
    /// Whether this tool can be uninstalled (builtin tools cannot be uninstalled)
    pub can_uninstall: bool,
}

/// Maximum allowed download size for zip files (100 MB).
pub const MAX_ZIP_BYTES: usize = 100 * 1024 * 1024;

/// A tool id / version usable as one folder-name segment (no separators, no `..`).
fn is_safe_segment(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && !s.starts_with('.')
        && !s.contains("..")
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// Create an installation preview by inspecting a local directory or HTTPS zip URL.
pub async fn preview_install(
    source: &str,
    scope: ToolInstallScope,
    ws: Option<&Workspace>,
    app_paths: &AppPaths,
) -> Result<ToolInstallPreview> {
    let temp_root = tempfile::Builder::new()
        .prefix("studio-tool-stage-")
        .tempdir()
        .map_err(|e| ToolError::Io {
            path: PathBuf::from("tempdir"),
            source: e,
        })?;
    let staged_dir = temp_root.keep();

    let source_trimmed = source.trim();
    if source_trimmed.starts_with("https://") {
        download_and_extract_zip(source_trimmed, &staged_dir).await?;
    } else if source_trimmed.starts_with("http://") {
        return Err(ToolError::InstallError(
            "Chỉ hỗ trợ tải công cụ qua giao thức HTTPS bảo mật (https://)".to_string(),
        ));
    } else {
        // Local directory
        let src_path = Path::new(source_trimmed);
        if !src_path.exists() {
            return Err(ToolError::InstallError(format!(
                "Thư mục nguồn không tồn tại: {}",
                src_path.display()
            )));
        }
        if !src_path.is_dir() {
            return Err(ToolError::InstallError(format!(
                "Đường dẫn nguồn phải là thư mục: {}",
                src_path.display()
            )));
        }
        copy_dir_recursive(src_path, &staged_dir)?;
    }

    // Locate tool manifest in staged dir
    let manifest_path = find_manifest_in_dir(&staged_dir)?;
    let content = fs::read_to_string(&manifest_path).map_err(|e| ToolError::Io {
        path: manifest_path.clone(),
        source: e,
    })?;

    let manifest: ToolManifest = serde_yaml::from_str(&content).map_err(|e| {
        ToolError::InstallError(format!("Tệp cấu hình tool.yaml không hợp lệ: {e}"))
    })?;

    let version = manifest
        .version
        .clone()
        .unwrap_or_else(|| "1.0.0".to_string());
    if !is_safe_segment(&manifest.id) || !is_safe_segment(&version) {
        return Err(ToolError::InstallError(format!(
            "id/version của công cụ không hợp lệ (chỉ cho phép chữ, số, '.', '-', '_'): {}@{}",
            manifest.id, version
        )));
    }
    let folder_name = format!("{}@{}", manifest.id, version);

    // Calculate target directory
    let target_dir = match scope {
        ToolInstallScope::App => app_paths.data_dir().join("tools").join(&folder_name),
        ToolInstallScope::Workspace => {
            let ws_ref = ws.ok_or_else(|| {
                ToolError::InstallError(
                    "Cần mở một không gian làm việc để cài đặt công cụ vào cấp Workspace"
                        .to_string(),
                )
            })?;
            ws_ref
                .root()
                .join(".studio")
                .join("tools")
                .join(&folder_name)
        }
    };

    let is_reinstall = target_dir.exists();

    // Check network declarations
    let network_needed = manifest
        .needs
        .iter()
        .any(|n| n.starts_with("network") || n.contains("http") || n.contains("api"));
    let network_summary = if network_needed {
        Some("Yêu cầu kết nối mạng / Internet".to_string())
    } else {
        None
    };

    let preview_id = uuid::Uuid::new_v4().to_string();

    Ok(ToolInstallPreview {
        preview_id,
        tool_id: manifest.id,
        version,
        kind: manifest.kind.to_vietnamese().to_string(),
        description: manifest.description,
        command: manifest.run.command,
        args: manifest.run.args,
        provides: manifest.provides,
        needs: manifest.needs,
        network: network_summary,
        side_effect: manifest.side_effect.to_vietnamese().to_string(),
        scope,
        target_dir: target_dir.to_string_lossy().to_string(),
        staged_dir: staged_dir.to_string_lossy().to_string(),
        is_reinstall,
    })
}

/// Confirm and finalize the installation of a staged tool preview.
pub fn confirm_install(
    preview: &ToolInstallPreview,
    secrets: &dyn SecretStore,
    ws: Option<&Workspace>,
) -> Result<InstalledToolInfo> {
    let staged_path = PathBuf::from(&preview.staged_dir);
    if !staged_path.exists() {
        return Err(ToolError::InstallError(
            "Phiên xem trước cài đặt đã hết hạn hoặc tệp tạm không còn tồn tại".to_string(),
        ));
    }

    let target_path = PathBuf::from(&preview.target_dir);

    // If target exists, remove it first (clean reinstall)
    if target_path.exists() {
        fs::remove_dir_all(&target_path).map_err(|e| ToolError::Io {
            path: target_path.clone(),
            source: e,
        })?;
    }

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(|e| ToolError::Io {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }

    // Try renaming staged_dir to target_dir. If on different volumes, copy & remove.
    if fs::rename(&staged_path, &target_path).is_err() {
        copy_dir_recursive(&staged_path, &target_path)?;
        let _ = fs::remove_dir_all(&staged_path);
    }

    // Load installed manifest
    let manifest_path = find_manifest_in_dir(&target_path)?;
    let content = fs::read_to_string(&manifest_path).map_err(|e| ToolError::Io {
        path: manifest_path.clone(),
        source: e,
    })?;
    let manifest: ToolManifest = serde_yaml::from_str(&content).map_err(|e| {
        ToolError::InstallError(format!("Lỗi phân tích manifest sau khi cài đặt: {e}"))
    })?;

    // Run declared probe ONLY - never execute arbitrary commands during install
    let probe_res = probe(&manifest, secrets, ws);

    Ok(InstalledToolInfo {
        id: manifest.id.clone(),
        version: preview.version.clone(),
        scope: preview.scope.to_vietnamese().to_string(),
        path: target_path.to_string_lossy().to_string(),
        manifest,
        probe: probe_res,
        can_uninstall: true,
    })
}

/// Uninstall an installed tool given its ID and optional version.
pub fn uninstall_tool(
    tool_id: &str,
    version: Option<&str>,
    scope: Option<ToolInstallScope>,
    ws: Option<&Workspace>,
    app_paths: &AppPaths,
) -> Result<bool> {
    let mut deleted_any = false;

    // Check App scope
    if scope.is_none() || scope == Some(ToolInstallScope::App) {
        let app_tools = app_paths.data_dir().join("tools");
        deleted_any |= remove_tool_from_dir(&app_tools, tool_id, version)?;
    }

    // Check Workspace scope
    if scope.is_none() || scope == Some(ToolInstallScope::Workspace) {
        if let Some(ws_ref) = ws {
            let ws_tools = ws_ref.root().join(".studio").join("tools");
            deleted_any |= remove_tool_from_dir(&ws_tools, tool_id, version)?;
        }
    }

    if !deleted_any {
        return Err(ToolError::InstallError(format!(
            "Không tìm thấy công cụ đã cài đặt '{}' để gỡ bỏ",
            tool_id
        )));
    }

    Ok(true)
}

/// List all tools (built-in and installed) with their probe results and scope.
pub fn list_tools_with_probe(
    registry: &Registry,
    secrets: &dyn SecretStore,
    ws: Option<&Workspace>,
    app_paths: &AppPaths,
) -> Vec<InstalledToolInfo> {
    let mut list = Vec::new();

    // 1. Scan app installed tools: <app>/tools/<id>@<version>
    let app_tools_dir = app_paths.data_dir().join("tools");
    scan_installed_dir(&app_tools_dir, "App", secrets, ws, &mut list);

    // 2. Scan workspace installed tools: <ws>/.studio/tools/<id>@<version>
    if let Some(w) = ws {
        let ws_tools_dir = w.root().join(".studio").join("tools");
        scan_installed_dir(&ws_tools_dir, "Workspace", secrets, ws, &mut list);
    }

    // 3. Builtin tools and other registry tools
    for tool in registry.list_tools() {
        // If already in list as installed tool, skip duplicate
        if list.iter().any(|item| item.id == tool.id) {
            continue;
        }
        let probe_res = probe(tool, secrets, ws);
        list.push(InstalledToolInfo {
            id: tool.id.clone(),
            version: tool
                .version
                .clone()
                .unwrap_or_else(|| "builtin".to_string()),
            scope: "Có sẵn (Built-in)".to_string(),
            path: "builtin".to_string(),
            manifest: tool.clone(),
            probe: probe_res,
            can_uninstall: false,
        });
    }

    list.sort_by(|a, b| a.id.cmp(&b.id));
    list
}

fn scan_installed_dir(
    dir: &Path,
    scope_label: &str,
    secrets: &dyn SecretStore,
    ws: Option<&Workspace>,
    out: &mut Vec<InstalledToolInfo>,
) {
    if !dir.exists() || !dir.is_dir() {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        if let Ok(manifest_path) = find_manifest_in_dir(&p) {
            if let Ok(content) = fs::read_to_string(&manifest_path) {
                if let Ok(manifest) = serde_yaml::from_str::<ToolManifest>(&content) {
                    let probe_res = probe(&manifest, secrets, ws);
                    let folder_name = p.file_name().unwrap_or_default().to_string_lossy();
                    let version = manifest
                        .version
                        .clone()
                        .or_else(|| folder_name.split_once('@').map(|(_, v)| v.to_string()))
                        .unwrap_or_else(|| "1.0.0".to_string());

                    out.push(InstalledToolInfo {
                        id: manifest.id.clone(),
                        version,
                        scope: scope_label.to_string(),
                        path: p.to_string_lossy().to_string(),
                        manifest,
                        probe: probe_res,
                        can_uninstall: true,
                    });
                }
            }
        }
    }
}

fn remove_tool_from_dir(dir: &Path, tool_id: &str, version: Option<&str>) -> Result<bool> {
    if !dir.exists() || !dir.is_dir() {
        return Ok(false);
    }
    let mut removed = false;
    let entries = fs::read_dir(dir).map_err(|e| ToolError::Io {
        path: dir.to_path_buf(),
        source: e,
    })?;

    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        let folder_name = p.file_name().unwrap_or_default().to_string_lossy();
        let matches = match version {
            Some(v) => folder_name == format!("{tool_id}@{v}"),
            None => folder_name == tool_id || folder_name.starts_with(&format!("{tool_id}@")),
        };

        if matches {
            fs::remove_dir_all(&p).map_err(|e| ToolError::Io {
                path: p.clone(),
                source: e,
            })?;
            removed = true;
        }
    }

    Ok(removed)
}

fn find_manifest_in_dir(dir: &Path) -> Result<PathBuf> {
    let yml = dir.join("tool.yaml");
    if yml.exists() {
        return Ok(yml);
    }
    let yaml = dir.join("tool.yml");
    if yaml.exists() {
        return Ok(yaml);
    }
    // Check if there is a single subdirectory containing tool.yaml (common in zip downloads)
    if let Ok(entries) = fs::read_dir(dir) {
        let subdirs: Vec<_> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        if subdirs.len() == 1 {
            let nested_yml = subdirs[0].join("tool.yaml");
            if nested_yml.exists() {
                return Ok(nested_yml);
            }
            let nested_yaml = subdirs[0].join("tool.yml");
            if nested_yaml.exists() {
                return Ok(nested_yaml);
            }
        }
    }

    Err(ToolError::InstallError(
        "Không tìm thấy tệp cấu hình tool.yaml trong gói công cụ".to_string(),
    ))
}

async fn download_and_extract_zip(url_str: &str, target_dir: &Path) -> Result<()> {
    let parsed_url = Url::parse(url_str)
        .map_err(|e| ToolError::InstallError(format!("Địa chỉ URL không hợp lệ: {e}")))?;

    if parsed_url.scheme() != "https" {
        return Err(ToolError::InstallError(
            "Chỉ cho phép tải công cụ qua giao thức HTTPS (https://)".to_string(),
        ));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| ToolError::InstallError(format!("Không thể tạo HTTP client: {e}")))?;

    let resp = client
        .get(parsed_url.clone())
        .send()
        .await
        .map_err(|e| ToolError::InstallError(format!("Lỗi khi tải tệp từ {url_str}: {e}")))?;

    if !resp.status().is_success() {
        return Err(ToolError::InstallError(format!(
            "Máy chủ trả về mã lỗi HTTP {}: {}",
            resp.status(),
            url_str
        )));
    }

    // Check content length if available
    if let Some(len) = resp.content_length() {
        if len as usize > MAX_ZIP_BYTES {
            return Err(ToolError::InstallError(format!(
                "Kích thước tệp ({len} bytes) vượt quá giới hạn tối đa 100 MB",
            )));
        }
    }

    // Stream with a hard cap: Content-Length can be missing or wrong.
    let mut resp = resp;
    let mut bytes: Vec<u8> = Vec::new();
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| ToolError::InstallError(format!("Lỗi khi nhận dữ liệu tệp zip: {e}")))?
    {
        if bytes.len() + chunk.len() > MAX_ZIP_BYTES {
            return Err(ToolError::InstallError(
                "Kích thước tệp tải về vượt quá giới hạn tối đa 100 MB".to_string(),
            ));
        }
        bytes.extend_from_slice(&chunk);
    }

    extract_zip_safely(&bytes, target_dir)
}

/// Extract zip bytes safely with protection against zip-slip, symlinks, absolute paths.
pub fn extract_zip_safely(bytes: &[u8], target_dir: &Path) -> Result<()> {
    let reader = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| ToolError::InstallError(format!("Tệp nén zip không hợp lệ: {e}")))?;

    let target_dir = target_dir
        .canonicalize()
        .unwrap_or_else(|_| target_dir.to_path_buf());

    // Cumulative across entries (zip bomb guard).
    let mut total_read = 0usize;
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| ToolError::InstallError(format!("Lỗi đọc mục trong zip: {e}")))?;

        // 1. Refuse symlinks (on every platform: the archive may come from a Unix machine)
        if let Some(mode) = file.unix_mode() {
            if (mode & 0o170000) == 0o120000 {
                return Err(ToolError::InstallError(
                    "Từ chối giải nén: phát hiện symlink nguy hiểm trong tệp zip".to_string(),
                ));
            }
        }

        let raw_name = file.name();

        // 2. Refuse absolute paths or path traversal (zip-slip)
        let entry_path = Path::new(raw_name);
        for comp in entry_path.components() {
            match comp {
                Component::Prefix(_) | Component::RootDir => {
                    return Err(ToolError::InstallError(format!(
                        "Từ chối giải nén: phát hiện đường dẫn tuyệt đối trong zip: {raw_name}"
                    )));
                }
                Component::ParentDir => {
                    return Err(ToolError::InstallError(format!(
                        "Từ chối giải nén: phát hiện tấn công zip-slip (..): {raw_name}"
                    )));
                }
                _ => {}
            }
        }

        let outpath = target_dir.join(entry_path);

        // Additional zip-slip check after resolve
        if !outpath.starts_with(&target_dir) && !target_dir.starts_with(&outpath) {
            return Err(ToolError::InstallError(format!(
                "Từ chối giải nén: đường dẫn tệp nằm ngoài thư mục đích: {raw_name}"
            )));
        }

        if file.name().ends_with('/') || file.name().ends_with('\\') {
            fs::create_dir_all(&outpath).map_err(|e| ToolError::Io {
                path: outpath.clone(),
                source: e,
            })?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).map_err(|e| ToolError::Io {
                        path: p.to_path_buf(),
                        source: e,
                    })?;
                }
            }
            let mut outfile = fs::File::create(&outpath).map_err(|e| ToolError::Io {
                path: outpath.clone(),
                source: e,
            })?;
            let mut buffer = [0u8; 8192];
            loop {
                let n = file.read(&mut buffer).map_err(|e| ToolError::Io {
                    path: outpath.clone(),
                    source: e,
                })?;
                if n == 0 {
                    break;
                }
                total_read += n;
                if total_read > MAX_ZIP_BYTES {
                    return Err(ToolError::InstallError(
                        "Tổng dung lượng giải nén vượt quá 100 MB".to_string(),
                    ));
                }
                use std::io::Write;
                outfile.write_all(&buffer[..n]).map_err(|e| ToolError::Io {
                    path: outpath.clone(),
                    source: e,
                })?;
            }
        }
    }

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).map_err(|e| ToolError::Io {
        path: dst.to_path_buf(),
        source: e,
    })?;

    for entry in walkdir::WalkDir::new(src) {
        let entry = entry.map_err(|e| ToolError::Io {
            path: src.to_path_buf(),
            source: std::io::Error::other(e.to_string()),
        })?;
        let rel = entry
            .path()
            .strip_prefix(src)
            .map_err(|e| ToolError::InstallError(format!("Lỗi tính đường dẫn tương đối: {e}")))?;
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).map_err(|e| ToolError::Io {
                path: target.clone(),
                source: e,
            })?;
        } else if entry.file_type().is_file() {
            if let Some(p) = target.parent() {
                fs::create_dir_all(p).map_err(|e| ToolError::Io {
                    path: p.to_path_buf(),
                    source: e,
                })?;
            }
            fs::copy(entry.path(), &target).map_err(|e| ToolError::Io {
                path: target.clone(),
                source: e,
            })?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secret::MemoryStore;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn make_test_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            for (name, content) in entries {
                let options = SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated);
                writer.start_file(*name, options).unwrap();
                writer.write_all(content).unwrap();
            }
            writer.finish().unwrap();
        }
        buf
    }

    #[test]
    fn test_valid_manifest_install_preview_and_confirm() {
        let temp = tempfile::tempdir().unwrap();
        let tool_dir = temp.path().join("my-test-tool");
        fs::create_dir_all(&tool_dir).unwrap();

        let tool_yaml = r#"
id: test-tool
version: "1.2.3"
kind: generator
description: "Công cụ kiểm thử"
provides:
  - "echo@1"
run:
  type: cli
  command: "powershell"
  args: ["-NoProfile", "-Command", "echo 1"]
needs:
  - "bin:powershell"
side_effect: none
"#;
        fs::write(tool_dir.join("tool.yaml"), tool_yaml).unwrap();

        let app_paths = AppPaths::new(temp.path().join("app-data"));

        let rt = tokio::runtime::Runtime::new().unwrap();
        let preview = rt
            .block_on(preview_install(
                tool_dir.to_str().unwrap(),
                ToolInstallScope::App,
                None,
                &app_paths,
            ))
            .unwrap();

        assert_eq!(preview.tool_id, "test-tool");
        assert_eq!(preview.version, "1.2.3");
        assert_eq!(preview.command, "powershell");
        assert!(!preview.is_reinstall);

        let secrets = MemoryStore::new();
        let installed = confirm_install(&preview, &secrets, None).unwrap();
        assert_eq!(installed.id, "test-tool");
        assert_eq!(installed.version, "1.2.3");
        assert!(installed.probe.available);

        // Test reinstall same version
        let preview2 = rt
            .block_on(preview_install(
                tool_dir.to_str().unwrap(),
                ToolInstallScope::App,
                None,
                &app_paths,
            ))
            .unwrap();
        assert!(preview2.is_reinstall);
        let reinstalled = confirm_install(&preview2, &secrets, None).unwrap();
        assert_eq!(reinstalled.id, "test-tool");

        // Uninstall
        let uninstalled = uninstall_tool(
            "test-tool",
            Some("1.2.3"),
            Some(ToolInstallScope::App),
            None,
            &app_paths,
        )
        .unwrap();
        assert!(uninstalled);
    }

    #[test]
    fn test_safe_segment() {
        assert!(is_safe_segment("my.tool-1_x"));
        assert!(is_safe_segment("1.2.3"));
        for bad in [
            "", "..", "../x", "a/b", r"a\b", ".hidden", "a..b", "C:x", "a b",
        ] {
            assert!(!is_safe_segment(bad), "{bad}");
        }
    }

    #[test]
    fn test_bad_manifest_refused() {
        let temp = tempfile::tempdir().unwrap();
        let tool_dir = temp.path().join("bad-tool");
        fs::create_dir_all(&tool_dir).unwrap();
        fs::write(tool_dir.join("tool.yaml"), "invalid: yaml: content: [").unwrap();

        let app_paths = AppPaths::new(temp.path().join("app-data"));
        let rt = tokio::runtime::Runtime::new().unwrap();
        let res = rt.block_on(preview_install(
            tool_dir.to_str().unwrap(),
            ToolInstallScope::App,
            None,
            &app_paths,
        ));
        assert!(res.is_err());
    }

    #[test]
    fn test_zip_slip_refused() {
        let temp = tempfile::tempdir().unwrap();
        let bad_zip = make_test_zip(&[
            ("../../../escaped.txt", b"evil content"),
            (
                "tool.yaml",
                b"id: bad\nkind: generator\ndescription: bad\nrun:\n  type: cli\n  command: cmd\n",
            ),
        ]);

        let extract_dir = temp.path().join("out");
        fs::create_dir_all(&extract_dir).unwrap();
        let res = extract_zip_safely(&bad_zip, &extract_dir);
        assert!(res.is_err());
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains("zip-slip") || err_msg.contains("Từ chối"));
    }

    #[test]
    fn test_oversize_zip_refused() {
        let temp = tempfile::tempdir().unwrap();
        let huge_bytes = vec![0u8; 1000]; // Simulated check
        let zip_data = make_test_zip(&[("huge.bin", &huge_bytes)]);

        // Check our size limit constant
        assert_eq!(MAX_ZIP_BYTES, 100 * 1024 * 1024);

        let extract_dir = temp.path().join("out");
        fs::create_dir_all(&extract_dir).unwrap();
        let res = extract_zip_safely(&zip_data, &extract_dir);
        assert!(res.is_ok());
    }
}
