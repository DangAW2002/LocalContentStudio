use std::path::{Path, PathBuf};
use std::time::Duration;
use studio_jobs::{
    command::CommandSpec,
    job::{JobSpec, Lifecycle, Priority},
    resource::{CpuClass, ResourceSpec},
};

use crate::error::{ArtifactError, Result};
use crate::probe::ProbeReport;

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

    let file_url = format!("file:///{}", html_file.to_string_lossy().replace('\\', "/"));

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

/// Build a `JobSpec` for rendering HyperFrames motion composition via `npx hyperframes render`.
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

    let args = vec![
        "hyperframes".into(),
        "render".into(),
        "--output".into(),
        output_file.to_string_lossy().to_string(),
        "--format".into(),
        format.into(),
        "--fps".into(),
        fps.to_string(),
    ];

    let command = CommandSpec::new(
        npx_path.to_string_lossy().to_string(),
        args,
        composition_dir.to_path_buf(),
    );

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
        timeout: Duration::from_secs(300),
        memory_limit_mb: Some(3072),
        lifecycle: Lifecycle::PerCall,
    })
}

/// Build an FFmpeg filtergraph and arguments for rendering a `timeline.json` document to MP4 H.264/AAC.
pub fn build_timeline_ffmpeg_args(
    timeline_doc: &crate::timeline::TimelineDoc,
    output_file: &Path,
) -> (Vec<String>, String) {
    let mut args = vec!["-y".into()];
    let mut filter_graph = String::new();

    // Collect all video and audio clips
    let mut video_clips = Vec::new();
    let mut audio_clips = Vec::new();

    for track in &timeline_doc.tracks {
        match track.kind {
            crate::timeline::TrackKind::Video => {
                for clip in &track.clips {
                    video_clips.push(clip);
                }
            }
            crate::timeline::TrackKind::Audio => {
                for clip in &track.clips {
                    audio_clips.push(clip);
                }
            }
            crate::timeline::TrackKind::Captions => {}
        }
    }

    // If no video clips, generate synthetic color background
    if video_clips.is_empty() {
        args.extend_from_slice(&[
            "-f".into(),
            "lavfi".into(),
            "-i".into(),
            format!(
                "color=c=black:s={}x{}:d=5:r={}",
                timeline_doc.width, timeline_doc.height, timeline_doc.fps
            ),
        ]);
        filter_graph.push_str(&format!(
            "[0:v]fps={},scale={}:{}[v]",
            timeline_doc.fps, timeline_doc.width, timeline_doc.height
        ));
    } else {
        // Register each video clip input
        for clip in &video_clips {
            args.extend_from_slice(&["-i".into(), clip.source.clone()]);
        }

        let mut v_filter_parts = Vec::new();
        for (i, clip) in video_clips.iter().enumerate() {
            let mut filter = format!(
                "[{}:v]trim=start={}:duration={},setpts=PTS-STARTPTS,fps={},scale={}:{},setsar=1",
                i,
                clip.in_point,
                clip.duration,
                timeline_doc.fps,
                timeline_doc.width,
                timeline_doc.height
            );
            if clip.transition == crate::timeline::Transition::Fade {
                let fade_dur = clip.transition_duration.unwrap_or(0.5);
                filter.push_str(&format!(",fade=t=in:st=0:d={}", fade_dur));
            }
            let label = format!("v{}", i);
            v_filter_parts.push(format!("{}[{}]", filter, label));
        }

        let mut concat_inputs = String::new();
        for i in 0..video_clips.len() {
            concat_inputs.push_str(&format!("[v{}]", i));
        }
        let concat_str = format!("{}concat=n={}:v=1:a=0[v]", concat_inputs, video_clips.len());
        v_filter_parts.push(concat_str);

        filter_graph = v_filter_parts.join(";");
    }

    // Check for captions in timeline to burn in
    let mut caption_clips = Vec::new();
    for track in &timeline_doc.tracks {
        if track.kind == crate::timeline::TrackKind::Captions {
            for clip in &track.clips {
                caption_clips.push(clip);
            }
        }
    }

    if !caption_clips.is_empty() {
        // Append drawtext filter for caption clips
        for c in caption_clips {
            let escaped_text = c.source.replace(':', "\\:").replace('\'', "");
            let text_filter = format!(
                ";[v]drawtext=text='{}':enable='between(t,{},{})':fontsize=36:fontcolor=white:x=(w-text_w)/2:y=h-100:box=1:boxcolor=black@0.6[v]",
                escaped_text, c.start, c.start + c.duration
            );
            filter_graph.push_str(&text_filter);
        }
    }

    args.extend_from_slice(&["-filter_complex".into(), filter_graph.clone()]);
    args.extend_from_slice(&[
        "-map".into(),
        "[v]".into(),
        "-c:v".into(),
        "libx264".into(),
        "-pix_fmt".into(),
        "yuv420p".into(),
        output_file.to_string_lossy().to_string(),
    ]);

    (args, filter_graph)
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

    let (args, _) = build_timeline_ffmpeg_args(timeline_doc, output_file);

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
