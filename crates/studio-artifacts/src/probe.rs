use std::path::{Path, PathBuf};
use std::process::Command;

/// Probe result for an external binary/tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeReport {
    /// Name of tool/binary probed
    pub name: String,
    /// Whether the tool is available and functioning
    pub available: bool,
    /// Absolute path to executable if found
    pub path: Option<PathBuf>,
    /// Version string if probe succeeded
    pub version: Option<String>,
    /// Notes or error messages encountered during probe
    pub note: String,
}

/// Probe Chromium-family browsers in order: env `STUDIO_CHROME`, `chrome-headless-shell`, Chrome, Edge.
///
/// Never launches a full browser: on Windows `chrome.exe --version` opens a browser window instead of
/// printing a version, so browser versions are read from the executable's file metadata.
/// Only `chrome-headless-shell` (a CLI) is executed with `--version`.
pub fn probe_chromium() -> ProbeReport {
    let found = |name: &str, path: PathBuf, note: String| {
        let version = browser_version(&path);
        ProbeReport {
            name: name.into(),
            available: true,
            path: Some(path),
            version,
            note,
        }
    };

    if let Ok(env_path) = std::env::var("STUDIO_CHROME") {
        let p = PathBuf::from(&env_path);
        if p.is_file() {
            return found(
                "STUDIO_CHROME",
                p,
                "Dùng trình duyệt từ biến môi trường STUDIO_CHROME".into(),
            );
        }
    }
    if let Ok(path) = which::which("chrome-headless-shell") {
        if let Some(ver) = get_binary_version(&path, &["--version"]) {
            return ProbeReport {
                name: "chrome-headless-shell".into(),
                available: true,
                path: Some(path),
                version: Some(ver),
                note: "Tìm thấy chrome-headless-shell trên PATH".into(),
            };
        }
    }

    let mut candidates: Vec<(&str, PathBuf)> = Vec::new();
    #[cfg(windows)]
    {
        for base in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
            if let Ok(dir) = std::env::var(base) {
                candidates.push((
                    "chrome",
                    PathBuf::from(&dir).join(r"Google\Chrome\Application\chrome.exe"),
                ));
            }
        }
        for base in ["ProgramFiles(x86)", "ProgramFiles"] {
            if let Ok(dir) = std::env::var(base) {
                candidates.push((
                    "msedge",
                    PathBuf::from(&dir).join(r"Microsoft\Edge\Application\msedge.exe"),
                ));
            }
        }
    }
    for name in [
        "chrome",
        "google-chrome",
        "chromium",
        "msedge",
        "microsoft-edge",
    ] {
        if let Ok(path) = which::which(name) {
            candidates.push((
                if name.contains("edge") {
                    "msedge"
                } else {
                    "chrome"
                },
                path,
            ));
        }
    }
    for (name, path) in candidates {
        if path.is_file() {
            let note = format!("Tìm thấy {} tại {}", name, path.display());
            return found(name, path, note);
        }
    }

    ProbeReport {
        name: "chromium".into(),
        available: false,
        path: None,
        version: None,
        note:
            "Không tìm thấy Chrome/Edge/chrome-headless-shell. Cài Chrome hoặc đặt STUDIO_CHROME."
                .into(),
    }
}

/// Browser version without executing the browser.
fn browser_version(path: &Path) -> Option<String> {
    #[cfg(windows)]
    {
        let script = format!(
            "(Get-Item -LiteralPath '{}').VersionInfo.ProductVersion",
            path.display().to_string().replace('\'', "''")
        );
        let out = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output()
            .ok()?;
        let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
        (!v.is_empty()).then_some(v)
    }
    #[cfg(not(windows))]
    {
        get_binary_version(path, &["--version"])
    }
}

/// Helper to probe FFmpeg.
pub fn probe_ffmpeg() -> ProbeReport {
    if let Ok(path) = which::which("ffmpeg") {
        if let Some(ver) = get_binary_version(&path, &["-version"]) {
            return ProbeReport {
                name: "ffmpeg".into(),
                available: true,
                path: Some(path),
                version: Some(ver),
                note: "Found ffmpeg on PATH".into(),
            };
        }
    }

    ProbeReport {
        name: "ffmpeg".into(),
        available: false,
        path: None,
        version: None,
        note: "FFmpeg not found on system PATH".into(),
    }
}

/// Helper to probe FFprobe.
pub fn probe_ffprobe() -> ProbeReport {
    if let Ok(path) = which::which("ffprobe") {
        if let Some(ver) = get_binary_version(&path, &["-version"]) {
            return ProbeReport {
                name: "ffprobe".into(),
                available: true,
                path: Some(path),
                version: Some(ver),
                note: "Found ffprobe on PATH".into(),
            };
        }
    }

    ProbeReport {
        name: "ffprobe".into(),
        available: false,
        path: None,
        version: None,
        note: "FFprobe not found on system PATH".into(),
    }
}

/// Helper to probe Node and npx.
pub fn probe_node_npx() -> (ProbeReport, ProbeReport) {
    let node_report = if let Ok(path) = which::which("node") {
        let ver = get_binary_version(&path, &["--version"]);
        ProbeReport {
            name: "node".into(),
            available: true,
            path: Some(path),
            version: ver,
            note: "Found node on PATH".into(),
        }
    } else {
        ProbeReport {
            name: "node".into(),
            available: false,
            path: None,
            version: None,
            note: "Node not found on system PATH".into(),
        }
    };

    let npx_report = if let Ok(path) = which::which("npx") {
        let ver = get_binary_version(&path, &["--version"]);
        ProbeReport {
            name: "npx".into(),
            available: true,
            path: Some(path),
            version: ver,
            note: "Found npx on PATH".into(),
        }
    } else {
        ProbeReport {
            name: "npx".into(),
            available: false,
            path: None,
            version: None,
            note: "npx not found on system PATH".into(),
        }
    };

    (node_report, npx_report)
}

fn get_binary_version(path: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new(path).args(args).output().ok()?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let first_line = stdout.lines().next().unwrap_or("").trim().to_string();
        if !first_line.is_empty() {
            return Some(first_line);
        }
    }
    None
}

// Minimal fallback which implementation
mod which {
    use std::path::PathBuf;

    pub fn which(name: &str) -> std::result::Result<PathBuf, ()> {
        let path_var = std::env::var_os("PATH").ok_or(())?;
        let pathext_var =
            std::env::var_os("PATHEXT").unwrap_or_else(|| ".EXE;.CMD;.BAT;.COM".into());
        let extensions: Vec<String> = std::env::split_paths(&pathext_var)
            .filter_map(|p| p.to_str().map(|s| s.to_string()))
            .collect();

        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Ok(candidate);
            }

            #[cfg(windows)]
            {
                for ext in &extensions {
                    let with_ext = dir.join(format!("{}{}", name, ext));
                    if with_ext.is_file() {
                        return Ok(with_ext);
                    }
                }
            }
        }
        Err(())
    }
}
