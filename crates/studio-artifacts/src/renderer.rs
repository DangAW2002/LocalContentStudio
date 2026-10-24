use std::path::{Path, PathBuf};
use std::time::Duration;
use studio_jobs::{
    command::CommandSpec,
    job::{JobSpec, Lifecycle, Priority},
    resource::{CpuClass, ResourceSpec},
};

use crate::error::{ArtifactError, Result};
use crate::probe::ProbeReport;

/// `file://` URL for a local path. Strips the Windows verbatim prefix (`\\?\`), which browsers do not
/// understand (Chrome silently opens a new-tab page instead), and percent-encodes spaces/non-ASCII.
pub fn file_url_for(path: &Path) -> Result<String> {
    let raw = path.to_string_lossy();
    let plain = if let Some(rest) = raw.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = raw.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        raw.into_owned()
    };
    url::Url::from_file_path(&plain)
        .map(|u| u.to_string())
        .map_err(|_| {
            ArtifactError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Không tạo được file URL cho {plain}"),
            ))
        })
}

/// Artboard size declared by an HTML source, in px. Looks, in order, at
/// `<meta name="viewport" content="width=W, height=H">`, a `data-artboard="WxH"` attribute, and the first CSS
/// rule whose selector mentions `artboard` or `frame` with `width: Wpx` and `height: Hpx`.
pub fn detect_artboard_size(html: &str) -> Option<(u32, u32)> {
    fn num_after(s: &str, key: &str) -> Option<u32> {
        let at = s.find(key)? + key.len();
        let digits: String = s[at..]
            .trim_start_matches([' ', ':', '=', '"'])
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        digits.parse().ok().filter(|n| *n > 0)
    }

    let lower = html.to_ascii_lowercase();
    // 1. viewport meta
    for (i, _) in lower.match_indices("name=\"viewport\"") {
        let tag_start = lower[..i].rfind('<').unwrap_or(0);
        let tag_end = lower[i..].find('>').map(|e| i + e).unwrap_or(lower.len());
        let tag = &lower[tag_start..tag_end];
        if let (Some(w), Some(h)) = (num_after(tag, "width"), num_after(tag, "height")) {
            return Some((w, h));
        }
    }
    // 2. data-artboard="WxH"
    if let Some(i) = lower.find("data-artboard=\"") {
        let rest = &lower[i + "data-artboard=\"".len()..];
        let value = &rest[..rest.find('"').unwrap_or(0)];
        if let Some((w, h)) = value.split_once('x') {
            if let (Ok(w), Ok(h)) = (w.trim().parse(), h.trim().parse()) {
                return Some((w, h));
            }
        }
    }
    // 3. CSS rule for the artboard/frame element
    let mut search = 0;
    while let Some(open) = lower[search..].find('{').map(|o| search + o) {
        let close = lower[open..].find('}').map(|c| open + c)?;
        let selector_start = lower[..open].rfind(['}', '>']).map(|s| s + 1).unwrap_or(0);
        let selector = &lower[selector_start..open];
        let page_root = selector
            .split(',')
            .any(|s| matches!(s.trim(), "html" | "body"));
        if selector.contains("artboard") || selector.contains("frame") || page_root {
            let body = &lower[open..close];
            if let (Some(w), Some(h)) = (
                num_after_prop(body, "width"),
                num_after_prop(body, "height"),
            ) {
                return Some((w, h));
            }
        }
        search = close + 1;
    }
    None
}

/// `width: 1280px` inside a CSS block, ignoring `min-width`/`max-width`/`line-height` style prefixes.
fn num_after_prop(body: &str, prop: &str) -> Option<u32> {
    for (i, _) in body.match_indices(prop) {
        let prev = body[..i].chars().last();
        if matches!(prev, Some(c) if c.is_ascii_alphanumeric() || c == '-') {
            continue;
        }
        let rest = body[i + prop.len()..].trim_start();
        let Some(rest) = rest.strip_prefix(':') else {
            continue;
        };
        let digits: String = rest
            .trim_start()
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        let after = &rest.trim_start()[digits.len()..];
        if after.starts_with("px") {
            if let Ok(n) = digits.parse::<u32>() {
                return Some(n);
            }
        }
    }
    None
}

#[cfg(test)]
mod artboard_tests {
    use super::detect_artboard_size;

    #[test]
    fn viewport_meta() {
        let html = r#"<meta name="viewport" content="width=1080, height=1350, initial-scale=1.0">"#;
        assert_eq!(detect_artboard_size(html), Some((1080, 1350)));
    }

    #[test]
    fn data_artboard_attr() {
        assert_eq!(
            detect_artboard_size(r#"<div data-artboard="1280x720"></div>"#),
            Some((1280, 720))
        );
    }

    #[test]
    fn css_frame_rule() {
        let html = "<style>html, body { margin: 0; }\n .frame {\n position: relative;\n max-width: 9px;\n width: 1280px;\n height: 720px;\n line-height: 3px; }</style>";
        assert_eq!(detect_artboard_size(html), Some((1280, 720)));
    }

    #[test]
    fn css_page_root_rule() {
        let html = "<style>
 html, body { margin: 0; width: 1280px; height: 720px; overflow: hidden; }</style>";
        assert_eq!(detect_artboard_size(html), Some((1280, 720)));
    }

    #[test]
    fn none_when_undeclared() {
        assert_eq!(detect_artboard_size("<p>hi</p>"), None);
    }
}

#[cfg(all(test, windows))]
mod file_url_tests {
    use super::file_url_for;
    use std::path::Path;

    #[test]
    fn strips_verbatim_prefix_and_encodes() {
        let url = file_url_for(Path::new(r"\\?\D:\Kênh thử\a b\index.html")).unwrap();
        assert_eq!(url, "file:///D:/K%C3%AAnh%20th%E1%BB%AD/a%20b/index.html");
        let plain = file_url_for(Path::new(r"C:\x\index.html")).unwrap();
        assert_eq!(plain, "file:///C:/x/index.html");
    }
}

/// What to render and where: one HTML artboard to a PNG screenshot or a PDF.
#[derive(Debug, Clone, Copy)]
pub struct HtmlRenderTarget<'a> {
    /// HTML file to load.
    pub html_file: &'a Path,
    /// Output PNG/PDF path.
    pub output_file: &'a Path,
    /// Viewport width in px.
    pub width: u32,
    /// Viewport height in px.
    pub height: u32,
    /// Print to PDF instead of a PNG screenshot.
    pub as_pdf: bool,
}

/// Build a `JobSpec` for rendering HTML to PNG or PDF using a headless Chromium-family browser.
///
/// Flags:
/// `--headless=new --disable-gpu --hide-scrollbars --window-size=W,H --screenshot=<out> file:///<abs_html_path>`
/// (or `--print-to-pdf=<out>` for PDF)
pub fn build_html_render_job(
    job_id: &str,
    workspace_id: &str,
    browser_report: &ProbeReport,
    target: &HtmlRenderTarget<'_>,
) -> Result<JobSpec> {
    let HtmlRenderTarget {
        html_file,
        output_file,
        width,
        height,
        as_pdf,
    } = *target;
    let browser_path = browser_report.path.as_ref().ok_or_else(|| {
        ArtifactError::ToolNotFound("Chromium browser is not available for HTML rendering".into())
    })?;

    let file_url = file_url_for(html_file)?;

    // A private, throwaway profile per job: never attach to (or open windows in) the user's running browser.
    let profile_dir = std::env::temp_dir().join(format!("lcs-browser-{}", job_id));
    let mut args = vec![
        "--headless=new".into(),
        "--disable-gpu".into(),
        "--hide-scrollbars".into(),
        "--no-first-run".into(),
        "--no-default-browser-check".into(),
        "--disable-extensions".into(),
        format!("--user-data-dir={}", profile_dir.display()),
        format!("--window-size={},{}", width, height),
    ];

    if as_pdf {
        args.push(format!("--print-to-pdf={}", output_file.display()));
    } else {
        args.push(format!("--screenshot={}", output_file.display()));
    }

    args.push(file_url);

    let cwd = html_file
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let command = CommandSpec::new(browser_path.to_string_lossy().to_string(), args, cwd);

    let resources = ResourceSpec {
        ram_mb: 800,
        cpu: CpuClass::Light,
        gpu_vram_mb: 0,
        exclusive: Vec::new(),
        rate_key: None,
    };

    Ok(JobSpec {
        id: job_id.to_string(),
        workspace_id: workspace_id.to_string(),
        tool_id: if as_pdf {
            "render.html_pdf@1".into()
        } else {
            "render.html_png@1".into()
        },
        command,
        resources,
        priority: Priority::TurnBlocking,
        timeout: Duration::from_secs(60),
        memory_limit_mb: Some(1200),
        lifecycle: Lifecycle::PerCall,
    })
}

/// npm package spec used for HyperFrames renders (verified on Windows 2026-10-17).
pub const HYPERFRAMES_PACKAGE: &str = "hyperframes@0.8.167";

/// Build a `JobSpec` for rendering a HyperFrames composition (`npx --yes hyperframes@<pinned> render`).
///
/// On Windows `npx.cmd` is a batch script: npm's `node.exe npx-cli.js` is run directly when it sits next to
/// `npx.cmd` (no cmd.exe quoting pitfalls); otherwise `cmd.exe /C` is used and paths containing cmd
/// metacharacters are refused. Paths are stripped of the verbatim prefix (`\\?\`), which Node.js does not
/// understand.
pub fn build_hyperframes_render_job(
    job_id: &str,
    workspace_id: &str,
    npx_report: &ProbeReport,
    composition_dir: &Path,
    output_file: &Path,
    format: &str,
    fps: u32,
) -> Result<JobSpec> {
    let npx_path = npx_report.path.as_ref().ok_or_else(|| {
        ArtifactError::ToolNotFound(
            "npx is required for HyperFrames rendering but was not found".into(),
        )
    })?;

    // Strip verbatim prefix so Node.js / HyperFrames receives a plain Win32 path.
    let out_plain = strip_verbatim(output_file);

    // Strip verbatim prefix from composition_dir and npx_path too.
    let cwd_plain = PathBuf::from(strip_verbatim(composition_dir));
    let npx_plain = strip_verbatim(npx_path);

    // Pinned: renders must be reproducible and must not pull whatever npm publishes next.
    let mut hf_args: Vec<String> = vec![
        "--yes".into(),
        HYPERFRAMES_PACKAGE.into(),
        "render".into(),
        "--output".into(),
        out_plain.clone(),
    ];
    if !format.is_empty() {
        hf_args.push("--format".into());
        hf_args.push(format.to_string());
    }
    if fps > 0 {
        hf_args.push("--fps".into());
        hf_args.push(fps.to_string());
    }

    // On Windows `npx.cmd` is a batch file. Prefer running npm's own `node npx-cli.js` (no cmd.exe, so no
    // cmd quoting/metacharacter pitfalls); fall back to `cmd.exe /C` only for paths cmd can parse safely.
    #[cfg(windows)]
    let (exe, args) = {
        let npx_dir = Path::new(&npx_plain).parent().map(Path::to_path_buf);
        let direct = npx_dir.as_ref().and_then(|d| {
            let node = d.join("node.exe");
            let cli = d
                .join("node_modules")
                .join("npm")
                .join("bin")
                .join("npx-cli.js");
            (node.is_file() && cli.is_file()).then_some((node, cli))
        });
        match direct {
            Some((node, cli)) => {
                let mut a = vec![cli.to_string_lossy().into_owned()];
                a.extend(hf_args.iter().cloned());
                (node.to_string_lossy().into_owned(), a)
            }
            None => {
                for part in [
                    &npx_plain,
                    &out_plain,
                    &cwd_plain.to_string_lossy().into_owned(),
                ] {
                    if part.contains(['&', '|', '<', '>', '^', '%', '!', '"']) {
                        return Err(ArtifactError::ToolNotFound(format!(
                            "Đường dẫn có ký tự cmd.exe không xử lý an toàn được: {part}"
                        )));
                    }
                }
                let mut a = vec!["/C".to_string(), npx_plain.clone()];
                a.extend(hf_args.iter().cloned());
                ("cmd.exe".to_string(), a)
            }
        }
    };

    #[cfg(not(windows))]
    let (exe, args) = (npx_plain.clone(), hf_args);

    let mut command = CommandSpec::new(exe, args, cwd_plain);

    // Pass essential system paths to ensure Node, Chrome, and Puppeteer operate correctly
    // within the sanitized process environment.
    for var in [
        "ProgramFiles",
        "ProgramFiles(x86)",
        "CommonProgramFiles",
        "CommonProgramFiles(x86)",
        "CHROME_PATH",
        "PUPPETEER_EXECUTABLE_PATH",
        "STUDIO_CHROME",
    ] {
        if let Ok(val) = std::env::var(var) {
            command.env.insert(var.into(), val);
        }
    }

    let resources = ResourceSpec {
        ram_mb: 2048,
        cpu: CpuClass::Heavy,
        gpu_vram_mb: 0,
        exclusive: Vec::new(),
        rate_key: None,
    };

    Ok(JobSpec {
        id: job_id.to_string(),
        workspace_id: workspace_id.to_string(),
        tool_id: "render.html_video@1".into(),
        command,
        resources,
        priority: Priority::TurnBlocking,
        // HyperFrames spawns Chrome and encodes with FFmpeg — give it plenty of time.
        timeout: Duration::from_secs(600),
        memory_limit_mb: Some(3072),
        lifecycle: Lifecycle::PerCall,
    })
}

/// Strip the Windows verbatim prefix (`\\?\` or `\\?\UNC\`) from a path and return the
/// plain string. On non-Windows this is a no-op.
pub fn strip_verbatim(path: &Path) -> String {
    let raw = path.to_string_lossy();
    if let Some(rest) = raw.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = raw.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        raw.into_owned()
    }
}

/// `color:#RRGGBB` (or `color:RRGGBB`) clip source → `RRGGBB`; anything else (or an invalid hex) → `None`.
/// Only 6 hex digits are accepted so the value is safe to splice into a lavfi graph.
pub fn color_source_hex(src: &str) -> Option<String> {
    let hex = src.strip_prefix("color:")?.trim().trim_start_matches('#');
    (hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit())).then(|| hex.to_ascii_lowercase())
}

/// Helper to determine if a source path references a static image format.
pub fn is_image_source(src: &str) -> bool {
    let lower = src.to_ascii_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".webp")
        || lower.ends_with(".bmp")
}

/// Helper to determine if a source path references a video file.
pub fn is_video_source(src: &str) -> bool {
    let lower = src.to_ascii_lowercase();
    lower.ends_with(".mp4")
        || lower.ends_with(".mov")
        || lower.ends_with(".webm")
        || lower.ends_with(".mkv")
        || lower.ends_with(".avi")
        || lower.ends_with(".m4v")
        || lower.ends_with(".ts")
        || lower.ends_with(".flv")
}

/// Helper to determine if a source path references a subtitle file (SRT, VTT, ASS).
pub fn is_subtitle_file(src: &str) -> bool {
    let lower = src.to_ascii_lowercase();
    lower.ends_with(".srt") || lower.ends_with(".vtt") || lower.ends_with(".ass")
}

/// Escape a path for use in FFmpeg `subtitles='...'` filter on Windows and Unix.
///
/// Strips verbatim prefix `\\?\`, replaces Windows backslashes with forward slashes `/`,
/// escapes the Windows drive colon (`C:` -> `C\:`), and escapes single quotes and backslashes.
pub fn escape_ffmpeg_subtitles_path(path: &Path) -> String {
    let plain = strip_verbatim(path);
    let normalized = plain.replace('\\', "/");
    let escaped_colon = if normalized.len() >= 2
        && normalized.as_bytes()[1] == b':'
        && normalized.as_bytes()[0].is_ascii_alphabetic()
    {
        format!("{}\\{}", &normalized[..1], &normalized[1..])
    } else {
        normalized
    };
    escaped_colon.replace('\'', "'\\''")
}

/// Escape special characters in text for FFmpeg `drawtext=text='...'`.
///
/// Escapes `\`, `:`, `%`, and `,` per FFmpeg filtergraph and drawtext rules. A straight quote
/// cannot be escaped inside the quoted value (it ends the value and breaks the whole graph), so it
/// becomes the typographic apostrophe.
pub fn escape_drawtext_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str(r"\\"),
            '\'' => out.push('\u{2019}'),
            ':' => out.push_str(r"\:"),
            '%' => out.push_str(r"\%"),
            ',' => out.push_str(r"\,"),
            _ => out.push(ch),
        }
    }
    out
}

/// Whether the local FFmpeg's drawtext has `text_align` (FFmpeg 6.1+), used to center each
/// caption line; older builds reject unknown options, so it is only added when present.
pub fn drawtext_supports_text_align() -> bool {
    static SUPPORTED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *SUPPORTED.get_or_init(|| {
        let Some(ffmpeg) = crate::probe::probe_ffmpeg().path else {
            return false;
        };
        std::process::Command::new(ffmpeg)
            .args(["-hide_banner", "-h", "filter=drawtext"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("text_align"))
            .unwrap_or(false)
    })
}

/// Resolve a font file for captions with Vietnamese glyph support.
///
/// On Windows, searches `C:/Windows/Fonts`:
/// - If `style_font` names a font, checks common filenames (e.g. Segoe UI -> segoeui.ttf, Arial -> arial.ttf).
/// - If not found or not specified, defaults to `segoeui.ttf` (or `segoeuib.ttf` if bold was requested),
///   falling back to `arial.ttf`.
///
/// Returns the escaped path string suitable for FFmpeg `fontfile='...'`, or None if not found.
pub fn resolve_caption_font(style_font: Option<&str>) -> Option<String> {
    #[cfg(windows)]
    {
        let fonts_dir = Path::new("C:/Windows/Fonts");
        if fonts_dir.is_dir() {
            if let Some(font_name) = style_font {
                let trimmed = font_name.trim();
                let lower = trimmed.to_ascii_lowercase();
                let is_bold =
                    lower.contains("bold") || lower.ends_with(" b") || lower.ends_with("bd");
                let stripped = lower.replace("bold", "").replace([' ', '-', '_'], "");

                let mut candidates = Vec::new();
                if trimmed.ends_with(".ttf") || trimmed.ends_with(".otf") {
                    candidates.push(trimmed.to_string());
                }
                if is_bold {
                    candidates.push(format!("{stripped}b.ttf"));
                    candidates.push(format!("{stripped}bd.ttf"));
                    candidates.push(format!("{stripped}_bold.ttf"));
                }
                candidates.push(format!("{stripped}.ttf"));
                candidates.push(format!("{stripped}.otf"));

                for candidate in candidates {
                    let p = fonts_dir.join(&candidate);
                    if p.is_file() {
                        return Some(escape_ffmpeg_subtitles_path(&p));
                    }
                }
            }

            let segoe = fonts_dir.join("segoeui.ttf");
            if segoe.is_file() {
                return Some(escape_ffmpeg_subtitles_path(&segoe));
            }
            let arial = fonts_dir.join("arial.ttf");
            if arial.is_file() {
                return Some(escape_ffmpeg_subtitles_path(&arial));
            }
        }
    }

    #[cfg(not(windows))]
    {
        for dir in [
            "/usr/share/fonts",
            "/usr/local/share/fonts",
            "/System/Library/Fonts",
            "/Library/Fonts",
        ] {
            let p = Path::new(dir);
            if p.is_dir() {
                for candidate in ["DejaVuSans.ttf", "LiberationSans-Regular.ttf", "Arial.ttf"] {
                    let font_file = p.join(candidate);
                    if font_file.is_file() {
                        return Some(escape_ffmpeg_subtitles_path(&font_file));
                    }
                }
            }
        }
    }

    None
}

/// Measurement statistics from FFmpeg loudnorm pass 1 (`print_format=json`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LoudnormMeasurement {
    pub input_i: String,
    pub input_tp: String,
    pub input_lra: String,
    pub input_thresh: String,
    pub target_offset: String,
}

/// Parse loudnorm JSON measurement output from FFmpeg stderr.
///
/// FFmpeg loudnorm prints a JSON object containing `input_i`, `input_tp`, `input_lra`,
/// `input_thresh`, and `target_offset` to stderr. This function finds the JSON block
/// even when surrounded by other FFmpeg log lines.
pub fn parse_loudnorm_measurement(stderr: &str) -> Option<LoudnormMeasurement> {
    for (start, _) in stderr.rmatch_indices('{') {
        if let Some(end) = stderr[start..].find('}') {
            let json_str = &stderr[start..=start + end];
            if json_str.contains("input_i") && json_str.contains("target_offset") {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
                    let val_str = |key: &str| -> Option<String> {
                        match v.get(key)? {
                            serde_json::Value::String(s) => Some(s.clone()),
                            serde_json::Value::Number(n) => Some(n.to_string()),
                            _ => None,
                        }
                    };
                    let input_i = val_str("input_i")?;
                    let input_tp = val_str("input_tp")?;
                    let input_lra = val_str("input_lra")?;
                    let input_thresh = val_str("input_thresh")?;
                    let target_offset = val_str("target_offset")?;
                    return Some(LoudnormMeasurement {
                        input_i,
                        input_tp,
                        input_lra,
                        input_thresh,
                        target_offset,
                    });
                }
            }
        }
    }
    None
}

/// Format the loudnorm filter string for the render pass.
///
/// If a measurement is provided, formats two-pass loudnorm:
/// `[amix]loudnorm=I={lufs}:TP=-1.5:LRA=11:measured_I={input_i}:measured_TP={input_tp}:measured_LRA={input_lra}:measured_thresh={input_thresh}:offset={target_offset}:linear=true,aresample=48000[a]`
/// Otherwise, falls back to single-pass dynamic loudnorm:
/// `[amix]loudnorm=I={lufs}:TP=-1.5:LRA=11,aresample=48000[a]`
pub fn format_loudnorm_filter(lufs: f64, measurement: Option<&LoudnormMeasurement>) -> String {
    if let Some(m) = measurement {
        format!(
            "[amix]loudnorm=I={lufs}:TP=-1.5:LRA=11:measured_I={}:measured_TP={}:measured_LRA={}:measured_thresh={}:offset={}:linear=true,aresample=48000[a]",
            m.input_i, m.input_tp, m.input_lra, m.input_thresh, m.target_offset
        )
    } else {
        format!("[amix]loudnorm=I={lufs}:TP=-1.5:LRA=11,aresample=48000[a]")
    }
}

/// Build FFmpeg arguments for pass 1 audio loudness measurement.
///
/// Takes only the audio clips from the timeline, builds the `atrim/adelay/amix` chain,
/// attaches `loudnorm=I={lufs}:TP=-1.5:LRA=11:print_format=json`, and outputs to `-f null -`.
pub fn build_loudnorm_measure_ffmpeg_args(
    timeline_doc: &crate::timeline::TimelineDoc,
) -> Option<Vec<String>> {
    let lufs = timeline_doc.loudness_lufs?;

    let mut audio_clips = Vec::new();
    for track in timeline_doc
        .tracks
        .iter()
        .filter(|t| t.kind == crate::timeline::TrackKind::Audio)
    {
        for clip in &track.clips {
            audio_clips.push(clip);
        }
    }

    if audio_clips.is_empty() {
        return None;
    }

    let mut args = vec!["-y".into()];
    for clip in &audio_clips {
        args.extend_from_slice(&["-i".into(), strip_verbatim(Path::new(&clip.source))]);
    }

    let mix_out = "amix";
    let mut a_filter_parts = Vec::new();
    for (j, clip) in audio_clips.iter().enumerate() {
        let atrim = format!(
            "[{}:a]atrim=start={}:duration={},asetpts=PTS-STARTPTS",
            j, clip.in_point, clip.duration
        );
        let start_ms = (clip.start * 1000.0).round() as u64;
        if audio_clips.len() == 1 {
            if start_ms > 0 {
                a_filter_parts.push(format!(
                    "{},adelay={}|{}[{}]",
                    atrim, start_ms, start_ms, mix_out
                ));
            } else {
                a_filter_parts.push(format!("{}[{}]", atrim, mix_out));
            }
        } else {
            if start_ms > 0 {
                a_filter_parts.push(format!(
                    "{},adelay={}|{}[a{}]",
                    atrim, start_ms, start_ms, j
                ));
            } else {
                a_filter_parts.push(format!("{}[a{}]", atrim, j));
            }
        }
    }

    if audio_clips.len() > 1 {
        let mut mix_inputs = String::new();
        for j in 0..audio_clips.len() {
            mix_inputs.push_str(&format!("[a{}]", j));
        }
        a_filter_parts.push(format!(
            "{}amix=inputs={}:duration=longest:dropout_transition=0[{}]",
            mix_inputs,
            audio_clips.len(),
            mix_out
        ));
    }

    a_filter_parts.push(format!(
        "[amix]loudnorm=I={lufs}:TP=-1.5:LRA=11:print_format=json"
    ));

    let filter_graph = a_filter_parts.join(";");
    args.extend_from_slice(&[
        "-vn".into(),
        "-filter_complex".into(),
        filter_graph,
        "-f".into(),
        "null".into(),
        "-".into(),
    ]);

    Some(args)
}

/// Run pass 1 audio loudness measurement using FFmpeg.
///
/// Returns `Some(LoudnormMeasurement)` if audio exists and measurement succeeds.
/// If measurement fails or stderr cannot be parsed, logs a warning and returns `None`.
pub fn measure_audio_loudnorm(
    ffmpeg_path: &Path,
    timeline_doc: &crate::timeline::TimelineDoc,
    cwd: &Path,
) -> Option<LoudnormMeasurement> {
    let args = build_loudnorm_measure_ffmpeg_args(timeline_doc)?;
    let mut cmd = std::process::Command::new(ffmpeg_path);
    cmd.args(&args).current_dir(cwd);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let output = match cmd.output() {
        Ok(out) => out,
        Err(e) => {
            tracing::warn!("Không thể chạy ffmpeg đo âm lượng pass 1: {e}");
            return None;
        }
    };

    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        tracing::warn!(
            "FFmpeg đo âm lượng pass 1 thất bại (mã lỗi {:?}): {}",
            output.status.code(),
            stderr.chars().take(300).collect::<String>()
        );
        return None;
    }

    match parse_loudnorm_measurement(&stderr) {
        Some(m) => Some(m),
        None => {
            tracing::warn!(
                "Không parse được JSON loudnorm từ ffmpeg stderr: {}",
                stderr.chars().take(300).collect::<String>()
            );
            None
        }
    }
}

/// Build an FFmpeg filtergraph and arguments for rendering a `timeline.json` document to MP4 H.264/AAC.
pub fn build_timeline_ffmpeg_args(
    timeline_doc: &crate::timeline::TimelineDoc,
    output_file: &Path,
) -> Result<(Vec<String>, String)> {
    build_timeline_ffmpeg_args_measured(timeline_doc, output_file, None)
}

/// Build an FFmpeg filtergraph and arguments for rendering a `timeline.json` document to MP4 H.264/AAC,
/// with optional pass 1 loudnorm measurement for two-pass loudness normalization.
pub fn build_timeline_ffmpeg_args_measured(
    timeline_doc: &crate::timeline::TimelineDoc,
    output_file: &Path,
    loudnorm: Option<&LoudnormMeasurement>,
) -> Result<(Vec<String>, String)> {
    // 0. Check for SVG sources: ffmpeg usually cannot decode them — reject with a clear Vietnamese error
    for track in timeline_doc
        .tracks
        .iter()
        .filter(|t| t.kind == crate::timeline::TrackKind::Video)
    {
        for clip in &track.clips {
            if clip.source.to_ascii_lowercase().ends_with(".svg") {
                return Err(ArtifactError::InvalidConfig(format!(
                    "Nguồn video/overlay '{}' có định dạng SVG. FFmpeg không thể giải mã trực tiếp định dạng SVG; vui lòng render file SVG sang PNG thông qua design artifact trước khi đưa vào timeline.",
                    clip.source
                )));
            }
        }
    }

    let mut args = vec!["-y".into()];

    // Separate video tracks, audio clips, and caption clips
    let mut video_tracks = Vec::new();
    let mut audio_clips = Vec::new();
    let mut caption_entries = Vec::new();

    for track in &timeline_doc.tracks {
        match track.kind {
            crate::timeline::TrackKind::Video => {
                video_tracks.push(track);
            }
            crate::timeline::TrackKind::Audio => {
                for clip in &track.clips {
                    audio_clips.push(clip);
                }
            }
            crate::timeline::TrackKind::Captions | crate::timeline::TrackKind::Caption => {
                for clip in &track.clips {
                    caption_entries.push((track.style.as_ref(), clip));
                }
            }
        }
    }

    let (base_clips, overlay_tracks) = if let Some((first, rest)) = video_tracks.split_first() {
        (&first.clips[..], rest)
    } else {
        (&[][..], &[][..])
    };

    let mut v_filter_parts = Vec::new();
    let mut total_video_inputs;

    // 1. Build base video sequence
    if base_clips.is_empty() {
        args.extend_from_slice(&[
            "-f".into(),
            "lavfi".into(),
            "-i".into(),
            format!(
                "color=c=black:s={}x{}:d=5:r={}",
                timeline_doc.width, timeline_doc.height, timeline_doc.fps
            ),
        ]);
        total_video_inputs = 1;
        v_filter_parts.push(format!(
            "[0:v]fps={},scale={}:{}[v]",
            timeline_doc.fps, timeline_doc.width, timeline_doc.height
        ));
    } else {
        // Register each base video clip input (`color:#RRGGBB` = solid colour generated by lavfi)
        for clip in base_clips {
            if let Some(hex) = color_source_hex(&clip.source) {
                args.extend_from_slice(&[
                    "-f".into(),
                    "lavfi".into(),
                    "-i".into(),
                    format!(
                        "color=c=0x{}:s={}x{}:r={}:d={}",
                        hex,
                        timeline_doc.width,
                        timeline_doc.height,
                        timeline_doc.fps,
                        clip.in_point + clip.duration
                    ),
                ]);
            } else if is_video_source(&clip.source) {
                args.extend_from_slice(&[
                    "-stream_loop".into(),
                    "-1".into(),
                    "-i".into(),
                    strip_verbatim(Path::new(&clip.source)),
                ]);
            } else {
                args.extend_from_slice(&["-i".into(), strip_verbatim(Path::new(&clip.source))]);
            }
        }
        total_video_inputs = base_clips.len();

        for (i, clip) in base_clips.iter().enumerate() {
            let filter = if is_image_source(&clip.source)
                && clip.motion.is_some()
                && clip.motion != Some(crate::timeline::MotionKind::None)
            {
                let motion = match clip.motion.unwrap_or_default() {
                    crate::timeline::MotionKind::Auto => crate::timeline::resolve_auto_motion(i),
                    m => m,
                };
                // `perspective` samples sub-pixel corners; zoompan rounds its crop to whole pixels,
                // which makes slow zooms and pans visibly wobble.
                use crate::timeline::MotionKind as M;
                let fps = timeline_doc.fps;
                let frames = ((clip.duration * fps).round() as u64).max(2);
                let p = format!("(in/{})", frames - 1);
                let s = clip.motion_strength.unwrap_or(0.08).clamp(0.001, 0.5);
                let z = match motion {
                    M::ZoomIn => format!("(1+{s:.4}*{p})"),
                    M::ZoomOut => format!("(1+{s:.4}*(1-{p}))"),
                    _ => format!("{:.4}", 1.0 + s),
                };
                let centered_x = format!("(W-W/{z})/2");
                let centered_y = format!("(H-H/{z})/2");
                let (left, top) = match motion {
                    M::PanLeft => (format!("(W-W/{z})*(1-{p})"), centered_y),
                    M::PanRight => (format!("(W-W/{z})*{p}"), centered_y),
                    M::PanUp => (centered_x, format!("(H-H/{z})*(1-{p})")),
                    M::PanDown => (centered_x, format!("(H-H/{z})*{p}")),
                    _ => (centered_x, centered_y),
                };
                let right = format!("{left}+W/{z}");
                let bottom = format!("{top}+H/{z}");
                let mut f = format!(
                    "[{i}:v]scale={w}:{h},setsar=1,loop=loop=-1:size=1:start=0,fps={fps},trim=start=0:duration={dur},setpts=PTS-STARTPTS,perspective=x0='{left}':y0='{top}':x1='{right}':y1='{top}':x2='{left}':y2='{bottom}':x3='{right}':y3='{bottom}':interpolation=cubic:eval=frame",
                    w = timeline_doc.width,
                    h = timeline_doc.height,
                    dur = clip.duration,
                );
                if clip.transition == crate::timeline::Transition::Fade {
                    let fade_dur = clip.transition_duration.unwrap_or(0.5);
                    f.push_str(&format!(",fade=t=in:st=0:d={fade_dur}"));
                }
                f
            } else {
                let loop_prefix = if is_image_source(&clip.source) {
                    "loop=loop=-1:size=1:start=0,"
                } else {
                    ""
                };
                let mut f = format!(
                    "[{}:v]{}trim=start={}:duration={},setpts=PTS-STARTPTS,fps={},scale={}:{},setsar=1",
                    i,
                    loop_prefix,
                    clip.in_point,
                    clip.duration,
                    timeline_doc.fps,
                    timeline_doc.width,
                    timeline_doc.height
                );
                if clip.transition == crate::timeline::Transition::Fade {
                    let fade_dur = clip.transition_duration.unwrap_or(0.5);
                    f.push_str(&format!(",fade=t=in:st=0:d={}", fade_dur));
                }
                f
            };
            let label = format!("v{}", i);
            v_filter_parts.push(format!("{}[{}]", filter, label));
        }

        let mut concat_inputs = String::new();
        for i in 0..base_clips.len() {
            concat_inputs.push_str(&format!("[v{}]", i));
        }
        let concat_str = format!("{}concat=n={}:v=1:a=0[v]", concat_inputs, base_clips.len());
        v_filter_parts.push(concat_str);
    }

    // 2. Build overlay tracks compositing
    let mut overlay_idx = 0;
    for track in overlay_tracks {
        for clip in &track.clips {
            let input_idx = total_video_inputs;
            total_video_inputs += 1;

            if let Some(hex) = color_source_hex(&clip.source) {
                args.extend_from_slice(&[
                    "-f".into(),
                    "lavfi".into(),
                    "-i".into(),
                    format!(
                        "color=c=0x{}:s={}x{}:r={}:d={}",
                        hex,
                        clip.width.unwrap_or(timeline_doc.width),
                        clip.height.unwrap_or(timeline_doc.height),
                        timeline_doc.fps,
                        clip.in_point + clip.duration
                    ),
                ]);
            } else if is_video_source(&clip.source) {
                args.extend_from_slice(&[
                    "-stream_loop".into(),
                    "-1".into(),
                    "-i".into(),
                    strip_verbatim(Path::new(&clip.source)),
                ]);
            } else {
                args.extend_from_slice(&["-i".into(), strip_verbatim(Path::new(&clip.source))]);
            }

            let loop_prefix = if is_image_source(&clip.source) {
                "loop=loop=-1:size=1:start=0,"
            } else {
                ""
            };

            let scale_filter = match (clip.width, clip.height) {
                (Some(w), Some(h)) => format!(",scale={w}:{h}"),
                (Some(w), None) => format!(",scale={w}:-1"),
                (None, Some(h)) => format!(",scale=-1:{h}"),
                (None, None) => String::new(),
            };

            let opacity_filter = match clip.opacity {
                Some(op) if op < 1.0 => format!(",format=rgba,colorchannelmixer=aa={op}"),
                _ => String::new(),
            };

            let ov_label = format!("ov{}", overlay_idx);
            let ov_filter = format!(
                "[{input_idx}:v]{loop_prefix}trim=start={}:duration={},setpts=PTS-STARTPTS+{}/TB,fps={}{scale_filter}{opacity_filter}[{ov_label}]",
                clip.in_point,
                clip.duration,
                clip.start,
                timeline_doc.fps
            );
            v_filter_parts.push(ov_filter);

            let x = clip.x.unwrap_or(0);
            let y = clip.y.unwrap_or(0);
            let end = clip.start + clip.duration;
            let composite = format!(
                "[v][{ov_label}]overlay=x={x}:y={y}:enable='between(t,{start},{end})':eof_action=pass[v]",
                start = clip.start
            );
            v_filter_parts.push(composite);
            overlay_idx += 1;
        }
    }

    // 3. Burn in captions if any
    for (style_opt, c) in &caption_entries {
        if is_subtitle_file(&c.source) {
            let escaped_path = escape_ffmpeg_subtitles_path(Path::new(&c.source));
            let sub_filter = format!("[v]subtitles='{}'[v]", escaped_path);
            v_filter_parts.push(sub_filter);
        } else {
            let fontfile_part =
                match resolve_caption_font(style_opt.and_then(|s| s.font.as_deref())) {
                    Some(ref path) => format!("fontfile='{}':", path),
                    None => String::new(),
                };

            let font_size = style_opt
                .and_then(|s| s.font_size)
                .unwrap_or_else(|| ((timeline_doc.height as f64) * 0.045).round() as u32);
            let margin_bottom = style_opt
                .and_then(|s| s.margin_bottom)
                .unwrap_or_else(|| ((timeline_doc.height as f64) * 0.08).round() as u32);

            let text_color_raw = style_opt
                .and_then(|s| s.color.as_deref())
                .unwrap_or("white");
            let font_color = if let Some(hex) = text_color_raw.strip_prefix('#') {
                format!("0x{}", hex)
            } else {
                text_color_raw.to_string()
            };

            let is_box = style_opt.and_then(|s| s.is_box).unwrap_or(true);
            let box_part = if is_box {
                let box_color_raw = style_opt
                    .and_then(|s| s.box_color.as_deref())
                    .unwrap_or("black@0.6");
                let box_color = if let Some(hex) = box_color_raw.strip_prefix('#') {
                    format!("0x{}", hex)
                } else {
                    box_color_raw.to_string()
                };
                let box_border_w = (font_size / 5).max(8);
                format!(":box=1:boxcolor={box_color}:boxborderw={box_border_w}")
            } else {
                ":box=0".to_string()
            };

            let outline_part = match style_opt.and_then(|s| s.outline) {
                Some(w) if w > 0 => format!(":borderw={w}:bordercolor=black"),
                _ => String::new(),
            };

            let line_spacing = (font_size / 6).max(4);
            let escaped_text = escape_drawtext_text(&c.source);

            let align_part = if drawtext_supports_text_align() {
                ":text_align=C"
            } else {
                ""
            };
            let text_filter = format!(
                "[v]drawtext={fontfile_part}text='{escaped_text}':expansion=none{align_part}:enable='between(t,{},{})':fontsize={font_size}:fontcolor={font_color}{box_part}{outline_part}:line_spacing={line_spacing}:x=(w-text_w)/2:y=h-text_h-{margin_bottom}[v]",
                c.start,
                c.start + c.duration
            );
            v_filter_parts.push(text_filter);
        }
    }

    let mut filter_graph = v_filter_parts.join(";");

    // 4. Build audio pipeline
    if !audio_clips.is_empty() {
        let audio_input_start = total_video_inputs;
        for clip in &audio_clips {
            args.extend_from_slice(&["-i".into(), strip_verbatim(Path::new(&clip.source))]);
        }

        // With a loudness target the mix ends in [amix] and loudnorm produces [a].
        let mix_out = if timeline_doc.loudness_lufs.is_some() {
            "amix"
        } else {
            "a"
        };
        let mut a_filter_parts = Vec::new();
        for (j, clip) in audio_clips.iter().enumerate() {
            let in_idx = audio_input_start + j;
            let atrim = format!(
                "[{}:a]atrim=start={}:duration={},asetpts=PTS-STARTPTS",
                in_idx, clip.in_point, clip.duration
            );
            let start_ms = (clip.start * 1000.0).round() as u64;
            if audio_clips.len() == 1 {
                if start_ms > 0 {
                    a_filter_parts.push(format!(
                        "{},adelay={}|{}[{}]",
                        atrim, start_ms, start_ms, mix_out
                    ));
                } else {
                    a_filter_parts.push(format!("{}[{}]", atrim, mix_out));
                }
            } else {
                if start_ms > 0 {
                    a_filter_parts.push(format!(
                        "{},adelay={}|{}[a{}]",
                        atrim, start_ms, start_ms, j
                    ));
                } else {
                    a_filter_parts.push(format!("{}[a{}]", atrim, j));
                }
            }
        }

        if audio_clips.len() > 1 {
            let mut mix_inputs = String::new();
            for j in 0..audio_clips.len() {
                mix_inputs.push_str(&format!("[a{}]", j));
            }
            a_filter_parts.push(format!(
                "{}amix=inputs={}:duration=longest:dropout_transition=0[{}]",
                mix_inputs,
                audio_clips.len(),
                mix_out
            ));
        }

        if let Some(lufs) = timeline_doc.loudness_lufs {
            a_filter_parts.push(format_loudnorm_filter(lufs, loudnorm));
        }

        filter_graph.push(';');
        filter_graph.push_str(&a_filter_parts.join(";"));
    }

    args.extend_from_slice(&["-filter_complex".into(), filter_graph.clone()]);
    args.extend_from_slice(&[
        "-map".into(),
        "[v]".into(),
        "-c:v".into(),
        "libx264".into(),
        "-pix_fmt".into(),
        "yuv420p".into(),
    ]);

    if !audio_clips.is_empty() {
        args.extend_from_slice(&[
            "-map".into(),
            "[a]".into(),
            "-c:a".into(),
            "aac".into(),
            "-b:a".into(),
            "192k".into(),
        ]);
    }

    args.push(strip_verbatim(output_file));

    Ok((args, filter_graph))
}

/// Calculate conservative baseline RAM limit (in MB) for timeline rendering
/// based on frame dimensions and frame rate.
///
/// - <= 720p (e.g. 1280x720): 512 MB
/// - <= 1080p (e.g. 1920x1080): 768 MB
/// - > 1080p (e.g. 4K / 3840x2160): 1024 MB
pub fn calculate_timeline_ram_mb(width: u32, height: u32, _fps: f64) -> u64 {
    let pixels = (width as u64) * (height as u64);
    if pixels <= 1280 * 720 {
        512
    } else if pixels <= 1920 * 1080 {
        768
    } else {
        1024
    }
}

/// Escape a file path for an FFmpeg concat demuxer file (`concat_list.txt`).
/// Normalizes backslashes to forward slashes and escapes single quotes.
pub fn escape_concat_path(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    s.replace('\'', "'\\''")
}

/// Compute deterministic cache key for a single timeline base clip.
pub fn compute_clip_chunk_hash(
    clip: &crate::timeline::Clip,
    clip_idx: usize,
    doc: &crate::timeline::TimelineDoc,
    cwd: &Path,
) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(doc.width.to_le_bytes());
    hasher.update(doc.height.to_le_bytes());
    hasher.update(doc.fps.to_bits().to_le_bytes());
    hasher.update((clip_idx as u64).to_le_bytes());
    hasher.update(clip.source.as_bytes());
    hasher.update(clip.in_point.to_bits().to_le_bytes());
    hasher.update(clip.duration.to_bits().to_le_bytes());
    if let Some(m) = clip.motion {
        let motion_str = format!("{:?}", m);
        hasher.update(motion_str.as_bytes());
    }
    if let Some(s) = clip.motion_strength {
        hasher.update(s.to_bits().to_le_bytes());
    }
    let trans_str = format!("{:?}", clip.transition);
    hasher.update(trans_str.as_bytes());
    if let Some(td) = clip.transition_duration {
        hasher.update(td.to_bits().to_le_bytes());
    }
    let p = cwd.join(&clip.source);
    if let Ok(meta) = std::fs::metadata(&p) {
        hasher.update(meta.len().to_le_bytes());
        if let Ok(mtime) = meta.modified() {
            if let Ok(dur) = mtime.duration_since(std::time::UNIX_EPOCH) {
                hasher.update(dur.as_millis().to_le_bytes());
            }
        }
    }
    let h = hex::encode(hasher.finalize());
    h[..16].to_string()
}

/// Build FFmpeg command-line arguments to render a single base clip into a standardized intermediate chunk MP4.
pub fn build_clip_chunk_ffmpeg_args(
    clip: &crate::timeline::Clip,
    clip_idx: usize,
    doc: &crate::timeline::TimelineDoc,
    out_chunk: &Path,
) -> (Vec<String>, String) {
    let mut args = vec!["-y".into()];
    let dur = clip.duration;
    let fps = doc.fps;
    let w = doc.width;
    let h = doc.height;
    let mut filter = String::new();

    if let Some(hex) = color_source_hex(&clip.source) {
        args.extend_from_slice(&[
            "-f".into(),
            "lavfi".into(),
            "-i".into(),
            format!("color=c=0x{}:s={}x{}:r={}:d={}", hex, w, h, fps, dur),
        ]);
        if clip.transition == crate::timeline::Transition::Fade {
            let fade_dur = clip.transition_duration.unwrap_or(0.5);
            filter = format!("fade=t=in:st=0:d={}", fade_dur);
        }
    } else if is_image_source(&clip.source)
        && clip.motion.is_some()
        && clip.motion != Some(crate::timeline::MotionKind::None)
    {
        args.extend_from_slice(&["-i".into(), strip_verbatim(Path::new(&clip.source))]);
        use crate::timeline::MotionKind as M;
        let motion = match clip.motion.unwrap_or_default() {
            crate::timeline::MotionKind::Auto => crate::timeline::resolve_auto_motion(clip_idx),
            m => m,
        };
        let frames = ((dur * fps).round() as u64).max(2);
        let p = format!("(in/{})", frames - 1);
        let s = clip.motion_strength.unwrap_or(0.08).clamp(0.001, 0.5);
        let z = match motion {
            M::ZoomIn => format!("(1+{s:.4}*{p})"),
            M::ZoomOut => format!("(1+{s:.4}*(1-{p}))"),
            _ => format!("{:.4}", 1.0 + s),
        };
        let centered_x = format!("(W-W/{z})/2");
        let centered_y = format!("(H-H/{z})/2");
        let (left, top) = match motion {
            M::PanLeft => (format!("(W-W/{z})*(1-{p})"), centered_y),
            M::PanRight => (format!("(W-W/{z})*{p}"), centered_y),
            M::PanUp => (centered_x, format!("(H-H/{z})*(1-{p})")),
            M::PanDown => (centered_x, format!("(H-H/{z})*{p}")),
            _ => (centered_x, centered_y),
        };
        let right = format!("{left}+W/{z}");
        let bottom = format!("{top}+H/{z}");
        let mut f = format!(
            "scale={w}:{h},setsar=1,loop=loop=-1:size=1:start=0,fps={fps},trim=start=0:duration={dur},setpts=PTS-STARTPTS,perspective=x0='{left}':y0='{top}':x1='{right}':y1='{top}':x2='{left}':y2='{bottom}':x3='{right}':y3='{bottom}':interpolation=cubic:eval=frame",
        );
        if clip.transition == crate::timeline::Transition::Fade {
            let fade_dur = clip.transition_duration.unwrap_or(0.5);
            f.push_str(&format!(",fade=t=in:st=0:d={fade_dur}"));
        }
        filter = f;
    } else {
        let is_img = is_image_source(&clip.source);
        if is_img {
            args.extend_from_slice(&["-i".into(), strip_verbatim(Path::new(&clip.source))]);
        } else {
            args.extend_from_slice(&[
                "-stream_loop".into(),
                "-1".into(),
                "-i".into(),
                strip_verbatim(Path::new(&clip.source)),
            ]);
        }
        let loop_prefix = if is_img {
            "loop=loop=-1:size=1:start=0,"
        } else {
            ""
        };
        let mut f = format!(
            "{loop_prefix}trim=start={}:duration={dur},setpts=PTS-STARTPTS,fps={fps},scale={w}:{h},setsar=1",
            clip.in_point
        );
        if clip.transition == crate::timeline::Transition::Fade {
            let fade_dur = clip.transition_duration.unwrap_or(0.5);
            f.push_str(&format!(",fade=t=in:st=0:d={fade_dur}"));
        }
        filter = f;
    }

    if !filter.is_empty() {
        args.extend_from_slice(&["-vf".into(), filter.clone()]);
    }

    args.extend_from_slice(&[
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        "veryfast".into(),
        "-crf".into(),
        "17".into(),
        "-pix_fmt".into(),
        "yuv420p".into(),
        "-r".into(),
        format!("{}", fps),
        "-video_track_timescale".into(),
        "90000".into(),
        "-an".into(),
        "-threads".into(),
        "4".into(),
        "-flags".into(),
        "+bitexact".into(),
        "-fflags".into(),
        "+bitexact".into(),
        "-t".into(),
        format!("{}", dur),
        strip_verbatim(out_chunk),
    ]);

    (args, filter)
}

/// Render a single base clip chunk to disk atomically if not already cached.
pub fn render_base_clip_chunk(
    ffmpeg_path: &Path,
    clip: &crate::timeline::Clip,
    clip_idx: usize,
    doc: &crate::timeline::TimelineDoc,
    cwd: &Path,
    out_chunk: &Path,
) -> Result<()> {
    if out_chunk.exists()
        && std::fs::metadata(out_chunk)
            .map(|m| m.len() > 1000)
            .unwrap_or(false)
    {
        return Ok(());
    }
    if let Some(parent) = out_chunk.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let temp_chunk = out_chunk.with_file_name(format!(
        "{}.tmp.{}.mp4",
        out_chunk.file_stem().unwrap_or_default().to_string_lossy(),
        std::process::id()
    ));
    let (args, _) = build_clip_chunk_ffmpeg_args(clip, clip_idx, doc, &temp_chunk);
    let mut cmd = std::process::Command::new(ffmpeg_path);
    cmd.args(&args).current_dir(cwd);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let output = cmd.output().map_err(|e| ArtifactError::ExecutionFailed {
        message: format!("Không thể khởi chạy ffmpeg render chunk: {e}"),
        details: None,
    })?;
    if !output.status.success() {
        let _ = std::fs::remove_file(&temp_chunk);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ArtifactError::ExecutionFailed {
            message: format!(
                "FFmpeg render chunk thất bại (mã lỗi {:?})",
                output.status.code()
            ),
            details: Some(stderr.chars().take(500).collect::<String>()),
        });
    }
    std::fs::rename(&temp_chunk, out_chunk).map_err(|e| {
        let _ = std::fs::remove_file(&temp_chunk);
        ArtifactError::Io(e)
    })?;
    Ok(())
}

/// Build FFmpeg filtergraph and arguments for assembling base chunks from a concat demuxer file
/// while applying overlay tracks, burned-in captions, and mixed/normalized audio.
pub fn build_timeline_concat_ffmpeg_args(
    timeline_doc: &crate::timeline::TimelineDoc,
    concat_list_file: &Path,
    output_file: &Path,
) -> Result<(Vec<String>, String)> {
    build_timeline_concat_ffmpeg_args_measured(timeline_doc, concat_list_file, output_file, None)
}

/// Build FFmpeg filtergraph and arguments for assembling base chunks from a concat demuxer file
/// with optional pass 1 loudnorm measurement for two-pass loudness normalization.
pub fn build_timeline_concat_ffmpeg_args_measured(
    timeline_doc: &crate::timeline::TimelineDoc,
    concat_list_file: &Path,
    output_file: &Path,
    loudnorm: Option<&LoudnormMeasurement>,
) -> Result<(Vec<String>, String)> {
    let mut args = vec!["-y".into()];
    args.extend_from_slice(&[
        "-threads".into(),
        "4".into(),
        "-f".into(),
        "concat".into(),
        "-safe".into(),
        "0".into(),
        "-i".into(),
        strip_verbatim(concat_list_file),
    ]);

    let mut video_tracks = Vec::new();
    let mut audio_clips = Vec::new();
    let mut caption_entries = Vec::new();

    for track in &timeline_doc.tracks {
        match track.kind {
            crate::timeline::TrackKind::Video => {
                video_tracks.push(track);
            }
            crate::timeline::TrackKind::Audio => {
                for clip in &track.clips {
                    audio_clips.push(clip);
                }
            }
            crate::timeline::TrackKind::Captions | crate::timeline::TrackKind::Caption => {
                for clip in &track.clips {
                    caption_entries.push((track.style.as_ref(), clip));
                }
            }
        }
    }

    let overlay_tracks = if video_tracks.len() > 1 {
        &video_tracks[1..]
    } else {
        &[][..]
    };

    let mut v_filter_parts = Vec::new();
    let mut total_video_inputs = 1; // input 0 is concat demuxer

    // 1. Overlay tracks compositing
    let mut overlay_idx = 0;
    for track in overlay_tracks {
        for clip in &track.clips {
            let input_idx = total_video_inputs;
            total_video_inputs += 1;
            if let Some(hex) = color_source_hex(&clip.source) {
                args.extend_from_slice(&[
                    "-f".into(),
                    "lavfi".into(),
                    "-i".into(),
                    format!(
                        "color=c=0x{}:s={}x{}:r={}:d={}",
                        hex,
                        clip.width.unwrap_or(timeline_doc.width),
                        clip.height.unwrap_or(timeline_doc.height),
                        timeline_doc.fps,
                        clip.in_point + clip.duration
                    ),
                ]);
            } else {
                // Repeat the source at the input and bound it there: a `loop` filter on a single
                // PNG never syncs with the concat-demuxer base (logo silently missing), and an
                // unbounded `-loop 1`/`-stream_loop -1` input keeps ffmpeg reading forever.
                let bound = format!("{}", clip.in_point + clip.duration);
                if is_image_source(&clip.source) {
                    args.extend_from_slice(&[
                        "-loop".into(),
                        "1".into(),
                        "-framerate".into(),
                        timeline_doc.fps.to_string(),
                        "-t".into(),
                        bound,
                    ]);
                } else if is_video_source(&clip.source) {
                    args.extend_from_slice(&[
                        "-stream_loop".into(),
                        "-1".into(),
                        "-t".into(),
                        bound,
                    ]);
                }
                args.extend_from_slice(&["-i".into(), strip_verbatim(Path::new(&clip.source))]);
            }
            let scale_filter = match (clip.width, clip.height) {
                (Some(w), Some(h)) => format!(",scale={w}:{h}"),
                (Some(w), None) => format!(",scale={w}:-1"),
                (None, Some(h)) => format!(",scale=-1:{h}"),
                (None, None) => String::new(),
            };
            let opacity_filter = match clip.opacity {
                Some(op) if op < 1.0 => format!(",format=rgba,colorchannelmixer=aa={op}"),
                _ => String::new(),
            };
            let ov_label = format!("ov{}", overlay_idx);
            let ov_filter = format!(
                "[{input_idx}:v]trim=start={}:duration={},setpts=PTS-STARTPTS+{}/TB,fps={}{scale_filter}{opacity_filter}[{ov_label}]",
                clip.in_point,
                clip.duration,
                clip.start,
                timeline_doc.fps
            );
            v_filter_parts.push(ov_filter);
            let x = clip.x.unwrap_or(0);
            let y = clip.y.unwrap_or(0);
            let end = clip.start + clip.duration;
            let prev_label = if overlay_idx == 0 { "0:v" } else { "v" };
            let composite = format!(
                "[{prev_label}][{ov_label}]overlay=x={x}:y={y}:enable='between(t,{start},{end})':eof_action=pass[v]",
                start = clip.start
            );
            v_filter_parts.push(composite);
            overlay_idx += 1;
        }
    }

    // 2. Burn in captions if any
    let has_overlay = overlay_idx > 0;
    let mut curr_v = if has_overlay {
        "v".to_string()
    } else {
        "0:v".to_string()
    };
    let mut cap_idx = 0;
    for (style_opt, c) in &caption_entries {
        if is_subtitle_file(&c.source) {
            let escaped_path = escape_ffmpeg_subtitles_path(Path::new(&c.source));
            let next_v = format!("v_cap{}", cap_idx);
            let sub_filter = format!("[{curr_v}]subtitles='{}'[{next_v}]", escaped_path);
            v_filter_parts.push(sub_filter);
            curr_v = next_v;
            cap_idx += 1;
        } else {
            let fontfile_part =
                match resolve_caption_font(style_opt.and_then(|s| s.font.as_deref())) {
                    Some(ref path) => format!("fontfile='{}':", path),
                    None => String::new(),
                };
            let font_size = style_opt
                .and_then(|s| s.font_size)
                .unwrap_or_else(|| ((timeline_doc.height as f64) * 0.045).round() as u32);
            let margin_bottom = style_opt
                .and_then(|s| s.margin_bottom)
                .unwrap_or_else(|| ((timeline_doc.height as f64) * 0.08).round() as u32);
            let text_color_raw = style_opt
                .and_then(|s| s.color.as_deref())
                .unwrap_or("white");
            let font_color = if let Some(hex) = text_color_raw.strip_prefix('#') {
                format!("0x{}", hex)
            } else {
                text_color_raw.to_string()
            };
            let is_box = style_opt.and_then(|s| s.is_box).unwrap_or(true);
            let box_part = if is_box {
                let box_color_raw = style_opt
                    .and_then(|s| s.box_color.as_deref())
                    .unwrap_or("black@0.6");
                let box_color = if let Some(hex) = box_color_raw.strip_prefix('#') {
                    format!("0x{}", hex)
                } else {
                    box_color_raw.to_string()
                };
                let box_border_w = (font_size / 5).max(8);
                format!(":box=1:boxcolor={box_color}:boxborderw={box_border_w}")
            } else {
                ":box=0".to_string()
            };
            let outline_part = match style_opt.and_then(|s| s.outline) {
                Some(w) if w > 0 => format!(":borderw={w}:bordercolor=black"),
                _ => String::new(),
            };
            let line_spacing = (font_size / 6).max(4);
            let escaped_text = escape_drawtext_text(&c.source);
            let align_part = if drawtext_supports_text_align() {
                ":text_align=C"
            } else {
                ""
            };
            let next_v = format!("v_cap{}", cap_idx);
            let text_filter = format!(
                "[{curr_v}]drawtext={fontfile_part}text='{escaped_text}':expansion=none{align_part}:enable='between(t,{},{})':fontsize={font_size}:fontcolor={font_color}{box_part}{outline_part}:line_spacing={line_spacing}:x=(w-text_w)/2:y=h-text_h-{margin_bottom}[{next_v}]",
                c.start,
                c.start + c.duration
            );
            v_filter_parts.push(text_filter);
            curr_v = next_v;
            cap_idx += 1;
        }
    }

    let final_v_label = if cap_idx > 0 {
        curr_v
    } else if has_overlay {
        "v".to_string()
    } else {
        "0:v".to_string()
    };

    let mut filter_graph = v_filter_parts.join(";");

    // 3. Audio pipeline
    if !audio_clips.is_empty() {
        let audio_input_start = total_video_inputs;
        for clip in &audio_clips {
            args.extend_from_slice(&["-i".into(), strip_verbatim(Path::new(&clip.source))]);
        }
        let mix_out = if timeline_doc.loudness_lufs.is_some() {
            "amix"
        } else {
            "a"
        };
        let mut a_filter_parts = Vec::new();
        for (j, clip) in audio_clips.iter().enumerate() {
            let in_idx = audio_input_start + j;
            let atrim = format!(
                "[{}:a]atrim=start={}:duration={},asetpts=PTS-STARTPTS",
                in_idx, clip.in_point, clip.duration
            );
            let start_ms = (clip.start * 1000.0).round() as u64;
            if audio_clips.len() == 1 {
                if start_ms > 0 {
                    a_filter_parts.push(format!(
                        "{},adelay={}|{}[{}]",
                        atrim, start_ms, start_ms, mix_out
                    ));
                } else {
                    a_filter_parts.push(format!("{}[{}]", atrim, mix_out));
                }
            } else {
                if start_ms > 0 {
                    a_filter_parts.push(format!(
                        "{},adelay={}|{}[a{}]",
                        atrim, start_ms, start_ms, j
                    ));
                } else {
                    a_filter_parts.push(format!("{}[a{}]", atrim, j));
                }
            }
        }
        if audio_clips.len() > 1 {
            let mut mix_inputs = String::new();
            for j in 0..audio_clips.len() {
                mix_inputs.push_str(&format!("[a{}]", j));
            }
            a_filter_parts.push(format!(
                "{}amix=inputs={}:duration=longest:dropout_transition=0[{}]",
                mix_inputs,
                audio_clips.len(),
                mix_out
            ));
        }
        if let Some(lufs) = timeline_doc.loudness_lufs {
            a_filter_parts.push(format_loudnorm_filter(lufs, loudnorm));
        }
        if !filter_graph.is_empty() {
            filter_graph.push(';');
        }
        filter_graph.push_str(&a_filter_parts.join(";"));
    }

    if !filter_graph.is_empty() {
        args.extend_from_slice(&["-filter_complex".into(), filter_graph.clone()]);
        args.extend_from_slice(&["-map".into(), format!("[{final_v_label}]")]);
    } else {
        args.extend_from_slice(&["-map".into(), "0:v".into()]);
    }

    args.extend_from_slice(&[
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        "veryfast".into(),
        "-crf".into(),
        "17".into(),
        "-pix_fmt".into(),
        "yuv420p".into(),
        "-r".into(),
        format!("{}", timeline_doc.fps),
        "-flags".into(),
        "+bitexact".into(),
        "-fflags".into(),
        "+bitexact".into(),
    ]);

    if !audio_clips.is_empty() {
        args.extend_from_slice(&[
            "-map".into(),
            "[a]".into(),
            "-c:a".into(),
            "aac".into(),
            "-b:a".into(),
            "192k".into(),
        ]);
    }

    args.push(strip_verbatim(output_file));
    Ok((args, filter_graph))
}
/// Build a `JobSpec` for rendering timeline using FFmpeg.
///
/// Uses Chunked Pipeline architecture:
/// - If base video track has more than 1 scene/clip, each base clip is rendered/cached
///   independently to intermediate chunks (`.cache/timeline_chunks/chunk_{hash}.mp4`).
/// - The final assembly pass runs concat demuxer + overlays + captions + audio loudnorm,
///   keeping peak RAM strictly bounded below 768 MB (O(1) memory scalability).
/// - Declared RAM and memory limits scale conservatively based on timeline resolution:
///   <=720p: 512 MB (limit 768 MB), <=1080p: 768 MB (limit 1152 MB).
pub fn build_timeline_render_job(
    job_id: &str,
    workspace_id: &str,
    ffmpeg_report: &ProbeReport,
    timeline_doc: &crate::timeline::TimelineDoc,
    cwd: &Path,
    output_file: &Path,
) -> Result<JobSpec> {
    let ffmpeg_path = ffmpeg_report.path.as_ref().ok_or_else(|| {
        ArtifactError::ToolNotFound(
            "ffmpeg is required for timeline rendering but was not found".into(),
        )
    })?;

    let measured_loudnorm = if timeline_doc.loudness_lufs.is_some() {
        measure_audio_loudnorm(ffmpeg_path, timeline_doc, cwd)
    } else {
        None
    };

    let base_clips: Vec<_> = timeline_doc
        .tracks
        .iter()
        .find(|t| t.kind == crate::timeline::TrackKind::Video)
        .map(|t| t.clips.clone())
        .unwrap_or_default();

    let args = if base_clips.len() > 1 {
        // Resolve cache directory inside workspace or current working directory
        let cache_dir = cwd.join(".cache").join("timeline_chunks");
        let _ = std::fs::create_dir_all(&cache_dir);

        let mut chunk_paths = Vec::new();
        for (idx, clip) in base_clips.iter().enumerate() {
            let chunk_hash = compute_clip_chunk_hash(clip, idx, timeline_doc, cwd);
            let chunk_file = cache_dir.join(format!("chunk_{idx}_{chunk_hash}.mp4"));
            render_base_clip_chunk(ffmpeg_path, clip, idx, timeline_doc, cwd, &chunk_file)?;
            chunk_paths.push(chunk_file);
        }

        let concat_list_file = cache_dir.join(format!("concat_{job_id}.txt"));
        let mut list_content = String::new();
        for chunk in &chunk_paths {
            let esc_path = escape_concat_path(chunk);
            list_content.push_str(&format!("file '{esc_path}'\n"));
        }
        std::fs::write(&concat_list_file, list_content)?;

        let (concat_args, _) = build_timeline_concat_ffmpeg_args_measured(
            timeline_doc,
            &concat_list_file,
            output_file,
            measured_loudnorm.as_ref(),
        )?;
        concat_args
    } else {
        let (direct_args, _) = build_timeline_ffmpeg_args_measured(
            timeline_doc,
            output_file,
            measured_loudnorm.as_ref(),
        )?;
        direct_args
    };

    let command = CommandSpec::new(
        ffmpeg_path.to_string_lossy().to_string(),
        args,
        cwd.to_path_buf(),
    );

    let ram_mb =
        calculate_timeline_ram_mb(timeline_doc.width, timeline_doc.height, timeline_doc.fps);
    let memory_limit_mb = ((ram_mb as f64) * 1.5).ceil() as u64;

    let resources = ResourceSpec {
        ram_mb,
        cpu: CpuClass::Heavy,
        gpu_vram_mb: 0,
        exclusive: Vec::new(),
        rate_key: None,
    };

    Ok(JobSpec {
        id: job_id.to_string(),
        workspace_id: workspace_id.to_string(),
        tool_id: "render.timeline_ffmpeg@1".into(),
        command,
        resources,
        priority: Priority::TurnBlocking,
        timeout: Duration::from_secs(300),
        memory_limit_mb: Some(memory_limit_mb),
        lifecycle: Lifecycle::PerCall,
    })
}

/// Build a `JobSpec` for generating a 720p low-bitrate proxy video + poster thumbnail for any video asset/render.
///
/// Output proxy: 720p H.264 (fast preset, crf 28, max 1.5 Mbps) + AAC
/// Output poster: first frame JPEG thumbnail (s=1280x720)
pub fn build_proxy_job(
    job_id: &str,
    workspace_id: &str,
    ffmpeg_report: &ProbeReport,
    input_video: &Path,
    output_proxy_video: &Path,
    output_poster_image: &Path,
) -> Result<JobSpec> {
    let ffmpeg_path = ffmpeg_report.path.as_ref().ok_or_else(|| {
        ArtifactError::ToolNotFound(
            "ffmpeg is required for proxy generation but was not found".into(),
        )
    })?;

    // We can run a single ffmpeg command that outputs both proxy video and poster thumbnail
    // ffmpeg -y -i <input> -vf "scale=-2:720" -c:v libx264 -preset fast -crf 28 -b:v 1500k -c:a aac <proxy.mp4> -vframes 1 -vf "scale=-2:720" <poster.jpg>
    let args = vec![
        "-y".into(),
        "-i".into(),
        input_video.to_string_lossy().to_string(),
        "-vf".into(),
        "scale=-2:720".into(),
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        "fast".into(),
        "-crf".into(),
        "28".into(),
        "-b:v".into(),
        "1500k".into(),
        output_proxy_video.to_string_lossy().to_string(),
        "-ss".into(),
        "00:00:00.100".into(),
        "-vframes".into(),
        "1".into(),
        "-vf".into(),
        "scale=-2:720".into(),
        output_poster_image.to_string_lossy().to_string(),
    ];

    let cwd = input_video
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let command = CommandSpec::new(ffmpeg_path.to_string_lossy().to_string(), args, cwd);

    let resources = ResourceSpec {
        ram_mb: 768,
        cpu: CpuClass::Heavy,
        gpu_vram_mb: 0,
        exclusive: Vec::new(),
        rate_key: None,
    };

    Ok(JobSpec {
        id: job_id.to_string(),
        workspace_id: workspace_id.to_string(),
        tool_id: "render.proxy@1".into(),
        command,
        resources,
        priority: Priority::Interactive,
        timeout: Duration::from_secs(120),
        memory_limit_mb: Some(1024),
        lifecycle: Lifecycle::PerCall,
    })
}

#[cfg(test)]
mod color_source_tests {
    use super::color_source_hex;

    #[test]
    fn color_source_parses_only_six_hex_digits() {
        assert_eq!(color_source_hex("color:#1E88E5").as_deref(), Some("1e88e5"));
        assert_eq!(color_source_hex("color:1e88e5").as_deref(), Some("1e88e5"));
        assert_eq!(color_source_hex("color:#1e88e5:d=999"), None);
        assert_eq!(color_source_hex("color:red"), None);
        assert_eq!(color_source_hex("clips/a.mp4"), None);
    }
}

#[cfg(test)]
mod loudnorm_tests {
    use super::*;

    #[test]
    fn parse_loudnorm_measurement_from_sample_stderr() {
        let sample_stderr = r#"[Parsed_loudnorm_0 @ 00000168075009c0] 
{
	"input_i" : "-25.92",
	"input_tp" : "-2.44",
	"input_lra" : "20.10",
	"input_thresh" : "-37.82",
	"output_i" : "-14.33",
	"output_tp" : "-1.50",
	"output_lra" : "3.10",
	"output_thresh" : "-24.85",
	"normalization_type" : "dynamic",
	"target_offset" : "0.33"
}
[out#0/null @ 0000016807500780] video:0KiB audio:750KiB subtitle:0KiB
size=N/A time=00:01:11.30 bitrate=N/A speed=104x elapsed=0:00:00.68"#;

        let parsed = parse_loudnorm_measurement(sample_stderr).expect("parse loudnorm json");
        assert_eq!(parsed.input_i, "-25.92");
        assert_eq!(parsed.input_tp, "-2.44");
        assert_eq!(parsed.input_lra, "20.10");
        assert_eq!(parsed.input_thresh, "-37.82");
        assert_eq!(parsed.target_offset, "0.33");
    }

    #[test]
    fn parse_loudnorm_measurement_returns_none_for_invalid_stderr() {
        assert!(parse_loudnorm_measurement("some error occurred without json").is_none());
        assert!(parse_loudnorm_measurement(r#"{"input_i": "-20"}"#).is_none());
    }

    #[test]
    fn format_loudnorm_filter_single_pass_fallback() {
        let filter = format_loudnorm_filter(-14.0, None);
        assert_eq!(
            filter,
            "[amix]loudnorm=I=-14:TP=-1.5:LRA=11,aresample=48000[a]"
        );
    }

    #[test]
    fn format_loudnorm_filter_two_pass() {
        let m = LoudnormMeasurement {
            input_i: "-25.92".into(),
            input_tp: "-2.44".into(),
            input_lra: "20.10".into(),
            input_thresh: "-37.82".into(),
            target_offset: "0.33".into(),
        };
        let filter = format_loudnorm_filter(-14.0, Some(&m));
        assert_eq!(
            filter,
            "[amix]loudnorm=I=-14:TP=-1.5:LRA=11:measured_I=-25.92:measured_TP=-2.44:measured_LRA=20.10:measured_thresh=-37.82:offset=0.33:linear=true,aresample=48000[a]"
        );
    }

    #[test]
    fn build_loudnorm_measure_ffmpeg_args_without_audio_or_lufs() {
        let doc_no_lufs = crate::timeline::TimelineDoc {
            version: 1,
            width: 1920,
            height: 1080,
            fps: 30.0,
            loudness_lufs: None,
            tracks: vec![],
        };
        assert!(build_loudnorm_measure_ffmpeg_args(&doc_no_lufs).is_none());

        let doc_no_audio = crate::timeline::TimelineDoc {
            version: 1,
            width: 1920,
            height: 1080,
            fps: 30.0,
            loudness_lufs: Some(-14.0),
            tracks: vec![],
        };
        assert!(build_loudnorm_measure_ffmpeg_args(&doc_no_audio).is_none());
    }
}
