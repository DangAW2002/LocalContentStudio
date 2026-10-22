//! Shared image post-processing (resize, format conversion, and keep_original).
//!
//! Fits generated images to exact requested dimensions (`width` x `height`)
//! using FFmpeg with `cover` (scale to cover + center-crop), `contain` (scale to fit + pad),
//! or `none` (no resize).

use crate::error::{Result, ToolError};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;
use studio_workspace::Workspace;

/// Probes an image file's width and height using ffprobe.
/// Returns None if ffprobe is missing or file cannot be probed as a valid image.
pub fn probe_image_dimensions(file_path: &Path) -> Option<(u64, u64)> {
    let ffprobe_exe = crate::probe::find_in_path("ffprobe")?;
    let output = Command::new(ffprobe_exe)
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=s=x:p=0",
        ])
        .arg(file_path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let trimmed = text.trim();
    let mut parts = trimmed.split('x');
    let w: u64 = parts.next()?.trim().parse().ok()?;
    let h: u64 = parts.next()?.trim().parse().ok()?;
    Some((w, h))
}

/// Outcome of resizing a single image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResizeOutcome {
    /// Final path of the file.
    pub final_path: PathBuf,
    /// Requested width and height if requested.
    pub requested_size: Option<(u64, u64)>,
    /// Native image width and height before any resizing.
    pub native_size: Option<(u64, u64)>,
    /// Whether resizing was performed.
    pub resized: bool,
}

/// Resize and/or reformat a single image according to requested constraints.
pub fn fit_image_file(
    src_path: &Path,
    req_w: u64,
    req_h: u64,
    fit: &str,
    keep_original: bool,
    request_wants_png: bool,
) -> Result<ResizeOutcome> {
    let native_dims = probe_image_dimensions(src_path);

    let orig_ext = src_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("png")
        .to_lowercase();

    let target_ext = if request_wants_png {
        "png".to_string()
    } else {
        orig_ext.clone()
    };

    let Some((native_w, native_h)) = native_dims else {
        // If file cannot be probed (e.g. mock test payload), leave as-is without failing
        return Ok(ResizeOutcome {
            final_path: src_path.to_path_buf(),
            requested_size: Some((req_w, req_h)),
            native_size: None,
            resized: false,
        });
    };

    let needs_resize = fit != "none" && (native_w != req_w || native_h != req_h);
    let needs_reformat = orig_ext != target_ext;

    if !needs_resize && !needs_reformat {
        return Ok(ResizeOutcome {
            final_path: src_path.to_path_buf(),
            requested_size: Some((req_w, req_h)),
            native_size: Some((native_w, native_h)),
            resized: false,
        });
    }

    let ffmpeg_exe =
        crate::probe::find_in_path("ffmpeg").ok_or_else(|| ToolError::MissingDependency {
            tool_id: "image.resize".into(),
            missing: vec!["bin:ffmpeg".into()],
        })?;

    let parent_dir = src_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = src_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");

    // If keep_original: true, preserve the original file as <name>.orig.<ext>
    if keep_original {
        let orig_backup = parent_dir.join(format!("{stem}.orig.{orig_ext}"));
        if !orig_backup.exists() {
            std::fs::copy(src_path, &orig_backup).map_err(|e| ToolError::Io {
                path: orig_backup.clone(),
                source: e,
            })?;
        }
    }

    let target_path = parent_dir.join(format!("{stem}.{target_ext}"));
    let temp_output = parent_dir.join(format!("{stem}.tmp-{}.{target_ext}", uuid::Uuid::new_v4()));

    let mut cmd = Command::new(&ffmpeg_exe);
    cmd.arg("-y").arg("-i").arg(src_path);

    if needs_resize {
        let filter = match fit {
            "contain" => format!(
                "scale=w={req_w}:h={req_h}:force_original_aspect_ratio=decrease,pad={req_w}:{req_h}:(ow-iw)/2:(oh-ih)/2"
            ),
            // Default "cover"
            _ => format!(
                "scale=w={req_w}:h={req_h}:force_original_aspect_ratio=increase,crop={req_w}:{req_h}"
            ),
        };
        cmd.args(["-vf", &filter]);
    }

    cmd.args(["-update", "1", "-frames:v", "1"])
        .arg(&temp_output);

    let output = cmd.output().map_err(|e| ToolError::ExecutionFailed {
        tool_id: "image.resize".into(),
        reason: format!("Không thể thực thi ffmpeg: {e}"),
    })?;

    if !output.status.success() {
        let _ = std::fs::remove_file(&temp_output);
        return Err(ToolError::ExecutionFailed {
            tool_id: "image.resize".into(),
            reason: format!(
                "FFmpeg xử lý ảnh thất bại: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        });
    }

    // Atomically move temp file to target_path
    if target_path != *src_path && src_path.exists() {
        let _ = std::fs::remove_file(src_path);
    }

    if let Err(_e) = std::fs::rename(&temp_output, &target_path) {
        // Fallback to copy + remove
        std::fs::copy(&temp_output, &target_path).map_err(|io_err| ToolError::Io {
            path: target_path.clone(),
            source: io_err,
        })?;
        let _ = std::fs::remove_file(&temp_output);
    }

    Ok(ResizeOutcome {
        final_path: target_path,
        requested_size: Some((req_w, req_h)),
        native_size: Some((native_w, native_h)),
        resized: needs_resize,
    })
}

/// Shared post-processing pipeline for image generation tools.
/// Modifies the `output` JSON payload in place to record requested_size, native_size,
/// resized, and updates `files` paths if files were renamed/reformatted.
pub async fn post_process_image_output(
    ws: &Workspace,
    _out_dir: &Path,
    input: &Value,
    output: &mut Value,
) -> Result<()> {
    let req_w = input.get("width").and_then(Value::as_u64);
    let req_h = input.get("height").and_then(Value::as_u64);

    let fit = input.get("fit").and_then(Value::as_str).unwrap_or("cover");

    let keep_original = input
        .get("keep_original")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let request_wants_png = input.get("format").and_then(Value::as_str) == Some("png")
        || input.get("output_format").and_then(Value::as_str) == Some("png")
        || input.get("response_format").and_then(Value::as_str) == Some("png");

    let files_arr = match output.get("files").and_then(Value::as_array) {
        Some(arr) => arr.clone(),
        None => return Ok(()),
    };

    let mut new_files = Vec::new();
    let mut any_resized = false;
    let mut recorded_native = None;
    let mut recorded_req = None;

    for file_val in files_arr {
        let Some(file_str) = file_val.as_str() else {
            new_files.push(file_val);
            continue;
        };

        let resolved_path = if let Ok(scoped) = ws.scoped_path(file_str) {
            scoped
        } else {
            ws.root().join(file_str)
        };

        if !resolved_path.is_file() {
            new_files.push(file_val);
            continue;
        }

        if let (Some(rw), Some(rh)) = (req_w, req_h) {
            match fit_image_file(
                &resolved_path,
                rw,
                rh,
                fit,
                keep_original,
                request_wants_png,
            ) {
                Ok(outcome) => {
                    if outcome.resized {
                        any_resized = true;
                    }
                    if let Some(n) = outcome.native_size {
                        recorded_native = Some(format!("{}x{}", n.0, n.1));
                    }
                    if let Some(r) = outcome.requested_size {
                        recorded_req = Some(format!("{}x{}", r.0, r.1));
                    }

                    // Map back to workspace relative
                    let rel = outcome
                        .final_path
                        .strip_prefix(ws.root())
                        .map(|p| p.to_string_lossy().replace('\\', "/"))
                        .unwrap_or_else(|_| outcome.final_path.to_string_lossy().to_string());
                    new_files.push(json!(rel));
                }
                Err(e) => {
                    tracing::warn!("Lỗi khi post-process ảnh {}: {e}", resolved_path.display());
                    new_files.push(file_val);
                }
            }
        } else {
            // No width/height requested
            if let Some(dims) = probe_image_dimensions(&resolved_path) {
                recorded_native = Some(format!("{}x{}", dims.0, dims.1));
            }
            new_files.push(file_val);
        }
    }

    output["files"] = json!(new_files);

    if let (Some(rw), Some(rh)) = (req_w, req_h) {
        output["requested_size"] = json!(recorded_req.unwrap_or_else(|| format!("{rw}x{rh}")));
    }
    if let Some(native_str) = recorded_native {
        output["native_size"] = json!(native_str);
    }
    output["resized"] = json!(any_resized);

    Ok(())
}
