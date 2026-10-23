//! Text segmentation into clauses and sentences with natural pauses.
//!
//! Splits input text into discrete speech segments, assigning each segment
//! an associated silence duration (`pause_after_s`) based on punctuation,
//! paragraph breaks, or explicit in-text `[ngắt X]` tags.

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::LazyLock;

/// Pause duration configuration in seconds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PauseConfig {
    /// Pause after comma `,`, semicolon `;`, colon `:` (default 0.18s).
    pub comma: f32,
    /// Pause after semicolon `;` (default 0.18s).
    pub semicolon: f32,
    /// Pause after colon `:` (default 0.18s).
    pub colon: f32,
    /// Pause after sentence-ending period `.` (default 0.45s).
    pub period: f32,
    /// Pause after question mark `?` (default 0.50s).
    pub question: f32,
    /// Pause after exclamation mark `!` (default 0.50s).
    pub exclamation: f32,
    /// Pause after ellipsis `...` or `…` (default 0.60s).
    pub ellipsis: f32,
    /// Pause after empty line / new paragraph `\n\n` (default 0.80s).
    pub paragraph: f32,
}

impl Default for PauseConfig {
    fn default() -> Self {
        Self {
            comma: 0.18,
            semicolon: 0.18,
            colon: 0.18,
            period: 0.45,
            question: 0.50,
            exclamation: 0.50,
            ellipsis: 0.60,
            paragraph: 0.80,
        }
    }
}

impl PauseConfig {
    /// Construct a PauseConfig from an optional JSON Value (supporting both textual and symbol keys).
    pub fn from_value(val: Option<&Value>) -> Self {
        let mut cfg = Self::default();
        let Some(obj) = val.and_then(Value::as_object) else {
            return cfg;
        };

        let get_f32 = |keys: &[&str]| -> Option<f32> {
            for k in keys {
                if let Some(v) = obj.get(*k).and_then(Value::as_f64) {
                    return Some(v as f32);
                }
            }
            None
        };

        if let Some(v) = get_f32(&[",", "comma"]) {
            cfg.comma = v;
        }
        if let Some(v) = get_f32(&[";", "semicolon"]) {
            cfg.semicolon = v;
        }
        if let Some(v) = get_f32(&[":", "colon"]) {
            cfg.colon = v;
        }
        if let Some(v) = get_f32(&[".", "period"]) {
            cfg.period = v;
        }
        if let Some(v) = get_f32(&["?", "question"]) {
            cfg.question = v;
        }
        if let Some(v) = get_f32(&["!", "exclamation"]) {
            cfg.exclamation = v;
        }
        if let Some(v) = get_f32(&["...", "…", "ellipsis"]) {
            cfg.ellipsis = v;
        }
        if let Some(v) = get_f32(&["\n\n", "paragraph", "break"]) {
            cfg.paragraph = v;
        }

        cfg
    }
}

/// A synthesized speech segment with text and post-segment silence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TtsSegment {
    /// Text content to synthesize.
    pub text: String,
    /// Pause duration in seconds to append after this segment's audio.
    pub pause_after_s: f32,
}

static NGAT_TAG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[ngắt\s+([0-9]+(?:\.[0-9]+)?)(?:s|giây)?\]").expect("valid ngat tag regex")
});

static PARAGRAPH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\r?\n\s*\r?\n+").expect("valid paragraph regex"));

/// Split text into clauses / sentences with associated pause durations.
pub fn split_into_tts_segments(text: &str, pauses: &PauseConfig) -> Vec<TtsSegment> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    // First, split into raw tokens: text chunks, punctuation boundaries, or explicit tags.
    // We scan character by character or use a regex to match delimiters:
    // 1. `[ngắt X]`
    // 2. `\r?\n\s*\r?\n+` (paragraph break)
    // 3. `...` or `…`
    // 4. `?` or `!`
    // 5. `.`
    // 6. `,` or `;` or `:`
    // 7. single `\n`

    let delim_re = Regex::new(
        r"(?x)
        (?P<tag>\[ngắt\s+[0-9]+(?:\.[0-9]+)?(?:s|giây)?\])
        | (?P<para>\r?\n\s*\r?\n+)
        | (?P<ellipsis>\.{3,}|…)
        | (?P<punct>[?!.,;:])
        | (?P<newline>\r?\n)
        ",
    )
    .expect("valid delim regex");

    let mut raw_segments: Vec<(String, f32)> = Vec::new();
    let mut last_end = 0;
    let mut pending_text = String::new();

    for mat in delim_re.find_iter(text) {
        let chunk = &text[last_end..mat.start()];
        pending_text.push_str(chunk);
        let m_str = mat.as_str();

        let pause_val: f32;
        let mut keep_punct_in_text = false;

        if let Some(caps) = NGAT_TAG_RE.captures(m_str) {
            pause_val = caps[1].parse::<f32>().unwrap_or(0.5);
        } else if PARAGRAPH_RE.is_match(m_str) {
            pause_val = pauses.paragraph;
        } else if m_str.starts_with("...") || m_str.contains('…') {
            pause_val = pauses.ellipsis;
            keep_punct_in_text = true;
        } else if m_str == "?" {
            pause_val = pauses.question;
            keep_punct_in_text = true;
        } else if m_str == "!" {
            pause_val = pauses.exclamation;
            keep_punct_in_text = true;
        } else if m_str == "." {
            pause_val = pauses.period;
            keep_punct_in_text = true;
        } else if m_str == "," {
            pause_val = pauses.comma;
            keep_punct_in_text = true;
        } else if m_str == ";" {
            pause_val = pauses.semicolon;
            keep_punct_in_text = true;
        } else if m_str == ":" {
            pause_val = pauses.colon;
            keep_punct_in_text = true;
        } else if m_str == "\n" || m_str == "\r\n" {
            // Single line break: if text exists, pause like comma/clause
            pause_val = pauses.comma;
        } else {
            pause_val = pauses.comma;
        }

        if keep_punct_in_text {
            pending_text.push_str(m_str);
        }

        let clean_text = pending_text.trim().to_string();
        if !clean_text.is_empty() {
            raw_segments.push((clean_text, pause_val));
            pending_text.clear();
        } else if let Some(last) = raw_segments.last_mut() {
            // Accumulate pause onto previous segment if text was empty
            last.1 = last.1.max(pause_val);
        }

        last_end = mat.end();
    }

    // Trailing text after last delimiter
    let remaining = &text[last_end..];
    pending_text.push_str(remaining);
    let clean_tail = pending_text.trim().to_string();
    if !clean_tail.is_empty() {
        raw_segments.push((clean_tail, 0.0));
    }

    if raw_segments.is_empty() {
        return Vec::new();
    }

    // If the last segment has a terminal punctuation pause (e.g. from `.`),
    // check whether it was an explicit `[ngắt]` or just terminal punctuation.
    // For standard punctuation at the very end of the script, 0.0 trailing silence
    // is preferred so there is no dead audio buffer at the end of the WAV.
    let len = raw_segments.len();
    if len > 0 {
        let last_delim = &text[last_end..];
        let has_explicit_tail_ngat = NGAT_TAG_RE.is_match(last_delim);
        if !has_explicit_tail_ngat {
            raw_segments[len - 1].1 = 0.0;
        }
    }

    raw_segments
        .into_iter()
        .map(|(t, p)| TtsSegment {
            text: t,
            pause_after_s: (p * 1000.0).round() / 1000.0,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pause_config_defaults_and_overrides() {
        let cfg = PauseConfig::default();
        assert!((cfg.comma - 0.18).abs() < f32::EPSILON);
        assert!((cfg.period - 0.45).abs() < f32::EPSILON);
        assert!((cfg.question - 0.50).abs() < f32::EPSILON);
        assert!((cfg.exclamation - 0.50).abs() < f32::EPSILON);
        assert!((cfg.ellipsis - 0.60).abs() < f32::EPSILON);
        assert!((cfg.paragraph - 0.80).abs() < f32::EPSILON);

        let custom_json = serde_json::json!({
            ",": 0.25,
            "period": 0.65,
            "question": 0.75,
            "paragraph": 1.2
        });
        let custom_cfg = PauseConfig::from_value(Some(&custom_json));
        assert!((custom_cfg.comma - 0.25).abs() < f32::EPSILON);
        assert!((custom_cfg.period - 0.65).abs() < f32::EPSILON);
        assert!((custom_cfg.question - 0.75).abs() < f32::EPSILON);
        assert!((custom_cfg.paragraph - 1.2).abs() < f32::EPSILON);
        // Untouched remains default
        assert!((custom_cfg.ellipsis - 0.60).abs() < f32::EPSILON);
    }

    #[test]
    fn test_split_by_punctuation() {
        let text = "Xin chào các bạn, tôi là trợ lý ảo. Bạn có câu hỏi gì không? Hãy cho tôi biết!";
        let pauses = PauseConfig::default();
        let segments = split_into_tts_segments(text, &pauses);

        assert_eq!(segments.len(), 4);
        assert_eq!(segments[0].text, "Xin chào các bạn,");
        assert!((segments[0].pause_after_s - 0.18).abs() < 0.01);

        assert_eq!(segments[1].text, "tôi là trợ lý ảo.");
        assert!((segments[1].pause_after_s - 0.45).abs() < 0.01);

        assert_eq!(segments[2].text, "Bạn có câu hỏi gì không?");
        assert!((segments[2].pause_after_s - 0.50).abs() < 0.01);

        assert_eq!(segments[3].text, "Hãy cho tôi biết!");
        // Last segment trailing silence is 0.0
        assert!((segments[3].pause_after_s - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_split_with_ngat_tag() {
        let text = "Điều này rất quan trọng [ngắt 0.7] xin vui lòng chú ý.";
        let pauses = PauseConfig::default();
        let segments = split_into_tts_segments(text, &pauses);

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].text, "Điều này rất quan trọng");
        assert!((segments[0].pause_after_s - 0.70).abs() < 0.01);

        assert_eq!(segments[1].text, "xin vui lòng chú ý.");
        assert!((segments[1].pause_after_s - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_split_with_paragraph_and_ellipsis() {
        let text = "Đoạn một kết thúc...\n\nĐoạn hai bắt đầu.";
        let pauses = PauseConfig::default();
        let segments = split_into_tts_segments(text, &pauses);

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].text, "Đoạn một kết thúc...");
        // Paragraph break pause after segment 1: 0.8s
        assert!((segments[0].pause_after_s - 0.80).abs() < 0.01);

        assert_eq!(segments[1].text, "Đoạn hai bắt đầu.");
        assert!((segments[1].pause_after_s - 0.0).abs() < 0.01);
    }
}
