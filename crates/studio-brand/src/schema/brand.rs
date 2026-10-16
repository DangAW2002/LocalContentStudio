use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Color palette configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BrandColors {
    /// Primary brand color (hex, e.g. `#1E88E5`)
    pub primary: String,
    /// Accent color (hex)
    pub accent: String,
    /// Background color (hex)
    pub bg: String,
    /// Text color (hex)
    #[serde(default = "default_text_color")]
    pub text: String,
    /// Extra palette colors (hex)
    #[serde(default)]
    pub extra: Vec<String>,
}

fn default_text_color() -> String {
    "#111827".to_string()
}

/// Typography configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BrandFonts {
    /// Heading font family name
    pub heading: String,
    /// Body font family name
    pub body: String,
    /// Optional relative font file paths (e.g. `fonts/Inter-Regular.ttf`)
    #[serde(default)]
    pub files: Vec<String>,
}

/// Logo placement rules or constraints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LogoPlacementRule {
    /// Position description or CSS placement (e.g. `top-left`)
    pub position: String,
    /// Human-readable rule
    pub description: String,
}

/// Logo assets and usage constraints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct BrandLogos {
    /// Path to full logo relative to brand directory (e.g. `logo.svg`)
    pub full: String,
    /// Path to logo mark / symbol relative to brand directory (e.g. `mark.svg`)
    pub mark: String,
    /// Optional light-variant logo path
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub light: Option<String>,
    /// Optional dark-variant logo path
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dark: Option<String>,
    /// Minimum rendered size in pixels
    #[serde(default = "default_min_size_px")]
    pub min_size_px: u32,
    /// Clear space ratio around logo (e.g. 0.25 = 25% of logo height)
    #[serde(default = "default_clear_space_ratio")]
    pub clear_space_ratio: f32,
    /// Placement rules for the logo
    #[serde(default)]
    pub placement_rules: Vec<String>,
}

fn default_min_size_px() -> u32 {
    32
}

fn default_clear_space_ratio() -> f32 {
    0.2
}

/// Words or phrases preferences.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct VoiceWords {
    /// Preferred words or phrases
    #[serde(default)]
    pub prefer: Vec<String>,
    /// Words or phrases to avoid
    #[serde(default)]
    pub avoid: Vec<String>,
}

/// Motion graphics elements (paths relative to brand directory).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct BrandMotion {
    /// Intro animation path or directory
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intro: Option<String>,
    /// Outro animation path or directory
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outro: Option<String>,
    /// Lower-third overlay path or directory
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lower_third: Option<String>,
}

/// Brand specification schema (`brand@1`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct BrandSpec {
    /// Brand name
    pub name: String,
    /// Slogan or tagline
    #[serde(default)]
    pub slogan: String,
    /// Color palette
    pub colors: BrandColors,
    /// Font definitions
    pub fonts: BrandFonts,
    /// Logo definitions and rules
    pub logos: BrandLogos,
    /// Relative path to voice guide markdown (e.g. `voice.md`)
    #[serde(default = "default_voice_path")]
    pub voice: String,
    /// Word choices (prefer / avoid)
    #[serde(default)]
    pub voice_words: VoiceWords,
    /// Motion graphics assets
    #[serde(default)]
    pub motion: BrandMotion,
    /// Freeform brand rules
    #[serde(default)]
    pub rules: Vec<String>,
    /// Schema version (e.g. `brand@1` or `1`)
    #[serde(default = "default_version")]
    pub version: String,
}

fn default_voice_path() -> String {
    "voice.md".to_string()
}

fn default_version() -> String {
    "brand@1".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brand_spec_serde() {
        let yaml = r##"
name: "Kênh Của Tôi"
slogan: "Sáng tạo không giới hạn"
colors:
  primary: "#1E88E5"
  accent: "#FFB300"
  bg: "#FFFFFF"
  text: "#111827"
  extra: ["#43A047"]
fonts:
  heading: "Be Vietnam Pro"
  body: "Inter"
logos:
  full: "logo.svg"
  mark: "mark.svg"
  min_size_px: 48
voice: "voice.md"
voice_words:
  prefer: ["đột phá", "hữu ích"]
  avoid: ["cam kết 100%", "rẻ nhất"]
motion:
  intro: "intro/"
rules:
  - "Logo luôn ở góc trên trái"
version: "brand@1"
"##;
        let spec: BrandSpec = serde_yaml::from_str(yaml).expect("deserialize brand");
        assert_eq!(spec.name, "Kênh Của Tôi");
        assert_eq!(spec.colors.primary, "#1E88E5");
        assert_eq!(spec.logos.min_size_px, 48);
        assert_eq!(spec.voice_words.avoid.len(), 2);
    }
}
