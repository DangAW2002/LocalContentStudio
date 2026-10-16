use crate::checkers::color::Rgb;
use crate::checkers::{CheckInput, Checker, Finding};
use regex::Regex;
use std::sync::OnceLock;

/// Maximum Delta E CIE76 tolerance considered "within brand palette".
pub const DEFAULT_DELTA_E_TOLERANCE: f64 = 8.0;

/// Verifies that colors used in HTML/CSS or SVG sources lie within the brand palette ± ΔE tolerance.
#[derive(Debug, Default)]
pub struct PaletteChecker {
    tolerance: f64,
}

impl PaletteChecker {
    pub fn new() -> Self {
        Self {
            tolerance: DEFAULT_DELTA_E_TOLERANCE,
        }
    }

    pub fn with_tolerance(tolerance: f64) -> Self {
        Self { tolerance }
    }
}

static COLOR_HEX_RE: OnceLock<Regex> = OnceLock::new();
static RGB_RE: OnceLock<Regex> = OnceLock::new();

fn hex_regex() -> &'static Regex {
    COLOR_HEX_RE.get_or_init(|| Regex::new(r"#([0-9a-fA-F]{3,8})\b").expect("valid regex"))
}

fn rgb_regex() -> &'static Regex {
    RGB_RE.get_or_init(|| Regex::new(r"rgba?\s*\([^)]+\)").expect("valid regex"))
}

impl Checker for PaletteChecker {
    fn id(&self) -> &'static str {
        "palette"
    }

    fn check(&self, input: &CheckInput) -> Vec<Finding> {
        let mut findings = Vec::new();
        let content = match input.source_content {
            Some(c) => c,
            None => return findings,
        };

        // Collect brand palette RGBs
        let mut brand_colors: Vec<(&str, Rgb)> = Vec::new();
        if let Some(c) = Rgb::from_hex(&input.brand.colors.primary) {
            brand_colors.push(("primary", c));
        }
        if let Some(c) = Rgb::from_hex(&input.brand.colors.accent) {
            brand_colors.push(("accent", c));
        }
        if let Some(c) = Rgb::from_hex(&input.brand.colors.bg) {
            brand_colors.push(("bg", c));
        }
        if let Some(c) = Rgb::from_hex(&input.brand.colors.text) {
            brand_colors.push(("text", c));
        }
        for extra in input.brand.colors.extra.iter() {
            if let Some(c) = Rgb::from_hex(extra) {
                brand_colors.push(("extra", c));
            }
        }

        // Also permit standard black/white unless brand strictly forbids
        brand_colors.push(("white", Rgb::new(255, 255, 255)));
        brand_colors.push(("black", Rgb::new(0, 0, 0)));

        // Extract colors from content
        let mut extracted: Vec<(String, Rgb)> = Vec::new();

        for mat in hex_regex().find_iter(content) {
            let s = mat.as_str();
            if let Some(rgb) = Rgb::from_hex(s) {
                extracted.push((s.to_string(), rgb));
            }
        }

        for mat in rgb_regex().find_iter(content) {
            let s = mat.as_str();
            if let Some(rgb) = Rgb::parse_css(s) {
                extracted.push((s.to_string(), rgb));
            }
        }

        for (raw_str, color) in extracted {
            // Find minimum Delta E to any brand color
            let min_delta_e = brand_colors
                .iter()
                .map(|(_, b_color)| color.delta_e_cie76(b_color))
                .fold(f64::INFINITY, f64::min);

            if min_delta_e > self.tolerance {
                findings.push(
                    Finding::warn(
                        "PALETTE_OFF",
                        format!("Màu '{raw_str}' không nằm trong bảng màu brand (ΔE = {min_delta_e:.1} > {})", self.tolerance),
                    )
                    .with_fix_hint("Thay bằng màu trong palette của brand (primary, accent, bg, text, extra)."),
                );
            }
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{
        BrandColors, BrandFonts, BrandLogos, BrandSpec, ChannelSpec, FileConstraints, TextLimits,
    };
    use std::path::Path;

    fn test_brand() -> BrandSpec {
        BrandSpec {
            name: "Test Brand".to_string(),
            slogan: "".to_string(),
            colors: BrandColors {
                primary: "#1E88E5".to_string(),
                accent: "#FFB300".to_string(),
                bg: "#FFFFFF".to_string(),
                text: "#111827".to_string(),
                extra: vec![],
            },
            fonts: BrandFonts {
                heading: "Inter".to_string(),
                body: "Inter".to_string(),
                files: vec![],
            },
            logos: BrandLogos {
                full: "logo.svg".to_string(),
                mark: "mark.svg".to_string(),
                light: None,
                dark: None,
                min_size_px: 32,
                clear_space_ratio: 0.2,
                placement_rules: vec![],
            },
            voice: "voice.md".to_string(),
            voice_words: Default::default(),
            motion: Default::default(),
            rules: vec![],
            version: "brand@1".to_string(),
        }
    }

    fn test_channel() -> ChannelSpec {
        ChannelSpec {
            id: "yt".to_string(),
            platform: "youtube".to_string(),
            title: "YT".to_string(),
            formats: vec![],
            text: TextLimits::default(),
            captions_required: false,
            loudness_lufs: None,
            file: FileConstraints::default(),
            notes: "".to_string(),
        }
    }

    #[test]
    fn test_palette_checker() {
        let brand = test_brand();
        let channel = test_channel();
        let html_pass = r#"<div style="background-color: #1E88E5; color: #FFFFFF;">Text</div>"#;

        let input_pass = CheckInput {
            artifact_dir: Path::new("."),
            source_content: Some(html_pass),
            source_file: Some("index.html"),
            renders: &[],
            ffprobe_json: None,
            brand: &brand,
            brand_dir: None,
            channel: &channel,
        };

        let checker = PaletteChecker::new();
        let findings = checker.check(&input_pass);
        assert!(
            findings.is_empty(),
            "Brand colors should pass: {:?}",
            findings
        );

        let html_fail = r#"<div style="background-color: #E91E63;">Off palette color</div>"#;
        let input_fail = CheckInput {
            artifact_dir: Path::new("."),
            source_content: Some(html_fail),
            source_file: Some("index.html"),
            renders: &[],
            ffprobe_json: None,
            brand: &brand,
            brand_dir: None,
            channel: &channel,
        };
        let findings_fail = checker.check(&input_fail);
        assert_eq!(findings_fail.len(), 1);
        assert_eq!(findings_fail[0].code, "PALETTE_OFF");
    }
}
