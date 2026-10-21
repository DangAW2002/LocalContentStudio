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

// ===== G3 Consistency checker tests =====

/// Helper: create a fake project structure with a timeline artifact.
fn setup_series_project(
    ws_root: &std::path::Path,
    proj_slug: &str,
    series_slug: &str,
    intro_source: &str,
    outro_source: &str,
    font: &str,
    voice: &str,
) -> (std::path::PathBuf, std::path::PathBuf) {
    // project.yaml
    let proj_dir = ws_root.join("projects").join(proj_slug);
    let arts_dir = proj_dir.join("artifacts");
    let art_dir = arts_dir.join("timeline-01");
    fs::create_dir_all(&art_dir).unwrap();

    let project_yaml = format!("slug: {proj_slug}\ntitle: Title\nseries: {series_slug}\n");
    fs::write(proj_dir.join("project.yaml"), &project_yaml).unwrap();

    // timeline.json with intro/outro
    let timeline_json = serde_json::json!({
        "version": 1,
        "width": 1920,
        "height": 1080,
        "fps": 30.0,
        "tracks": [
            {
                "id": "video",
                "kind": "video",
                "clips": [
                    { "id": "c1", "source": intro_source, "in_point": 0.0, "out_point": 2.0, "start": 0.0, "duration": 2.0, "transition": "none" },
                    { "id": "c2", "source": "main.mp4", "in_point": 0.0, "out_point": 10.0, "start": 2.0, "duration": 10.0, "transition": "none" },
                    { "id": "c3", "source": outro_source, "in_point": 0.0, "out_point": 2.0, "start": 12.0, "duration": 2.0, "transition": "none" },
                ]
            }
        ]
    });
    fs::write(
        art_dir.join("timeline.json"),
        serde_json::to_string(&timeline_json).unwrap(),
    )
    .unwrap();

    // thumbnail with font
    let html = format!("<html><head><style>h1 {{ font-family: {font}; color: #fff; }}</style></head><body><h1>Thumb</h1></body></html>");
    let thumb_dir = arts_dir.join("thumb-01");
    fs::create_dir_all(&thumb_dir).unwrap();
    fs::write(thumb_dir.join("index.html"), &html).unwrap();

    // audio with voice
    let art_yaml = format!(
        "id: voice-01\nkind: audio\ntitle: Voice\nsource: script.md\nparams:\n  voice: {voice}\n"
    );
    let voice_dir = arts_dir.join("voice-01");
    fs::create_dir_all(&voice_dir).unwrap();
    fs::write(voice_dir.join("artifact.yaml"), &art_yaml).unwrap();
    fs::write(
        voice_dir.join("script.md"),
        format!("voice: {voice}\n\nHello world"),
    )
    .unwrap();

    (art_dir, thumb_dir)
}

#[test]
fn test_consistency_checker_intro_outro_match() {
    use studio_brand::checkers::consistency::ConsistencyChecker;
    use studio_brand::checkers::{CheckInput, Checker};

    let tmp = tempdir().unwrap();
    let ws_root = tmp.path();

    // Two episodes with SAME intro/outro
    let (art_dir1, _) = setup_series_project(
        ws_root,
        "ep-01",
        "my-series",
        "intro.mp4",
        "outro.mp4",
        "Roboto",
        "vn-female-1",
    );
    let (_art_dir2, _) = setup_series_project(
        ws_root,
        "ep-02",
        "my-series",
        "intro.mp4",
        "outro.mp4",
        "Roboto",
        "vn-female-1",
    );

    let brand = example_brand();
    let channel = youtube_channel();
    let checker = ConsistencyChecker::new();
    let input = CheckInput {
        artifact_dir: &art_dir1,
        source_content: None,
        source_file: None,
        renders: &[],
        ffprobe_json: None,
        brand: &brand,
        brand_dir: None,
        channel: &channel,
    };
    let findings = checker.check(&input);
    // Same intro/outro: no SERIES_INTRO_MISMATCH or SERIES_OUTRO_MISMATCH
    assert!(
        !findings.iter().any(|f| f.code == "SERIES_INTRO_MISMATCH"),
        "No intro mismatch expected"
    );
    assert!(
        !findings.iter().any(|f| f.code == "SERIES_OUTRO_MISMATCH"),
        "No outro mismatch expected"
    );
}

#[test]
fn test_consistency_checker_intro_outro_mismatch() {
    use studio_brand::checkers::consistency::ConsistencyChecker;
    use studio_brand::checkers::{CheckInput, Checker};

    let tmp = tempdir().unwrap();
    let ws_root = tmp.path();

    // Two episodes with DIFFERENT intro/outro
    let (art_dir1, _) = setup_series_project(
        ws_root,
        "ep-01",
        "my-series",
        "intro-a.mp4",
        "outro-a.mp4",
        "Roboto",
        "vn-female-1",
    );
    let (_art_dir2, _) = setup_series_project(
        ws_root,
        "ep-02",
        "my-series",
        "intro-b.mp4",
        "outro-b.mp4",
        "Roboto",
        "vn-female-1",
    );

    let brand = example_brand();
    let channel = youtube_channel();
    let checker = ConsistencyChecker::new();
    let input = CheckInput {
        artifact_dir: &art_dir1,
        source_content: None,
        source_file: None,
        renders: &[],
        ffprobe_json: None,
        brand: &brand,
        brand_dir: None,
        channel: &channel,
    };
    let findings = checker.check(&input);
    assert!(
        findings.iter().any(|f| f.code == "SERIES_INTRO_MISMATCH"),
        "Expected SERIES_INTRO_MISMATCH finding"
    );
    assert!(
        findings.iter().any(|f| f.code == "SERIES_OUTRO_MISMATCH"),
        "Expected SERIES_OUTRO_MISMATCH finding"
    );
}

#[test]
fn test_consistency_checker_reference_missing() {
    use studio_brand::checkers::consistency::ConsistencyChecker;
    use studio_brand::checkers::{CheckInput, Checker};

    let tmp = tempdir().unwrap();
    let ws_root = tmp.path();

    // Create series.yaml that requires reference "nv-an"
    let series_dir = ws_root.join("series").join("my-series");
    fs::create_dir_all(&series_dir).unwrap();
    let series_yaml = "slug: my-series\ntitle: My Series\nreferences:\n  - nv-an\n";
    fs::write(series_dir.join("series.yaml"), series_yaml).unwrap();

    // Project ep-01 does NOT have reference edge to "nv-an"
    let proj_dir = ws_root.join("projects").join("ep-01");
    let arts_dir = proj_dir.join("artifacts");
    let art_dir = arts_dir.join("script-01");
    fs::create_dir_all(&art_dir).unwrap();
    fs::write(
        proj_dir.join("project.yaml"),
        "slug: ep-01\ntitle: T\nseries: my-series\n",
    )
    .unwrap();
    fs::write(
        art_dir.join("artifact.yaml"),
        "id: script-01\nkind: text\ntitle: T\nsource: t.md\ninputs: []\n",
    )
    .unwrap();

    let brand = example_brand();
    let channel = youtube_channel();
    let checker = ConsistencyChecker::new();
    let input = CheckInput {
        artifact_dir: &art_dir,
        source_content: None,
        source_file: None,
        renders: &[],
        ffprobe_json: None,
        brand: &brand,
        brand_dir: None,
        channel: &channel,
    };
    let findings = checker.check(&input);
    assert!(
        findings
            .iter()
            .any(|f| f.code == "SERIES_REFERENCE_MISSING"),
        "Expected SERIES_REFERENCE_MISSING finding, got: {:?}",
        findings.iter().map(|f| &f.code).collect::<Vec<_>>()
    );
}

#[test]
fn test_consistency_checker_reference_present() {
    use studio_brand::checkers::consistency::ConsistencyChecker;
    use studio_brand::checkers::{CheckInput, Checker};

    let tmp = tempdir().unwrap();
    let ws_root = tmp.path();

    // Create series.yaml that requires reference "nv-an"
    let series_dir = ws_root.join("series").join("my-series");
    fs::create_dir_all(&series_dir).unwrap();
    let series_yaml = "slug: my-series\ntitle: My Series\nreferences:\n  - nv-an\n";
    fs::write(series_dir.join("series.yaml"), series_yaml).unwrap();

    // Project ep-01 HAS reference edge to "nv-an"
    let proj_dir = ws_root.join("projects").join("ep-01");
    let arts_dir = proj_dir.join("artifacts");
    let art_dir = arts_dir.join("script-01");
    fs::create_dir_all(&art_dir).unwrap();
    fs::write(
        proj_dir.join("project.yaml"),
        "slug: ep-01\ntitle: T\nseries: my-series\n",
    )
    .unwrap();
    let art_yaml = "id: script-01\nkind: text\ntitle: T\nsource: t.md\ninputs:\n  - ref: \"artifact:nv-an\"\n    role: reference\n";
    fs::write(art_dir.join("artifact.yaml"), art_yaml).unwrap();

    let brand = example_brand();
    let channel = youtube_channel();
    let checker = ConsistencyChecker::new();
    let input = CheckInput {
        artifact_dir: &art_dir,
        source_content: None,
        source_file: None,
        renders: &[],
        ffprobe_json: None,
        brand: &brand,
        brand_dir: None,
        channel: &channel,
    };
    let findings = checker.check(&input);
    assert!(
        !findings
            .iter()
            .any(|f| f.code == "SERIES_REFERENCE_MISSING"),
        "SERIES_REFERENCE_MISSING must NOT fire when reference edge is present"
    );
}

/// The shipped example brand (checkers under test do not depend on its values).
fn example_brand() -> BrandSpec {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("defaults");
    serde_yaml::from_str(&fs::read_to_string(dir.join("brands/example/brand.yaml")).unwrap())
        .unwrap()
}

/// The shipped youtube channel.
fn youtube_channel() -> ChannelSpec {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("defaults");
    serde_yaml::from_str(&fs::read_to_string(dir.join("channels/youtube.yaml")).unwrap()).unwrap()
}
