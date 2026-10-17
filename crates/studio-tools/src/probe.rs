//! Probe implementation for validating tool dependencies.

use crate::manifest::ToolManifest;
use crate::secret::SecretStore;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use std::process::Command;
use studio_workspace::Workspace;

/// Result of probing a tool's declared dependencies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeResult {
    /// True if all declared dependencies are available and verified.
    pub available: bool,
    /// List of declared dependencies that could not be verified.
    pub missing: Vec<String>,
    /// Vietnamese note describing status.
    #[serde(alias = "note_vi")]
    pub note_vi: String,
}

/// Probe a tool's declared needs using real filesystem and system checks.
/// Never returns fabricated success.
pub fn probe(
    tool: &ToolManifest,
    secret_store: &dyn SecretStore,
    ws: Option<&Workspace>,
) -> ProbeResult {
    let mut missing = Vec::new();

    let (org_name, ws_name) = match ws {
        Some(w) => (w.org().name().to_string(), Some(w.name())),
        None => (String::new(), None),
    };

    for need in &tool.needs {
        if let Some(bin) = need.strip_prefix("bin:") {
            if !probe_binary(bin) {
                missing.push(need.clone());
            }
        } else if let Some(secret_key) = need.strip_prefix("secret:") {
            if secret_store.get(&org_name, ws_name, secret_key).is_none() {
                missing.push(need.clone());
            }
        } else if let Some(app) = need.strip_prefix("app:") {
            if !probe_application(app) {
                missing.push(need.clone());
            }
        } else {
            // Unrecognized need category
            missing.push(need.clone());
        }
    }

    if missing.is_empty() {
        ProbeResult {
            available: true,
            missing: Vec::new(),
            note_vi: "Sẵn sàng".to_string(),
        }
    } else {
        let missing_vi: Vec<String> = missing
            .iter()
            .map(|m| {
                if let Some(bin) = m.strip_prefix("bin:") {
                    format!("lệnh '{bin}'")
                } else if let Some(secret) = m.strip_prefix("secret:") {
                    format!("khoá API {secret}")
                } else if let Some(app) = m.strip_prefix("app:") {
                    format!("ứng dụng '{app}'")
                } else {
                    m.clone()
                }
            })
            .collect();

        let note = if missing.iter().all(|m| m.starts_with("secret:")) {
            let keys: Vec<&str> = missing
                .iter()
                .filter_map(|m| m.strip_prefix("secret:"))
                .collect();
            format!(
                "Chưa có khoá API {} — thêm trong Cài đặt → Khoá API",
                keys.join(", ")
            )
        } else if let Some(first_secret) = missing.iter().find_map(|m| m.strip_prefix("secret:")) {
            let extra: Vec<String> = missing_vi
                .iter()
                .filter(|m| !m.starts_with("khoá API"))
                .cloned()
                .collect();
            if extra.is_empty() {
                format!("Chưa có khoá API {first_secret} — thêm trong Cài đặt → Khoá API")
            } else {
                format!(
                    "Chưa có khoá API {} — thêm trong Cài đặt → Khoá API (thiếu thêm: {})",
                    first_secret,
                    extra.join(", ")
                )
            }
        } else {
            format!("Thiếu phụ thuộc: {}", missing_vi.join(", "))
        };
        ProbeResult {
            available: false,
            missing,
            note_vi: note,
        }
    }
}

/// Verify executable exists in PATH and can be invoked.
pub fn probe_binary(bin: &str) -> bool {
    let exe_path = find_in_path(bin);
    if exe_path.is_none() {
        return false;
    }

    // Try invoking --version or -version or /?
    let target = exe_path.unwrap();
    let mut cmd = Command::new(&target);

    // Some programs (ffmpeg/ffprobe) use -version, powershell uses -Version, git uses --version
    if bin.eq_ignore_ascii_case("ffmpeg") || bin.eq_ignore_ascii_case("ffprobe") {
        cmd.arg("-version");
    } else if bin.eq_ignore_ascii_case("powershell") {
        cmd.args(["-NoProfile", "-Command", "$PSVersionTable.PSVersion"]);
    } else {
        cmd.arg("--version");
    }

    match cmd.output() {
        Ok(out) => out.status.success() || !out.stdout.is_empty() || !out.stderr.is_empty(),
        Err(_) => false,
    }
}

/// Locate binary executable in PATH considering OS-specific executable extensions.
pub fn find_in_path(bin: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;

    #[cfg(windows)]
    let extensions = ["", ".exe", ".cmd", ".bat", ".com"];
    #[cfg(not(windows))]
    let extensions = [""];

    for dir in env::split_paths(&path_var) {
        for ext in &extensions {
            let candidate = if ext.is_empty() {
                dir.join(bin)
            } else {
                dir.join(format!("{}{}", bin, ext))
            };

            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

/// Check known installation paths for heavy applications (Blender, After Effects).
pub fn probe_application(app: &str) -> bool {
    let app_lower = app.to_lowercase();

    // Check if accessible from PATH first
    if find_in_path(app).is_some() {
        return true;
    }

    if app_lower == "blender" {
        // Standard Windows Blender paths
        let base_dirs = [
            PathBuf::from(r"C:\Program Files\Blender Foundation"),
            PathBuf::from(r"C:\Program Files (x86)\Blender Foundation"),
        ];

        for base in &base_dirs {
            if let Ok(entries) = std::fs::read_dir(base) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() && path.join("blender.exe").is_file() {
                        return true;
                    }
                }
            }
        }
    } else if app_lower == "after_effects" || app_lower == "aftereffects" || app_lower == "ae" {
        // Standard Windows Adobe After Effects paths
        let adobe_base = PathBuf::from(r"C:\Program Files\Adobe");
        if let Ok(entries) = std::fs::read_dir(&adobe_base) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.contains("After Effects") {
                    let exe = entry.path().join("Support Files").join("AfterFX.exe");
                    if exe.is_file() {
                        return true;
                    }
                }
            }
        }
    }

    false
}
