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

/// Build an FFmpeg filtergraph and arguments for rendering a `timeline.json` document to MP4 H.264/AAC.
pub fn build_timeline_ffmpeg_args(
    timeline_doc: &crate::timeline::TimelineDoc,
    output_file: &Path,
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
                let fps = timeline_doc.fps;
                let total_frames = (((clip.in_point + clip.duration) * fps).round() as u64).max(1);
                let d_denom = if total_frames > 1 {
                    total_frames - 1
                } else {
                    1
                };
                let strength = clip.motion_strength.unwrap_or(0.08).max(0.001);
                let s_str = format!("{:.4}", strength);
                let one_plus_s = format!("{:.4}", 1.0 + strength);
                let (z_expr, x_expr, y_expr) = match motion {
                    crate::timeline::MotionKind::ZoomIn => (
                        format!("1+{s_str}*(on/{d_denom})"),
                        "(iw-iw/zoom)/2".to_string(),
                        "(ih-ih/zoom)/2".to_string(),
                    ),
                    crate::timeline::MotionKind::ZoomOut => (
                        format!("({one_plus_s})-{s_str}*(on/{d_denom})"),
                        "(iw-iw/zoom)/2".to_string(),
                        "(ih-ih/zoom)/2".to_string(),
                    ),
                    crate::timeline::MotionKind::PanLeft => (
                        one_plus_s,
                        format!("(iw-iw/zoom)*(1-on/{d_denom})"),
                        "(ih-ih/zoom)/2".to_string(),
                    ),
                    crate::timeline::MotionKind::PanRight => (
                        one_plus_s,
                        format!("(iw-iw/zoom)*(on/{d_denom})"),
                        "(ih-ih/zoom)/2".to_string(),
                    ),
                    crate::timeline::MotionKind::PanUp => (
                        one_plus_s,
                        "(iw-iw/zoom)/2".to_string(),
                        format!("(ih-ih/zoom)*(1-on/{d_denom})"),
                    ),
                    crate::timeline::MotionKind::PanDown => (
                        one_plus_s,
                        "(iw-iw/zoom)/2".to_string(),
                        format!("(ih-ih/zoom)*(on/{d_denom})"),
                    ),
                    crate::timeline::MotionKind::None | crate::timeline::MotionKind::Auto => {
                        unreachable!()
                    }
                };
                let mut f = format!(
                    "[{i}:v]scale={scale_w}:{scale_h},zoompan=z=\'{z_expr}\':x=\'{x_expr}\':y=\'{y_expr}\':d={total_frames}:s={w}x{h}:fps={fps},trim=start={in_pt}:duration={dur},setpts=PTS-STARTPTS,setsar=1",
                    scale_w = 2 * timeline_doc.width,
                    scale_h = 2 * timeline_doc.height,
                    w = timeline_doc.width,
                    h = timeline_doc.height,
                    in_pt = clip.in_point,
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
            // Single-pass loudnorm resamples internally (192 kHz); bring it back to 48 kHz.
            a_filter_parts.push(format!(
                "[amix]loudnorm=I={lufs}:TP=-1.5:LRA=11,aresample=48000[a]"
            ));
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

/// Build a `JobSpec` for rendering timeline using FFmpeg.
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

    let (args, _) = build_timeline_ffmpeg_args(timeline_doc, output_file)?;

    let command = CommandSpec::new(
        ffmpeg_path.to_string_lossy().to_string(),
        args,
        cwd.to_path_buf(),
    );

    let resources = ResourceSpec {
        ram_mb: 1024,
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
        memory_limit_mb: Some(1536),
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
