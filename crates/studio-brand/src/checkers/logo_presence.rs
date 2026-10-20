use crate::checkers::{CheckInput, Checker, Finding};
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

/// Verifies that an HTML or SVG source, or a timeline's overlay track, references a brand logo
/// and satisfies min_size_px constraint.
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
static DATA_LAYER_LOGO_RE: OnceLock<Regex> = OnceLock::new();

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

fn data_layer_logo_regex() -> &'static Regex {
    DATA_LAYER_LOGO_RE
        .get_or_init(|| Regex::new(r#"(?i)data-layer\s*=\s*["']logo["']"#).expect("valid regex"))
}

fn html_has_logo_layer(html: &str) -> bool {
    data_layer_logo_regex().is_match(html)
}

/// True when an overlay `source` is a render of (or a file inside) a design artifact of the same
/// project whose `index.html` has a `data-layer="logo"` layer. Renders live at
/// `projects/<p>/renders/<artifact-id>/<hash>/…` and the HTML at `projects/<p>/artifacts/<artifact-id>/`;
/// only that project folder is looked at.
fn is_design_artifact_with_logo(artifact_dir: &Path, source: &str) -> bool {
    // artifact_dir = projects/<p>/artifacts/<id>
    let Some(project_dir) = artifact_dir.parent().and_then(Path::parent) else {
        return false;
    };
    let normalized = source.replace('\\', "/");
    let segments: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
    let artifact_id = segments
        .windows(2)
        .rev()
        .find(|w| w[0] == "renders" || w[0] == "artifacts")
        .map(|w| w[1]);
    let Some(id) = artifact_id.filter(|id| !id.contains("..") && *id != ".") else {
        return false;
    };
    std::fs::read_to_string(project_dir.join("artifacts").join(id).join("index.html"))
        .map(|html| html_has_logo_layer(&html))
        .unwrap_or(false)
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

        // 1. Check if this is a timeline document
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(content) {
            if let Some(tracks) = val.get("tracks").and_then(|t| t.as_array()) {
                return self.check_timeline(input, tracks, &logo_names);
            }
        }

        // 2. Existing behaviour for HTML/SVG design artifacts
        let mut found_reference = false;
        let min_size = input.brand.logos.min_size_px;

        for logo_file in &logo_names {
            if content.contains(logo_file) {
                found_reference = true;

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

impl LogoPresenceChecker {
    fn check_timeline(
        &self,
        input: &CheckInput,
        tracks: &[serde_json::Value],
        logo_names: &[&str],
    ) -> Vec<Finding> {
        let mut findings = Vec::new();
        let min_size = input.brand.logos.min_size_px;

        // Partition video tracks: first video track is base, subsequent are overlays
        let mut video_tracks = Vec::new();
        for track in tracks {
            let kind = track.get("kind").and_then(|k| k.as_str()).unwrap_or("");
            if kind == "video" {
                video_tracks.push(track);
            }
        }

        let overlay_tracks = if video_tracks.len() > 1 {
            &video_tracks[1..]
        } else {
            &[][..]
        };

        let mut found_reference = false;

        for track in overlay_tracks {
            let clips = track
                .get("clips")
                .and_then(|c| c.as_array())
                .map(|a| a.as_slice())
                .unwrap_or(&[][..]);

            for clip in clips {
                let source = clip
                    .get("source")
                    .or_else(|| clip.get("path"))
                    .or_else(|| clip.get("text"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("");

                let clip_id = clip
                    .get("id")
                    .and_then(|i| i.as_str())
                    .unwrap_or("overlay_clip");

                // Check Option A: Direct brand logo filename match
                let matches_brand_logo = logo_names.iter().any(|name| {
                    source == *name
                        || source.ends_with(&format!("/{name}"))
                        || source.ends_with(&format!("\\{name}"))
                        || Path::new(source).file_name().and_then(|f| f.to_str()) == Some(*name)
                });

                // Check Option B: Render of a design artifact containing data-layer="logo"
                let matches_design_logo = if matches_brand_logo {
                    false
                } else {
                    is_design_artifact_with_logo(input.artifact_dir, source)
                };

                if matches_brand_logo || matches_design_logo {
                    found_reference = true;

                    let w = clip.get("width").and_then(|v| v.as_u64()).map(|n| n as u32);
                    let h = clip
                        .get("height")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as u32);

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
                                .with_location(format!("overlay:clip:{clip_id}"))
                                .with_fix_hint(format!(
                                    "Tăng kích thước hiển thị logo trên overlay lên tối thiểu {min_size}px."
                                )),
                            );
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
                        "Không tìm thấy logo của brand trong overlay track của timeline ('{}' hoặc '{}')",
                        input.brand.logos.full, input.brand.logos.mark
                    ),
                )
                .with_fix_hint("Thêm một video track overlay trên timeline sử dụng logo thương hiệu hoặc render từ design artifact có data-layer=\"logo\"."),
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
    use std::fs;
    use tempfile::tempdir;

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
                full: "logo.png".to_string(),
                mark: "mark.png".to_string(),
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
        let html = r#"<div><img src="logo.png" width="32" height="32" /></div>"#;
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
        let html = r#"<div><img src="logo.png" width="64" height="64" /></div>"#;
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

    #[test]
    fn test_timeline_logo_missing_when_only_in_base_track() {
        let brand = make_test_brand();
        let channel = make_test_channel();
        let timeline_json = r#"{
            "tracks": [
                {
                    "id": "base",
                    "kind": "video",
                    "clips": [
                        { "id": "c1", "source": "brand/logo.png", "duration": 5.0 }
                    ]
                }
            ]
        }"#;
        let input = CheckInput {
            artifact_dir: Path::new("."),
            source_content: Some(timeline_json),
            source_file: Some("timeline.json"),
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
    fn test_timeline_logo_pass_with_brand_logo_in_overlay_track() {
        let brand = make_test_brand();
        let channel = make_test_channel();
        let timeline_json = r#"{
            "tracks": [
                {
                    "id": "base",
                    "kind": "video",
                    "clips": [
                        { "id": "c1", "source": "clips/video1.mp4", "duration": 10.0 }
                    ]
                },
                {
                    "id": "overlay",
                    "kind": "video",
                    "clips": [
                        { "id": "logo", "source": "brand/logo.png", "start": 0.0, "duration": 10.0, "width": 80 }
                    ]
                }
            ]
        }"#;
        let input = CheckInput {
            artifact_dir: Path::new("."),
            source_content: Some(timeline_json),
            source_file: Some("timeline.json"),
            renders: &[],
            ffprobe_json: None,
            brand: &brand,
            brand_dir: None,
            channel: &channel,
        };
        let checker = LogoPresenceChecker::new();
        let findings = checker.check(&input);
        assert!(
            findings.is_empty(),
            "Findings should be empty: {findings:?}"
        );
    }

    #[test]
    fn test_timeline_logo_too_small_in_overlay() {
        let brand = make_test_brand();
        let channel = make_test_channel();
        let timeline_json = r#"{
            "tracks": [
                {
                    "id": "base",
                    "kind": "video",
                    "clips": [
                        { "id": "c1", "source": "clips/video1.mp4", "duration": 10.0 }
                    ]
                },
                {
                    "id": "overlay",
                    "kind": "video",
                    "clips": [
                        { "id": "logo", "source": "brand/logo.png", "start": 0.0, "duration": 10.0, "width": 30 }
                    ]
                }
            ]
        }"#;
        let input = CheckInput {
            artifact_dir: Path::new("."),
            source_content: Some(timeline_json),
            source_file: Some("timeline.json"),
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
    fn test_timeline_logo_pass_with_design_artifact_containing_data_layer_logo() {
        let tmp = tempdir().unwrap();
        let project_dir = tmp.path().join("projects").join("demo");
        let timeline_dir = project_dir.join("artifacts").join("video-timeline");
        let badge_dir = project_dir.join("artifacts").join("brand-badge");
        fs::create_dir_all(&timeline_dir).unwrap();
        fs::create_dir_all(&badge_dir).unwrap();
        // Studio layout: the render sits under projects/<p>/renders/<id>/<hash>/.
        let badge_renders = project_dir
            .join("renders")
            .join("brand-badge")
            .join("abc123");
        fs::create_dir_all(&badge_renders).unwrap();

        // Create design artifact HTML with data-layer="logo"
        let badge_html = r#"
            <!DOCTYPE html>
            <html>
                <body>
                    <div data-layer="logo" class="badge">Brand Badge</div>
                </body>
            </html>
        "#;
        fs::write(badge_dir.join("index.html"), badge_html).unwrap();
        fs::write(badge_renders.join("output.png"), b"fake png").unwrap();

        let brand = make_test_brand();
        let channel = make_test_channel();

        let timeline_json = r#"{
            "tracks": [
                {
                    "id": "base",
                    "kind": "video",
                    "clips": [
                        { "id": "c1", "source": "clips/video1.mp4", "duration": 10.0 }
                    ]
                },
                {
                    "id": "overlay",
                    "kind": "video",
                    "clips": [
                        { "id": "badge-overlay", "source": "projects/demo/renders/brand-badge/abc123/output.png", "start": 1.0, "duration": 5.0, "width": 120 }
                    ]
                }
            ]
        }"#;

        let input = CheckInput {
            artifact_dir: &timeline_dir,
            source_content: Some(timeline_json),
            source_file: Some("timeline.json"),
            renders: &[],
            ffprobe_json: None,
            brand: &brand,
            brand_dir: None,
            channel: &channel,
        };

        let checker = LogoPresenceChecker::new();
        let findings = checker.check(&input);
        assert!(
            findings.is_empty(),
            "Findings should be empty: {findings:?}"
        );
    }

    #[test]
    fn test_timeline_design_render_without_logo_layer_is_not_a_logo() {
        let tmp = tempdir().unwrap();
        let project_dir = tmp.path().join("projects").join("demo");
        let timeline_dir = project_dir.join("artifacts").join("video-timeline");
        let scene_dir = project_dir.join("artifacts").join("scene-01");
        fs::create_dir_all(&timeline_dir).unwrap();
        fs::create_dir_all(&scene_dir).unwrap();
        fs::write(
            scene_dir.join("index.html"),
            "<img data-layer=\"broll\" src=\"a.png\">",
        )
        .unwrap();

        let brand = make_test_brand();
        let channel = make_test_channel();
        let timeline_json = r#"{"tracks":[
            {"id":"base","kind":"video","clips":[{"id":"c1","source":"clips/video1.mp4","duration":10.0}]},
            {"id":"overlay","kind":"video","clips":[{"id":"o1","source":"projects/demo/renders/scene-01/h/output.png","duration":5.0}]}
        ]}"#;
        let input = CheckInput {
            artifact_dir: &timeline_dir,
            source_content: Some(timeline_json),
            source_file: Some("timeline.json"),
            renders: &[],
            ffprobe_json: None,
            brand: &brand,
            brand_dir: None,
            channel: &channel,
        };
        let findings = LogoPresenceChecker::new().check(&input);
        assert!(
            findings.iter().any(|f| f.code == "LOGO_MISSING"),
            "{findings:?}"
        );
    }
}
