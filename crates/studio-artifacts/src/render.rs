use std::fs;
use std::path::PathBuf;
use studio_jobs::{
    event::{FailedReason, TaskEvent},
    job::JobId,
    scheduler::Scheduler,
};
use studio_workspace::{Project, Workspace};

use crate::artifact::Artifact;
use crate::cache::{compute_input_hash, hash12, lookup_render_cache, record_render_outputs};
use crate::error::{ArtifactError, Result};
use crate::kind::KindRegistry;
use crate::probe::{probe_chromium, probe_ffmpeg, probe_ffprobe, probe_node_npx, ProbeReport};
use crate::renderer::*;
use crate::starter::DEFAULT_BRAND_CSS;
use crate::validate::{validate_png, validate_video};

/// Result of rendering an artifact.
#[derive(Debug, Clone)]
pub struct RenderOutcome {
    /// True if the output was retrieved from the render cache (no job scheduled).
    pub cached: bool,
    /// Absolute paths to the produced output files.
    pub outputs: Vec<PathBuf>,
    /// Scheduled job ID if a render job was submitted.
    pub job_id: Option<JobId>,
    /// Diagnostic report of probed tools.
    pub probe_report: Vec<ProbeReport>,
}

/// Per-render settings resolved by the caller (e.g. from the artifact's channels).
#[derive(Debug, Clone, Default)]
pub struct RenderOptions {
    /// Loudness target (LUFS) for timeline renders whose `timeline.json` sets none.
    pub loudness_lufs: Option<f64>,
}

/// Render with default options. See [`render_with`].
pub async fn render(
    workspace: &Workspace,
    project: &Project,
    artifact: &Artifact,
    scheduler: &Scheduler,
) -> Result<RenderOutcome> {
    render_with(
        workspace,
        project,
        artifact,
        scheduler,
        &RenderOptions::default(),
    )
    .await
}

/// Orchestrate rendering an artifact:
/// 1. Resolve brand version and renderer version.
/// 2. Compute deterministic input hash.
/// 3. Check render cache in WorkspaceDb: if hit, return cached outputs immediately.
/// 4. If cache miss, construct JobSpec using appropriate renderer.
/// 5. Submit to `Scheduler`, await completion.
/// 6. Post-validate generated outputs.
/// 7. Record outputs in `renders` table and return `RenderOutcome`.
pub async fn render_with(
    workspace: &Workspace,
    project: &Project,
    artifact: &Artifact,
    scheduler: &Scheduler,
    options: &RenderOptions,
) -> Result<RenderOutcome> {
    let brand_version = artifact.brand.as_deref().unwrap_or("default@1");
    let renderer_id = artifact
        .renderer
        .clone()
        // artifact.yaml may omit `renderer`: fall back to the kind's default renderer.
        .or_else(|| KindRegistry::get(&artifact.kind).and_then(|k| k.renderer))
        .unwrap_or_else(|| "default".into());

    // 1. Prepare brand.css if it's a design artifact and brand.css is missing
    if artifact.kind == "design" {
        let brand_css_path = artifact.dir(project).join("brand.css");
        if !brand_css_path.exists() {
            let _ = fs::write(&brand_css_path, DEFAULT_BRAND_CSS);
        }
    }

    // 2. Compute input hash
    let mut full_hash =
        compute_input_hash(workspace, project, artifact, brand_version, &renderer_id)?;
    // The channel loudness target changes the timeline output, so it is part of the cache key.
    if renderer_id == "render.timeline_ffmpeg@1" {
        if let Some(lufs) = options.loudness_lufs {
            use sha2::{Digest, Sha256};
            full_hash = hex::encode(Sha256::digest(
                format!("{full_hash}|loudness={lufs}").as_bytes(),
            ));
        }
    }
    let short_hash = hash12(&full_hash);

    // 3. Check render cache
    if let Some(cached_files) = lookup_render_cache(workspace, &artifact.id, &full_hash)? {
        return Ok(RenderOutcome {
            cached: true,
            outputs: cached_files,
            job_id: None,
            probe_report: Vec::new(),
        });
    }

    // 4. Output directory: projects/<slug>/renders/<artifact-id>/<hash12>/
    let output_dir = project.renders_dir().join(&artifact.id).join(&short_hash);
    fs::create_dir_all(&output_dir)?;

    let job_id = format!(
        "{}-{}-{}",
        artifact.id,
        short_hash,
        chrono::Utc::now().timestamp_millis()
    );
    let ws_id = workspace.root().to_string_lossy().to_string();

    let mut probe_reports = Vec::new();
    let mut expected_outputs = Vec::new();

    // 5. Build JobSpec based on renderer / kind
    let job_spec = match renderer_id.as_str() {
        "render.html_png@1" | "render.html_pdf@1" => {
            let browser = probe_chromium();
            probe_reports.push(browser.clone());
            if !browser.available {
                return Err(ArtifactError::ToolNotFound(browser.note));
            }

            let as_pdf = renderer_id == "render.html_pdf@1";
            let out_file = if as_pdf {
                output_dir.join("output.pdf")
            } else {
                output_dir.join("output.png")
            };
            expected_outputs.push(out_file.clone());

            let html_source = artifact.source_path(project);
            let (width, height) = artboard_size(artifact, project).unwrap_or((1080, 1350));

            build_html_render_job(
                &job_id,
                &ws_id,
                &browser,
                &HtmlRenderTarget {
                    html_file: &html_source,
                    output_file: &out_file,
                    width,
                    height,
                    as_pdf,
                },
            )?
        }
        "render.html_video@1" => {
            let (_, npx) = probe_node_npx();
            let ffprobe = probe_ffprobe();
            probe_reports.push(npx.clone());
            probe_reports.push(ffprobe);

            if !npx.available {
                return Err(ArtifactError::ToolNotFound(npx.note));
            }

            let out_file = output_dir.join("output.mp4");
            expected_outputs.push(out_file.clone());

            let format = artifact.params["format"].as_str().unwrap_or("mp4");
            let fps = artifact.params["fps"].as_u64().unwrap_or(30) as u32;

            build_hyperframes_render_job(
                &job_id,
                &ws_id,
                &npx,
                &artifact.dir(project),
                &out_file,
                format,
                fps,
            )?
        }
        "render.timeline_ffmpeg@1" => {
            let ffmpeg = probe_ffmpeg();
            let ffprobe = probe_ffprobe();
            probe_reports.push(ffmpeg.clone());
            probe_reports.push(ffprobe);

            if !ffmpeg.available {
                return Err(ArtifactError::ToolNotFound(ffmpeg.note));
            }

            let timeline_file = artifact.source_path(project);
            let content = fs::read_to_string(&timeline_file)?;
            let mut timeline_doc: crate::timeline::TimelineDoc = serde_json::from_str(&content)
                .map_err(|e| ArtifactError::InvalidConfig(timeline_schema_hint(&e)))?;
            if timeline_doc.loudness_lufs.is_none() {
                timeline_doc.loudness_lufs = options.loudness_lufs;
            }
            resolve_workspace_relative_sources(
                &mut timeline_doc,
                &artifact.dir(project),
                workspace.root(),
            );

            let out_file = output_dir.join("output.mp4");
            expected_outputs.push(out_file.clone());

            build_timeline_render_job(
                &job_id,
                &ws_id,
                &ffmpeg,
                &timeline_doc,
                &artifact.dir(project),
                &out_file,
            )?
        }
        _ => {
            return Err(ArtifactError::UnsupportedKind(format!(
                "Unsupported renderer: {}",
                renderer_id
            )));
        }
    };

    // 6. Submit job to scheduler
    let submitted_job_id = job_spec.id.clone();
    let mut handle = scheduler.submit(job_spec).await;

    // Wait for job completion by draining events until Finished or Failed
    let mut job_success = false;
    let mut failure_reason = None;

    let mut stdout_lines = Vec::new();
    let mut stderr_lines = Vec::new();

    while let Some(event) = handle.next_event().await {
        match event {
            TaskEvent::Stdout(line) => stdout_lines.push(line),
            TaskEvent::Stderr(line) => stderr_lines.push(line),
            TaskEvent::Finished { exit_code, .. } => {
                if exit_code == 0 {
                    job_success = true;
                } else {
                    let details = output_tail(&stderr_lines, &stdout_lines);
                    failure_reason = Some(format!("Exit code {}: {}", exit_code, details));
                }
                break;
            }
            TaskEvent::Failed { reason } => {
                let details = output_tail(&stderr_lines, &stdout_lines);
                let reason_str = match reason {
                    FailedReason::OutOfMemory => {
                        format!("Vượt bộ nhớ (Out of Memory): {}", details)
                    }
                    FailedReason::Timeout => {
                        format!("Hết thời gian thực thi (Timeout): {}", details)
                    }
                    FailedReason::Cancelled => "Đã bị hủy (Cancelled)".to_string(),
                    FailedReason::SpawnError(msg) => format!("Lỗi khởi tạo process: {}", msg),
                    FailedReason::Exit(code) => {
                        format!("Process kết thúc với mã lỗi {}: {}", code, details)
                    }
                };
                failure_reason = Some(reason_str);
                break;
            }
            _ => {}
        }
    }

    if !job_success {
        return Err(ArtifactError::JobFailed(
            failure_reason.unwrap_or_else(|| "Tác vụ render không hoàn thành".into()),
        ));
    }

    // 7. Post-validate outputs
    for out_file in &expected_outputs {
        if !out_file.exists() {
            return Err(ArtifactError::ValidationFailed(format!(
                "Expected render output does not exist: {}",
                out_file.display()
            )));
        }

        let ext = out_file
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext == "png" {
            let size = if artifact.kind == "design" {
                artboard_size(artifact, project)
            } else {
                None
            };
            validate_png(out_file, size.map(|s| s.0), size.map(|s| s.1))?;
        } else if ext == "mp4" || ext == "webm" {
            validate_video(out_file, None, None)?;
        }
    }

    // 8. Record in WorkspaceDb cache
    record_render_outputs(workspace, &artifact.id, &full_hash, &expected_outputs)?;

    Ok(RenderOutcome {
        cached: false,
        outputs: expected_outputs,
        job_id: Some(submitted_job_id),
        probe_report: probe_reports,
    })
}

/// Artboard size for an HTML artifact: `params.width/height` first, then what the HTML source declares.
fn artboard_size(artifact: &Artifact, project: &Project) -> Option<(u32, u32)> {
    let w = artifact.params["width"].as_u64();
    let h = artifact.params["height"].as_u64();
    if let (Some(w), Some(h)) = (w, h) {
        return Some((w as u32, h as u32));
    }
    let html = fs::read_to_string(artifact.source_path(project)).ok()?;
    detect_artboard_size(&html)
}

/// Explains a `timeline.json` parse error together with the expected shape, so an agent can fix
/// the file in one go instead of guessing field by field.
pub(crate) fn timeline_schema_hint(err: &serde_json::Error) -> String {
    format!(
        "timeline.json không hợp lệ (dòng {}, cột {}): {err}. Cấu trúc đúng: \
         {{\"width\":1920,\"height\":1080,\"fps\":30,\"tracks\":[{{\"id\":\"v1\",\"kind\":\"video\"|\"audio\"|\"captions\",\
         \"clips\":[{{\"id\":\"c1\",\"source\":\"<đường dẫn tương đối từ thư mục artifact, hoặc color:#RRGGBB, \
         hoặc chữ phụ đề>\",\"start\":0,\"duration\":5,\"in_point\":0,\"transition\":\"none\"|\"fade\"}}]}}]}}. \
         Mỗi track và mỗi clip bắt buộc có \"id\"; mỗi clip bắt buộc có \"source\" và \"duration\".",
        err.line(),
        err.column()
    )
}

#[cfg(test)]
mod timeline_hint_tests {
    #[test]
    fn hint_names_missing_field_and_required_keys() {
        let err = serde_json::from_str::<crate::timeline::TimelineDoc>(
            r#"{"tracks":[{"id":"v","kind":"video","clips":[{"id":"c","duration":1}]}]}"#,
        )
        .unwrap_err();
        let msg = super::timeline_schema_hint(&err);
        assert!(msg.contains("source"), "{msg}");
        assert!(msg.contains("bắt buộc"), "{msg}");
    }
}

/// Last meaningful lines of a failed job's output. ffmpeg prints a long version/config banner
/// first, which used to fill the whole error preview while the actual cause sat at the end.
pub(crate) fn output_tail(stderr: &[String], stdout: &[String]) -> String {
    const TAIL: usize = 15;
    let lines = if stderr.is_empty() { stdout } else { stderr };
    let meaningful: Vec<&str> = lines
        .iter()
        .map(|l| l.trim_end())
        .filter(|l| {
            let t = l.trim_start();
            !(t.is_empty()
                || t.starts_with("ffmpeg version")
                || t.starts_with("ffprobe version")
                || t.starts_with("built with")
                || t.starts_with("configuration:")
                || t.starts_with("lib"))
        })
        .collect();
    let start = meaningful.len().saturating_sub(TAIL);
    meaningful[start..].join("\n")
}

#[cfg(test)]
mod output_tail_tests {
    #[test]
    fn drops_ffmpeg_banner_and_keeps_the_end() {
        let lines: Vec<String> = [
            "ffmpeg version 8.1.1 Copyright (c) 2000-2026",
            "  built with gcc 15.2.0",
            "  configuration: --enable-gpl",
            "  libavutil      60.  8.100",
            "Input #0, png_pipe",
            "captions.srt: No such file or directory",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let tail = super::output_tail(&lines, &[]);
        assert!(!tail.contains("ffmpeg version"));
        assert!(!tail.contains("configuration"));
        assert!(tail.ends_with("captions.srt: No such file or directory"));
    }
}

/// Clip sources are resolved by ffmpeg from the artifact folder. Studio tools hand out
/// workspace-relative paths (`projects/<slug>/assets/...`), so a source that does not exist
/// next to the artifact but does exist under the workspace root is rewritten to that file.
/// Colours, caption text and anything outside the workspace are left untouched.
pub(crate) fn resolve_workspace_relative_sources(
    doc: &mut crate::timeline::TimelineDoc,
    artifact_dir: &std::path::Path,
    workspace_root: &std::path::Path,
) {
    for track in &mut doc.tracks {
        for clip in &mut track.clips {
            let src = clip.source.trim();
            if src.is_empty() || src.starts_with("color:") || src.contains('\n') {
                continue;
            }
            let rel = std::path::Path::new(src);
            if rel.is_absolute() || artifact_dir.join(rel).exists() {
                continue;
            }
            let candidate = workspace_root.join(rel);
            let inside = candidate
                .canonicalize()
                .ok()
                .zip(workspace_root.canonicalize().ok())
                .is_some_and(|(c, root)| c.starts_with(root));
            if inside && candidate.is_file() {
                clip.source = candidate.to_string_lossy().into_owned();
            }
        }
    }
}

#[cfg(test)]
mod source_resolution_tests {
    use crate::timeline::TimelineDoc;

    #[test]
    fn workspace_relative_sources_are_rewritten_only_when_found_inside() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("ws");
        let art = root.join("projects/p/artifacts/video");
        std::fs::create_dir_all(&art).unwrap();
        std::fs::create_dir_all(root.join("projects/p/assets")).unwrap();
        std::fs::write(root.join("projects/p/assets/a.wav"), b"x").unwrap();
        std::fs::write(art.join("local.png"), b"x").unwrap();

        let mut doc: TimelineDoc = serde_json::from_str(
            r#"{"tracks":[{"id":"v","kind":"video","clips":[
                {"id":"1","source":"projects/p/assets/a.wav","duration":1},
                {"id":"2","source":"local.png","duration":1},
                {"id":"3","source":"color:#000000","duration":1},
                {"id":"4","source":"../../../../outside.wav","duration":1}]}]}"#,
        )
        .unwrap();
        super::resolve_workspace_relative_sources(&mut doc, &art, &root);
        let c = &doc.tracks[0].clips;
        assert!(c[0].source.ends_with("a.wav") && std::path::Path::new(&c[0].source).is_absolute());
        assert_eq!(c[1].source, "local.png");
        assert_eq!(c[2].source, "color:#000000");
        assert_eq!(c[3].source, "../../../../outside.wav");
    }
}
