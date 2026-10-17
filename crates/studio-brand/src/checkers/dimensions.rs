use crate::checkers::{CheckInput, Checker, Finding};
use std::fs::File;
use std::io::BufReader;

/// Dimensions checker verifying that output images (via `image` header read) or video (via ffprobe JSON)
/// match the required channel format width and height.
#[derive(Debug, Default)]
pub struct DimensionsChecker;

impl DimensionsChecker {
    pub fn new() -> Self {
        Self
    }
}

impl Checker for DimensionsChecker {
    fn id(&self) -> &'static str {
        "dimensions"
    }

    fn check(&self, input: &CheckInput) -> Vec<Finding> {
        let mut findings = Vec::new();

        if input.channel.formats.is_empty() {
            return findings;
        }

        // If no renders and no ffprobe_json are present, warn that render is required for dimension checks
        if input.renders.is_empty() && input.ffprobe_json.is_none() {
            findings.push(
                Finding::warn(
                    "NO_RENDERS",
                    "Chưa có bản render — bấm Render trước khi kiểm tra kích thước/thời lượng",
                )
                .with_fix_hint(
                    "Bấm nút Render hoặc chạy công cụ artifact_render trước khi kiểm tra.",
                ),
            );
            return findings;
        }

        // 1. Check video dimensions via ffprobe_json if provided
        if let Some(ffprobe) = input.ffprobe_json {
            let mut found_video_stream = false;
            if let Some(streams) = ffprobe.get("streams").and_then(|s| s.as_array()) {
                for stream in streams {
                    let codec_type = stream.get("codec_type").and_then(|c| c.as_str());
                    if codec_type == Some("video") {
                        found_video_stream = true;
                        let width = stream
                            .get("width")
                            .and_then(|w| w.as_u64())
                            .map(|w| w as u32);
                        let height = stream
                            .get("height")
                            .and_then(|h| h.as_u64())
                            .map(|h| h as u32);

                        if let (Some(w), Some(h)) = (width, height) {
                            let matches_any_format = input
                                .channel
                                .formats
                                .iter()
                                .any(|f| f.width == w && f.height == h);
                            if !matches_any_format {
                                let expected_list: Vec<String> = input
                                    .channel
                                    .formats
                                    .iter()
                                    .map(|f| format!("{}×{} ({})", f.width, f.height, f.name))
                                    .collect();
                                findings.push(
                                    Finding::error(
                                        "DIMENSIONS_MISMATCH",
                                        format!(
                                            "Kích thước video ({w}×{h}) không khớp bất kỳ định dạng nào của kênh: {}",
                                            expected_list.join(", ")
                                        ),
                                    )
                                    .with_fix_hint("Render lại video đúng độ phân giải yêu cầu của kênh."),
                                );
                            }
                        }
                    }
                }
            }
            if found_video_stream {
                return findings;
            }
        }

        // 2. Check rendered image files via `image::io::Reader`
        for render_path in input.renders {
            let ext = render_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            if ext == "png" || ext == "jpg" || ext == "jpeg" || ext == "webp" {
                if let Ok(file) = File::open(render_path) {
                    let reader = BufReader::new(file);
                    if let Ok(dim_reader) = image::ImageReader::new(reader).with_guessed_format() {
                        if let Ok((w, h)) = dim_reader.into_dimensions() {
                            let matches_any_format = input
                                .channel
                                .formats
                                .iter()
                                .any(|f| f.width == w && f.height == h);
                            if !matches_any_format {
                                let expected_list: Vec<String> = input
                                    .channel
                                    .formats
                                    .iter()
                                    .map(|f| format!("{}×{} ({})", f.width, f.height, f.name))
                                    .collect();
                                findings.push(
                                    Finding::error(
                                        "DIMENSIONS_MISMATCH",
                                        format!(
                                            "Kích thước ảnh render {:?} ({w}×{h}) không khớp định dạng kênh: {}",
                                            render_path.file_name().unwrap_or_default(),
                                            expected_list.join(", ")
                                        ),
                                    )
                                    .with_location(render_path.to_string_lossy().to_string())
                                    .with_fix_hint("Xuất lại ảnh theo kích thước chuẩn của kênh."),
                                );
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
        BrandColors, BrandFonts, BrandLogos, BrandSpec, ChannelSpec, FileConstraints, FormatSpec,
        SafeZone, TextLimits,
    };
    use std::path::Path;
    use tempfile::NamedTempFile;

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
    fn test_dimensions_ffprobe() {
        let brand = make_test_brand();
        let channel = make_test_channel();
        let checker = DimensionsChecker::new();

        let ffprobe_pass = serde_json::json!({
            "streams": [
                {
                    "codec_type": "video",
                    "width": 1080,
                    "height": 1920
                }
            ]
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

        let ffprobe_fail = serde_json::json!({
            "streams": [
                {
                    "codec_type": "video",
                    "width": 1920,
                    "height": 1080
                }
            ]
        });
        let input_fail = CheckInput {
            artifact_dir: Path::new("."),
            source_content: None,
            source_file: None,
            renders: &[],
            ffprobe_json: Some(&ffprobe_fail),
            brand: &brand,
            brand_dir: None,
            channel: &channel,
        };
        let findings_fail = checker.check(&input_fail);
        assert_eq!(findings_fail.len(), 1);
        assert_eq!(findings_fail[0].code, "DIMENSIONS_MISMATCH");
    }

    #[test]
    fn test_dimensions_image_read() {
        let brand = make_test_brand();
        let channel = make_test_channel();
        let checker = DimensionsChecker::new();

        // Create a 1080x1920 tiny test PNG
        let img_buf = image::RgbImage::new(1080, 1920);
        let tmp_file = NamedTempFile::new().unwrap();
        let path = tmp_file.path().with_extension("png");
        img_buf.save(&path).unwrap();

        let renders = vec![path.clone()];
        let input = CheckInput {
            artifact_dir: Path::new("."),
            source_content: None,
            source_file: None,
            renders: &renders,
            ffprobe_json: None,
            brand: &brand,
            brand_dir: None,
            channel: &channel,
        };

        let findings = checker.check(&input);
        assert!(
            findings.is_empty(),
            "1080x1920 PNG should pass: {:?}",
            findings
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_dimensions_no_renders_warning() {
        let brand = make_test_brand();
        let channel = make_test_channel();
        let checker = DimensionsChecker::new();

        let input = CheckInput {
            artifact_dir: Path::new("."),
            source_content: None,
            source_file: None,
            renders: &[],
            ffprobe_json: None,
            brand: &brand,
            brand_dir: None,
            channel: &channel,
        };

        let findings = checker.check(&input);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "NO_RENDERS");
        assert_eq!(findings[0].severity, crate::checkers::Severity::Warn);
        assert!(findings[0].message_vi.contains("Chưa có bản render"));
    }

    #[test]
    fn test_dimensions_image_mismatch_1080x1350_vs_1280x720() {
        let brand = make_test_brand();
        let mut channel = make_test_channel();
        channel.formats = vec![FormatSpec {
            name: "youtube_thumbnail".to_string(),
            width: 1280,
            height: 720,
            aspect: "16:9".to_string(),
            fps: None,
            max_duration_s: None,
            min_duration_s: None,
            safe_zone: SafeZone::default(),
        }];
        let checker = DimensionsChecker::new();

        // 1080x1350 PNG fixture
        let img_buf = image::RgbImage::new(1080, 1350);
        let tmp_file = NamedTempFile::new().unwrap();
        let path = tmp_file.path().with_extension("png");
        img_buf.save(&path).unwrap();

        let renders = vec![path.clone()];
        let input = CheckInput {
            artifact_dir: Path::new("."),
            source_content: None,
            source_file: None,
            renders: &renders,
            ffprobe_json: None,
            brand: &brand,
            brand_dir: None,
            channel: &channel,
        };

        let findings = checker.check(&input);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "DIMENSIONS_MISMATCH");
        assert_eq!(findings[0].severity, crate::checkers::Severity::Error);
        assert!(findings[0].message_vi.contains("1080×1350"));
        assert!(findings[0].message_vi.contains("1280×720"));

        let _ = std::fs::remove_file(path);
    }
}
