//! `runtime.json` lockfile and `runtime.d` pointer directory that lets the stdio shim find the running Studio (ADR-008 §4).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Contents of `%APPDATA%/LocalContentStudio/runtime.json` or pointer file in `runtime.d`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeInfo {
    /// MCP server port on 127.0.0.1.
    pub port: u16,
    /// Secret the shim presents to `/shim/token`.
    pub shim_secret: String,
    /// Studio process id.
    pub pid: u32,
    /// Optional path to the full runtime.json file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_path: Option<PathBuf>,
}

/// Default location (respects `STUDIO_APP_DIR` like the rest of the app).
pub fn default_path() -> PathBuf {
    studio_workspace::AppPaths::from_env()
        .data_dir()
        .join("runtime.json")
}

/// Fixed default location for runtime.json (`%APPDATA%/LocalContentStudio/runtime.json`), ignoring `STUDIO_APP_DIR`.
pub fn fixed_default_path() -> PathBuf {
    let base = if let Ok(app_data) = std::env::var("APPDATA") {
        PathBuf::from(app_data)
    } else if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        PathBuf::from(home).join(".localcontentstudio")
    } else {
        PathBuf::from(".studio-app-data")
    };
    base.join("LocalContentStudio").join("runtime.json")
}

/// Fixed per-user location for `runtime.d`: `%LOCALAPPDATA%/LocalContentStudio/runtime.d`.
pub fn runtime_d_dir() -> PathBuf {
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        PathBuf::from(local)
            .join("LocalContentStudio")
            .join("runtime.d")
    } else if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        PathBuf::from(home)
            .join("AppData")
            .join("Local")
            .join("LocalContentStudio")
            .join("runtime.d")
    } else {
        PathBuf::from(".studio-local-data")
            .join("LocalContentStudio")
            .join("runtime.d")
    }
}

/// Pointer file for a specific PID: `%LOCALAPPDATA%/LocalContentStudio/runtime.d/<pid>.json`.
pub fn runtime_d_path(pid: u32) -> PathBuf {
    runtime_d_dir().join(format!("{pid}.json"))
}

/// Write the lockfile; on Windows restrict it to the current user.
pub fn write(path: &Path, info: &RuntimeInfo) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_string_pretty(info).map_err(std::io::Error::other)?;
    std::fs::write(path, text)?;
    restrict_to_owner(path);
    Ok(())
}

/// Write pointer file into `runtime.d` with owner-only ACL.
pub fn write_runtime_d(info: &RuntimeInfo) -> std::io::Result<PathBuf> {
    let dir = runtime_d_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", info.pid));
    let text = serde_json::to_string_pretty(info).map_err(std::io::Error::other)?;
    std::fs::write(&path, text)?;
    restrict_to_owner(&path);
    Ok(path)
}

/// Read the lockfile.
pub fn read(path: &Path) -> std::io::Result<RuntimeInfo> {
    let text = std::fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(std::io::Error::other)
}

#[derive(Debug, Deserialize)]
struct PointerHelper {
    port: Option<u16>,
    shim_secret: Option<String>,
    pid: Option<u32>,
    runtime_path: Option<PathBuf>,
    path: Option<PathBuf>,
}

/// Read a runtime entry which may be a pointer file or a full `RuntimeInfo`.
pub fn read_entry(path: &Path) -> std::io::Result<RuntimeInfo> {
    let text = std::fs::read_to_string(path)?;
    if let Ok(helper) = serde_json::from_str::<PointerHelper>(&text) {
        if let (Some(port), Some(shim_secret), Some(pid)) =
            (helper.port, helper.shim_secret, helper.pid)
        {
            return Ok(RuntimeInfo {
                port,
                shim_secret,
                pid,
                runtime_path: helper.runtime_path.or(helper.path),
            });
        }
        if let Some(target) = helper.runtime_path.or(helper.path) {
            return read(&target);
        }
    }
    serde_json::from_str(&text).map_err(std::io::Error::other)
}

/// Remove the lockfile (Studio shutting down).
pub fn remove(path: &Path) {
    let _ = std::fs::remove_file(path);
}

/// Remove pointer file from `runtime.d` on shutdown.
pub fn remove_runtime_d(pid: u32) {
    let _ = std::fs::remove_file(runtime_d_path(pid));
}

/// Restrict file access to the current owner/user.
#[cfg(windows)]
pub fn restrict_to_owner(path: &Path) {
    // Owner-only ACL: drop inherited entries and grant the current user full control.
    if let Ok(user) = std::env::var("USERNAME") {
        let _ = std::process::Command::new("icacls")
            .arg(path)
            .args(["/inheritance:r", "/grant:r", &format!("{user}:F")])
            .output();
    }
}

/// Restrict file access to the current owner/user on Unix targets.
#[cfg(not(windows))]
pub fn restrict_to_owner(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

/// Check if a process with `pid` is currently alive.
#[cfg(windows)]
pub fn is_process_alive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, FALSE, STILL_ACTIVE};
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    if pid == 0 {
        return false;
    }
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
        if handle == 0 {
            use windows_sys::Win32::Foundation::GetLastError;
            return GetLastError() == 5; // ERROR_ACCESS_DENIED means process exists
        }
        let mut exit_code = 0u32;
        let res = GetExitCodeProcess(handle, &mut exit_code);
        CloseHandle(handle);
        res != 0 && exit_code == STILL_ACTIVE as u32
    }
}

/// Check if a process with `pid` is currently alive on Unix targets.
#[cfg(not(windows))]
pub fn is_process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
