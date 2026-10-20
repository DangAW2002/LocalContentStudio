//! Builtin provider for media.inspect@1 capability.
//!
//! Inspects audio and video container formats, codecs, stream properties, loudness (LUFS),
//! and extracts sample frames at 10%, 50%, and 90% of duration.

use crate::error::{Result, ToolError};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;
use studio_workspace::Workspace;

/// Parse integrated loudness (LUFS) from FFmpeg ebur128 output stderr.
pub fn parse_ebur128_lufs(stderr: &str) -> Option<f64> {
    let summary_pos = stderr.rfind("Summary:")?;
    let summary = &stderr[summary_pos..];
    let i_pos = summary.find("I:")?;
    let after_i = &summary[i_pos + 2..];
    let lufs_pos = after_i.find("LUFS")?;
    let val_str = after_i[..lufs_pos].trim();
    val_str.parse::<f64>().ok()
}

/// Parse framerate string like "30/1" or "29.97" into f64.
pub fn parse_framerate(fps_str: &str) -> Option<f64> {
    if let Some((num, den)) = fps_str.split_once('/') {
        let n: f64 = num.trim().parse().ok()?;
        let d: f64 = den.trim().parse().ok()?;
        if d > 0.0 {
            Some((n / d * 100.0).round() / 100.0)
        } else {
            None
        }
    } else {
        fps_str.trim().parse::<f64>().ok()
    }
}

/// Run media inspection in-process.
pub async fn run_media_inspect(ws: &Workspace, _out_dir: &Path, input: &Value) -> Result<Value> {
    let path_str = input.get("path").and_then(Value::as_str).ok_or_else(|| {
        ToolError::ParseError("Thiếu tham số 'path' (đường dẫn tương đối trong workspace)".into())
    })?;

    if path_str.trim().is_empty() {
        return Err(ToolError::ParseError(
            "Tham số 'path' không được để trống".into(),
        ));
    }

    // Must stay strictly inside the workspace
    let target_path = ws
        .scoped_path(path_str)
        .map_err(|e| ToolError::ExecutionFailed {
            tool_id: "media.inspect".into(),
            reason: format!("Đường dẫn không hợp lệ hoặc vượt ra ngoài workspace: {e}"),
        })?;

    if !target_path.is_file() {
        return Err(ToolError::ExecutionFailed {
            tool_id: "media.inspect".into(),
            reason: format!("Không tìm thấy tệp phương tiện: {path_str}"),
        });
    }

    let ffprobe_exe =
        crate::probe::find_in_path("ffprobe").ok_or_else(|| ToolError::MissingDependency {
            tool_id: "media.inspect".into(),
            missing: vec!["bin:ffprobe".into()],
        })?;

    let ffmpeg_exe =
        crate::probe::find_in_path("ffmpeg").ok_or_else(|| ToolError::MissingDependency {
            tool_id: "media.inspect".into(),
            missing: vec!["bin:ffmpeg".into()],
        })?;

    // 1. Run ffprobe
    let probe_out = Command::new(&ffprobe_exe)
        .args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            &target_path.to_string_lossy(),
        ])
        .output()
        .map_err(|e| ToolError::ExecutionFailed {
            tool_id: "media.inspect".into(),
            reason: format!("Không thể chạy ffprobe: {e}"),
        })?;

    if !probe_out.status.success() {
        return Err(ToolError::ExecutionFailed {
            tool_id: "media.inspect".into(),
            reason: format!(
                "ffprobe không đọc được tệp: {}",
                String::from_utf8_lossy(&probe_out.stderr)
            ),
        });
    }

    let probe_json: Value = serde_json::from_slice(&probe_out.stdout).map_err(|e| {
        ToolError::ParseError(format!("Không thể đọc kết quả JSON từ ffprobe: {e}"))
    })?;

    // Extract duration
    let duration_seconds = probe_json
        .get("format")
        .and_then(|f| f.get("duration"))
        .and_then(Value::as_str)
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| {
            probe_json
                .get("streams")
                .and_then(Value::as_array)?
                .iter()
                .find_map(|s| {
                    s.get("duration")
                        .and_then(Value::as_str)
                        .and_then(|d| d.parse::<f64>().ok())
                })
        })
        .unwrap_or(0.0);
    let duration_seconds = (duration_seconds * 1000.0).round() / 1000.0;

    let streams = probe_json
        .get("streams")
        .and_then(Value::as_array)
        .map(|a| a.as_slice())
        .unwrap_or(&[][..]);

    let video_stream = streams
        .iter()
        .find(|s| s.get("codec_type").and_then(Value::as_str) == Some("video"));

    let audio_stream = streams
        .iter()
        .find(|s| s.get("codec_type").and_then(Value::as_str) == Some("audio"));

    let has_audio = audio_stream.is_some();

    let width = video_stream
        .and_then(|s| s.get("width"))
        .and_then(Value::as_u64)
        .map(|w| w as u32);

    let height = video_stream
        .and_then(|s| s.get("height"))
        .and_then(Value::as_u64)
        .map(|h| h as u32);

    let fps = video_stream
        .and_then(|s| {
            s.get("r_frame_rate")
                .or_else(|| s.get("avg_frame_rate"))
                .and_then(Value::as_str)
        })
        .and_then(parse_framerate);

    // 2. Measure loudness if audio is present via ebur128
    let loudness_lufs = if has_audio {
        let ebur = Command::new(&ffmpeg_exe)
            .args([
                "-nostats",
                "-i",
                &target_path.to_string_lossy(),
                "-filter_complex",
                "ebur128",
                "-f",
                "null",
                "-",
            ])
            .output();

        match ebur {
            Ok(out) => parse_ebur128_lufs(&String::from_utf8_lossy(&out.stderr)),
            Err(_) => None,
        }
    } else {
        None
    };

    // 3. Extract sample frames at 10%, 50%, 90%
    let mut sample_frames = Vec::new();
    if video_stream.is_some() && duration_seconds > 0.0 {
        // Hash the target path to isolate cache entries per media file
        let mut hasher = Sha256::new();
        hasher.update(path_str.as_bytes());
        let hash_prefix = &hex::encode(hasher.finalize())[..12];

        // Frame cache dir: inside the project if path belongs to a project, else workspace cache
        let cache_dir = determine_cache_dir(ws, path_str, hash_prefix);
        std::fs::create_dir_all(&cache_dir).map_err(|e| ToolError::Io {
            path: cache_dir.clone(),
            source: e,
        })?;

        let sample_points = [
            ("10", duration_seconds * 0.10),
            ("50", duration_seconds * 0.50),
            ("90", duration_seconds * 0.90),
        ];

        for (label, sec) in sample_points {
            let frame_filename = format!("frame_{label}.png");
            let frame_path = cache_dir.join(&frame_filename);

            let status = Command::new(&ffmpeg_exe)
                .args([
                    "-y",
                    "-ss",
                    &format!("{sec:.3}"),
                    "-i",
                    &target_path.to_string_lossy(),
                    "-vframes",
                    "1",
                    &frame_path.to_string_lossy(),
                ])
                .output();

            if let Ok(out) = status {
                if out.status.success() && frame_path.is_file() {
                    if let Ok(rel) = frame_path.strip_prefix(ws.root()) {
                        sample_frames.push(rel.to_string_lossy().replace('\\', "/"));
                    }
                }
            }
        }
    }

    Ok(json!({
        "duration_seconds": duration_seconds,
        "width": width,
        "height": height,
        "fps": fps,
        "has_audio": has_audio,
        "loudness_lufs": loudness_lufs,
        "sample_frames": sample_frames,
    }))
}

/// Determine frame cache directory inside the project, never outside workspace.
fn determine_cache_dir(ws: &Workspace, path_str: &str, hash: &str) -> PathBuf {
    let normalized = path_str.replace('\\', "/");
    let parts: Vec<&str> = normalized.split('/').filter(|p| !p.is_empty()).collect();

    if parts.len() >= 2 && parts[0] == "projects" {
        let project_slug = parts[1];
        ws.root()
            .join("projects")
            .join(project_slug)
            .join(".cache")
            .join("media_inspect")
            .join(hash)
    } else {
        ws.cache_dir().join("media_inspect").join(hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_framerate() {
        assert_eq!(parse_framerate("30/1"), Some(30.0));
        assert_eq!(parse_framerate("24000/1001"), Some(23.98));
        assert_eq!(parse_framerate("25"), Some(25.0));
        assert_eq!(parse_framerate("invalid"), None);
    }

    #[test]
    fn test_parse_ebur128() {
        let sample_stderr = r#"
[Parsed_ebur128_0 @ 0000018d3ea67c00] Summary:

  Integrated loudness:
    I:         -21.4 LUFS
    Threshold: -31.4 LUFS

  Loudness range:
    LRA:         0.0 LU
"#;
        assert_eq!(parse_ebur128_lufs(sample_stderr), Some(-21.4));
    }
}
