use std::fs;
use std::path::Path;
use studio_brand::checkers::{run_checks, CheckSubject};
use studio_brand::loader::BrandSet;
use studio_brand::schema::{export_json_schemas, BrandSpec, ChannelSpec, Playbook};
use studio_workspace::{AppPaths, ScopeChain, Workspace};
use tempfile::tempdir;

#[test]
fn test_default_files_parse() {
    let defaults_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("defaults");

    // 1. Check all channel YAML files in defaults/channels/
    let channels_dir = defaults_dir.join("channels");
    assert!(channels_dir.exists(), "defaults/channels must exist");
    let mut channel_count = 0;
    for entry in fs::read_dir(&channels_dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("yaml") {
            let content = fs::read_to_string(&path).unwrap();
            let spec: ChannelSpec = serde_yaml::from_str(&content)
                .unwrap_or_else(|e| panic!("Failed to parse channel spec {:?}: {e}", path));
            assert!(!spec.id.is_empty());
            assert!(
                !spec.formats.is_empty(),
                "Channel {} should have formats",
                spec.id
            );
            channel_count += 1;
        }
    }
    assert!(
        channel_count >= 9,
        "Expected at least 9 default channels, found {channel_count}"
    );

    // 2. Check all playbooks in defaults/playbooks/
    let playbooks_dir = defaults_dir.join("playbooks");
    assert!(playbooks_dir.exists(), "defaults/playbooks must exist");
    let mut playbook_count = 0;
    for entry in fs::read_dir(&playbooks_dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let content = fs::read_to_string(&path).unwrap();
            let stem = path.file_stem().unwrap().to_str().unwrap();
            let pb = Playbook::parse(stem, &content)
                .unwrap_or_else(|e| panic!("Failed to parse playbook {:?}: {e}", path));
            assert_eq!(pb.frontmatter.kind, "playbook");
            assert!(!pb.frontmatter.steps.is_empty());
            assert!(!pb.frontmatter.checkers.is_empty());
            playbook_count += 1;
        }
    }
    assert!(
        playbook_count >= 4,
        "Expected at least 4 default playbooks, found {playbook_count}"
    );

    // 3. Check sample brand
    let example_brand_yaml = defaults_dir.join("brands/example/brand.yaml");
    assert!(example_brand_yaml.exists());
    let brand_content = fs::read_to_string(example_brand_yaml).unwrap();
    let brand_spec: BrandSpec = serde_yaml::from_str(&brand_content).unwrap();
    assert_eq!(brand_spec.name, "Example Studio");
    assert_eq!(brand_spec.logos.full, "logo.svg");
    assert_eq!(brand_spec.logos.mark, "mark.svg");
}

#[test]
fn test_schema_export() {
    let schemas = export_json_schemas();
    assert!(schemas.get("brand@1").is_some());
    assert!(schemas.get("channel@1").is_some());
    assert!(schemas.get("playbook@1").is_some());
}

#[test]
fn test_scope_chain_loader_override() {
    let tmp = tempdir().unwrap();
    let app_dir = tmp.path().join("app_data");
    let ws_dir = tmp.path().join("ws");

    let app_paths = AppPaths::new(&app_dir);

    // App layer channel
    let app_channels = app_paths.shared_dir().join("channels");
    fs::create_dir_all(&app_channels).unwrap();
    fs::write(
        app_channels.join("youtube.yaml"),
        r#"
id: "youtube"
platform: "youtube"
title: "YouTube App Level"
formats:
  - name: "std"
    width: 1920
    height: 1080
    aspect: "16:9"
notes: "App layer"
"#,
    )
    .unwrap();

    // Create workspace
    fs::create_dir_all(&ws_dir).unwrap();
    let ws = Workspace::create(&ws_dir, "Test Workspace").unwrap();

    // 1. Initial load from App layer
    let scope1 = ScopeChain::with_app_paths(&ws, app_paths.clone());
    let brand_set1 = BrandSet::load_with_scope(&scope1).unwrap();
    assert_eq!(
        brand_set1.get_channel("youtube").unwrap().title,
        "YouTube App Level"
    );

    // 2. Override in workspace layer
    let ws_channels = ws.root().join("channels");
    fs::create_dir_all(&ws_channels).unwrap();
    fs::write(
        ws_channels.join("youtube.yaml"),
        r#"
id: "youtube"
platform: "youtube"
title: "YouTube Workspace Override"
formats:
  - name: "std"
    width: 1920
    height: 1080
    aspect: "16:9"
notes: "Workspace layer"
"#,
    )
    .unwrap();

    let scope2 = ScopeChain::with_app_paths(&ws, app_paths);
    let brand_set2 = BrandSet::load_with_scope(&scope2).unwrap();
    assert_eq!(
        brand_set2.get_channel("youtube").unwrap().title,
        "YouTube Workspace Override"
    );
}

#[test]
fn test_brand_version_sha256() {
    let tmp = tempdir().unwrap();
    let app_dir = tmp.path().join("app_data");
    let ws_dir = tmp.path().join("ws");
    let app_paths = AppPaths::new(&app_dir);

    let ws = Workspace::create(&ws_dir, "Test Workspace").unwrap();
    let brand_dir = ws.root().join("brands/main");
    fs::create_dir_all(&brand_dir).unwrap();

    fs::write(
        brand_dir.join("brand.yaml"),
        r##"
name: "Versioned Brand"
slogan: ""
colors:
  primary: "#1E88E5"
  accent: "#FFB300"
  bg: "#FFFFFF"
fonts:
  heading: "Inter"
  body: "Inter"
logos:
  full: "logo.svg"
  mark: "mark.svg"
"##,
    )
    .unwrap();
    fs::write(brand_dir.join("voice.md"), "# Voice Guide").unwrap();

    let scope = ScopeChain::with_app_paths(&ws, app_paths);
    let brand_set = BrandSet::load_with_scope(&scope).unwrap();
    let brand = brand_set.get_brand("main").unwrap();

    assert!(!brand.version.is_empty());
    assert_eq!(brand.version.len(), 64); // sha256 hex length
    assert_eq!(brand.voice_text.as_deref(), Some("# Voice Guide"));
}

#[test]
fn test_run_checks_end_to_end() {
    let tmp = tempdir().unwrap();
    let app_dir = tmp.path().join("app_data");
    let ws_dir = tmp.path().join("ws");
    let app_paths = AppPaths::new(&app_dir);

    let ws = Workspace::create(&ws_dir, "Test WS").unwrap();

    // Setup brand
    let brand_dir = ws.root().join("brands/test_brand");
    fs::create_dir_all(&brand_dir).unwrap();
    fs::write(
        brand_dir.join("brand.yaml"),
        r##"
name: "Test"
colors:
  primary: "#1E88E5"
  accent: "#FFB300"
  bg: "#FFFFFF"
fonts:
  heading: "Inter"
  body: "Inter"
logos:
  full: "logo.svg"
  mark: "mark.svg"
  min_size_px: 32
voice_words:
  avoid: ["cam kết 100%"]
"##,
    )
    .unwrap();

    // Setup channel
    let channels_dir = ws.root().join("channels");
    fs::create_dir_all(&channels_dir).unwrap();
    fs::write(
        channels_dir.join("shorts.yaml"),
        r#"
id: "shorts"
platform: "youtube"
title: "Shorts"
formats:
  - name: "shorts"
    width: 1080
    height: 1920
    aspect: "9:16"
    max_duration_s: 60.0
captions_required: true
"#,
    )
    .unwrap();

    let scope = ScopeChain::with_app_paths(&ws, app_paths);
    let brand_set = BrandSet::load_with_scope(&scope).unwrap();

    let artifact_dir = tmp.path().join("artifact");
    fs::create_dir_all(&artifact_dir).unwrap();

    let html_content = r#"
<div style="background-color: #1E88E5; color: #FFFFFF;">
  <img src="logo.svg" width="48" height="48" />
  <h1>Cam kết 100% uy tín</h1>
</div>
"#;

    let ffprobe_mock = serde_json::json!({
        "format": {
            "duration": "45.0"
        },
        "streams": [
            {
                "codec_type": "video",
                "width": 1080,
                "height": 1920
            }
        ]
    });

    let srt_render = vec![artifact_dir.join("subtitles.srt")];
    fs::write(&srt_render[0], "1\n00:00:00,000 --> 00:00:01,000\nHello").unwrap();

    let report = run_checks(
        CheckSubject {
            artifact_dir: &artifact_dir,
            source_content: Some(html_content),
            source_file: Some("index.html"),
            renders: &srt_render,
            ffprobe_json: Some(&ffprobe_mock),
        },
        &brand_set,
        "test_brand",
        &["shorts"],
    )
    .expect("report should be returned");

    let ch_report = report.channels.get("shorts").unwrap();
    // Dimensions match, duration 45 <= 60, logo 48 >= 32, srt provided, palette #1E88E5 is primary.
    // However, "Cam kết 100%" triggers VOICE_WORD_AVOID as a Warn finding!
    let avoid_finding = ch_report
        .findings
        .iter()
        .find(|f| f.code == "VOICE_WORD_AVOID");
    assert!(avoid_finding.is_some());
    // Since VOICE_WORD_AVOID is Warn, not Error, overall channel passed should be true
    assert!(ch_report.passed);
}
