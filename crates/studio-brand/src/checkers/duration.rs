use crate::checkers::{CheckInput, Checker, Finding};

/// Verifies that video duration (from ffprobe JSON) satisfies channel format min_duration_s and max_duration_s.
#[derive(Debug, Default)]
pub struct DurationChecker;

impl DurationChecker {
    pub fn new() -> Self {
        Self
    }
}

impl Checker for DurationChecker {
    fn id(&self) -> &'static str {
        "duration"
    }

    fn check(&self, input: &CheckInput) -> Vec<Finding> {
        let mut findings = Vec::new();

        let ffprobe = match input.ffprobe_json {
            Some(f) => f,
            None => return findings,
        };

        // Extract duration from format or video stream
        let mut duration_s: Option<f64> = None;

        if let Some(format_obj) = ffprobe.get("format") {
            if let Some(dur_val) = format_obj.get("duration") {
                if let Some(dur_str) = dur_val.as_str() {
                    duration_s = dur_str.parse().ok();
                } else if let Some(dur_num) = dur_val.as_f64() {
                    duration_s = Some(dur_num);
                }
            }
        }

        if duration_s.is_none() {
            if let Some(streams) = ffprobe.get("streams").and_then(|s| s.as_array()) {
                for stream in streams {
                    if stream.get("codec_type").and_then(|c| c.as_str()) == Some("video") {
                        if let Some(dur_val) = stream.get("duration") {
                            if let Some(dur_str) = dur_val.as_str() {
                                duration_s = dur_str.parse().ok();
                            } else if let Some(dur_num) = dur_val.as_f64() {
                                duration_s = Some(dur_num);
                            }
                        }
                    }
                }
            }
        }

        let dur = match duration_s {
            Some(d) => d,
            None => return findings,
        };

        for format in &input.channel.formats {
            if let Some(max_s) = format.max_duration_s {
                if dur > max_s {
                    findings.push(
                        Finding::error(
                            "DURATION_EXCEEDED",
                            format!(
                                "Thời lượng video ({dur:.1}s) vượt quá mức tối đa ({max_s:.1}s) của định dạng '{}'",
                                format.name
                            ),
                        )
                        .with_fix_hint(format!("Rút ngắn nội dung video xuống dưới {max_s:.0} giây.")),
                    );
                }
            }

            if let Some(min_s) = format.min_duration_s {
                if dur < min_s {
                    findings.push(
                        Finding::error(
                            "DURATION_TOO_SHORT",
                            format!(
                                "Thời lượng video ({dur:.1}s) ngắn hơn mức tối thiểu ({min_s:.1}s) của định dạng '{}'",
                                format.name
                            ),
                        )
                        .with_fix_hint(format!("Kéo dài video tối thiểu đạt {min_s:.0} giây.")),
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
                safe_zone: SafeZone::default(),
            }],
            text: TextLimits::default(),
            captions_required: false,
            loudness_lufs: None,
            file: FileConstraints::default(),
            notes: "".to_string(),
        }
    }

    #[test]
    fn test_duration_checker() {
        let brand = make_test_brand();
        let channel = make_test_channel();
        let checker = DurationChecker::new();

        let ffprobe_pass = serde_json::json!({
            "format": {
                "duration": "45.5"
            }
        });
        let input_pass = CheckInput {
            artifact_dir: Path::new("."),
            source_content: None,
            source_file: None,
            renders: &[],
            ffprobe_json: Some(&ffprobe_pass),
            brand: &brand,
            brand_dir: None,
            channel: &channel,
        };
        let findings_pass = checker.check(&input_pass);
        assert!(findings_pass.is_empty());

        let ffprobe_too_long = serde_json::json!({
            "format": {
                "duration": "65.2"
            }
        });
        let input_too_long = CheckInput {
            artifact_dir: Path::new("."),
            source_content: None,
            source_file: None,
            renders: &[],
            ffprobe_json: Some(&ffprobe_too_long),
            brand: &brand,
            brand_dir: None,
            channel: &channel,
        };
        let findings_long = checker.check(&input_too_long);
        assert_eq!(findings_long.len(), 1);
        assert_eq!(findings_long[0].code, "DURATION_EXCEEDED");
    }
}
