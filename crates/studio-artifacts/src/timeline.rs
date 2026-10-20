use serde::{Deserialize, Serialize};

/// Timeline model representation in `timeline.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimelineDoc {
    /// Schema version
    #[serde(default = "default_version")]
    pub version: u32,
    /// Video width in pixels
    #[serde(default = "default_width")]
    pub width: u32,
    /// Video height in pixels
    #[serde(default = "default_height")]
    pub height: u32,
    /// Framerate
    #[serde(default = "default_fps")]
    pub fps: f64,
    /// Ordered list of tracks
    #[serde(default)]
    pub tracks: Vec<Track>,
    /// Integrated loudness target (LUFS) for the mixed audio. When absent, the render uses the
    /// target of the artifact's channel, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loudness_lufs: Option<f64>,
}

fn default_version() -> u32 {
    1
}
fn default_width() -> u32 {
    1920
}
fn default_height() -> u32 {
    1080
}
fn default_fps() -> f64 {
    30.0
}

/// Track in a timeline document.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Track {
    /// Track identifier
    pub id: String,
    /// Track kind: "video", "audio", or "captions"
    pub kind: TrackKind,
    /// Ordered list of clips in this track
    #[serde(default)]
    pub clips: Vec<Clip>,
}

/// Track kind.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TrackKind {
    Video,
    Audio,
    Captions,
    #[serde(rename = "caption")]
    Caption,
}

impl TrackKind {
    /// Returns true if this track is for captions/subtitles.
    pub fn is_caption(&self) -> bool {
        matches!(self, Self::Captions | Self::Caption)
    }
}

/// Transition specification between clips.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum Transition {
    #[default]
    None,
    Fade,
}

/// Clip placed on a track.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Clip {
    /// Unique clip identifier within track
    pub id: String,
    /// Source reference: asset path, render path, or text (for captions).
    /// `text` / `path` are accepted too — agents naturally write captions as `text`.
    #[serde(alias = "text", alias = "path")]
    pub source: String,
    /// Source in-point in seconds
    #[serde(default)]
    pub in_point: f64,
    /// Source out-point in seconds (if None, whole source is used)
    #[serde(default)]
    pub out_point: Option<f64>,
    /// Start time on the timeline in seconds
    #[serde(default)]
    pub start: f64,
    /// Duration on the timeline in seconds
    pub duration: f64,
    /// Transition into this clip
    #[serde(default)]
    pub transition: Transition,
    /// Transition duration in seconds if transition is Fade
    #[serde(default)]
    pub transition_duration: Option<f64>,
    /// Horizontal offset in pixels from top-left (default: 0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<i32>,
    /// Vertical offset in pixels from top-left (default: 0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<i32>,
    /// Optional scaled width in pixels. If height is omitted, aspect ratio is preserved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// Optional scaled height in pixels. If width is omitted, aspect ratio is preserved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// Opacity from 0.0 (transparent) to 1.0 (fully opaque).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f64>,
}

#[cfg(test)]
mod alias_tests {
    use super::*;

    #[test]
    fn caption_clip_accepts_text_for_source() {
        let doc: TimelineDoc = serde_json::from_str(
            r#"{"tracks":[{"id":"s","kind":"captions","clips":[{"id":"t1","text":"Xin chào","start":0,"duration":2}]}]}"#,
        )
        .unwrap();
        assert_eq!(doc.tracks[0].clips[0].source, "Xin chào");
    }

    #[test]
    fn clip_optional_overlay_fields_deserialization() {
        let doc: TimelineDoc = serde_json::from_str(
            r#"{"tracks":[{"id":"v2","kind":"video","clips":[{"id":"logo","source":"logo.png","start":1.0,"duration":5.0,"x":50,"y":30,"width":200,"opacity":0.85}]}]}"#,
        )
        .unwrap();
        let clip = &doc.tracks[0].clips[0];
        assert_eq!(clip.x, Some(50));
        assert_eq!(clip.y, Some(30));
        assert_eq!(clip.width, Some(200));
        assert_eq!(clip.height, None);
        assert_eq!(clip.opacity, Some(0.85));
    }
}
