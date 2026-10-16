use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Safe zone defined as fractional margins [0.0, 1.0] from canvas edges.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SafeZone {
    /// Top margin fraction (e.g. 0.05 = 5%)
    #[serde(default)]
    pub top: f32,
    /// Right margin fraction
    #[serde(default)]
    pub right: f32,
    /// Bottom margin fraction (e.g. 0.20 = 20% for Shorts UI overlay)
    #[serde(default)]
    pub bottom: f32,
    /// Left margin fraction
    #[serde(default)]
    pub left: f32,
}

impl Default for SafeZone {
    fn default() -> Self {
        Self {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }
    }
}

/// Output media format specification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FormatSpec {
    /// Format identifier name (e.g. `shorts`, `post`, `landscape`)
    pub name: String,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Aspect ratio string (e.g. `9:16`, `16:9`, `1:1`)
    pub aspect: String,
    /// Target or recommended frames per second
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fps: Option<f32>,
    /// Maximum duration in seconds (if video)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_duration_s: Option<f64>,
    /// Minimum duration in seconds (if video)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_duration_s: Option<f64>,
    /// Safe zone within the frame
    #[serde(default)]
    pub safe_zone: SafeZone,
}

/// Text limits for content published to the channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct TextLimits {
    /// Maximum title character count
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_max: Option<usize>,
    /// Maximum description character count
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description_max: Option<usize>,
    /// Maximum caption / post text character count
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_max: Option<usize>,
    /// Maximum hashtag count
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hashtags_max: Option<usize>,
}

/// File packaging constraints.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct FileConstraints {
    /// Allowed container / extension list (e.g. `["mp4", "mov"]` or `["png", "jpg"]`)
    #[serde(default)]
    pub containers: Vec<String>,
    /// Maximum file size in megabytes
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_size_mb: Option<u64>,
}

/// Channel specification schema (`channel@1`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ChannelSpec {
    /// Channel identifier (e.g. `youtube`, `youtube-shorts`, `tiktok`)
    pub id: String,
    /// Social / video platform name (e.g. `youtube`, `tiktok`, `instagram`)
    pub platform: String,
    /// Channel display title
    pub title: String,
    /// List of supported media formats
    pub formats: Vec<FormatSpec>,
    /// Text constraints
    #[serde(default)]
    pub text: TextLimits,
    /// Whether captions/subtitles are required
    #[serde(default)]
    pub captions_required: bool,
    /// Target integrated loudness in LUFS (e.g. -14.0 for YouTube)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loudness_lufs: Option<f32>,
    /// File constraints
    #[serde(default)]
    pub file: FileConstraints,
    /// Editorial or platform notes (e.g. "cần xác minh với nền tảng")
    #[serde(default)]
    pub notes: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_spec_serde() {
        let yaml = r#"
id: "youtube-shorts"
platform: "youtube"
title: "YouTube Shorts"
formats:
  - name: "shorts"
    width: 1080
    height: 1920
    aspect: "9:16"
    fps: 30.0
    max_duration_s: 60.0
    min_duration_s: 3.0
    safe_zone:
      top: 0.05
      bottom: 0.20
      left: 0.05
      right: 0.05
text:
  title_max: 100
  description_max: 5000
captions_required: true
loudness_lufs: -14.0
file:
  containers: ["mp4", "mov"]
  max_size_mb: 500
notes: "Tối đa 60 giây"
"#;
        let spec: ChannelSpec = serde_yaml::from_str(yaml).expect("deserialize channel");
        assert_eq!(spec.id, "youtube-shorts");
        assert_eq!(spec.formats[0].width, 1080);
        assert_eq!(spec.formats[0].height, 1920);
        assert_eq!(spec.formats[0].safe_zone.bottom, 0.20);
        assert!(spec.captions_required);
    }
}
