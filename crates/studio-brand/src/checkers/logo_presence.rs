use crate::checkers::{CheckInput, Checker, Finding};
use regex::Regex;
use std::sync::OnceLock;

/// Verifies that an HTML or SVG source references a brand logo file and satisfies min_size_px constraint.
#[derive(Debug, Default)]
pub struct LogoPresenceChecker;

impl LogoPresenceChecker {
    pub fn new() -> Self {
        Self
    }
}

static IMG_TAG_RE: OnceLock<Regex> = OnceLock::new();
static WIDTH_ATTR_RE: OnceLock<Regex> = OnceLock::new();
static HEIGHT_ATTR_RE: OnceLock<Regex> = OnceLock::new();

fn img_tag_regex() -> &'static Regex {
    IMG_TAG_RE.get_or_init(|| Regex::new(r#"<img[^>]+>"#).expect("valid regex"))
}

fn width_regex() -> &'static Regex {
    WIDTH_ATTR_RE
        .get_or_init(|| Regex::new(r#"width\s*=\s*["']?(\d+)(?:px)?["']?"#).expect("valid regex"))
}

fn height_regex() -> &'static Regex {
    HEIGHT_ATTR_RE
        .get_or_init(|| Regex::new(r#"height\s*=\s*["']?(\d+)(?:px)?["']?"#).expect("valid regex"))
}

impl Checker for LogoPresenceChecker {
    fn id(&self) -> &'static str {
        "logo_presence"
    }

    fn check(&self, input: &CheckInput) -> Vec<Finding> {
        let mut findings = Vec::new();
        let content = match input.source_content {
            Some(c) => c,
            None => return findings,
        };

        // Allowed brand logo filenames
        let mut logo_names = vec![
            input.brand.logos.full.as_str(),
            input.brand.logos.mark.as_str(),
        ];
        if let Some(ref light) = input.brand.logos.light {
            logo_names.push(light.as_str());
        }
        if let Some(ref dark) = input.brand.logos.dark {
            logo_names.push(dark.as_str());
        }

        // Check if content references any brand logo file
        let mut found_reference = false;
        let min_size = input.brand.logos.min_size_px;

        for logo_file in &logo_names {
            // Check direct string presence or src="...logo.svg"
            if content.contains(logo_file) {
                found_reference = true;

                // Check img tag dimensions if known
                for mat in img_tag_regex().find_iter(content) {
                    let tag = mat.as_str();
                    if tag.contains(logo_file) {
                        let w: Option<u32> = width_regex()
                            .captures(tag)
                            .and_then(|c| c.get(1))
                            .and_then(|m| m.as_str().parse().ok());
                        let h: Option<u32> = height_regex()
                            .captures(tag)
                            .and_then(|c| c.get(1))
                            .and_then(|m| m.as_str().parse().ok());

                        let size = match (w, h) {
                            (Some(w_val), Some(h_val)) => Some(w_val.min(h_val)),
                            (Some(w_val), None) => Some(w_val),
                            (None, Some(h_val)) => Some(h_val),
                            (None, None) => None,
                        };

                        if let Some(s) = size {
                            if s < min_size {
                                findings.push(
                                    Finding::error(
                                        "LOGO_TOO_SMALL",
                                        format!(
                                            "Kích thước logo ({s}px) nhỏ hơn kích thước tối thiểu của brand ({min_size}px)"
                                        ),
                                    )
                                    .with_location(tag.to_string())
                                    .with_fix_hint(format!("Tăng kích thước hiển thị logo lên tối thiểu {min_size}px.")),
                                );
                            }
                        }
                    }
                }
            }
        }

        if !found_reference {
            findings.push(
                Finding::warn(
                    "LOGO_MISSING",
                    format!(
                        "Không tìm thấy logo của brand trong source ('{}' hoặc '{}')",
                        input.brand.logos.full, input.brand.logos.mark
                    ),
                )
                .with_fix_hint("Chèn thẻ <img> hoặc SVG logo của thương hiệu vào artifact."),
            );
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

    fn make_test_brand() -> BrandSpec {
        BrandSpec {
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
                min_size_px: 48,
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

    fn make_test_channel() -> ChannelSpec {
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
    fn test_logo_presence_missing() {
        let brand = make_test_brand();
        let channel = make_test_channel();
        let html = r#"<div>No logo here</div>"#;
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
        let checker = LogoPresenceChecker::new();
        let findings = checker.check(&input);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "LOGO_MISSING");
    }

    #[test]
    fn test_logo_presence_too_small() {
        let brand = make_test_brand();
        let channel = make_test_channel();
        let html = r#"<div><img src="logo.svg" width="32" height="32" /></div>"#;
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
        let checker = LogoPresenceChecker::new();
        let findings = checker.check(&input);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "LOGO_TOO_SMALL");
    }

    #[test]
    fn test_logo_presence_pass() {
        let brand = make_test_brand();
        let channel = make_test_channel();
        let html = r#"<div><img src="logo.svg" width="64" height="64" /></div>"#;
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
        let checker = LogoPresenceChecker::new();
        let findings = checker.check(&input);
        assert!(findings.is_empty());
    }
}
