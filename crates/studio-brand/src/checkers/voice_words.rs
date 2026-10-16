use crate::checkers::{CheckInput, Checker, Finding};

/// Normalizes Vietnamese text by converting to lowercase and stripping accents / diacritics.
pub fn normalize_vietnamese(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let lower = c.to_lowercase().next().unwrap_or(c);
        let normalized = match lower {
            'a' | 'à' | 'á' | 'ạ' | 'ả' | 'ã' | 'â' | 'ầ' | 'ấ' | 'ậ' | 'ẩ' | 'ẫ' | 'ă' | 'ằ'
            | 'ắ' | 'ặ' | 'ẳ' | 'ẵ' => 'a',
            'e' | 'è' | 'é' | 'ẹ' | 'ẻ' | 'ẽ' | 'ê' | 'ề' | 'ế' | 'ệ' | 'ể' | 'ễ' => {
                'e'
            }
            'i' | 'ì' | 'í' | 'ị' | 'ỉ' | 'ĩ' => 'i',
            'o' | 'ò' | 'ó' | 'ọ' | 'ỏ' | 'õ' | 'ô' | 'ồ' | 'ố' | 'ộ' | 'ổ' | 'ỗ' | 'ơ' | 'ờ'
            | 'ớ' | 'ợ' | 'ở' | 'ỡ' => 'o',
            'u' | 'ù' | 'ú' | 'ụ' | 'ủ' | 'ũ' | 'ư' | 'ừ' | 'ứ' | 'ự' | 'ử' | 'ữ' => {
                'u'
            }
            'y' | 'ỳ' | 'ý' | 'ỵ' | 'ỷ' | 'ỹ' => 'y',
            'd' | 'đ' => 'd',
            other => other,
        };
        out.push(normalized);
    }
    out
}

/// Voice words checker verifying that avoid words/phrases are not found in the content
/// using Vietnamese case- and diacritics-insensitive matching.
#[derive(Debug, Default)]
pub struct VoiceWordsChecker;

impl VoiceWordsChecker {
    pub fn new() -> Self {
        Self
    }
}

impl Checker for VoiceWordsChecker {
    fn id(&self) -> &'static str {
        "voice_words"
    }

    fn check(&self, input: &CheckInput) -> Vec<Finding> {
        let mut findings = Vec::new();

        if input.brand.voice_words.avoid.is_empty() {
            return findings;
        }

        let content = match input.source_content {
            Some(c) => c,
            None => return findings,
        };

        let norm_content = normalize_vietnamese(content);

        for avoid_word in &input.brand.voice_words.avoid {
            let norm_avoid = normalize_vietnamese(avoid_word.trim());
            if norm_avoid.is_empty() {
                continue;
            }

            if norm_content.contains(&norm_avoid) {
                findings.push(
                    Finding::warn(
                        "VOICE_WORD_AVOID",
                        format!(
                            "Phát hiện từ cấm/tránh dùng của brand trong nội dung: '{avoid_word}'"
                        ),
                    )
                    .with_fix_hint(
                        "Thay thế bằng từ ngữ phù hợp hơn theo voice guide của thương hiệu.",
                    ),
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
        VoiceWords,
    };
    use std::path::Path;

    #[test]
    fn test_normalize_vietnamese() {
        assert_eq!(normalize_vietnamese("Tiếng Việt"), "tieng viet");
        assert_eq!(normalize_vietnamese("CAM KẾT 100%"), "cam ket 100%");
        assert_eq!(normalize_vietnamese("Đột Phá"), "dot pha");
    }

    #[test]
    fn test_voice_words_checker() {
        let brand = BrandSpec {
            name: "Test".to_string(),
            slogan: "".to_string(),
            colors: BrandColors {
                primary: "#000".to_string(),
                accent: "#FFF".to_string(),
                bg: "#FFF".to_string(),
                text: "#000".to_string(),
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
            voice_words: VoiceWords {
                prefer: vec!["đột phá".to_string()],
                avoid: vec!["cam kết 100%".to_string(), "rẻ nhất".to_string()],
            },
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

        let checker = VoiceWordsChecker::new();

        // Content contains uppercase with accents
        let content_bad = "Sản phẩm của chúng tôi CAM KẾT 100% đem lại hiệu quả cao!";
        let input_bad = CheckInput {
            artifact_dir: Path::new("."),
            source_content: Some(content_bad),
            source_file: Some("text.md"),
            renders: &[],
            ffprobe_json: None,
            brand: &brand,
            brand_dir: None,
            channel: &channel,
        };
        let findings_bad = checker.check(&input_bad);
        assert_eq!(findings_bad.len(), 1);
        assert_eq!(findings_bad[0].code, "VOICE_WORD_AVOID");

        // Content contains clean text
        let content_good = "Giải pháp đột phá giúp tối ưu hoá năng suất!";
        let input_good = CheckInput {
            artifact_dir: Path::new("."),
            source_content: Some(content_good),
            source_file: Some("text.md"),
            renders: &[],
            ffprobe_json: None,
            brand: &brand,
            brand_dir: None,
            channel: &channel,
        };
        let findings_good = checker.check(&input_good);
        assert!(findings_good.is_empty());
    }
}
