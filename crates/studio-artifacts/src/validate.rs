use std::fs;
use std::path::Path;
use std::process::Command;

use crate::error::{ArtifactError, Result};
use crate::probe::probe_ffprobe;

/// Metadata for a validated image file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageMeta {
    pub width: u32,
    pub height: u32,
    pub format: String,
}

/// Validate PNG image file dimensions by parsing its 8-byte signature and IHDR chunk.
///
/// PNG signature: 89 50 4E 47 0D 0A 1A 0A
/// IHDR chunk immediately follows: 4 bytes length (usually 13), 4 bytes "IHDR",
/// then 4 bytes width (big-endian), 4 bytes height (big-endian).
pub fn validate_png(
    path: &Path,
    expected_width: Option<u32>,
    expected_height: Option<u32>,
) -> Result<ImageMeta> {
    if !path.exists() {
        return Err(ArtifactError::ValidationFailed(format!(
            "PNG file does not exist: {}",
            path.display()
        )));
    }

    let bytes = fs::read(path)?;
    if bytes.len() < 24 {
        return Err(ArtifactError::ValidationFailed(format!(
            "PNG file is too short ({} bytes): {}",
            bytes.len(),
            path.display()
        )));
    }

    const PNG_HEADER: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    if bytes[0..8] != PNG_HEADER {
        return Err(ArtifactError::ValidationFailed(format!(
            "File is not a valid PNG (invalid header): {}",
            path.display()
        )));
    }

    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);

    if let Some(exp_w) = expected_width {
        if width != exp_w {
            return Err(ArtifactError::ValidationFailed(format!(
                "PNG width mismatch in {}: expected {}, got {}",
                path.display(),
                exp_w,
                width
            )));
        }
    }

    if let Some(exp_h) = expected_height {
        if height != exp_h {
            return Err(ArtifactError::ValidationFailed(format!(
                "PNG height mismatch in {}: expected {}, got {}",
                path.display(),
                exp_h,
                height
            )));
        }
    }

    Ok(ImageMeta {
        width,
        height,
        format: "png".into(),
    })
}

/// Metadata for a validated video file via ffprobe.
#[derive(Debug, Clone, PartialEq)]
pub struct VideoMeta {
    pub duration_seconds: f64,
    pub width: u32,
    pub height: u32,
    pub video_codec: String,
    pub audio_codec: Option<String>,
    pub num_streams: usize,
}

/// Validate video output file using ffprobe.
pub fn validate_video(
    path: &Path,
    ffprobe_path: Option<&Path>,
    min_duration: Option<f64>,
) -> Result<VideoMeta> {
    if !path.exists() {
        return Err(ArtifactError::ValidationFailed(format!(
            "Video file does not exist: {}",
            path.display()
        )));
    }

    let probe = probe_ffprobe();
    let bin_path = if let Some(p) = ffprobe_path {
        p.to_path_buf()
    } else if let Some(p) = probe.path {
        p
    } else {
        return Err(ArtifactError::ToolNotFound(
            "ffprobe is required to validate video output but was not found".into(),
        ));
    };

    // Run ffprobe: ffprobe -v quiet -print_format json -show_format -show_streams <path>
    let output = Command::new(&bin_path)
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            path.to_str().unwrap_or_default(),
        ])
        .output()
        .map_err(|e| ArtifactError::ExecutionFailed {
            message: format!("Failed to execute ffprobe: {}", e),
            details: None,
        })?;

    if !output.status.success() {
        let err_text = String::from_utf8_lossy(&output.stderr);
        return Err(ArtifactError::ValidationFailed(format!(
            "ffprobe validation failed: {}",
            err_text
        )));
    }

    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout)?;

    let streams = parsed["streams"]
        .as_array()
        .ok_or_else(|| ArtifactError::ValidationFailed("No streams found in video".into()))?;

    let mut video_codec = None;
    let mut audio_codec = None;
    let mut width = 0;
    let mut height = 0;

    for s in streams {
        let codec_type = s["codec_type"].as_str().unwrap_or("");
        if codec_type == "video" && video_codec.is_none() {
            video_codec = s["codec_name"].as_str().map(|s| s.to_string());
            width = s["width"].as_u64().unwrap_or(0) as u32;
            height = s["height"].as_u64().unwrap_or(0) as u32;
        } else if codec_type == "audio" && audio_codec.is_none() {
            audio_codec = s["codec_name"].as_str().map(|s| s.to_string());
        }
    }

    let video_codec = video_codec.ok_or_else(|| {
        ArtifactError::ValidationFailed("No video stream found in container".into())
    })?;

    let duration_str = parsed["format"]["duration"].as_str().unwrap_or("0.0");
    let duration: f64 = duration_str.parse().unwrap_or(0.0);

    if let Some(min_d) = min_duration {
        if duration < min_d {
            return Err(ArtifactError::ValidationFailed(format!(
                "Video duration too short: {}s (expected >= {}s)",
                duration, min_d
            )));
        }
    }

    Ok(VideoMeta {
        duration_seconds: duration,
        width,
        height,
        video_codec,
        audio_codec,
        num_streams: streams.len(),
    })
}
