use crate::checkers::{CheckInput, Checker, Finding};
use serde_yaml::Value;
use std::fs;

/// Text limits checker inspecting title, description, caption, and hashtags
/// from an artifact's `text.md` frontmatter or content vs channel limits.
#[derive(Debug, Default)]
pub struct TextLimitsChecker;

impl TextLimitsChecker {
    pub fn new() -> Self {
        Self
    }
}

impl Checker for TextLimitsChecker {
    fn id(&self) -> &'static str {
        "text_limits"
    }

    fn check(&self, input: &CheckInput) -> Vec<Finding> {
        let mut findings = Vec::new();

        // 1. Look for text.md either in source_content or in artifact_dir
        let text_content = if input.source_file == Some("text.md") {
            input.source_content.map(|s| s.to_string())
        } else {
            let p = input.artifact_dir.join("text.md");
            if p.exists() {
                fs::read_to_string(p).ok()
            } else {
                None
            }
        };

        let content = match text_content {
            Some(c) => c,
            None => return findings,
        };

        // Parse frontmatter
        let trimmed = content.trim_start();
        if let Some(after_first) = trimmed.strip_prefix("---") {
            if let Some(end_idx) = after_first.find("\n---") {
                let yaml_str = &after_first[..end_idx];
                if let Ok(val) = serde_yaml::from_str::<Value>(yaml_str) {
                    if let Some(mapping) = val.as_mapping() {
                        // Title check
                        if let Some(title_val) = mapping.get(Value::String("title".to_string())) {
                            if let Some(title_str) = title_val.as_str() {
                                if let Some(max_len) = input.channel.text.title_max {
                                    let char_count = title_str.chars().count();
                                    if char_count > max_len {
                                        findings.push(
                                            Finding::error(
                                                "TITLE_TOO_LONG",
                                                format!(
                                                    "Tiêu đề ({char_count} ký tự) vượt quá giới hạn tối đa ({max_len} ký tự) của kênh '{}'",
                                                    input.channel.title
                                                ),
                                            )
                                            .with_fix_hint(format!("Rút ngắn tiêu đề còn dưới {max_len} ký tự.")),
                                        );
                                    }
                                }
                            }
                        }

                        // Description check
                        if let Some(desc_val) =
                            mapping.get(Value::String("description".to_string()))
                        {
                            if let Some(desc_str) = desc_val.as_str() {
                                if let Some(max_len) = input.channel.text.description_max {
                                    let char_count = desc_str.chars().count();
                                    if char_count > max_len {
                                        findings.push(
                                            Finding::error(
                                                "DESCRIPTION_TOO_LONG",
                                                format!(
                                                    "Mô tả ({char_count} ký tự) vượt quá giới hạn tối đa ({max_len} ký tự) của kênh '{}'",
                                                    input.channel.title
                                                ),
                                            )
                                            .with_fix_hint(format!("Rút ngắn mô tả còn dưới {max_len} ký tự.")),
                                        );
                                    }
                                }
                            }
                        }

                        // Caption check
                        if let Some(cap_val) = mapping.get(Value::String("caption".to_string())) {
                            if let Some(cap_str) = cap_val.as_str() {
                                if let Some(max_len) = input.channel.text.caption_max {
                                    let char_count = cap_str.chars().count();
                                    if char_count > max_len {
                                        findings.push(
                                            Finding::error(
                                                "CAPTION_TOO_LONG",
                                                format!(
                                                    "Caption ({char_count} ký tự) vượt quá giới hạn tối đa ({max_len} ký tự) của kênh '{}'",
                                                    input.channel.title
                                                ),
                                            )
                                            .with_fix_hint(format!("Rút ngắn caption còn dưới {max_len} ký tự.")),
                                        );
                                    }
                                }
                            }
                        }

                        // Hashtags count check
                        if let Some(tags_val) = mapping.get(Value::String("hashtags".to_string())) {
                            let count = if let Some(seq) = tags_val.as_sequence() {
                                seq.len()
                            } else if let Some(s) = tags_val.as_str() {
                                s.split_whitespace().filter(|w| w.starts_with('#')).count()
                            } else {
                                0
                            };

                            if let Some(max_tags) = input.channel.text.hashtags_max {
                                if count > max_tags {
                                    findings.push(
                                        Finding::warn(
                                            "HASHTAGS_EXCEEDED",
                                            format!(
                                                "Số lượng hashtag ({count}) vượt quá mức khuyến nghị ({max_tags}) của kênh '{}'",
                                                input.channel.title
                                            ),
                                        )
                                        .with_fix_hint(format!("Giảm số hashtag còn tối đa {max_tags}.")),
                                    );
                                }
                            }
                        }
                    }
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
            id: "yt-shorts".to_string(),
            platform: "youtube".to_string(),
            title: "Shorts".to_string(),
            formats: vec![],
            text: TextLimits {
                title_max: Some(100),
                description_max: Some(500),
                caption_max: None,
                hashtags_max: Some(3),
            },
            captions_required: false,
            loudness_lufs: None,
            file: FileConstraints::default(),
            notes: "".to_string(),
        }
    }

    #[test]
    fn test_text_limits_checker() {
        let brand = make_test_brand();
        let channel = make_test_channel();
        let checker = TextLimitsChecker::new();

        let long_title = "A".repeat(105);
        let md = format!(
            r##"---
title: "{}"
hashtags:
  - "#one"
  - "#two"
  - "#three"
  - "#four"
---
# Nội dung
"##,
            long_title
        );

        let input = CheckInput {
            artifact_dir: Path::new("."),
            source_content: Some(&md),
            source_file: Some("text.md"),
            renders: &[],
            ffprobe_json: None,
            brand: &brand,
            brand_dir: None,
            channel: &channel,
        };

        let findings = checker.check(&input);
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].code, "TITLE_TOO_LONG");
        assert_eq!(findings[1].code, "HASHTAGS_EXCEEDED");
    }
}
