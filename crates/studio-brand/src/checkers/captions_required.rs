use crate::checkers::{CheckInput, Checker, Finding};

/// Captions requirement checker verifying that video composition has a captions track
/// or that an .srt / .vtt render exists when captions are required by the channel.
#[derive(Debug, Default)]
pub struct CaptionsRequiredChecker;

impl CaptionsRequiredChecker {
    pub fn new() -> Self {
        Self
    }
}

impl Checker for CaptionsRequiredChecker {
    fn id(&self) -> &'static str {
        "captions_required"
    }

    fn check(&self, input: &CheckInput) -> Vec<Finding> {
        let mut findings = Vec::new();

        if !input.channel.captions_required {
            return findings;
        }

        // 1. Check if any render is a subtitle file (.srt, .vtt)
        let has_subtitle_render = input.renders.iter().any(|p| {
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            ext == "srt" || ext == "vtt" || ext == "ass"
        });

        if has_subtitle_render {
            return findings;
        }

        // 2. Check if ffprobe reveals subtitle stream
        if let Some(ffprobe) = input.ffprobe_json {
            if let Some(streams) = ffprobe.get("streams").and_then(|s| s.as_array()) {
                let has_sub_stream = streams
                    .iter()
                    .any(|s| s.get("codec_type").and_then(|c| c.as_str()) == Some("subtitle"));
                if has_sub_stream {
                    return findings;
                }
            }
        }

        // 3. Check source content (e.g. timeline.json or composition HTML) for captions / subtitle track
        if let Some(content) = input.source_content {
            if content.contains("\"captions\"")
                || content.contains("\"subtitles\"")
                || content.contains("<track kind=\"subtitles\"")
                || content.contains("<track kind=\"captions\"")
            {
                return findings;
            }
        }

        // Also check if artifact directory contains .srt or .vtt file
        if let Ok(entries) = std::fs::read_dir(input.artifact_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if ext == "srt" || ext == "vtt" {
                    return findings;
                }
            }
        }

        findings.push(
            Finding::error(
                "CAPTIONS_MISSING",
                format!(
                    "Kênh '{}' yêu cầu có phụ đề (captions/subtitles), nhưng không tìm thấy file .srt/.vtt hoặc track phụ đề",
                    input.channel.title
                ),
            )
            .with_fix_hint("Tạo file phụ đề .srt/.vtt hoặc thêm track phụ đề vào video timeline."),
        );

        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{
        BrandColors, BrandFonts, BrandLogos, BrandSpec, ChannelSpec, FileConstraints, TextLimits,
    };
    use std::path::{Path, PathBuf};

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

    fn make_test_channel(captions_required: bool) -> ChannelSpec {
        ChannelSpec {
            id: "tiktok".to_string(),
            platform: "tiktok".to_string(),
            title: "TikTok".to_string(),
            formats: vec![],
            text: TextLimits::default(),
            captions_required,
            loudness_lufs: None,
            file: FileConstraints::default(),
            notes: "".to_string(),
        }
    }

    #[test]
    fn test_captions_checker() {
        let brand = make_test_brand();
        let channel_req = make_test_channel(true);
        let checker = CaptionsRequiredChecker::new();

        // No captions provided
        let input_missing = CheckInput {
            artifact_dir: Path::new("."),
            source_content: Some(r#"{"tracks": []}"#),
            source_file: Some("timeline.json"),
            renders: &[],
            ffprobe_json: None,
            brand: &brand,
            brand_dir: None,
            channel: &channel_req,
        };
        let findings = checker.check(&input_missing);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "CAPTIONS_MISSING");

        // With .srt render
        let srt_render = vec![PathBuf::from("renders/sub.srt")];
        let input_ok = CheckInput {
            artifact_dir: Path::new("."),
            source_content: None,
            source_file: None,
            renders: &srt_render,
            ffprobe_json: None,
            brand: &brand,
            brand_dir: None,
            channel: &channel_req,
        };
        let findings_ok = checker.check(&input_ok);
        assert!(findings_ok.is_empty());
    }
}
