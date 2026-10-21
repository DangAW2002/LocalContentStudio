//! Checker to warn when timeline or design artifacts reference files not declared in `inputs`.
//! Plan 0009 §4, Task G2 §5.

use crate::checkers::{CheckInput, Checker, Finding};
use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

static HTML_IMG_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)<img[^>]+src=["']([^"']+)["']"#).unwrap());

static CSS_URL_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)url\(["']?([^"')]+)["']?\)"#).unwrap());

/// Checker that warns when an artifact references external workspace files
/// that are not explicitly declared in `artifact.yaml` `inputs`.
pub struct DeclaredInputsChecker;

impl DeclaredInputsChecker {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DeclaredInputsChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Checker for DeclaredInputsChecker {
    fn id(&self) -> &'static str {
        "declared_inputs"
    }

    fn check(&self, input: &CheckInput) -> Vec<Finding> {
        let mut findings = Vec::new();

        // 1. Load artifact.yaml to see declared inputs
        let art_yaml_path = input.artifact_dir.join("artifact.yaml");
        if !art_yaml_path.exists() {
            return findings;
        }

        let Ok(yaml_content) = std::fs::read_to_string(&art_yaml_path) else {
            return findings;
        };

        let Ok(art_val) = serde_yaml::from_str::<serde_json::Value>(&yaml_content) else {
            return findings;
        };

        let mut declared = HashSet::new();
        if let Some(inputs) = art_val.get("inputs").and_then(|v| v.as_array()) {
            for inp in inputs {
                if let Some(s) = inp.as_str() {
                    declared.insert(normalize_input_ref(s));
                } else if let Some(r) = inp.get("ref").and_then(|v| v.as_str()) {
                    declared.insert(normalize_input_ref(r));
                }
            }
        }

        // 2. Check timeline sources
        if let Some(src_content) = input.source_content {
            if let Some(src_file) = input.source_file {
                if src_file.ends_with(".json") || src_file.contains("timeline") {
                    if let Ok(timeline_json) =
                        serde_json::from_str::<serde_json::Value>(src_content)
                    {
                        check_timeline_sources(&timeline_json, &declared, &mut findings);
                    }
                } else if src_file.ends_with(".html") || src_file.ends_with(".css") {
                    check_html_css_sources(src_content, &declared, &mut findings);
                }
            }
        }

        findings
    }
}

fn normalize_input_ref(r: &str) -> String {
    let clean = r.replace('\\', "/");
    if let Some(s) = clean.strip_prefix("artifact:") {
        s.to_string()
    } else if let Some(s) = clean.strip_prefix("asset:") {
        s.to_string()
    } else {
        clean
    }
}

fn check_timeline_sources(
    val: &serde_json::Value,
    declared: &HashSet<String>,
    findings: &mut Vec<Finding>,
) {
    // Scan recursively for "source", "audio_path", "video_path", "image_path"
    match val {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                if matches!(
                    k.as_str(),
                    "source" | "audio_path" | "video_path" | "image_path" | "src"
                ) {
                    if let Some(s) = v.as_str() {
                        let norm = normalize_input_ref(s);
                        // Skip built-in color/solid generators or http urls
                        if !s.starts_with("http://")
                            && !s.starts_with("https://")
                            && !s.starts_with("color:")
                            && !norm.is_empty()
                            && !declared.contains(&norm)
                            && !declared
                                .iter()
                                .any(|d| norm.ends_with(d) || d.ends_with(&norm))
                        {
                            findings.push(
                                Finding::warn(
                                    "INPUTS_UNDECLARED",
                                    format!("Tệp \"{s}\" được sử dụng trong timeline nhưng chưa được khai báo trong inputs."),
                                )
                                .with_fix_hint(format!("Thêm tệp \"{s}\" vào inputs của artifact.yaml để đồ thị theo dõi đúng thay đổi.")),
                            );
                        }
                    }
                } else {
                    check_timeline_sources(v, declared, findings);
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                check_timeline_sources(item, declared, findings);
            }
        }
        _ => {}
    }
}

fn check_html_css_sources(content: &str, declared: &HashSet<String>, findings: &mut Vec<Finding>) {
    // Check <img src="...">
    for cap in HTML_IMG_REGEX.captures_iter(content) {
        if let Some(src) = cap.get(1) {
            let s = src.as_str();
            verify_ref(s, declared, findings);
        }
    }

    // Check url(...)
    for cap in CSS_URL_REGEX.captures_iter(content) {
        if let Some(url) = cap.get(1) {
            let s = url.as_str();
            verify_ref(s, declared, findings);
        }
    }
}

fn verify_ref(s: &str, declared: &HashSet<String>, findings: &mut Vec<Finding>) {
    let s = s.trim();
    if s.starts_with("http://")
        || s.starts_with("https://")
        || s.starts_with("data:")
        || s.is_empty()
        || s == "brand.css"
    {
        return;
    }

    let norm = normalize_input_ref(s);
    if !declared.contains(&norm)
        && !declared
            .iter()
            .any(|d| norm.ends_with(d) || d.ends_with(&norm))
    {
        findings.push(
            Finding::warn(
                "INPUTS_UNDECLARED",
                format!("Tài nguyên \"{s}\" được tham chiếu trong mã nguồn nhưng chưa được khai báo trong inputs."),
            )
            .with_fix_hint(format!("Khai báo \"{s}\" trong trường inputs của artifact.yaml để quản lý liên kết trên Sơ đồ.")),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checkers::Severity;
    use crate::schema::{BrandColors, BrandFonts, BrandLogos, BrandSpec, ChannelSpec, FormatSpec};
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
                safe_zone: Default::default(),
            }],
            text: Default::default(),
            captions_required: false,
            loudness_lufs: None,
            file: Default::default(),
            notes: "".to_string(),
        }
    }

    #[test]
    fn test_declared_inputs_checker() {
        let dir = tempdir().unwrap();
        let art_dir = dir.path().join("thumb-01");
        fs::create_dir_all(&art_dir).unwrap();

        // Write artifact.yaml with inputs declaring only logo.png
        fs::write(
            art_dir.join("artifact.yaml"),
            "id: thumb-01\nkind: design\ntitle: Thumb\nsource: index.html\ninputs:\n  - asset:lib/logo.png\n",
        )
        .unwrap();

        let checker = DeclaredInputsChecker::new();
        let brand = make_test_brand();
        let channel = make_test_channel();

        // 1. Source contains declared logo and undeclared bg.png
        let html_content = r#"<div><img src="lib/logo.png"><img src="assets/bg.png"></div>"#;
        let input = CheckInput {
            artifact_dir: &art_dir,
            source_content: Some(html_content),
            source_file: Some("index.html"),
            renders: &[],
            ffprobe_json: None,
            brand: &brand,
            brand_dir: None,
            channel: &channel,
        };

        let findings = checker.check(&input);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warn);
        assert!(findings[0].message_vi.contains("assets/bg.png"));
    }
}
