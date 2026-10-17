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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Clip {
    /// Unique clip identifier within track
    pub id: String,
    /// Source reference: asset path, render path, or text (for captions)
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
}
