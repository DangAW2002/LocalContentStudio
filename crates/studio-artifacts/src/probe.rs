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
                note: "Tìm thấy chrome-headless-shell trong PATH".into(),
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
        note: "Không tìm thấy Chrome/Edge/chrome-headless-shell — hãy cài đặt Google Chrome hoặc đặt biến môi trường STUDIO_CHROME.".into(),
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
                note: "Tìm thấy ffmpeg trong PATH".into(),
            };
        }
    }

    ProbeReport {
        name: "ffmpeg".into(),
        available: false,
        path: None,
        version: None,
        note: "Không tìm thấy FFmpeg trong PATH — hãy cài đặt qua winget: 'winget install Gyan.FFmpeg' hoặc tải từ https://ffmpeg.org".into(),
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
                note: "Tìm thấy ffprobe trong PATH".into(),
            };
        }
    }

    ProbeReport {
        name: "ffprobe".into(),
        available: false,
        path: None,
        version: None,
        note: "Không tìm thấy ffprobe trong PATH — đi kèm FFmpeg, hãy cài đặt qua winget: 'winget install Gyan.FFmpeg'".into(),
    }
}

/// Helper to probe Node and npx.
///
/// On Windows, `npx` is a `.cmd` batch script; `which` resolves it to `npx.cmd`.
/// The path is stored in `ProbeReport.path` and must be invoked via `cmd.exe /C` on Windows.
pub fn probe_node_npx() -> (ProbeReport, ProbeReport) {
    let node_report = if let Ok(path) = which::which("node") {
        let ver = get_binary_version(&path, &["--version"]);
        ProbeReport {
            name: "node".into(),
            available: true,
            path: Some(path),
            version: ver,
            note: "Tìm thấy node trong PATH".into(),
        }
    } else {
        ProbeReport {
            name: "node".into(),
            available: false,
            path: None,
            version: None,
            note: "Không tìm thấy Node.js trong PATH — hãy cài đặt qua winget: 'winget install OpenJS.NodeJS' hoặc tải từ https://nodejs.org".into(),
        }
    };

    // On Windows npx is npx.cmd; which() already appends PATHEXT so we get the .cmd path.
    let npx_name = if cfg!(windows) { "npx.cmd" } else { "npx" };
    let npx_report = if let Ok(path) = which::which(npx_name) {
        // Use --version; npx.cmd needs cmd.exe on Windows, but --version works via direct spawn too.
        let ver = get_npx_version(&path);
        ProbeReport {
            name: "npx".into(),
            available: true,
            path: Some(path),
            version: ver,
            note: format!("Tìm thấy {} trong PATH", npx_name),
        }
    } else if let Ok(path) = which::which("npx") {
        // Fallback: plain 'npx' (non-Windows or npx.cmd not found under that name)
        let ver = get_npx_version(&path);
        ProbeReport {
            name: "npx".into(),
            available: true,
            path: Some(path),
            version: ver,
            note: "Tìm thấy npx trong PATH".into(),
        }
    } else {
        ProbeReport {
            name: "npx".into(),
            available: false,
            path: None,
            version: None,
            note: "Không tìm thấy npx trong PATH — đi kèm Node.js, hãy cài đặt Node.js từ https://nodejs.org".into(),
        }
    };

    (node_report, npx_report)
}

/// Get npx version, handling `.cmd` scripts on Windows that require `cmd.exe /C`.
fn get_npx_version(path: &std::path::Path) -> Option<String> {
    #[cfg(windows)]
    {
        let out = std::process::Command::new("cmd.exe")
            .args(["/C", &path.to_string_lossy(), "--version"])
            .output()
            .ok()?;
        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        // npx --version prints to stdout; pick first non-empty line from either
        for line in stdout.lines().chain(stderr.lines()) {
            let t = line.trim();
            if !t.is_empty()
                && t.chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
            {
                return Some(t.to_string());
            }
        }
        None
    }
    #[cfg(not(windows))]
    {
        get_binary_version(path, &["--version"])
    }
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
        let pathext_var =
            std::env::var_os("PATHEXT").unwrap_or_else(|| ".EXE;.CMD;.BAT;.COM".into());
        let extensions: Vec<String> = std::env::split_paths(&pathext_var)
            .filter_map(|p| p.to_str().map(|s| s.to_string()))
            .collect();

        for dir in studio_workspace::env_path::get_fresh_paths(false) {
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
