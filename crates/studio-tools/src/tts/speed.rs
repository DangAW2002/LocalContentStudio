//! Speech rate control, syllable counting, and deterministic tempo adjustment for TTS.
//!
//! Provides accurate speech rhythm calculation:
//! - Syllable counting for normalized Vietnamese text (ignoring pause markers).
//! - Target duration comparison vs. 1.0 speed baseline.
//! - Deterministic FFmpeg `atempo` filter application (clamped to [0.7, 1.0]) for any deficit.
//! - Effective speed and syllables/second metric computation.

use crate::error::{Result, ToolError};
use std::path::Path;
use std::process::Command;

/// Count spoken syllables (tiếng) in normalized Vietnamese text.
///
/// Strips custom pause tags `[ngắt ...]` and tokenizes based on whitespace
/// and punctuation boundaries. Each non-empty alphanumeric token counts as 1 syllable.
pub fn count_vietnamese_syllables(text: &str) -> usize {
    // 1. Remove pause tags `[ngắt ...]`
    let cleaned = regex::Regex::new(r"\[ngắt\s+[^\]]+\]")
        .map(|re| re.replace_all(text, " ").to_string())
        .unwrap_or_else(|_| text.to_string());

    // 2. Count tokens that contain at least one alphanumeric character
    cleaned
        .split_whitespace()
        .filter(|token| {
            // Trim common punctuation marks from token edges
            let trimmed = token.trim_matches(|c: char| {
                c.is_ascii_punctuation()
                    || matches!(c, '…' | '—' | '–' | '“' | '”' | '‘' | '’' | '«' | '»')
            });
            trimmed.chars().any(|c| c.is_alphanumeric())
        })
        .count()
}

/// Calculate the required FFmpeg `atempo` ratio to achieve target duration.
///
/// Returns `Some(tempo)` clamped within `[0.7, 1.0]` if the synthesized audio
/// is faster than the requested duration, or `None` if the target pace is already reached.
///
/// - `duration_base`: duration at speed 1.0
/// - `speed_target`: requested speed multiplier (e.g. 0.85)
/// - `duration_measured`: actual measured duration from synthesizer (with length_scale)
pub fn calculate_atempo(
    duration_base: f64,
    speed_target: f32,
    duration_measured: f64,
) -> Option<f64> {
    if duration_base <= 0.0 || duration_measured <= 0.0 || speed_target <= 0.0 {
        return None;
    }

    let target_duration = duration_base / (speed_target as f64);

    // If actual synthesized duration is already >= target duration (within 2ms tolerance),
    // no speed slowing is needed.
    if duration_measured >= target_duration - 0.002 {
        return None;
    }

    // Ratio needed to stretch duration_measured to target_duration:
    // With ffmpeg filter atempo=R, output_duration = input_duration / R
    // So R = duration_measured / target_duration = (duration_measured * speed_target) / duration_base
    let needed_ratio = duration_measured / target_duration;

    // Clamp within safety limits [0.7, 1.0] to prevent audio degradation
    let clamped_ratio = needed_ratio.clamp(0.7, 1.0);

    // Only apply if slowing factor is noticeable (< 0.995)
    if clamped_ratio < 0.995 {
        Some((clamped_ratio * 10000.0).round() / 10000.0)
    } else {
        None
    }
}

/// Apply FFmpeg `atempo` audio filter to stretch audio duration while preserving pitch.
pub fn apply_atempo_filter(input_wav: &Path, output_wav: &Path, tempo: f64) -> Result<()> {
    let ffmpeg_exe =
        crate::probe::find_in_path("ffmpeg").ok_or_else(|| ToolError::MissingDependency {
            tool_id: "tts.atempo".into(),
            missing: vec!["bin:ffmpeg (Yêu cầu FFmpeg để điều chỉnh tốc độ giọng đọc)".into()],
        })?;

    if let Some(parent) = output_wav.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let filter_arg = format!("atempo={tempo:.4}");
    let mut cmd = Command::new(&ffmpeg_exe);
    cmd.arg("-y")
        .arg("-i")
        .arg(input_wav)
        .arg("-filter:a")
        .arg(&filter_arg)
        .arg("-vn")
        .arg(output_wav);

    let output = cmd.output().map_err(|e| ToolError::ExecutionFailed {
        tool_id: "tts.atempo".into(),
        reason: format!("Không thể thực thi lệnh FFmpeg: {e}"),
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ToolError::ExecutionFailed {
            tool_id: "tts.atempo".into(),
            reason: format!(
                "FFmpeg atempo thất bại ({}) cho tỷ lệ {tempo:.4}: {}",
                output.status,
                stderr.trim()
            ),
        });
    }

    if !output_wav.is_file() {
        return Err(ToolError::ExecutionFailed {
            tool_id: "tts.atempo".into(),
            reason: format!(
                "FFmpeg không tạo được tệp âm thanh đầu ra: {}",
                output_wav.display()
            ),
        });
    }

    Ok(())
}

/// Calculate effective speed multiplier comparing base duration vs. final spoken duration.
pub fn compute_effective_speed(total_base_duration: f64, total_spoken_duration: f64) -> f32 {
    if total_spoken_duration <= 0.0 || total_base_duration <= 0.0 {
        return 1.0;
    }
    let eff = (total_base_duration / total_spoken_duration) as f32;
    (eff * 100.0).round() / 100.0
}

/// Calculate syllables per second for the final composite audio (including pause gaps).
pub fn compute_syllables_per_second(syllables: usize, total_duration_seconds: f64) -> f32 {
    if total_duration_seconds <= 0.0 || syllables == 0 {
        return 0.0;
    }
    let rate = (syllables as f64 / total_duration_seconds) as f32;
    (rate * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_vietnamese_syllables() {
        // Simple sentence
        assert_eq!(count_vietnamese_syllables("Xin chào bạn."), 3);

        // Sentence with numbers, symbols, pauses
        let text = "Hôm nay, ngày hai mươi bốn tháng mười, tại thành phố Hồ Chí Minh [ngắt 0.7] nhiệt độ đạt ba mươi hai độ C.";
        // "Hôm nay" (2) + "ngày hai mươi bốn tháng mười" (6) + "tại thành phố Hồ Chí Minh" (6) + "nhiệt độ đạt ba mươi hai độ C" (8) = 22
        assert_eq!(count_vietnamese_syllables(text), 22);

        // Text with punctuation and multiple pause tags
        let text2 = "Một [ngắt 0.5], hai [ngắt 1.0]... ba?!";
        assert_eq!(count_vietnamese_syllables(text2), 3);
    }

    #[test]
    fn test_calculate_atempo_logic() {
        let base_dur = 4.0;

        // Case 1: Target speed is 1.0, synth achieves 4.0 -> No atempo needed
        assert_eq!(calculate_atempo(base_dur, 1.0, 4.0), None);

        // Case 2: Target speed is 0.8 -> Target duration = 4.0 / 0.8 = 5.0s
        // Synth only achieved 4.36s (Piper typical 9% stretch instead of 25%)
        // Needed ratio = 4.36 / 5.0 = 0.872
        let ratio = calculate_atempo(base_dur, 0.8, 4.36);
        assert!(ratio.is_some());
        let val = ratio.unwrap();
        assert!((val - 0.872).abs() < 0.001);

        // Case 3: Target speed is 0.5 (very slow) -> Target duration = 8.0s
        // Synth achieved 4.5s
        // Needed ratio = 4.5 / 8.0 = 0.5625 -> Clamped to 0.7
        let ratio_clamped = calculate_atempo(base_dur, 0.5, 4.5);
        assert_eq!(ratio_clamped, Some(0.7));

        // Case 4: Synth already reached target duration (5.1s >= 5.0s) -> None
        assert_eq!(calculate_atempo(base_dur, 0.8, 5.1), None);
    }

    #[test]
    fn test_effective_speed_and_syllables_per_second() {
        let base = 10.0;
        let spoken = 12.5; // slow down by 1.25x -> speed 0.80
        assert_eq!(compute_effective_speed(base, spoken), 0.80);

        let syllables = 60;
        let total_time = 20.0;
        assert_eq!(compute_syllables_per_second(syllables, total_time), 3.0);
    }
}
