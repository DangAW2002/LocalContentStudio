//! Helpers for configuring and registering Studio MCP shim with `agy`.

use crate::error::{Result, ToolError};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Returns the standard path to `%USERPROFILE%/.gemini/antigravity-cli/settings.json`.
pub fn default_agy_settings_path() -> Option<PathBuf> {
    let home = env::var("USERPROFILE").or_else(|_| env::var("HOME")).ok()?;
    Some(
        PathBuf::from(home)
            .join(".gemini")
            .join("antigravity-cli")
            .join("settings.json"),
    )
}

/// Setup agy integration with custom settings file path and CLI execution flag (useful for testing).
pub fn setup_agy_at(settings_path: &Path, studio_mcp_exe: &Path, run_cli: bool) -> Result<()> {
    if run_cli {
        let exe_str = studio_mcp_exe.to_string_lossy();
        let status = Command::new("agy")
            .args(["mcp", "add", "studio", "--", exe_str.as_ref(), "--shim"])
            .status();

        if let Err(err) = status {
            tracing::warn!("Không thể chạy lệnh 'agy mcp add': {}", err);
        }
    }

    // Update settings.json preserving all other keys
    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent).map_err(|e| ToolError::Io {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }

    let mut doc: serde_json::Value = if settings_path.is_file() {
        let content = fs::read_to_string(settings_path).map_err(|e| ToolError::Io {
            path: settings_path.to_path_buf(),
            source: e,
        })?;
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if !doc.is_object() {
        doc = serde_json::json!({});
    }

    let permissions = doc
        .as_object_mut()
        .unwrap()
        .entry("permissions")
        .or_insert_with(|| serde_json::json!({}));

    if !permissions.is_object() {
        *permissions = serde_json::json!({});
    }

    let allow = permissions
        .as_object_mut()
        .unwrap()
        .entry("allow")
        .or_insert_with(|| serde_json::json!([]));

    if !allow.is_array() {
        *allow = serde_json::json!([]);
    }

    let allow_arr = allow.as_array_mut().unwrap();
    let rule = serde_json::Value::String("mcp(studio)".to_string());
    if !allow_arr.contains(&rule) {
        allow_arr.push(rule);
    }

    let updated =
        serde_json::to_string_pretty(&doc).map_err(|e| ToolError::ParseError(e.to_string()))?;
    fs::write(settings_path, updated).map_err(|e| ToolError::Io {
        path: settings_path.to_path_buf(),
        source: e,
    })?;

    Ok(())
}

/// Remove agy integration with custom settings file path (useful for testing).
pub fn remove_agy_at(settings_path: &Path, run_cli: bool) -> Result<()> {
    if run_cli {
        let _ = Command::new("agy")
            .args(["mcp", "remove", "studio"])
            .status();
    }

    if !settings_path.is_file() {
        return Ok(());
    }

    let content = fs::read_to_string(settings_path).map_err(|e| ToolError::Io {
        path: settings_path.to_path_buf(),
        source: e,
    })?;

    let mut doc: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Ok(()),
    };

    if let Some(perms) = doc.get_mut("permissions").and_then(|p| p.as_object_mut()) {
        if let Some(allow) = perms.get_mut("allow").and_then(|a| a.as_array_mut()) {
            let rule = serde_json::Value::String("mcp(studio)".to_string());
            allow.retain(|item| item != &rule);
        }
    }

    let updated =
        serde_json::to_string_pretty(&doc).map_err(|e| ToolError::ParseError(e.to_string()))?;
    fs::write(settings_path, updated).map_err(|e| ToolError::Io {
        path: settings_path.to_path_buf(),
        source: e,
    })?;

    Ok(())
}

/// Configure agy global registration for studio-mcp and add allow rule.
pub fn setup_agy(studio_mcp_exe: &Path) -> Result<()> {
    let settings_path = default_agy_settings_path().ok_or_else(|| {
        ToolError::SetupError("Không thể xác định thư mục người dùng (%USERPROFILE%)".to_string())
    })?;
    setup_agy_at(&settings_path, studio_mcp_exe, true)
}

/// Remove agy global registration for studio-mcp and clean allow rule.
pub fn remove_agy() -> Result<()> {
    let settings_path = default_agy_settings_path().ok_or_else(|| {
        ToolError::SetupError("Không thể xác định thư mục người dùng (%USERPROFILE%)".to_string())
    })?;
    remove_agy_at(&settings_path, true)
}
