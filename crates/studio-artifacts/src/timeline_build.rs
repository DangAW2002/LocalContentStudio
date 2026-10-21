//! Module for automated timeline construction (`timeline_build`).
//!
//! Lays out segments end-to-end based on audio durations, formats caption clips
//! split by sentence and weighted by length (max 2 lines of ~42 characters each),
//! configures base video and overlay tracks, and writes `artifact.yaml` + `timeline.json`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use studio_workspace::{Project, Workspace};

use crate::artifact::Artifact;
use crate::probe::probe_ffprobe;
use crate::timeline::{Clip, TimelineDoc, Track, TrackKind, Transition};

const MAX_LINE_CHARS: usize = 42;
const MAX_CARD_CHARS: usize = 84;

/// Input specification for a single story segment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SegmentInput {
    /// Workspace-relative path to audio file (e.g. WAV or MP3).
    pub audio: String,
    /// Script / dialog text for this segment used to generate captions.
    pub text: String,
    /// Workspace-relative visual asset path (image, video) or "color:#RRGGBB".
    pub visual: String,
}

/// Input specification for an overlay visual clip (e.g. logo or graphic badge).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct OverlayInput {
    /// Workspace-relative path to overlay image/asset (PNG/JPEG/WEBP).
    pub source: String,
    /// Horizontal offset in pixels from top-left (default: 0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<i32>,
    /// Vertical offset in pixels from top-left (default: 0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<i32>,
    /// Scaled width in pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// Scaled height in pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// Opacity from 0.0 to 1.0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f64>,
    /// Optional start time in seconds (default: 0.0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<f64>,
    /// Optional duration in seconds (default: whole video).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
}

/// Input parameters for `timeline_build`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimelineBuildParams {
    /// Project slug.
    pub project: String,
    /// Artifact ID to create or overwrite (kind: timeline).
    pub artifact: String,
    /// Ordered list of story segments.
    pub segments: Vec<SegmentInput>,
    /// Lead buffer time in seconds before first audio starts (default: 0.5s).
    #[serde(default = "default_lead_s")]
    pub lead_s: f64,
    /// Gap buffer time in seconds between segment audios (default: 0.4s).
    #[serde(default = "default_gap_s")]
    pub gap_s: f64,
    /// Tail buffer time in seconds after last audio ends (default: 1.5s).
    #[serde(default = "default_tail_s")]
    pub tail_s: f64,
    /// Visual transition between video clips: "fade" (default 0.5s) or "none".
    #[serde(default = "default_transition")]
    pub transition: String,
    /// Caption mode: "sentences" (default) or "none".
    #[serde(default = "default_captions")]
    pub captions: String,
    /// Optional overlay clips (e.g. brand logo).
    #[serde(default)]
    pub overlays: Vec<OverlayInput>,
    /// Optional brand slug.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brand: Option<String>,
    /// Target channels (e.g. ["youtube"]).
    #[serde(default)]
    pub channels: Vec<String>,
    /// Optional video width (default: from first channel format, else 1920).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// Optional video height (default: from first channel format, else 1080).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// Optional frame rate (default: from first channel format, else 30.0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fps: Option<f64>,
    /// Minimum total duration in seconds (pads tail if video is shorter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_duration_s: Option<f64>,
}

fn default_lead_s() -> f64 {
    0.5
}
fn default_gap_s() -> f64 {
    0.4
}
fn default_tail_s() -> f64 {
    1.5
}
fn default_transition() -> String {
    "fade".into()
}
fn default_captions() -> String {
    "sentences".into()
}

/// Output timing for a single segment on the timeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SegmentOutput {
    /// Start time on the timeline in seconds.
    pub start: f64,
    /// Duration on the timeline in seconds.
    pub duration: f64,
}

/// Result returned by `timeline_build`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimelineBuildResult {
    /// Artifact ID created or overwritten.
    pub artifact: String,
    /// Total duration of the timeline video in seconds.
    pub duration_s: f64,
    /// Timing of each segment on the timeline.
    pub segments: Vec<SegmentOutput>,
    /// Total number of caption clips generated.
    pub captions_count: usize,
}

/// Caption card chunk with formatted text and character count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptionCard {
    /// Line-wrapped text (at most 2 lines, each <= ~42 chars).
    pub text: String,
    /// Character count excluding newlines.
    pub char_len: usize,
}

/// Helper to round an f64 to 3 decimal places (milliseconds).
pub fn round_ms(val: f64) -> f64 {
    (val * 1000.0).round() / 1000.0
}

/// Probe audio duration using `ffprobe`.
pub fn probe_audio_duration(
    ffprobe_path: &Path,
    audio_file: &Path,
    rel_display: &str,
) -> Result<f64, String> {
    let output = Command::new(ffprobe_path)
        .args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            audio_file.to_str().unwrap_or_default(),
        ])
        .output()
        .map_err(|e| {
            format!(
                "Không thể thực thi ffprobe trên tệp âm thanh \"{}\": {}. Vui lòng kiểm tra cài đặt FFmpeg.",
                rel_display, e
            )
        })?;

    if !output.status.success() {
        let err_text = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Không thể đọc thời lượng tệp âm thanh \"{}\": {}. Vui lòng kiểm tra tệp âm thanh có đúng định dạng hợp lệ (WAV, MP3, AAC...).",
            rel_display,
            err_text.trim()
        ));
    }

    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|e| {
        format!(
            "Không thể phân tích kết quả ffprobe cho tệp \"{}\": {}.",
            rel_display, e
        )
    })?;

    let dur_opt = parsed["format"]["duration"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| {
            parsed["streams"].as_array().and_then(|streams| {
                streams.iter().find_map(|s| {
                    if s["codec_type"].as_str() == Some("audio") {
                        s["duration"].as_str().and_then(|d| d.parse::<f64>().ok())
                    } else {
                        None
                    }
                })
            })
        });

    let dur = dur_opt.ok_or_else(|| {
        format!(
            "Không tìm thấy thông tin thời lượng trong tệp âm thanh \"{}\". Vui lòng kiểm tra lại luồng âm thanh.",
            rel_display
        )
    })?;

    if dur <= 0.0 {
        return Err(format!(
            "Thời lượng tệp âm thanh \"{}\" không hợp lệ ({:.3}s). Thời lượng phải lớn hơn 0.",
            rel_display, dur
        ));
    }

    Ok(dur)
}

/// Split input text into distinct sentences based on punctuation and line breaks.
pub fn split_text_to_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return sentences;
    }

    let chars: Vec<char> = trimmed.chars().collect();
    let n = chars.len();
    let mut start = 0;
    let mut i = 0;

    while i < n {
        let ch = chars[i];
        let is_newline = ch == '\n' || ch == '\r';
        let is_punct = ch == '.' || ch == '!' || ch == '?' || ch == '…';

        if is_newline {
            let sentence: String = chars[start..i].iter().collect();
            let s_trim = sentence.trim();
            if !s_trim.is_empty() {
                sentences.push(s_trim.to_string());
            }
            while i < n && (chars[i] == '\n' || chars[i] == '\r' || chars[i].is_whitespace()) {
                i += 1;
            }
            start = i;
            continue;
        }

        if is_punct {
            // Check if this period is part of a decimal number (e.g. 3.14)
            let is_decimal = ch == '.'
                && i > 0
                && chars[i - 1].is_ascii_digit()
                && i + 1 < n
                && chars[i + 1].is_ascii_digit();

            if !is_decimal {
                // Consume consecutive punctuation marks like "...", "!?", "?!"
                let mut p_end = i + 1;
                while p_end < n
                    && (chars[p_end] == '.'
                        || chars[p_end] == '!'
                        || chars[p_end] == '?'
                        || chars[p_end] == '…')
                {
                    p_end += 1;
                }
                // Include trailing quote/bracket if immediately after punctuation
                while p_end < n
                    && (chars[p_end] == '"'
                        || chars[p_end] == '”'
                        || chars[p_end] == '\''
                        || chars[p_end] == ')'
                        || chars[p_end] == ']')
                {
                    p_end += 1;
                }

                // Sentence ends if at end of string or followed by whitespace
                if p_end == n || chars[p_end].is_whitespace() {
                    let sentence: String = chars[start..p_end].iter().collect();
                    let s_trim = sentence.trim();
                    if !s_trim.is_empty() {
                        sentences.push(s_trim.to_string());
                    }
                    while p_end < n && chars[p_end].is_whitespace() {
                        p_end += 1;
                    }
                    start = p_end;
                    i = p_end;
                    continue;
                }
            }
        }

        i += 1;
    }

    if start < n {
        let sentence: String = chars[start..n].iter().collect();
        let s_trim = sentence.trim();
        if !s_trim.is_empty() {
            sentences.push(s_trim.to_string());
        }
    }

    sentences
}

/// Split a sentence longer than MAX_CARD_CHARS into smaller chunks.
pub fn split_sentence_into_chunks(sentence: &str) -> Vec<String> {
    let char_count = sentence.chars().count();
    if char_count <= MAX_CARD_CHARS {
        return vec![sentence.to_string()];
    }

    let mut chunks = Vec::new();
    let mut remaining = sentence.trim().to_string();

    while remaining.chars().count() > MAX_CARD_CHARS {
        let chars: Vec<char> = remaining.chars().collect();
        let limit = MAX_CARD_CHARS.min(chars.len());

        // 1. Look for punctuation break in [limit / 2 .. limit]
        let mut best_split = None;
        let p_delims = [',', ';', ':', '—', '-'];

        for idx in (limit / 2..limit).rev() {
            let c = chars[idx];
            if p_delims.contains(&c) && (idx + 1 < chars.len() && chars[idx + 1].is_whitespace()) {
                best_split = Some(idx + 1);
                break;
            }
        }

        // 2. If no punctuation break, look for rightmost space in [limit / 3 .. limit]
        if best_split.is_none() {
            for idx in (limit / 3..limit).rev() {
                if chars[idx].is_whitespace() {
                    best_split = Some(idx);
                    break;
                }
            }
        }

        // 3. Fallback: first space after limit, or hard cut
        if best_split.is_none() {
            for (idx, ch) in chars.iter().enumerate().skip(limit) {
                if ch.is_whitespace() {
                    best_split = Some(idx);
                    break;
                }
            }
        }

        let split_pos = best_split.unwrap_or(limit);
        let left: String = chars[..split_pos].iter().collect();
        let right: String = chars[split_pos..].iter().collect();

        let left_trim = left.trim();
        if !left_trim.is_empty() {
            chunks.push(left_trim.to_string());
        }
        remaining = right.trim().to_string();
    }

    if !remaining.is_empty() {
        chunks.push(remaining);
    }

    chunks
}

/// Wrap a chunk into at most 2 lines of ~42 characters each.
pub fn format_card_text(chunk: &str) -> String {
    let trimmed = chunk.trim();
    let char_count = trimmed.chars().count();
    if char_count <= MAX_LINE_CHARS {
        return trimmed.to_string();
    }

    let chars: Vec<char> = trimmed.chars().collect();
    let total_len = chars.len();
    let target = total_len / 2;

    // Find space candidates and score them
    let mut best_idx = None;
    let mut best_score = usize::MAX;

    for (idx, &ch) in chars.iter().enumerate() {
        if ch.is_whitespace() {
            let line1_len = idx;
            let line2_len = total_len.saturating_sub(idx + 1);

            let overflow1 = line1_len.saturating_sub(MAX_LINE_CHARS);
            let overflow2 = line2_len.saturating_sub(MAX_LINE_CHARS);

            let diff = idx.abs_diff(target);
            let score = overflow1 * 100 + overflow2 * 100 + diff;

            if score < best_score {
                best_score = score;
                best_idx = Some(idx);
            }
        }
    }

    if let Some(idx) = best_idx {
        let line1: String = chars[..idx].iter().collect();
        let line2: String = chars[idx + 1..].iter().collect();
        format!("{}\n{}", line1.trim(), line2.trim())
    } else {
        trimmed.to_string()
    }
}

/// Split a segment's text into caption clips weighted by length.
pub fn split_captions(text: &str, audio_start: f64, audio_dur: f64, seg_index: usize) -> Vec<Clip> {
    if text.trim().is_empty() || audio_dur <= 0.0 {
        return Vec::new();
    }

    let sentences = split_text_to_sentences(text);
    let mut cards = Vec::new();

    for s in &sentences {
        let chunks = split_sentence_into_chunks(s);
        for chunk in chunks {
            let formatted = format_card_text(&chunk);
            let char_len = formatted.chars().filter(|c| *c != '\n').count();
            if char_len > 0 {
                cards.push(CaptionCard {
                    text: formatted,
                    char_len,
                });
            }
        }
    }

    if cards.is_empty() {
        return Vec::new();
    }

    let total_chars: usize = cards.iter().map(|c| c.char_len).sum();
    let start_ms = (audio_start * 1000.0).round() as i64;
    let dur_ms = (audio_dur * 1000.0).round() as i64;
    let mut current_ms = start_ms;

    let mut clips = Vec::with_capacity(cards.len());
    let card_count = cards.len();

    for (card_idx, card) in cards.iter().enumerate() {
        let card_dur_ms = if card_idx == card_count - 1 {
            (start_ms + dur_ms) - current_ms
        } else {
            let frac = (card.char_len as f64) / (total_chars as f64);
            (dur_ms as f64 * frac).round() as i64
        };

        let clip_start = (current_ms as f64) / 1000.0;
        let clip_dur = (card_dur_ms as f64) / 1000.0;
        current_ms += card_dur_ms;

        clips.push(Clip {
            id: format!("sub_{}_{}", seg_index + 1, card_idx + 1),
            source: card.text.clone(),
            in_point: 0.0,
            out_point: None,
            start: round_ms(clip_start),
            duration: round_ms(clip_dur),
            transition: Transition::None,
            transition_duration: None,
            ..Default::default()
        });
    }

    clips
}

/// Compute timeline layout math for segments, overlays, and total duration.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineLayout {
    pub total_duration: f64,
    pub video_clips: Vec<Clip>,
    pub audio_clips: Vec<Clip>,
    pub overlay_clips: Vec<Clip>,
    pub segment_outputs: Vec<SegmentOutput>,
    pub starts: Vec<f64>,
}

/// Calculate the timeline layout deterministically from audio durations and inputs.
pub fn compute_timeline_layout(
    params: &TimelineBuildParams,
    audio_durations: &[f64],
) -> Result<TimelineLayout, String> {
    let n = params.segments.len();
    if n == 0 {
        return Err(
            "Danh sách 'segments' không được để trống. Cần ít nhất 1 phân cảnh để dựng timeline."
                .into(),
        );
    }
    if audio_durations.len() != n {
        return Err("Số lượng thời lượng âm thanh không khớp với số lượng phân cảnh.".into());
    }

    let mut starts = Vec::with_capacity(n);
    let mut current_start = params.lead_s;

    for dur in audio_durations {
        starts.push(round_ms(current_start));
        current_start += *dur + params.gap_s;
    }

    let last_audio_dur = audio_durations[n - 1];
    let raw_end = starts[n - 1] + last_audio_dur + params.tail_s;
    let total_duration = if let Some(min_d) = params.min_duration_s {
        round_ms(raw_end.max(min_d))
    } else {
        round_ms(raw_end)
    };

    let (transition_kind, transition_dur) = match params.transition.to_lowercase().as_str() {
        "none" => (Transition::None, None),
        _ => (Transition::Fade, Some(0.5)),
    };

    let mut video_clips = Vec::with_capacity(n);
    let mut audio_clips = Vec::with_capacity(n);
    let mut segment_outputs = Vec::with_capacity(n);

    for (i, seg) in params.segments.iter().enumerate() {
        let vs = if i == 0 { 0.0 } else { starts[i] };
        let ve = if i + 1 < n {
            starts[i + 1]
        } else {
            total_duration
        };

        let vs_r = round_ms(vs);
        let ve_r = round_ms(ve);
        let dur_r = round_ms(ve_r - vs_r);

        video_clips.push(Clip {
            id: format!("v{}", i + 1),
            source: seg.visual.clone(),
            in_point: 0.0,
            out_point: None,
            start: vs_r,
            duration: dur_r,
            transition: transition_kind,
            transition_duration: transition_dur,
            ..Default::default()
        });

        let a_start_r = starts[i];
        let a_dur_r = round_ms(audio_durations[i]);

        audio_clips.push(Clip {
            id: format!("vo{}", i + 1),
            source: seg.audio.clone(),
            in_point: 0.0,
            out_point: None,
            start: a_start_r,
            duration: a_dur_r,
            transition: Transition::None,
            transition_duration: None,
            ..Default::default()
        });

        segment_outputs.push(SegmentOutput {
            start: vs_r,
            duration: dur_r,
        });
    }

    let mut overlay_clips = Vec::new();
    for (idx, ov) in params.overlays.iter().enumerate() {
        let ov_start = ov.start.unwrap_or(0.0);
        let ov_dur = ov.duration.unwrap_or(total_duration - ov_start);
        let ov_start_r = round_ms(ov_start);
        let ov_dur_r = round_ms(ov_dur);

        overlay_clips.push(Clip {
            id: format!("ov{}", idx + 1),
            source: ov.source.clone(),
            in_point: 0.0,
            out_point: None,
            start: ov_start_r,
            duration: ov_dur_r,
            transition: Transition::None,
            transition_duration: None,
            x: ov.x,
            y: ov.y,
            width: ov.width,
            height: ov.height,
            opacity: ov.opacity,
        });
    }

    Ok(TimelineLayout {
        total_duration,
        video_clips,
        audio_clips,
        overlay_clips,
        segment_outputs,
        starts,
    })
}

/// Execute `timeline_build`: validates paths, probes audio files, lays out timeline,
/// splits captions, writes `artifact.yaml` + `timeline.json`, and returns summary.
pub fn build_timeline(
    ws: &Workspace,
    params: TimelineBuildParams,
) -> Result<TimelineBuildResult, String> {
    if params.segments.is_empty() {
        return Err(
            "Danh sách 'segments' không được để trống. Cần ít nhất 1 phân cảnh để dựng timeline."
                .into(),
        );
    }

    // The id becomes a directory name under artifacts/: keep it a plain name.
    let id_ok = !params.artifact.is_empty()
        && params.artifact.len() <= 64
        && params
            .artifact
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !id_ok {
        return Err(format!(
            "ID artifact \"{}\" không hợp lệ: chỉ dùng chữ không dấu, số, '-' hoặc '_' (tối đa 64 ký tự), ví dụ 'video-main'.",
            params.artifact
        ));
    }

    // 1. Open project
    let project = Project::open(ws, &params.project).map_err(|e| {
        format!(
            "Không tìm thấy dự án \"{}\" trong workspace: {}. Vui lòng tạo dự án trước.",
            params.project, e
        )
    })?;

    // 2. Locate ffprobe
    let probe = probe_ffprobe();
    let ffprobe_path = probe.path.ok_or_else(|| {
        "Không tìm thấy công cụ ffprobe — hãy cài đặt qua winget: 'winget install Gyan.FFmpeg' hoặc tải từ https://ffmpeg.org".to_string()
    })?;

    // 3. Validate paths and probe audio durations
    let mut audio_durations = Vec::with_capacity(params.segments.len());
    let mut collected_inputs = Vec::new();

    let validate_workspace_path = |rel_str: &str, field_name: &str| -> Result<PathBuf, String> {
        let clean = rel_str.trim().replace('\\', "/");
        if clean.is_empty() {
            return Err(format!(
                "Đường dẫn trong trường \"{field_name}\" không được để trống."
            ));
        }
        let p = ws.scoped_path(&clean).map_err(|e| {
            format!(
                "Đường dẫn \"{clean}\" của {field_name} không hợp lệ hoặc nằm ngoài workspace: {e}. Vui lòng chỉ sử dụng đường dẫn tương đối trong workspace (ví dụ: 'projects/<slug>/assets/...')."
            )
        })?;
        if !p.exists() {
            return Err(format!(
                "Không tìm thấy tệp \"{clean}\" của {field_name} tại \"{}\". Vui lòng kiểm tra lại đường dẫn và đảm bảo tệp đã được tạo trước khi dựng timeline.",
                p.display()
            ));
        }
        Ok(p)
    };

    for (i, seg) in params.segments.iter().enumerate() {
        // Audio
        let audio_path =
            validate_workspace_path(&seg.audio, &format!("audio phân cảnh {}", i + 1))?;
        let dur = probe_audio_duration(&ffprobe_path, &audio_path, &seg.audio)?;
        audio_durations.push(dur);
        collected_inputs.push(format!("asset:{}", seg.audio.replace('\\', "/")));

        // Visual
        let visual_src = seg.visual.trim();
        if visual_src.to_ascii_lowercase().ends_with(".svg") {
            return Err(format!(
                "Nguồn visual \"{visual_src}\" có định dạng SVG. FFmpeg không thể giải mã trực tiếp định dạng SVG; vui lòng render file SVG sang PNG thông qua design artifact trước khi đưa vào timeline."
            ));
        }
        if !visual_src.starts_with("color:") {
            validate_workspace_path(visual_src, &format!("visual phân cảnh {}", i + 1))?;
            collected_inputs.push(format!("asset:{}", visual_src.replace('\\', "/")));
        }
    }

    for (i, ov) in params.overlays.iter().enumerate() {
        let ov_src = ov.source.trim();
        if ov_src.to_ascii_lowercase().ends_with(".svg") {
            return Err(format!(
                "Nguồn overlay \"{ov_src}\" có định dạng SVG. FFmpeg không thể giải mã trực tiếp định dạng SVG; vui lòng render file SVG sang PNG thông qua design artifact trước khi đưa vào timeline."
            ));
        }
        if !ov_src.starts_with("color:") {
            validate_workspace_path(ov_src, &format!("overlay {}", i + 1))?;
            collected_inputs.push(format!("asset:{}", ov_src.replace('\\', "/")));
        }
    }

    // 4. Resolve width, height, fps, loudness_lufs
    let brand_set = studio_brand::BrandSet::load(ws).ok();
    let first_channel_spec = params
        .channels
        .first()
        .and_then(|c| brand_set.as_ref().and_then(|set| set.get_channel(c)));

    let first_format = first_channel_spec.and_then(|ch| ch.formats.first());

    let width = params
        .width
        .or_else(|| first_format.map(|f| f.width))
        .unwrap_or(1920);

    let height = params
        .height
        .or_else(|| first_format.map(|f| f.height))
        .unwrap_or(1080);

    let fps = params
        .fps
        .or_else(|| first_format.and_then(|f| f.fps).map(|fps| fps as f64))
        .unwrap_or(30.0);

    let loudness_lufs = first_channel_spec
        .and_then(|ch| ch.loudness_lufs)
        .map(f64::from);

    // 5. Compute layout
    let layout = compute_timeline_layout(&params, &audio_durations)?;

    // 6. Split captions if enabled
    let mut caption_clips = Vec::new();
    if params.captions.to_lowercase() != "none" {
        for (i, seg) in params.segments.iter().enumerate() {
            let caps = split_captions(&seg.text, layout.starts[i], audio_durations[i], i);
            caption_clips.extend(caps);
        }
    }
    let captions_count = caption_clips.len();

    // 7. Assemble tracks
    let mut tracks = vec![Track {
        id: "video".into(),
        kind: TrackKind::Video,
        clips: layout.video_clips,
    }];

    if !layout.overlay_clips.is_empty() {
        tracks.push(Track {
            id: "overlays".into(),
            kind: TrackKind::Video,
            clips: layout.overlay_clips,
        });
    }

    tracks.push(Track {
        id: "audio".into(),
        kind: TrackKind::Audio,
        clips: layout.audio_clips,
    });

    if !caption_clips.is_empty() {
        tracks.push(Track {
            id: "captions".into(),
            kind: TrackKind::Captions,
            clips: caption_clips,
        });
    }

    let timeline_doc = TimelineDoc {
        version: 1,
        width,
        height,
        fps,
        tracks,
        loudness_lufs,
    };

    // 8. Write timeline.json and artifact.yaml (never turn another kind into a timeline)
    let existing_art = Artifact::load(&project, &params.artifact).ok();
    if let Some(existing) = &existing_art {
        if existing.kind != "timeline" {
            return Err(format!(
                "Artifact \"{}\" đã tồn tại với loại \"{}\" — không ghi đè thành timeline. Hãy chọn ID khác.",
                params.artifact, existing.kind
            ));
        }
    }
    let art_dir = project.artifacts_dir().join(&params.artifact);
    fs::create_dir_all(&art_dir).map_err(|e| {
        format!(
            "Không thể tạo thư mục artifact \"{}\": {e}",
            art_dir.display()
        )
    })?;

    let json_bytes = serde_json::to_string_pretty(&timeline_doc)
        .map_err(|e| format!("Không thể serialize timeline.json: {e}"))?;
    fs::write(art_dir.join("timeline.json"), json_bytes).map_err(|e| {
        format!(
            "Không thể ghi tệp timeline.json tại \"{}\": {e}",
            art_dir.display()
        )
    })?;

    let title = existing_art
        .as_ref()
        .map(|a| a.title.clone())
        .unwrap_or_else(|| format!("Timeline {}", params.artifact));
    let brand = params
        .brand
        .clone()
        .or_else(|| existing_art.as_ref().and_then(|a| a.brand.clone()));
    let channels = if !params.channels.is_empty() {
        params.channels.clone()
    } else {
        existing_art
            .as_ref()
            .map(|a| a.channels.clone())
            .unwrap_or_default()
    };

    let artifact = Artifact {
        id: params.artifact.clone(),
        kind: "timeline".into(),
        title,
        brand,
        channels,
        source: "timeline.json".into(),
        renderer: Some("render.timeline_ffmpeg@1".into()),
        inputs: collected_inputs
            .into_iter()
            .map(crate::artifact::ArtifactInput::from_str_ref)
            .collect(),
        params: serde_json::json!({
            "width": width,
            "height": height,
            "fps": fps,
        }),
        width: Some(width),
        height: Some(height),
        // Rebuilding keeps the user's lock and notes on the artifact.
        pinned: existing_art.as_ref().is_some_and(|a| a.pinned),
        notes: existing_art
            .as_ref()
            .map(|a| a.notes.clone())
            .unwrap_or_default(),
    };
    artifact
        .save(&project)
        .map_err(|e| format!("Không thể ghi artifact.yaml: {e}"))?;

    Ok(TimelineBuildResult {
        artifact: params.artifact,
        duration_s: layout.total_duration,
        segments: layout.segment_outputs,
        captions_count,
    })
}

/// Thin MCP tool entrypoint for `timeline_build`.
pub async fn build_timeline_tool(
    ws: &Workspace,
    args: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let params: TimelineBuildParams = serde_json::from_value(args.clone())
        .map_err(|e| format!("Tham số timeline_build không hợp lệ: {e}"))?;
    let result = build_timeline(ws, params)?;
    serde_json::to_value(result).map_err(|e| format!("Không thể chuyển đổi kết quả JSON: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_vietnamese_text_sentences() {
        let text = "Cà phê phin là biểu tượng văn hóa Việt Nam! Từng giọt tí tách rơi thật chậm. Bạn đã thử chưa?";
        let sentences = split_text_to_sentences(text);
        assert_eq!(sentences.len(), 3);
        assert_eq!(sentences[0], "Cà phê phin là biểu tượng văn hóa Việt Nam!");
        assert_eq!(sentences[1], "Từng giọt tí tách rơi thật chậm.");
        assert_eq!(sentences[2], "Bạn đã thử chưa?");
    }

    #[test]
    fn test_split_sentences_with_numbers_and_ellipsis() {
        let text = "Nước sôi đạt 95.5 độ C là lý tưởng… Hãy rót khoảng 30 ml nước trước.";
        let sentences = split_text_to_sentences(text);
        assert_eq!(sentences.len(), 2);
        assert_eq!(sentences[0], "Nước sôi đạt 95.5 độ C là lý tưởng…");
        assert_eq!(sentences[1], "Hãy rót khoảng 30 ml nước trước.");
    }

    #[test]
    fn test_format_card_text_wrap() {
        // Short text <= 42 chars -> single line
        let short = "Xin chào các bạn!";
        assert_eq!(format_card_text(short), short);

        // Medium text 43-84 chars -> wrapped into 2 lines
        let medium = "Cà phê phin truyền thống thơm ngon đậm đà khó quên.";
        let wrapped = format_card_text(medium);
        let lines: Vec<&str> = wrapped.split('\n').collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].chars().count() <= MAX_LINE_CHARS);
        assert!(lines[1].chars().count() <= MAX_LINE_CHARS);
        assert_eq!(format!("{} {}", lines[0], lines[1]), medium);
    }

    #[test]
    fn test_split_long_sentence_chunks() {
        // Sentence > 84 chars with comma
        let long_s = "Từng giọt cà phê chầm chậm rơi qua chiếc phin nhôm nhỏ bé, lắng đọng lại tất cả sự tinh tế và kiên nhẫn của người thưởng thức.";
        assert!(long_s.chars().count() > 84);
        let chunks = split_sentence_into_chunks(long_s);
        assert!(chunks.len() >= 2);
        for chunk in &chunks {
            assert!(chunk.chars().count() <= MAX_CARD_CHARS);
        }
    }

    #[test]
    fn test_split_captions_timing_and_durations() {
        let text = "Cà phê phin Việt Nam. Từng giọt tí tách rơi.";
        let clips = split_captions(text, 0.5, 4.0, 0);
        assert_eq!(clips.len(), 2);
        assert_eq!(clips[0].start, 0.5);
        assert_eq!(clips[0].duration + clips[1].duration, 4.0);
        assert_eq!(round_ms(clips[0].start + clips[0].duration), clips[1].start);
    }

    #[test]
    fn test_layout_math_and_determinism() {
        let params = TimelineBuildParams {
            project: "p1".into(),
            artifact: "video".into(),
            segments: vec![
                SegmentInput {
                    audio: "a1.wav".into(),
                    text: "Đoạn 1".into(),
                    visual: "v1.png".into(),
                },
                SegmentInput {
                    audio: "a2.wav".into(),
                    text: "Đoạn 2".into(),
                    visual: "color:#1E88E5".into(),
                },
            ],
            lead_s: 0.5,
            gap_s: 0.4,
            tail_s: 1.5,
            transition: "fade".into(),
            captions: "sentences".into(),
            overlays: vec![OverlayInput {
                source: "logo.png".into(),
                x: Some(50),
                y: Some(50),
                width: Some(150),
                height: None,
                opacity: Some(0.9),
                start: None,
                duration: None,
            }],
            brand: Some("demo".into()),
            channels: vec!["youtube".into()],
            width: Some(1920),
            height: Some(1080),
            fps: Some(30.0),
            min_duration_s: Some(10.0),
        };

        let durations = vec![3.2, 4.5];
        let layout1 = compute_timeline_layout(&params, &durations).unwrap();
        let layout2 = compute_timeline_layout(&params, &durations).unwrap();

        assert_eq!(layout1, layout2);
        // starts: a1 starts at 0.5; a2 starts at 0.5 + 3.2 + 0.4 = 4.1
        assert_eq!(layout1.starts, vec![0.5, 4.1]);
        // raw_end = 4.1 + 4.5 + 1.5 = 10.1 (>= 10.0 min_duration_s)
        assert_eq!(layout1.total_duration, 10.1);

        // Visual clips:
        // Clip 0: start 0.0, duration 4.1 (reaches starts[1])
        assert_eq!(layout1.video_clips[0].start, 0.0);
        assert_eq!(layout1.video_clips[0].duration, 4.1);
        assert_eq!(layout1.video_clips[0].transition, Transition::Fade);

        // Clip 1: start 4.1, duration 10.1 - 4.1 = 6.0
        assert_eq!(layout1.video_clips[1].start, 4.1);
        assert_eq!(layout1.video_clips[1].duration, 6.0);

        // Overlay: whole video (0.0 to 10.1)
        assert_eq!(layout1.overlay_clips[0].start, 0.0);
        assert_eq!(layout1.overlay_clips[0].duration, 10.1);
        assert_eq!(layout1.overlay_clips[0].width, Some(150));
    }
}
