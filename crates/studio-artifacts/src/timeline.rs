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
    /// Optional caption styling (applied to caption tracks)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<CaptionStyle>,
}

/// Styling options for burned-in captions on a caption track.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct CaptionStyle {
    /// Font family name (resolved in Windows Fonts directory, e.g. "Segoe UI", "Arial")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    /// Font size in pixels (default: scaled to ~4.5% of video height)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<u32>,
    /// Text color (#RRGGBB or named color, default: "white")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Outline border width in pixels (default: 0)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outline: Option<u32>,
    /// Whether to show background box behind captions (default: true)
    #[serde(default, rename = "box", skip_serializing_if = "Option::is_none")]
    pub is_box: Option<bool>,
    /// Background box color (#RRGGBBAA or #RRGGBB or color@opacity, default: "black@0.6")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub box_color: Option<String>,
    /// Bottom margin in pixels from bottom edge (default: scaled to ~8% of video height)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_bottom: Option<u32>,
    /// Maximum width ratio of captions relative to video width (0.0 to 1.0)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_width_ratio: Option<f64>,
}

impl CaptionStyle {
    /// Allowed field names for caption style.
    pub const ALLOWED_FIELDS: &'static [&'static str] = &[
        "font",
        "font_size",
        "color",
        "outline",
        "box",
        "box_color",
        "margin_bottom",
        "max_width_ratio",
    ];
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
    Cut,
}

/// Ken Burns motion effect for still visual clips.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MotionKind {
    #[default]
    None,
    ZoomIn,
    ZoomOut,
    PanLeft,
    PanRight,
    PanUp,
    PanDown,
    Auto,
}

/// Deterministically resolve an `Auto` motion based on scene index.
///
/// Cycles through `[zoom_in, zoom_out, pan_left, pan_right]` to create natural,
/// alternating visual rhythm across scenes.
pub fn resolve_auto_motion(index: usize) -> MotionKind {
    match index % 4 {
        0 => MotionKind::ZoomIn,
        1 => MotionKind::ZoomOut,
        2 => MotionKind::PanLeft,
        3 => MotionKind::PanRight,
        _ => MotionKind::ZoomIn,
    }
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
    /// Optional Ken Burns motion effect for still images
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motion: Option<MotionKind>,
    /// Motion strength / zoom ratio (e.g. 0.08 for 8% zoom, default: 0.08)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motion_strength: Option<f64>,
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

    #[test]
    fn caption_style_deserialization_and_serialization() {
        let json_str = r##"{"font":"segoeui","font_size":48,"color":"#ffffff","outline":2,"box":true,"box_color":"black@0.6","margin_bottom":86,"max_width_ratio":0.8}"##;
        let style: CaptionStyle = serde_json::from_str(json_str).unwrap();
        assert_eq!(style.font, Some("segoeui".to_string()));
        assert_eq!(style.font_size, Some(48));
        assert_eq!(style.color, Some("#ffffff".to_string()));
        assert_eq!(style.outline, Some(2));
        assert_eq!(style.is_box, Some(true));
        assert_eq!(style.box_color, Some("black@0.6".to_string()));
        assert_eq!(style.margin_bottom, Some(86));
        assert_eq!(style.max_width_ratio, Some(0.8));

        let track = Track {
            id: "cap".into(),
            kind: TrackKind::Captions,
            clips: vec![],
            style: Some(style),
        };
        let serialized = serde_json::to_string(&track).unwrap();
        assert!(serialized.contains(r#""box":true"#));
        assert!(serialized.contains(r#""font":"segoeui""#));

        // When style is None, "style" key must not be serialized (hash unchanged)
        let track_none = Track {
            id: "cap2".into(),
            kind: TrackKind::Captions,
            clips: vec![],
            style: None,
        };
        let serialized_none = serde_json::to_string(&track_none).unwrap();
        assert!(!serialized_none.contains("style"));
    }

    #[test]
    fn caption_style_denies_unknown_fields() {
        let json_str = r#"{"font":"segoeui","unknown_field":123}"#;
        let err = serde_json::from_str::<CaptionStyle>(json_str).unwrap_err();
        let err_msg = err.to_string();
        assert!(err_msg.contains("unknown field `unknown_field`"));
        assert!(err_msg.contains("font"));
        assert!(err_msg.contains("margin_bottom"));
    }
}
