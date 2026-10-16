//! `runtime.json` lockfile that lets the stdio shim find the running Studio (ADR-008 §4).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Contents of `%APPDATA%/LocalContentStudio/runtime.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeInfo {
    /// MCP server port on 127.0.0.1.
    pub port: u16,
    /// Secret the shim presents to `/shim/token`.
    pub shim_secret: String,
    /// Studio process id.
    pub pid: u32,
}

/// Default location (respects `STUDIO_APP_DIR` like the rest of the app).
pub fn default_path() -> PathBuf {
    studio_workspace::AppPaths::from_env()
        .data_dir()
        .join("runtime.json")
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

/// Read the lockfile.
pub fn read(path: &Path) -> std::io::Result<RuntimeInfo> {
    let text = std::fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(std::io::Error::other)
}

/// Remove the lockfile (Studio shutting down).
pub fn remove(path: &Path) {
    let _ = std::fs::remove_file(path);
}

#[cfg(windows)]
fn restrict_to_owner(path: &Path) {
    // Owner-only ACL: drop inherited entries and grant the current user full control.
    if let Ok(user) = std::env::var("USERNAME") {
        let _ = std::process::Command::new("icacls")
            .arg(path)
            .args(["/inheritance:r", "/grant:r", &format!("{user}:F")])
            .output();
    }
}

#[cfg(not(windows))]
fn restrict_to_owner(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}
