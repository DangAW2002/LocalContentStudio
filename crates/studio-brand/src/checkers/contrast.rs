use crate::checkers::color::Rgb;
use crate::checkers::{CheckInput, Checker, Finding};
use regex::Regex;
use std::sync::OnceLock;

/// WCAG 2.1 contrast checker for text color vs background pairs declared in CSS/HTML styles.
#[derive(Debug, Default)]
pub struct ContrastChecker;

impl ContrastChecker {
    pub fn new() -> Self {
        Self
    }
}

static STYLE_COLOR_RE: OnceLock<Regex> = OnceLock::new();
static STYLE_BG_RE: OnceLock<Regex> = OnceLock::new();
static INLINE_STYLE_RE: OnceLock<Regex> = OnceLock::new();

fn inline_style_regex() -> &'static Regex {
    INLINE_STYLE_RE
        .get_or_init(|| Regex::new(r#"style\s*=\s*["']([^"']+)["']"#).expect("valid regex"))
}

fn color_regex() -> &'static Regex {
    STYLE_COLOR_RE
        .get_or_init(|| Regex::new(r"(?:^|;)\s*color\s*:\s*([^;]+)").expect("valid regex"))
}

fn bg_regex() -> &'static Regex {
    STYLE_BG_RE.get_or_init(|| {
        Regex::new(r"(?:^|;)\s*background(?:-color)?\s*:\s*([^;]+)").expect("valid regex")
    })
}

impl Checker for ContrastChecker {
    fn id(&self) -> &'static str {
        "contrast"
    }

    fn check(&self, input: &CheckInput) -> Vec<Finding> {
        let mut findings = Vec::new();
        let content = match input.source_content {
            Some(c) => c,
            None => return findings,
        };

        // Parse inline styles that specify both color and background
        for cap in inline_style_regex().captures_iter(content) {
            let style_inner = cap.get(1).map(|m| m.as_str()).unwrap_or_default();

            let text_color_match = color_regex().captures(style_inner).and_then(|c| c.get(1));
            let bg_color_match = bg_regex().captures(style_inner).and_then(|c| c.get(1));

            if let (Some(tc), Some(bc)) = (text_color_match, bg_color_match) {
                if let (Some(text_rgb), Some(bg_rgb)) =
                    (Rgb::parse_css(tc.as_str()), Rgb::parse_css(bc.as_str()))
                {
                    let ratio = text_rgb.contrast_ratio(&bg_rgb);
                    // WCAG AA normal text threshold is 4.5, large text is 3.0
                    if ratio < 4.5 {
                        let severity = if ratio < 3.0 {
                            // Fails even large text
                            crate::checkers::Severity::Error
                        } else {
                            // Passes large text, fails normal text
                            crate::checkers::Severity::Warn
                        };

                        findings.push(Finding {
                            severity,
                            code: "CONTRAST_LOW".to_string(),
                            message_vi: format!(
                                "Độ tương phản màu chữ '{}' và nền '{}' quá thấp ({ratio:.2}:1 < 4.5:1 tiêu chuẩn WCAG AA)",
                                tc.as_str().trim(),
                                bc.as_str().trim()
                            ),
                            location: Some(style_inner.to_string()),
                            fix_hint_vi: Some("Tăng độ sáng của màu chữ hoặc làm tối màu nền để đạt tỷ lệ tương phản tối thiểu 4.5:1.".to_string()),
                        });
                    }
                }
            }
        }

        // Also check default brand text vs brand background
        if let (Some(text_rgb), Some(bg_rgb)) = (
            Rgb::parse_css(&input.brand.colors.text),
            Rgb::parse_css(&input.brand.colors.bg),
        ) {
            let ratio = text_rgb.contrast_ratio(&bg_rgb);
            if ratio < 4.5 {
                findings.push(
                    Finding::warn(
                        "BRAND_CONTRAST_LOW",
                        format!(
                            "Màu chữ mặc định của brand '{}' và màu nền '{}' có tương phản thấp ({ratio:.2}:1 < 4.5:1)",
                            input.brand.colors.text, input.brand.colors.bg
                        ),
                    )
                    .with_fix_hint("Cân nhắc điều chỉnh palette brand text hoặc bg."),
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

    #[test]
    fn test_contrast_checker() {
        let brand = BrandSpec {
            name: "Test".to_string(),
            slogan: "".to_string(),
            colors: BrandColors {
                primary: "#000000".to_string(),
                accent: "#FFFFFF".to_string(),
                bg: "#FFFFFF".to_string(),
                text: "#000000".to_string(),
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
        };

        let channel = ChannelSpec {
            id: "yt".to_string(),
            platform: "youtube".to_string(),
            title: "YT".to_string(),
            formats: vec![],
            text: TextLimits::default(),
            captions_required: false,
            loudness_lufs: None,
            file: FileConstraints::default(),
            notes: "".to_string(),
        };

        let html = r#"<div style="color: #777777; background-color: #888888;">Low contrast</div>"#;
        let input = CheckInput {
            artifact_dir: Path::new("."),
            source_content: Some(html),
            source_file: Some("index.html"),
            renders: &[],
            ffprobe_json: None,
            brand: &brand,
            brand_dir: None,
            channel: &channel,
        };

        let checker = ContrastChecker::new();
        let findings = checker.check(&input);
        assert!(!findings.is_empty());
        assert_eq!(findings[0].code, "CONTRAST_LOW");
    }
}
