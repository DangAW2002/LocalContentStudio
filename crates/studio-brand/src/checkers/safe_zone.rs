use crate::checkers::{CheckInput, Checker, Finding};
use regex::Regex;
use std::sync::OnceLock;

/// Safe zone checker verifying that elements with `data-layer` and absolute px boxes in HTML source
/// lie within the format's safe zone boundaries.
#[derive(Debug, Default)]
pub struct SafeZoneChecker;

impl SafeZoneChecker {
    pub fn new() -> Self {
        Self
    }
}

static DATA_LAYER_EL_RE: OnceLock<Regex> = OnceLock::new();
static TOP_PX_RE: OnceLock<Regex> = OnceLock::new();
static BOTTOM_PX_RE: OnceLock<Regex> = OnceLock::new();
static LEFT_PX_RE: OnceLock<Regex> = OnceLock::new();
static RIGHT_PX_RE: OnceLock<Regex> = OnceLock::new();
static WIDTH_PX_RE: OnceLock<Regex> = OnceLock::new();
static HEIGHT_PX_RE: OnceLock<Regex> = OnceLock::new();

fn data_layer_regex() -> &'static Regex {
    DATA_LAYER_EL_RE.get_or_init(|| {
        Regex::new(r#"<([a-zA-Z0-9]+)[^>]*data-layer(?:="([^"]*)")?[^>]*>"#).expect("valid regex")
    })
}

fn top_px_regex() -> &'static Regex {
    TOP_PX_RE.get_or_init(|| Regex::new(r#"(?:^|;)\s*top\s*:\s*(\d+)px"#).expect("valid regex"))
}

fn bottom_px_regex() -> &'static Regex {
    BOTTOM_PX_RE
        .get_or_init(|| Regex::new(r#"(?:^|;)\s*bottom\s*:\s*(\d+)px"#).expect("valid regex"))
}

fn left_px_regex() -> &'static Regex {
    LEFT_PX_RE.get_or_init(|| Regex::new(r#"(?:^|;)\s*left\s*:\s*(\d+)px"#).expect("valid regex"))
}

fn right_px_regex() -> &'static Regex {
    RIGHT_PX_RE.get_or_init(|| Regex::new(r#"(?:^|;)\s*right\s*:\s*(\d+)px"#).expect("valid regex"))
}

fn width_px_regex() -> &'static Regex {
    WIDTH_PX_RE.get_or_init(|| Regex::new(r#"(?:^|;)\s*width\s*:\s*(\d+)px"#).expect("valid regex"))
}

fn height_px_regex() -> &'static Regex {
    HEIGHT_PX_RE
        .get_or_init(|| Regex::new(r#"(?:^|;)\s*height\s*:\s*(\d+)px"#).expect("valid regex"))
}

impl Checker for SafeZoneChecker {
    fn id(&self) -> &'static str {
        "safe_zone"
    }

    fn check(&self, input: &CheckInput) -> Vec<Finding> {
        let mut findings = Vec::new();

        let content = match input.source_content {
            Some(c) => c,
            None => return findings,
        };

        if input.channel.formats.is_empty() {
            return findings;
        }

        // We check against the primary format (first format in channel spec)
        let format = &input.channel.formats[0];
        let sz = format.safe_zone;

        let safe_top = (format.height as f32 * sz.top).round() as i32;
        let safe_bottom = (format.height as f32 * (1.0 - sz.bottom)).round() as i32;
        let safe_left = (format.width as f32 * sz.left).round() as i32;
        let safe_right = (format.width as f32 * (1.0 - sz.right)).round() as i32;

        for cap in data_layer_regex().captures_iter(content) {
            let full_tag = cap.get(0).map(|m| m.as_str()).unwrap_or_default();
            let layer_name = cap.get(2).map(|m| m.as_str()).unwrap_or("layer");

            // Look for inline styles
            let top: Option<i32> = top_px_regex()
                .captures(full_tag)
                .and_then(|c| c[1].parse().ok());
            let bottom: Option<i32> = bottom_px_regex()
                .captures(full_tag)
                .and_then(|c| c[1].parse().ok());
            let left: Option<i32> = left_px_regex()
                .captures(full_tag)
                .and_then(|c| c[1].parse().ok());
            let right: Option<i32> = right_px_regex()
                .captures(full_tag)
                .and_then(|c| c[1].parse().ok());
            let width: Option<i32> = width_px_regex()
                .captures(full_tag)
                .and_then(|c| c[1].parse().ok());
            let height: Option<i32> = height_px_regex()
                .captures(full_tag)
                .and_then(|c| c[1].parse().ok());

            // Check vertical safe zone violations
            if let Some(t) = top {
                if t < safe_top {
                    findings.push(
                        Finding::warn(
                            "SAFE_ZONE_TOP_VIOLATION",
                            format!(
                                "Layer '{layer_name}' có top: {t}px nằm ngoài safe zone trên (tối thiểu {safe_top}px) của kênh '{}'",
                                input.channel.title
                            ),
                        )
                        .with_location(full_tag.to_string())
                        .with_fix_hint(format!("Dời vị trí top xuống ≥ {safe_top}px.")),
                    );
                }

                if let Some(h) = height {
                    let elem_bottom = t + h;
                    if elem_bottom > safe_bottom {
                        findings.push(
                            Finding::warn(
                                "SAFE_ZONE_BOTTOM_VIOLATION",
                                format!(
                                    "Layer '{layer_name}' (đáy {elem_bottom}px) vượt quá safe zone dưới ({safe_bottom}px) của kênh '{}'",
                                    input.channel.title
                                ),
                            )
                            .with_location(full_tag.to_string())
                            .with_fix_hint(format!("Giữ đáy phần tử nằm trên {safe_bottom}px để tránh bị UI kênh che.")),
                        );
                    }
                }
            }

            if let Some(b) = bottom {
                let bottom_margin = (format.height as f32 * sz.bottom).round() as i32;
                if b < bottom_margin {
                    findings.push(
                        Finding::warn(
                            "SAFE_ZONE_BOTTOM_VIOLATION",
                            format!(
                                "Layer '{layer_name}' có bottom: {b}px nằm quá sát mép dưới (yêu cầu an toàn ≥ {bottom_margin}px cho UI kênh)"
                            ),
                        )
                        .with_location(full_tag.to_string())
                        .with_fix_hint(format!("Tăng khoảng cách bottom lên ≥ {bottom_margin}px.")),
                    );
                }
            }

            // Check horizontal safe zone violations
            if let Some(l) = left {
                if l < safe_left {
                    findings.push(
                        Finding::warn(
                            "SAFE_ZONE_LEFT_VIOLATION",
                            format!("Layer '{layer_name}' có left: {l}px nằm ngoài safe zone trái ({safe_left}px)"),
                        )
                        .with_location(full_tag.to_string())
                        .with_fix_hint(format!("Dời left sang ≥ {safe_left}px.")),
                    );
                }

                if let Some(w) = width {
                    let elem_right = l + w;
                    if elem_right > safe_right {
                        findings.push(
                            Finding::warn(
                                "SAFE_ZONE_RIGHT_VIOLATION",
                                format!("Layer '{layer_name}' (phải {elem_right}px) vượt safe zone phải ({safe_right}px)"),
                            )
                            .with_location(full_tag.to_string())
                            .with_fix_hint(format!("Giữ mép phải phần tử nằm trong {safe_right}px.")),
                        );
                    }
                }
            }

            if let Some(r) = right {
                let right_margin = (format.width as f32 * sz.right).round() as i32;
                if r < right_margin {
                    findings.push(
                        Finding::warn(
                            "SAFE_ZONE_RIGHT_VIOLATION",
                            format!("Layer '{layer_name}' có right: {r}px nằm quá sát mép phải (yêu cầu ≥ {right_margin}px)"),
                        )
                        .with_location(full_tag.to_string())
                        .with_fix_hint(format!("Tăng right lên ≥ {right_margin}px.")),
                    );
                }
            }
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{
        BrandColors, BrandFonts, BrandLogos, BrandSpec, ChannelSpec, FileConstraints, FormatSpec,
        SafeZone, TextLimits,
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

    fn make_test_channel() -> ChannelSpec {
        ChannelSpec {
            id: "youtube-shorts".to_string(),
            platform: "youtube".to_string(),
            title: "Shorts".to_string(),
            formats: vec![FormatSpec {
                name: "shorts".to_string(),
                width: 1080,
                height: 1920,
                aspect: "9:16".to_string(),
                fps: Some(30.0),
                max_duration_s: Some(60.0),
                min_duration_s: Some(3.0),
                safe_zone: SafeZone {
                    top: 0.05,
                    bottom: 0.20,
                    left: 0.05,
                    right: 0.05,
                },
            }],
            text: TextLimits::default(),
            captions_required: false,
            loudness_lufs: None,
            file: FileConstraints::default(),
            notes: "".to_string(),
        }
    }

    #[test]
    fn test_safe_zone_violation() {
        let brand = make_test_brand();
        let channel = make_test_channel();
        let checker = SafeZoneChecker::new();

        // 1920 * 0.20 = 384px safe bottom margin, safe_bottom is 1920 - 384 = 1536px
        // top: 1600px with height: 200px -> 1800px > 1536px (violates safe zone bottom)
        let html_bad = r#"<div data-layer="caption" style="position: absolute; top: 1600px; height: 200px; left: 100px;">Text</div>"#;
        let input_bad = CheckInput {
            artifact_dir: Path::new("."),
            source_content: Some(html_bad),
            source_file: Some("index.html"),
            renders: &[],
            ffprobe_json: None,
            brand: &brand,
            brand_dir: None,
            channel: &channel,
        };
        let findings_bad = checker.check(&input_bad);
        assert!(!findings_bad.is_empty());
        assert_eq!(findings_bad[0].code, "SAFE_ZONE_BOTTOM_VIOLATION");

        // Safe coordinates: top: 300px, height: 100px, left: 100px, width: 400px
        let html_good = r#"<div data-layer="title" style="position: absolute; top: 300px; height: 100px; left: 100px; width: 400px;">Title</div>"#;
        let input_good = CheckInput {
            artifact_dir: Path::new("."),
            source_content: Some(html_good),
            source_file: Some("index.html"),
            renders: &[],
            ffprobe_json: None,
            brand: &brand,
            brand_dir: None,
            channel: &channel,
        };
        let findings_good = checker.check(&input_good);
        assert!(
            findings_good.is_empty(),
            "Findings should be empty: {:?}",
            findings_good
        );
    }
}
