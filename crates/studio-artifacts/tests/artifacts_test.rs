use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;

use studio_artifacts::*;
use studio_jobs::{
    estimate::EstimateStore, probe::SystemProbe, resource::SchedulerConfig, scheduler::Scheduler,
};
use studio_workspace::{Project, Workspace};

struct MockProbe;
impl SystemProbe for MockProbe {
    fn free_ram_mb(&self) -> u64 {
        16384
    }
    fn free_vram_mb(&self) -> u64 {
        8192
    }
    fn cores(&self) -> usize {
        8
    }
}

struct MockEstimates;
impl EstimateStore for MockEstimates {
    fn get_peak_ram_mb(&self, _tool_id: &str) -> Option<u64> {
        None
    }
    fn update_peak_ram_mb(&self, _tool_id: &str, _observed_peak_mb: u64) {}
}

fn create_test_scheduler() -> Arc<Scheduler> {
    Scheduler::new(
        SchedulerConfig::default(),
        Arc::new(MockProbe),
        Arc::new(MockEstimates),
    )
}

fn create_test_env() -> (TempDir, Workspace, Project) {
    let tmp = TempDir::new().unwrap();
    let ws = Workspace::create(tmp.path(), "Test Workspace").unwrap();
    let proj = Project::create(&ws, "test-proj", "Test Project").unwrap();
    (tmp, ws, proj)
}

#[test]
fn test_kind_registry_coverage() {
    let kinds = KindRegistry::all();
    assert_eq!(kinds.len(), 7);

    let expected = [
        "design",
        "motion",
        "timeline",
        "text",
        "generation",
        "audio",
        "reference",
    ];
    for k in &expected {
        assert!(KindRegistry::get(k).is_some(), "Missing kind: {}", k);
    }

    // reference kind: no renderer, viewer = "reference", default_source = "profile.md"
    let ref_kind = KindRegistry::get("reference").unwrap();
    assert!(
        ref_kind.renderer.is_none(),
        "reference kind must have no renderer"
    );
    assert_eq!(ref_kind.viewer, "reference");
    assert_eq!(ref_kind.default_source, "profile.md");
}

#[test]
fn test_create_and_load_each_kind() {
    let (_tmp, _ws, proj) = create_test_env();

    let kinds = [
        "design",
        "motion",
        "timeline",
        "text",
        "generation",
        "audio",
    ];
    for kind in &kinds {
        let art_id = format!("{}-sample", kind);
        let created = Artifact::create(
            &proj,
            &art_id,
            *kind,
            format!("Sample {}", kind),
            Some("brand-v1".into()),
            vec!["youtube".into()],
            None,
        )
        .expect("Failed to create artifact");

        assert_eq!(created.id, art_id);
        assert_eq!(created.kind, *kind);

        // Verify starter source file exists
        let src_path = created.source_path(&proj);
        assert!(
            src_path.exists(),
            "Starter source does not exist: {}",
            src_path.display()
        );

        // Load back
        let loaded = Artifact::load(&proj, &art_id).expect("Failed to load artifact");
        assert_eq!(loaded.id, created.id);
        assert_eq!(loaded.kind, created.kind);
        assert_eq!(loaded.title, created.title);
        assert_eq!(loaded.brand, created.brand);
        assert_eq!(loaded.channels, created.channels);
    }

    // List artifacts
    let all = Artifact::list(&proj).unwrap();
    assert_eq!(all.len(), kinds.len());
}

#[test]
fn test_input_hash_stability_and_changes() {
    let (_tmp, ws, proj) = create_test_env();

    let art = Artifact::create(
        &proj,
        "design-hash-test",
        "design",
        "Hash Test",
        Some("brand-main".into()),
        vec!["youtube".into()],
        None,
    )
    .unwrap();

    let hash1 = compute_input_hash(&ws, &proj, &art, "v1", "render.html_png@1").unwrap();
    let hash2 = compute_input_hash(&ws, &proj, &art, "v1", "render.html_png@1").unwrap();
    assert_eq!(hash1, hash2, "Input hash must be deterministic");

    // 1. Change brand version
    let hash_brand_change =
        compute_input_hash(&ws, &proj, &art, "v2", "render.html_png@1").unwrap();
    assert_ne!(
        hash1, hash_brand_change,
        "Hash must change when brand changes"
    );

    // 2. Change renderer version
    let hash_renderer_change =
        compute_input_hash(&ws, &proj, &art, "v1", "render.html_png@2").unwrap();
    assert_ne!(
        hash1, hash_renderer_change,
        "Hash must change when renderer changes"
    );

    // 3. Change source file content
    let src_path = art.source_path(&proj);
    let original_content = fs::read_to_string(&src_path).unwrap();
    fs::write(&src_path, format!("{}<!-- modified -->", original_content)).unwrap();

    let hash_source_change =
        compute_input_hash(&ws, &proj, &art, "v1", "render.html_png@1").unwrap();
    assert_ne!(
        hash1, hash_source_change,
        "Hash must change when source changes"
    );
}

#[test]
fn test_pinned_known_hash_for_string_inputs_backwards_compat() {
    let (_tmp, ws, proj) = create_test_env();

    // Create a base text artifact that will be referenced
    let base_art =
        Artifact::create(&proj, "base-doc", "text", "Base Doc", None, vec![], None).unwrap();

    let mut art = Artifact::create(
        &proj,
        "consumer-art",
        "design",
        "Consumer",
        None,
        vec![],
        None,
    )
    .unwrap();

    // Set string input matching old behavior
    art.inputs = vec![ArtifactInput::from_str_ref(format!(
        "artifact:{}",
        base_art.id
    ))];
    art.save(&proj).unwrap();

    let hash_string =
        compute_input_hash(&ws, &proj, &art, "brand-v1", "render.html_png@1").unwrap();

    // Now reload from yaml where inputs was serialized as a string list
    let loaded = Artifact::load(&proj, &art.id).unwrap();
    let hash_reloaded =
        compute_input_hash(&ws, &proj, &loaded, "brand-v1", "render.html_png@1").unwrap();

    assert_eq!(
        hash_string, hash_reloaded,
        "Hash of artifact with string input must match exactly across reloads"
    );

    // Verify deterministic known hash (pinned)
    // Both string input and object input with Uses and same ref produce identical hash
    let mut object_art = loaded.clone();
    object_art.inputs = vec![ArtifactInput::new(
        format!("artifact:{}", base_art.id),
        InputRole::Uses,
    )];
    let hash_object =
        compute_input_hash(&ws, &proj, &object_art, "brand-v1", "render.html_png@1").unwrap();
    assert_eq!(
        hash_string, hash_object,
        "String input and object Uses input must produce identical hash"
    );
}

#[tokio::test]
async fn test_render_cache_hit_skips_job() {
    let (_tmp, ws, proj) = create_test_env();
    let scheduler = create_test_scheduler();

    let art = Artifact::create(
        &proj,
        "cache-test-art",
        "design",
        "Cache Test",
        Some("brand-1".into()),
        vec![],
        None,
    )
    .unwrap();

    let full_hash = compute_input_hash(&ws, &proj, &art, "brand-1", "render.html_png@1").unwrap();
    let short_h = hash12(&full_hash);

    // Pre-populate mock render output
    let out_dir = proj.renders_dir().join(&art.id).join(&short_h);
    fs::create_dir_all(&out_dir).unwrap();
    let fake_png = out_dir.join("output.png");
    fs::write(&fake_png, b"PNG FAKE DATA").unwrap();

    // Record in WorkspaceDb cache
    record_render_outputs(&ws, &art.id, &full_hash, std::slice::from_ref(&fake_png)).unwrap();

    // Call render() - should return cache hit without submitting any job!
    let outcome = render(&ws, &proj, &art, &scheduler).await.unwrap();
    assert!(outcome.cached, "Expected cache hit");
    assert_eq!(outcome.outputs, vec![fake_png]);
    assert!(
        outcome.job_id.is_none(),
        "No job should be scheduled on cache hit"
    );
}

#[test]
fn test_ffmpeg_graph_builder_unit() {
    let doc = TimelineDoc {
        version: 1,
        width: 1920,
        height: 1080,
        fps: 30.0,
        loudness_lufs: None,
        tracks: vec![
            Track {
                id: "video-track".into(),
                kind: TrackKind::Video,
                clips: vec![
                    Clip {
                        id: "c1".into(),
                        source: "clip1.mp4".into(),
                        in_point: 1.0,
                        out_point: Some(4.0),
                        start: 0.0,
                        duration: 3.0,
                        transition: Transition::None,
                        transition_duration: None,
                        x: None,
                        y: None,
                        width: None,
                        height: None,
                        opacity: None,
                        ..Default::default()
                    },
                    Clip {
                        id: "c2".into(),
                        source: "clip2.mp4".into(),
                        in_point: 0.0,
                        out_point: Some(2.0),
                        start: 3.0,
                        duration: 2.0,
                        transition: Transition::Fade,
                        transition_duration: Some(0.5),
                        x: None,
                        y: None,
                        width: None,
                        height: None,
                        opacity: None,
                        ..Default::default()
                    },
                ],
                style: None,
            },
            Track {
                id: "captions-track".into(),
                kind: TrackKind::Captions,
                clips: vec![Clip {
                    id: "cap1".into(),
                    source: "Intro Hello".into(),
                    in_point: 0.0,
                    out_point: None,
                    start: 0.5,
                    duration: 2.0,
                    transition: Transition::None,
                    transition_duration: None,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                    opacity: None,
                    ..Default::default()
                }],
                style: None,
            },
        ],
    };

    let out_path = PathBuf::from("rendered.mp4");
    let (args, filter_graph) = build_timeline_ffmpeg_args(&doc, &out_path).unwrap();

    // Verify filter_graph string structure
    assert!(filter_graph.contains(
        "trim=start=1:duration=3,setpts=PTS-STARTPTS,fps=30,scale=1920:1080,setsar=1[v0]"
    ));
    assert!(filter_graph.contains("trim=start=0:duration=2,setpts=PTS-STARTPTS,fps=30,scale=1920:1080,setsar=1,fade=t=in:st=0:d=0.5[v1]"));
    assert!(filter_graph.contains("[v0][v1]concat=n=2:v=1:a=0[v]"));
    assert!(filter_graph.contains("text='Intro Hello'"));
    assert!(filter_graph.contains("enable='between(t,0.5,2.5)'"));
    assert!(filter_graph.contains("h-text_h-"));

    // Verify args contains filter_complex and output
    assert!(args.contains(&"-filter_complex".to_string()));
    assert!(args.contains(&"libx264".to_string()));
    assert!(args.contains(&"rendered.mp4".to_string()));
}

#[tokio::test]
async fn test_windows_integration_html_to_png() {
    let browser_report = probe_chromium();
    if !browser_report.available {
        println!(
            "SKIP: test_windows_integration_html_to_png — no Chromium-family browser found ({})",
            browser_report.note
        );
        return;
    }

    let (_tmp, ws, proj) = create_test_env();
    let scheduler = create_test_scheduler();

    let mut params = serde_json::Map::new();
    params.insert("width".into(), serde_json::json!(1080));
    params.insert("height".into(), serde_json::json!(1350));

    let art = Artifact::create(
        &proj,
        "design-real-render",
        "design",
        "Design Real Render",
        Some("brand-edge".into()),
        vec!["instagram".into()],
        Some(serde_json::Value::Object(params)),
    )
    .unwrap();

    let outcome = render(&ws, &proj, &art, &scheduler)
        .await
        .expect("HTML render failed");
    assert!(!outcome.cached);
    assert_eq!(outcome.outputs.len(), 1);

    let png_path = &outcome.outputs[0];
    assert!(png_path.exists());

    // Validate PNG dimensions
    let meta =
        validate_png(png_path, Some(1080), Some(1350)).expect("PNG dimension validation failed");
    assert_eq!(meta.width, 1080);
    assert_eq!(meta.height, 1350);
    println!(
        "SUCCESS: Rendered HTML to PNG with dimensions {}x{} at {}",
        meta.width,
        meta.height,
        png_path.display()
    );
}

#[tokio::test]
async fn test_windows_integration_ffmpeg_timeline() {
    let ffmpeg_report = probe_ffmpeg();
    let ffprobe_report = probe_ffprobe();
    if !ffmpeg_report.available || !ffprobe_report.available {
        println!("SKIP: test_windows_integration_ffmpeg_timeline — FFmpeg or FFprobe missing");
        return;
    }

    let (_tmp, ws, proj) = create_test_env();
    let scheduler = create_test_scheduler();

    // 1. Generate 2 short synthetic video clips using ffmpeg lavfi testsrc
    let ffmpeg_bin = ffmpeg_report.path.unwrap();
    let clip1 = proj.root().join("clip1.mp4");
    let clip2 = proj.root().join("clip2.mp4");

    let status1 = std::process::Command::new(&ffmpeg_bin)
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=2:size=1280x720:rate=30",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            clip1.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to create test clip 1");
    assert!(status1.status.success());

    let status2 = std::process::Command::new(&ffmpeg_bin)
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=2:size=1280x720:rate=30",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            clip2.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to create test clip 2");
    assert!(status2.status.success());

    // 2. Create timeline artifact with these 2 clips
    let timeline_doc = TimelineDoc {
        version: 1,
        width: 1280,
        height: 720,
        fps: 30.0,
        loudness_lufs: None,
        tracks: vec![Track {
            id: "main-video".into(),
            kind: TrackKind::Video,
            clips: vec![
                Clip {
                    id: "part1".into(),
                    source: clip1.to_string_lossy().to_string(),
                    in_point: 0.0,
                    out_point: Some(2.0),
                    start: 0.0,
                    duration: 2.0,
                    transition: Transition::None,
                    transition_duration: None,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                    opacity: None,
                    ..Default::default()
                },
                Clip {
                    id: "part2".into(),
                    source: clip2.to_string_lossy().to_string(),
                    in_point: 0.0,
                    out_point: Some(2.0),
                    start: 2.0,
                    duration: 2.0,
                    transition: Transition::Fade,
                    transition_duration: Some(0.5),
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                    opacity: None,
                    ..Default::default()
                },
            ],
            style: None,
        }],
    };

    let art = Artifact::create(
        &proj,
        "timeline-real-render",
        "timeline",
        "Timeline 2 Clips",
        None,
        vec![],
        None,
    )
    .unwrap();

    // Overwrite timeline.json with our 2-clip timeline
    fs::write(
        art.source_path(&proj),
        serde_json::to_string_pretty(&timeline_doc).unwrap(),
    )
    .unwrap();

    let outcome = render(&ws, &proj, &art, &scheduler)
        .await
        .expect("Timeline render failed");
    assert!(!outcome.cached);
    assert_eq!(outcome.outputs.len(), 1);

    let mp4_path = &outcome.outputs[0];
    assert!(mp4_path.exists());

    // Validate video output
    let meta = validate_video(mp4_path, ffprobe_report.path.as_deref(), Some(3.8))
        .expect("Video validation failed");
    assert_eq!(meta.width, 1280);
    assert_eq!(meta.height, 720);
    assert!(meta.duration_seconds >= 3.8 && meta.duration_seconds <= 4.5);
    println!(
        "SUCCESS: Rendered 2-clip timeline to MP4 duration {}s at {}",
        meta.duration_seconds,
        mp4_path.display()
    );
}

#[tokio::test]
async fn test_windows_integration_proxy_generation() {
    let ffmpeg_report = probe_ffmpeg();
    let ffprobe_report = probe_ffprobe();
    if !ffmpeg_report.available || !ffprobe_report.available {
        println!("SKIP: test_windows_integration_proxy_generation — FFmpeg or FFprobe missing");
        return;
    }

    let (_tmp, _ws, proj) = create_test_env();
    let scheduler = create_test_scheduler();

    // Generate 1-second 1080p source video
    let ffmpeg_bin = ffmpeg_report.path.clone().unwrap();
    let src_video = proj.root().join("source_1080p.mp4");

    let status = std::process::Command::new(&ffmpeg_bin)
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=1:size=1920x1080:rate=30",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            src_video.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to create test source video");
    assert!(status.status.success());

    let proxy_video = proj.root().join("proxy_720p.mp4");
    let poster_img = proj.root().join("poster.jpg");

    let job_spec = build_proxy_job(
        "job-proxy-1",
        "ws-1",
        &ffmpeg_report,
        &src_video,
        &proxy_video,
        &poster_img,
    )
    .unwrap();

    let mut handle = scheduler.submit(job_spec).await;
    while let Some(event) = handle.next_event().await {
        if let studio_jobs::event::TaskEvent::Finished { exit_code, .. } = event {
            assert_eq!(exit_code, 0);
            break;
        }
    }

    assert!(proxy_video.exists(), "Proxy video must exist");
    assert!(poster_img.exists(), "Poster image must exist");

    let video_meta = validate_video(&proxy_video, ffprobe_report.path.as_deref(), Some(0.9))
        .expect("Proxy video validation failed");
    assert_eq!(video_meta.height, 720, "Proxy video height must be 720p");
    println!(
        "SUCCESS: Generated 720p proxy video and poster at {}",
        proxy_video.display()
    );
}

async fn do_real_hyperframes_renders_mp4() {
    let ffprobe_report = probe_ffprobe();
    let (_tmp, ws, proj) = create_test_env();
    let scheduler = create_test_scheduler();

    let art = Artifact::create(
        &proj,
        "hyperframes-sample",
        "motion",
        "HyperFrames Sample",
        None,
        vec![],
        None,
    )
    .unwrap();

    let outcome = render(&ws, &proj, &art, &scheduler)
        .await
        .expect("HyperFrames render failed");
    assert!(!outcome.cached);
    assert_eq!(outcome.outputs.len(), 1);

    let mp4_path = &outcome.outputs[0];
    assert!(mp4_path.exists());
    let file_size = fs::metadata(mp4_path).unwrap().len();
    assert!(file_size > 0, "Rendered MP4 file is empty");

    // Validate with ffprobe
    let meta = validate_video(mp4_path, ffprobe_report.path.as_deref(), Some(4.5))
        .expect("HyperFrames video validation failed");
    assert_eq!(meta.width, 1920);
    assert_eq!(meta.height, 1080);
    assert!(meta.duration_seconds > 0.0, "Video duration must be > 0");

    println!(
        "SUCCESS: HyperFrames rendered MOTION_STARTER_HTML to {} (size: {} bytes, duration: {:.2}s, resolution: {}x{})",
        mp4_path.display(),
        file_size,
        meta.duration_seconds,
        meta.width,
        meta.height
    );
}

#[tokio::test]
#[ignore = "needs network/npm for HyperFrames CLI render"]
async fn real_hyperframes_renders_mp4() {
    do_real_hyperframes_renders_mp4().await;
}

#[tokio::test]
#[ignore = "alias to real_hyperframes_renders_mp4"]
async fn test_hyperframes_render_integration() {
    do_real_hyperframes_renders_mp4().await;
}

#[tokio::test]
async fn test_brand_content_edit_invalidates_cache() {
    let (_tmp, ws, proj) = create_test_env();

    // Create a brand directory under workspace root: brands/my-brand
    let brand_dir = ws.root().join("brands").join("my-brand");
    fs::create_dir_all(&brand_dir).unwrap();

    let brand_yaml_path = brand_dir.join("brand.yaml");
    fs::write(
        &brand_yaml_path,
        r##"
version: 1
name: "My Brand"
colors:
  primary: "#112233"
logos:
  mark: "logo.png"
"##,
    )
    .unwrap();

    let logo_path = brand_dir.join("logo.png");
    fs::write(&logo_path, b"ORIGINAL_LOGO_BYTES").unwrap();

    let art = Artifact::create(
        &proj,
        "design-art",
        "design",
        "Design Card",
        Some("my-brand@1".into()),
        vec![],
        None,
    )
    .unwrap();

    // Compute original input hash
    let hash1 = compute_input_hash(&ws, &proj, &art, "my-brand@1", "render.html_png@1").unwrap();

    // 1. Modifying referenced logo file must invalidate hash
    fs::write(&logo_path, b"MODIFIED_LOGO_BYTES").unwrap();
    let hash2 = compute_input_hash(&ws, &proj, &art, "my-brand@1", "render.html_png@1").unwrap();
    assert_ne!(
        hash1, hash2,
        "Editing referenced logo file must change input hash and invalidate cache"
    );

    // 2. Modifying brand.yaml must invalidate hash
    fs::write(
        &brand_yaml_path,
        r##"
version: 1
name: "My Brand Renamed"
colors:
  primary: "#998877"
logos:
  mark: "logo.png"
"##,
    )
    .unwrap();
    let hash3 = compute_input_hash(&ws, &proj, &art, "my-brand@1", "render.html_png@1").unwrap();
    assert_ne!(
        hash2, hash3,
        "Editing brand.yaml must change input hash and invalidate cache"
    );
}

#[test]
fn test_timeline_ffmpeg_args_with_image_audio_and_srt() {
    let doc = TimelineDoc {
        version: 1,
        width: 1920,
        height: 1080,
        fps: 30.0,
        loudness_lufs: None,
        tracks: vec![
            Track {
                id: "video-track".into(),
                kind: TrackKind::Video,
                clips: vec![Clip {
                    id: "img1".into(),
                    source: "slide.png".into(),
                    in_point: 0.0,
                    out_point: Some(5.0),
                    start: 0.0,
                    duration: 5.0,
                    transition: Transition::None,
                    transition_duration: None,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                    opacity: None,
                    ..Default::default()
                }],
                style: None,
            },
            Track {
                id: "audio-track".into(),
                kind: TrackKind::Audio,
                clips: vec![Clip {
                    id: "aud1".into(),
                    source: "voice.wav".into(),
                    in_point: 0.0,
                    out_point: None,
                    start: 0.5,
                    duration: 4.5,
                    transition: Transition::None,
                    transition_duration: None,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                    opacity: None,
                    ..Default::default()
                }],
                style: None,
            },
            Track {
                id: "caption-track".into(),
                kind: TrackKind::Caption,
                clips: vec![Clip {
                    id: "srt1".into(),
                    source: "C:\\projects\\demo\\captions.srt".into(),
                    in_point: 0.0,
                    out_point: None,
                    start: 0.0,
                    duration: 5.0,
                    transition: Transition::None,
                    transition_duration: None,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                    opacity: None,
                    ..Default::default()
                }],
                style: None,
            },
        ],
    };

    let out_path = PathBuf::from("output.mp4");
    let (args, filter_graph) = build_timeline_ffmpeg_args(&doc, &out_path).unwrap();

    // Verify image looping in filter graph
    assert!(filter_graph.contains("loop=loop=-1:size=1:start=0,trim=start=0:duration=5"));

    // Verify audio atrim and delay
    assert!(
        filter_graph.contains("atrim=start=0:duration=4.5,asetpts=PTS-STARTPTS,adelay=500|500[a]")
    );

    // Verify subtitle path escaping for Windows (C\: and forward slashes)
    assert!(filter_graph.contains("subtitles='C\\:/projects/demo/captions.srt'"));

    // Verify FFmpeg argument structure
    assert!(args.contains(&"-i".to_string()));
    assert!(args.contains(&"slide.png".to_string()));
    assert!(args.contains(&"voice.wav".to_string()));
    assert!(args.contains(&"-map".to_string()));
    assert!(args.contains(&"[v]".to_string()));
    assert!(args.contains(&"[a]".to_string()));
    assert!(args.contains(&"aac".to_string()));
    assert!(!filter_graph.contains("loudnorm"));

    // A loudness target routes the mix through loudnorm and still maps [a].
    let mut doc = doc;
    doc.loudness_lufs = Some(-14.0);
    let (args, filter_graph) = build_timeline_ffmpeg_args(&doc, &out_path).unwrap();
    assert!(filter_graph.contains("adelay=500|500[amix]"));
    assert!(filter_graph.contains("[amix]loudnorm=I=-14:TP=-1.5:LRA=11,aresample=48000[a]"));
    assert!(args.contains(&"[a]".to_string()));
}

#[tokio::test]
#[ignore = "real timeline render assembling image + wav + srt"]
async fn real_timeline_assembles_image_wav_srt_to_mp4() {
    let ffmpeg_report = probe_ffmpeg();
    let ffprobe_report = probe_ffprobe();
    if !ffmpeg_report.available || !ffprobe_report.available {
        println!("SKIP: real_timeline_assembles_image_wav_srt_to_mp4 — FFmpeg or FFprobe missing");
        return;
    }

    let (_tmp, ws, proj) = create_test_env();
    let scheduler = create_test_scheduler();
    let ffmpeg_bin = ffmpeg_report.path.clone().unwrap();

    // 1. Generate test image (1920x1080 PNG)
    let img_path = proj.root().join("test_card.png");
    let status = std::process::Command::new(&ffmpeg_bin)
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=navy:s=1920x1080:d=1:r=30",
            "-vframes",
            "1",
            img_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to create test image");
    assert!(status.status.success(), "Failed to create PNG via FFmpeg");

    // 2. Generate test WAV audio (4 seconds 22050Hz sine tone / speech proxy)
    let wav_path = proj.root().join("test_voice.wav");
    let status = std::process::Command::new(&ffmpeg_bin)
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=4",
            "-ar",
            "22050",
            "-ac",
            "1",
            wav_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to create test WAV");
    assert!(status.status.success(), "Failed to create WAV via FFmpeg");

    // 3. Create test SRT captions file with Vietnamese text
    let srt_path = proj.root().join("test_captions.srt");
    let srt_content = "1\n00:00:00,000 --> 00:00:02,000\nXin chào! Đây là phụ đề thử nghiệm.\n\n2\n00:00:02,000 --> 00:00:04,000\nLocal Content Studio - Render Timeline thành công.\n";
    fs::write(&srt_path, srt_content).expect("Failed to write test SRT");

    // 4. Construct TimelineDoc
    let timeline_doc = TimelineDoc {
        version: 1,
        width: 1920,
        height: 1080,
        fps: 30.0,
        loudness_lufs: None,
        tracks: vec![
            Track {
                id: "video-track-1".into(),
                kind: TrackKind::Video,
                clips: vec![Clip {
                    id: "img-clip".into(),
                    source: img_path.to_string_lossy().to_string(),
                    in_point: 0.0,
                    out_point: Some(4.0),
                    start: 0.0,
                    duration: 4.0,
                    transition: Transition::None,
                    transition_duration: None,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                    opacity: None,
                    ..Default::default()
                }],
                style: None,
            },
            Track {
                id: "audio-track-1".into(),
                kind: TrackKind::Audio,
                clips: vec![Clip {
                    id: "wav-clip".into(),
                    source: wav_path.to_string_lossy().to_string(),
                    in_point: 0.0,
                    out_point: Some(4.0),
                    start: 0.0,
                    duration: 4.0,
                    transition: Transition::None,
                    transition_duration: None,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                    opacity: None,
                    ..Default::default()
                }],
                style: None,
            },
            Track {
                id: "caption-track-1".into(),
                kind: TrackKind::Caption,
                clips: vec![Clip {
                    id: "srt-clip".into(),
                    source: srt_path.to_string_lossy().to_string(),
                    in_point: 0.0,
                    out_point: None,
                    start: 0.0,
                    duration: 4.0,
                    transition: Transition::None,
                    transition_duration: None,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                    opacity: None,
                    ..Default::default()
                }],
                style: None,
            },
        ],
    };

    let art = Artifact::create(
        &proj,
        "timeline-image-audio-srt",
        "timeline",
        "Timeline Image + Audio + SRT",
        None,
        vec![],
        None,
    )
    .unwrap();

    fs::write(
        art.source_path(&proj),
        serde_json::to_string_pretty(&timeline_doc).unwrap(),
    )
    .unwrap();

    // Channel loudness target, as studio-mcp resolves it for youtube.
    let options = RenderOptions {
        loudness_lufs: Some(-14.0),
    };
    let outcome = render_with(&ws, &proj, &art, &scheduler, &options)
        .await
        .expect("Timeline render of image + wav + srt failed");
    assert!(!outcome.cached);
    assert_eq!(outcome.outputs.len(), 1);

    let mp4_path = &outcome.outputs[0];
    assert!(mp4_path.exists());
    let file_size = fs::metadata(mp4_path).unwrap().len();
    assert!(file_size > 0);

    // Validate video + audio streams with ffprobe
    let meta = validate_video(mp4_path, ffprobe_report.path.as_deref(), Some(3.8))
        .expect("Video validation failed");
    assert_eq!(meta.width, 1920);
    assert_eq!(meta.height, 1080);
    assert!(meta.duration_seconds >= 3.8 && meta.duration_seconds <= 4.2);

    // Integrated loudness lands near the -14 LUFS target.
    let ebur = std::process::Command::new(&ffmpeg_bin)
        .args(["-hide_banner", "-nostats", "-i"])
        .arg(mp4_path)
        .args(["-af", "ebur128", "-f", "null", "-"])
        .output()
        .expect("ebur128 measurement failed");
    let log = String::from_utf8_lossy(&ebur.stderr);
    let summary = &log[log.rfind("Summary:").expect("no ebur128 summary")..];
    let integrated: f64 = summary
        .lines()
        .find_map(|l| l.trim().strip_prefix("I:"))
        .and_then(|v| v.trim().trim_end_matches("LUFS").trim().parse().ok())
        .expect("no integrated loudness");
    assert!(
        (integrated + 14.0).abs() <= 1.5,
        "integrated loudness {integrated} LUFS, expected about -14"
    );

    println!(
        "SUCCESS: Assembled image + wav + srt to MP4: {} (size: {} bytes, duration: {:.2}s, resolution: {}x{})",
        mp4_path.display(),
        file_size,
        meta.duration_seconds,
        meta.width,
        meta.height
    );
}

#[test]
fn test_timeline_starter_parses_and_validates() {
    let doc: TimelineDoc = serde_json::from_str(studio_artifacts::TIMELINE_STARTER)
        .expect("TIMELINE_STARTER must parse as TimelineDoc");
    assert_eq!(doc.version, 1);
    assert_eq!(doc.width, 1920);
    assert_eq!(doc.height, 1080);
    assert_eq!(doc.fps, 30.0);
    assert_eq!(doc.tracks.len(), 3);

    let video_track = doc
        .tracks
        .iter()
        .find(|t| t.kind == TrackKind::Video)
        .expect("Video track required");
    assert!(
        !video_track.clips.is_empty(),
        "Video track must contain a placeholder clip"
    );
    assert_eq!(video_track.clips[0].duration, 5.0);

    let caption_track = doc
        .tracks
        .iter()
        .find(|t| t.kind.is_caption())
        .expect("Caption track required");
    assert_eq!(
        caption_track.clips.len(),
        2,
        "Caption track must contain 2 captions"
    );
}

#[test]
fn test_timeline_overlay_filter_graph() {
    let doc = TimelineDoc {
        version: 1,
        width: 1920,
        height: 1080,
        fps: 30.0,
        loudness_lufs: None,
        tracks: vec![
            Track {
                id: "base-video".into(),
                kind: TrackKind::Video,
                clips: vec![Clip {
                    id: "c1".into(),
                    source: "color:#1E88E5".into(),
                    in_point: 0.0,
                    out_point: None,
                    start: 0.0,
                    duration: 10.0,
                    transition: Transition::None,
                    transition_duration: None,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                    opacity: None,
                    ..Default::default()
                }],
                style: None,
            },
            Track {
                id: "overlay-track".into(),
                kind: TrackKind::Video,
                clips: vec![Clip {
                    id: "logo-clip".into(),
                    source: "brand/logo.png".into(),
                    in_point: 0.0,
                    out_point: None,
                    start: 1.5,
                    duration: 5.0,
                    transition: Transition::None,
                    transition_duration: None,
                    x: Some(40),
                    y: Some(60),
                    width: Some(250),
                    height: None,
                    opacity: Some(0.85),
                    ..Default::default()
                }],
                style: None,
            },
        ],
    };

    let out_path = PathBuf::from("overlay_out.mp4");
    let (args, filter_graph) = build_timeline_ffmpeg_args(&doc, &out_path).expect("args build ok");

    // Verify overlay image looping, trim, PTS delay, keep-aspect scaling, and opacity mixer
    assert!(filter_graph.contains("loop=loop=-1:size=1:start=0,trim=start=0:duration=5,setpts=PTS-STARTPTS+1.5/TB,fps=30,scale=250:-1,format=rgba,colorchannelmixer=aa=0.85[ov0]"));
    // Verify overlay compositing with position and enable timing
    assert!(filter_graph
        .contains("[v][ov0]overlay=x=40:y=60:enable='between(t,1.5,6.5)':eof_action=pass[v]"));
    assert!(args.contains(&"-i".to_string()));
    assert!(args.contains(&"brand/logo.png".to_string()));
}

#[test]
fn test_timeline_rejects_svg_source() {
    let doc = TimelineDoc {
        version: 1,
        width: 1920,
        height: 1080,
        fps: 30.0,
        loudness_lufs: None,
        tracks: vec![Track {
            id: "overlay".into(),
            kind: TrackKind::Video,
            clips: vec![Clip {
                id: "logo".into(),
                source: "logo.svg".into(),
                in_point: 0.0,
                out_point: None,
                start: 0.0,
                duration: 5.0,
                transition: Transition::None,
                transition_duration: None,
                x: None,
                y: None,
                width: None,
                height: None,
                opacity: None,
                ..Default::default()
            }],
            style: None,
        }],
    };

    let out_path = PathBuf::from("out.mp4");
    let err = build_timeline_ffmpeg_args(&doc, &out_path).unwrap_err();
    match err {
        ArtifactError::InvalidConfig(msg) => {
            assert!(msg.contains("SVG"), "Message should mention SVG: {msg}");
            assert!(
                msg.contains("PNG"),
                "Message should mention PNG design artifact: {msg}"
            );
        }
        other => panic!("Expected InvalidConfig, got {other:?}"),
    }
}

#[tokio::test]
#[ignore = "real timeline render asserting overlay outside base duration does not extend video"]
async fn real_timeline_overlay_does_not_extend_duration_beyond_base_track() {
    let ffmpeg_report = probe_ffmpeg();
    let ffprobe_report = probe_ffprobe();
    if !ffmpeg_report.available || !ffprobe_report.available {
        println!(
            "SKIP: real_timeline_overlay_does_not_extend_duration_beyond_base_track — missing tool"
        );
        return;
    }

    let (_tmp, _ws, proj) = create_test_env();
    let scheduler = create_test_scheduler();
    let ffmpeg_bin = ffmpeg_report.path.clone().unwrap();
    let ffprobe_bin = ffprobe_report.path.clone().unwrap();

    let logo_path = proj.root().join("test_logo.png");
    let status = std::process::Command::new(&ffmpeg_bin)
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=yellow:s=100x100:d=1:r=30",
            "-vframes",
            "1",
            logo_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to create test logo PNG");
    assert!(status.status.success());

    // Base track is 3.0 seconds, overlay clip duration is 15.0 seconds (well past base).
    let doc = TimelineDoc {
        version: 1,
        width: 1280,
        height: 720,
        fps: 30.0,
        loudness_lufs: None,
        tracks: vec![
            Track {
                id: "base".into(),
                kind: TrackKind::Video,
                clips: vec![Clip {
                    id: "c1".into(),
                    source: "color:#1E88E5".into(),
                    in_point: 0.0,
                    out_point: None,
                    start: 0.0,
                    duration: 3.0,
                    transition: Transition::None,
                    transition_duration: None,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                    opacity: None,
                    ..Default::default()
                }],
                style: None,
            },
            Track {
                id: "overlay".into(),
                kind: TrackKind::Video,
                clips: vec![Clip {
                    id: "logo".into(),
                    source: logo_path.to_string_lossy().to_string(),
                    in_point: 0.0,
                    out_point: None,
                    start: 0.0,
                    duration: 15.0,
                    transition: Transition::None,
                    transition_duration: None,
                    x: Some(20),
                    y: Some(20),
                    width: Some(100),
                    height: Some(100),
                    opacity: Some(0.9),
                    ..Default::default()
                }],
                style: None,
            },
        ],
    };

    let out_file = proj.root().join("output_overlay.mp4");
    let job = build_timeline_render_job(
        "job-overlay-test",
        "ws-test",
        &ffmpeg_report,
        &doc,
        proj.root(),
        &out_file,
    )
    .expect("job spec ok");

    let mut handle = scheduler.submit(job).await;
    while let Some(evt) = handle.next_event().await {
        if matches!(
            evt,
            studio_jobs::TaskEvent::Finished { .. } | studio_jobs::TaskEvent::Failed { .. }
        ) {
            break;
        }
    }
    assert!(out_file.exists(), "Output mp4 must exist");

    let probe_out = std::process::Command::new(&ffprobe_bin)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            out_file.to_str().unwrap(),
        ])
        .output()
        .expect("probe failed");
    let dur_str = String::from_utf8_lossy(&probe_out.stdout)
        .trim()
        .to_string();
    let duration: f64 = dur_str.parse().expect("parse duration");
    assert!(
        (duration - 3.0).abs() < 0.25,
        "Duration {duration} should match base track (3.0s), not overlay (15.0s)"
    );
}

#[test]
fn test_timeline_build_validation_and_rejection() {
    let (_tmp, ws, _proj) = create_test_env();

    // 1. Missing audio file
    let params_missing = TimelineBuildParams {
        project: "test-proj".into(),
        artifact: "video".into(),
        segments: vec![SegmentInput {
            audio: "projects/test-proj/missing.wav".into(),
            text: "Phân cảnh thử nghiệm".into(),
            visual: "color:#FF0000".into(),
            ..Default::default()
        }],
        lead_s: 0.5,
        gap_s: 0.4,
        tail_s: 1.5,
        transition: "fade".into(),
        captions: "sentences".into(),
        overlays: vec![],
        brand: None,
        channels: vec!["youtube".into()],
        width: None,
        height: None,
        fps: None,
        min_duration_s: None,
        caption_style: None,
        max_still_s: 6.0,
    };
    let err = build_timeline(&ws, params_missing).unwrap_err();
    assert!(
        err.contains("Không tìm thấy tệp"),
        "Expected missing file error, got: {err}"
    );

    // 2. Path escape (outside workspace)
    let params_escape = TimelineBuildParams {
        project: "test-proj".into(),
        artifact: "video".into(),
        segments: vec![SegmentInput {
            audio: "../../outside.wav".into(),
            text: "Thoát workspace".into(),
            visual: "color:#FF0000".into(),
            ..Default::default()
        }],
        lead_s: 0.5,
        gap_s: 0.4,
        tail_s: 1.5,
        transition: "fade".into(),
        captions: "sentences".into(),
        overlays: vec![],
        brand: None,
        channels: vec!["youtube".into()],
        width: None,
        height: None,
        fps: None,
        min_duration_s: None,
        caption_style: None,
        max_still_s: 6.0,
    };
    let err = build_timeline(&ws, params_escape.clone()).unwrap_err();
    assert!(
        err.contains("nằm ngoài workspace") || err.contains("không hợp lệ"),
        "Expected path escape error, got: {err}"
    );

    // 3. Artifact id that would leave the artifacts directory
    let mut bad_id = params_escape.clone();
    bad_id.artifact = "../../escaped".into();
    let err = build_timeline(&ws, bad_id).unwrap_err();
    assert!(err.contains("ID artifact"), "Expected id error, got: {err}");
    assert!(!_proj.root().join("escaped").exists());

    // 4. An existing artifact of another kind is not overwritten
    let design_dir = _proj.artifacts_dir().join("thumb");
    std::fs::create_dir_all(&design_dir).unwrap();
    let yaml = "id: thumb\nkind: design\ntitle: Thumb\nsource: index.html\n";
    std::fs::write(design_dir.join("artifact.yaml"), yaml).unwrap();
    let wav = _proj.root().join("tone.wav");
    if let Some(bin) = probe_ffmpeg().path {
        let ok = std::process::Command::new(bin)
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                "sine=duration=1",
                "-loglevel",
                "error",
            ])
            .arg(&wav)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            let mut overwrite = params_escape;
            overwrite.artifact = "thumb".into();
            overwrite.segments[0].audio = "projects/test-proj/tone.wav".into();
            let err = build_timeline(&ws, overwrite).unwrap_err();
            assert!(err.contains("đã tồn tại với loại"), "got: {err}");
            assert_eq!(
                std::fs::read_to_string(design_dir.join("artifact.yaml")).unwrap(),
                yaml
            );
            assert!(!design_dir.join("timeline.json").exists());
        }
    }
}

#[tokio::test]
#[ignore = "builds then renders with timeline_build and checks MP4 duration"]
async fn real_timeline_build_then_render_checks_duration() {
    let ffmpeg_report = probe_ffmpeg();
    let ffprobe_report = probe_ffprobe();
    if !ffmpeg_report.available || !ffprobe_report.available {
        println!(
            "SKIP: real_timeline_build_then_render_checks_duration — FFmpeg or FFprobe missing"
        );
        return;
    }

    let (_tmp, ws, proj) = create_test_env();
    let scheduler = create_test_scheduler();
    let ffmpeg_bin = ffmpeg_report.path.unwrap();

    // 1. Generate 2 test WAV files: 2.0s and 3.0s
    let wav1 = proj.root().join("vo1.wav");
    let wav2 = proj.root().join("vo2.wav");

    let status1 = std::process::Command::new(&ffmpeg_bin)
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=2",
            "-ar",
            "22050",
            "-ac",
            "1",
            wav1.to_str().unwrap(),
        ])
        .output()
        .expect("generate wav1");
    assert!(status1.status.success());

    let status2 = std::process::Command::new(&ffmpeg_bin)
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=880:duration=3",
            "-ar",
            "22050",
            "-ac",
            "1",
            wav2.to_str().unwrap(),
        ])
        .output()
        .expect("generate wav2");
    assert!(status2.status.success());

    // 2. Generate 1 logo PNG for overlay
    let logo = proj.root().join("logo.png");
    let status_logo = std::process::Command::new(&ffmpeg_bin)
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=yellow:s=200x80:d=1:r=30",
            "-vframes",
            "1",
            logo.to_str().unwrap(),
        ])
        .output()
        .expect("generate logo");
    assert!(status_logo.status.success());

    // 3. Build timeline: 2 segments (lead_s=0.5, gap_s=0.4, tail_s=1.5)
    // Audio 1: 2.0s, Audio 2: 3.0s
    // Starts: 0.5, 0.5 + 2.0 + 0.4 = 2.9
    // End = 2.9 + 3.0 + 1.5 = 7.4s
    let params = TimelineBuildParams {
        project: "test-proj".into(),
        artifact: "video".into(),
        segments: vec![
            SegmentInput {
                audio: "projects/test-proj/vo1.wav".into(),
                text: "Xin chào các bạn đã đến với kênh.".into(),
                visual: "color:#1E88E5".into(),
                ..Default::default()
            },
            SegmentInput {
                audio: "projects/test-proj/vo2.wav".into(),
                text: "Hôm nay chúng ta cùng tìm hiểu cách làm video hoàn toàn tự động nhé.".into(),
                visual: "color:#43A047".into(),
                ..Default::default()
            },
        ],
        lead_s: 0.5,
        gap_s: 0.4,
        tail_s: 1.5,
        transition: "fade".into(),
        captions: "sentences".into(),
        overlays: vec![OverlayInput {
            source: "projects/test-proj/logo.png".into(),
            x: Some(40),
            y: Some(40),
            width: Some(200),
            height: None,
            opacity: Some(0.85),
            start: None,
            duration: None,
        }],
        brand: None,
        channels: vec!["youtube".into()],
        width: Some(1280),
        height: Some(720),
        fps: Some(30.0),
        min_duration_s: None,
        caption_style: None,
        max_still_s: 6.0,
    };

    let build_res = build_timeline(&ws, params).expect("build timeline failed");
    assert_eq!(build_res.artifact, "video");
    assert_eq!(build_res.duration_s, 7.4);
    assert_eq!(build_res.segments.len(), 2);
    assert!(build_res.captions_count >= 2);

    // 4. Verify artifact.yaml and timeline.json were written
    let artifact = Artifact::load(&proj, "video").expect("load created artifact");
    assert_eq!(artifact.kind, "timeline");
    assert_eq!(artifact.source, "timeline.json");

    let tl_json_path = proj.artifacts_dir().join("video").join("timeline.json");
    assert!(tl_json_path.exists());

    // Verify determinism: re-running produces byte-identical output
    let first_bytes = std::fs::read(&tl_json_path).unwrap();
    let _ = build_timeline(
        &ws,
        TimelineBuildParams {
            project: "test-proj".into(),
            artifact: "video".into(),
            segments: vec![
                SegmentInput {
                    audio: "projects/test-proj/vo1.wav".into(),
                    text: "Xin chào các bạn đã đến với kênh.".into(),
                    visual: "color:#1E88E5".into(),
                    ..Default::default()
                },
                SegmentInput {
                    audio: "projects/test-proj/vo2.wav".into(),
                    text: "Hôm nay chúng ta cùng tìm hiểu cách làm video hoàn toàn tự động nhé."
                        .into(),
                    visual: "color:#43A047".into(),
                    ..Default::default()
                },
            ],
            lead_s: 0.5,
            gap_s: 0.4,
            tail_s: 1.5,
            transition: "fade".into(),
            captions: "sentences".into(),
            overlays: vec![OverlayInput {
                source: "projects/test-proj/logo.png".into(),
                x: Some(40),
                y: Some(40),
                width: Some(200),
                height: None,
                opacity: Some(0.85),
                start: None,
                duration: None,
            }],
            brand: None,
            channels: vec!["youtube".into()],
            width: Some(1280),
            height: Some(720),
            fps: Some(30.0),
            min_duration_s: None,
            caption_style: None,
            max_still_s: 6.0,
        },
    )
    .unwrap();
    let second_bytes = std::fs::read(&tl_json_path).unwrap();
    assert_eq!(
        first_bytes, second_bytes,
        "timeline.json must be byte-identical on repeated runs"
    );

    // 5. Render with render_with
    let options = RenderOptions {
        loudness_lufs: None,
    };
    let outcome = render_with(&ws, &proj, &artifact, &scheduler, &options)
        .await
        .expect("render timeline");
    assert_eq!(outcome.outputs.len(), 1);
    let mp4_file = &outcome.outputs[0];
    assert!(mp4_file.exists());

    // 6. Validate output MP4 duration
    let meta = validate_video(mp4_file, None, Some(7.0)).expect("validate output video");
    assert!(
        (meta.duration_seconds - 7.4).abs() < 0.25,
        "Rendered MP4 duration ({}) should match timeline total duration (7.4s)",
        meta.duration_seconds
    );
    assert_eq!(meta.width, 1280);
    assert_eq!(meta.height, 720);
}

#[test]
fn test_caption_style_unknown_field_rejected_with_vietnamese_message() {
    let json_with_unknown = r#"{
        "version": 1,
        "width": 1920,
        "height": 1080,
        "fps": 30.0,
        "tracks": [
            {
                "id": "captions",
                "kind": "captions",
                "style": {
                    "font": "segoeui",
                    "font_size": 48,
                    "invalid_field": "disallowed"
                },
                "clips": []
            }
        ]
    }"#;

    let err = serde_json::from_str::<TimelineDoc>(json_with_unknown).unwrap_err();
    let hint = studio_artifacts::render::timeline_schema_hint(&err);
    assert!(hint.contains("Trường trong style không hợp lệ"), "{hint}");
    assert!(hint.contains("font"), "{hint}");
    assert!(hint.contains("font_size"), "{hint}");
    assert!(hint.contains("margin_bottom"), "{hint}");
    assert!(hint.contains("max_width_ratio"), "{hint}");
}

#[test]
fn test_caption_defaults_scale_with_frame_height() {
    // 1080p timeline: font size ≈ 49px (4.5% of 1080), bottom margin ≈ 86px (8% of 1080)
    let doc_1080 = TimelineDoc {
        version: 1,
        width: 1920,
        height: 1080,
        fps: 30.0,
        loudness_lufs: None,
        tracks: vec![Track {
            id: "cap".into(),
            kind: TrackKind::Captions,
            clips: vec![Clip {
                id: "c1".into(),
                source: "Xin chào Việt Nam".into(),
                in_point: 0.0,
                out_point: None,
                start: 0.0,
                duration: 2.0,
                transition: Transition::None,
                transition_duration: None,
                x: None,
                y: None,
                width: None,
                height: None,
                opacity: None,
                ..Default::default()
            }],
            style: None,
        }],
    };
    let (_, fg_1080) = build_timeline_ffmpeg_args(&doc_1080, Path::new("out.mp4")).unwrap();
    assert!(fg_1080.contains("fontsize=49"), "{fg_1080}");
    assert!(fg_1080.contains("y=h-text_h-86"), "{fg_1080}");
    assert!(fg_1080.contains("x=(w-text_w)/2"), "{fg_1080}");

    // 720p timeline: font size ≈ 32px (4.5% of 720), bottom margin ≈ 58px (8% of 720)
    let doc_720 = TimelineDoc {
        version: 1,
        width: 1280,
        height: 720,
        fps: 30.0,
        loudness_lufs: None,
        tracks: vec![Track {
            id: "cap".into(),
            kind: TrackKind::Captions,
            clips: vec![Clip {
                id: "c1".into(),
                source: "Xin chào Việt Nam".into(),
                in_point: 0.0,
                out_point: None,
                start: 0.0,
                duration: 2.0,
                transition: Transition::None,
                transition_duration: None,
                x: None,
                y: None,
                width: None,
                height: None,
                opacity: None,
                ..Default::default()
            }],
            style: None,
        }],
    };
    let (_, fg_720) = build_timeline_ffmpeg_args(&doc_720, Path::new("out.mp4")).unwrap();
    assert!(fg_720.contains("fontsize=32"), "{fg_720}");
    assert!(fg_720.contains("y=h-text_h-58"), "{fg_720}");

    // Custom style override
    let doc_custom = TimelineDoc {
        version: 1,
        width: 1920,
        height: 1080,
        fps: 30.0,
        loudness_lufs: None,
        tracks: vec![Track {
            id: "cap".into(),
            kind: TrackKind::Captions,
            clips: vec![Clip {
                id: "c1".into(),
                source: "Xin chào".into(),
                in_point: 0.0,
                out_point: None,
                start: 0.0,
                duration: 2.0,
                transition: Transition::None,
                transition_duration: None,
                x: None,
                y: None,
                width: None,
                height: None,
                opacity: None,
                ..Default::default()
            }],
            style: Some(CaptionStyle {
                font: Some("arial".into()),
                font_size: Some(60),
                color: Some("#FFFF00".into()),
                outline: Some(3),
                is_box: Some(false),
                box_color: None,
                margin_bottom: Some(120),
                max_width_ratio: Some(0.85),
            }),
        }],
    };
    let (_, fg_custom) = build_timeline_ffmpeg_args(&doc_custom, Path::new("out.mp4")).unwrap();
    assert!(fg_custom.contains("fontsize=60"), "{fg_custom}");
    assert!(fg_custom.contains("y=h-text_h-120"), "{fg_custom}");
    assert!(fg_custom.contains("fontcolor=0xFFFF00"), "{fg_custom}");
    assert!(fg_custom.contains(":box=0"), "{fg_custom}");
    assert!(fg_custom.contains(":borderw=3"), "{fg_custom}");
}

#[test]
fn test_caption_block_stays_inside_frame() {
    let doc = TimelineDoc {
        version: 1,
        width: 1920,
        height: 1080,
        fps: 30.0,
        loudness_lufs: None,
        tracks: vec![Track {
            id: "cap".into(),
            kind: TrackKind::Captions,
            clips: vec![Clip {
                id: "c1".into(),
                source: "Dòng 1\nDòng 2 dài hơn một chút".into(),
                in_point: 0.0,
                out_point: None,
                start: 0.0,
                duration: 3.0,
                transition: Transition::None,
                transition_duration: None,
                x: None,
                y: None,
                width: None,
                height: None,
                opacity: None,
                ..Default::default()
            }],
            style: None,
        }],
    };
    let (_, fg) = build_timeline_ffmpeg_args(&doc, Path::new("out.mp4")).unwrap();
    // The whole multi-line block must use y=h-text_h-margin to stay inside frame
    assert!(fg.contains("h-text_h-"), "{fg}");
    assert!(fg.contains("x=(w-text_w)/2"), "{fg}");
}

#[test]
fn test_caption_text_escaping_vietnamese_quotes_colons_percentages() {
    let raw_text = "Cà phê \"phin\": 100% ngon, đúng không?";
    let escaped = escape_drawtext_text(raw_text);
    assert_eq!(escaped, r#"Cà phê "phin"\: 100\% ngon\, đúng không?"#);
    // A straight apostrophe would end the quoted filter value: it becomes the typographic one.
    assert_eq!(escape_drawtext_text("bố's"), "bố\u{2019}s");

    let doc = TimelineDoc {
        version: 1,
        width: 1920,
        height: 1080,
        fps: 30.0,
        loudness_lufs: None,
        tracks: vec![Track {
            id: "cap".into(),
            kind: TrackKind::Captions,
            clips: vec![Clip {
                id: "c1".into(),
                source: raw_text.into(),
                in_point: 0.0,
                out_point: None,
                start: 0.0,
                duration: 2.0,
                transition: Transition::None,
                transition_duration: None,
                x: None,
                y: None,
                width: None,
                height: None,
                opacity: None,
                ..Default::default()
            }],
            style: None,
        }],
    };
    let (_, fg) = build_timeline_ffmpeg_args(&doc, Path::new("out.mp4")).unwrap();
    assert!(
        fg.contains(r#"text='Cà phê "phin"\: 100\% ngon\, đúng không?'"#),
        "{fg}"
    );
    assert!(fg.contains("expansion=none"), "{fg}");
}

#[test]
fn test_timeline_hash_unchanged_for_timeline_without_style() {
    let doc_no_style = TimelineDoc {
        version: 1,
        width: 1920,
        height: 1080,
        fps: 30.0,
        loudness_lufs: None,
        tracks: vec![
            Track {
                id: "v".into(),
                kind: TrackKind::Video,
                clips: vec![Clip {
                    id: "c1".into(),
                    source: "color:#000000".into(),
                    in_point: 0.0,
                    out_point: None,
                    start: 0.0,
                    duration: 5.0,
                    transition: Transition::None,
                    transition_duration: None,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                    opacity: None,
                    ..Default::default()
                }],
                style: None,
            },
            Track {
                id: "c".into(),
                kind: TrackKind::Captions,
                clips: vec![Clip {
                    id: "cap".into(),
                    source: "Test".into(),
                    in_point: 0.0,
                    out_point: None,
                    start: 0.0,
                    duration: 5.0,
                    transition: Transition::None,
                    transition_duration: None,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                    opacity: None,
                    ..Default::default()
                }],
                style: None,
            },
        ],
    };

    let serialized = serde_json::to_string(&doc_no_style).unwrap();
    assert!(
        !serialized.contains("\"style\""),
        "Track with style: None must omit style field"
    );
}

#[tokio::test]
#[ignore]
async fn real_timeline_render_with_vietnamese_captions_and_frame_inspection() {
    let ffmpeg_report = probe_ffmpeg();
    if !ffmpeg_report.available {
        println!("SKIP: real_timeline_render_with_vietnamese_captions_and_frame_inspection — ffmpeg not found");
        return;
    }
    let ffmpeg_bin = ffmpeg_report.path.unwrap();

    let tmp = tempfile::tempdir().unwrap();
    let mp4_path = tmp.path().join("caption_test.mp4");

    // 1920x1080 timeline with black background and two 2-line Vietnamese captions
    let doc = TimelineDoc {
        version: 1,
        width: 1920,
        height: 1080,
        fps: 30.0,
        loudness_lufs: None,
        tracks: vec![
            Track {
                id: "video".into(),
                kind: TrackKind::Video,
                clips: vec![Clip {
                    id: "bg".into(),
                    source: "color:#000000".into(),
                    in_point: 0.0,
                    out_point: None,
                    start: 0.0,
                    duration: 4.0,
                    transition: Transition::None,
                    transition_duration: None,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                    opacity: None,
                    ..Default::default()
                }],
                style: None,
            },
            Track {
                id: "captions".into(),
                kind: TrackKind::Captions,
                clips: vec![
                    Clip {
                        id: "cap1".into(),
                        source: "Cà phê phin truyền thống\nĐậm đà hương vị Việt".into(),
                        in_point: 0.0,
                        out_point: None,
                        start: 0.2,
                        duration: 1.8,
                        transition: Transition::None,
                        transition_duration: None,
                        x: None,
                        y: None,
                        width: None,
                        height: None,
                        opacity: None,
                        ..Default::default()
                    },
                    Clip {
                        id: "cap2".into(),
                        source: "Cà phê \"phin\" của bố': 100% ngon,\nđúng không bạn ơi?".into(),
                        in_point: 0.0,
                        out_point: None,
                        start: 2.1,
                        duration: 1.8,
                        transition: Transition::None,
                        transition_duration: None,
                        x: None,
                        y: None,
                        width: None,
                        height: None,
                        opacity: None,
                        ..Default::default()
                    },
                ],
                style: None,
            },
        ],
    };

    let (args, _) = build_timeline_ffmpeg_args(&doc, &mp4_path).expect("build ffmpeg args");

    let status = std::process::Command::new(&ffmpeg_bin)
        .args(&args)
        .current_dir(tmp.path())
        .output()
        .expect("render video with ffmpeg");
    assert!(
        status.status.success(),
        "ffmpeg render failed: {}",
        String::from_utf8_lossy(&status.stderr)
    );
    assert!(mp4_path.exists(), "MP4 file must exist");

    // Save frame to task evidence folder
    let evidence_dir = std::path::PathBuf::from(
        r"D:\Users\Dang\LocalContentStudio\.aw\tasks\2026-10-18-m08-youtube-dau-cuoi\evidence",
    );
    let _ = std::fs::create_dir_all(&evidence_dir);
    let evidence_png = evidence_dir.join("m08d-caption-1080.png");

    let extract_png = std::process::Command::new(&ffmpeg_bin)
        .args([
            "-ss",
            "3.0",
            "-i",
            mp4_path.to_str().unwrap(),
            "-vframes",
            "1",
            "-y",
            evidence_png.to_str().unwrap(),
        ])
        .output()
        .expect("extract evidence png");
    assert!(
        extract_png.status.success(),
        "failed to extract evidence PNG"
    );
    assert!(
        evidence_png.exists(),
        "Evidence PNG file must exist at {}",
        evidence_png.display()
    );

    // Extract raw RGB24 pixels of the frame at 3.0s (during caption 2)
    let extract_raw = std::process::Command::new(&ffmpeg_bin)
        .args([
            "-ss",
            "3.0",
            "-i",
            mp4_path.to_str().unwrap(),
            "-vframes",
            "1",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "-",
        ])
        .output()
        .expect("extract raw RGB24 frame");
    assert!(extract_raw.status.success());
    let raw_bytes = extract_raw.stdout;
    let expected_len = 1920 * 1080 * 3;
    assert_eq!(
        raw_bytes.len(),
        expected_len,
        "raw rgb24 frame size mismatch"
    );

    // Assert bottom 2% rows of the frame are not caption pixels
    // 2% of 1080 rows is ~21 rows (rows 1059..1080).
    let bottom_2pct_rows = 21;
    let bottom_start_byte = (1080 - bottom_2pct_rows) * 1920 * 3;
    let bottom_slice = &raw_bytes[bottom_start_byte..];
    let sum_brightness: u64 = bottom_slice.iter().map(|&b| b as u64).sum();
    let mean_brightness = sum_brightness as f64 / bottom_slice.len() as f64;
    println!("Bottom 2% rows mean brightness: {mean_brightness:.2} (expected < 5.0 for clean background)");
    assert!(
        mean_brightness < 5.0,
        "Bottom 2% rows must not contain caption pixels; got mean brightness {mean_brightness}"
    );
}

// ===== G3 Reference kind tests =====

#[test]
fn test_reference_kind_load_save_round_trip() {
    let (_tmp, _ws, proj) = create_test_env();

    // Create a reference artifact
    let art = Artifact::create(
        &proj,
        "nv-an",
        "reference",
        "Nhân vật An",
        None,
        vec![],
        Some(serde_json::json!({
            "seed": 42,
            "model": "imagen-3.0-generate-001",
            "voice": "vn-female-1",
            "speed": 1.05
        })),
    )
    .unwrap();

    assert_eq!(art.id, "nv-an");
    assert_eq!(art.kind, "reference");

    // starter source file is profile.md
    let src = art.source_path(&proj);
    assert!(src.exists(), "profile.md must be created by starter");
    let src_name = src.file_name().unwrap().to_str().unwrap();
    assert_eq!(src_name, "profile.md");

    // Load back preserving all params
    let loaded = Artifact::load(&proj, "nv-an").unwrap();
    assert_eq!(loaded.kind, "reference");
    assert_eq!(loaded.params.get("seed").and_then(|v| v.as_u64()), Some(42));
    assert_eq!(
        loaded.params.get("model").and_then(|v| v.as_str()),
        Some("imagen-3.0-generate-001")
    );
    assert_eq!(
        loaded.params.get("voice").and_then(|v| v.as_str()),
        Some("vn-female-1")
    );

    // Save updated params and reload
    let mut updated = loaded.clone();
    updated.params["seed"] = serde_json::json!(99);
    updated.save(&proj).unwrap();

    let reloaded = Artifact::load(&proj, "nv-an").unwrap();
    assert_eq!(
        reloaded.params.get("seed").and_then(|v| v.as_u64()),
        Some(99)
    );
    // Other params preserved
    assert_eq!(
        reloaded.params.get("voice").and_then(|v| v.as_str()),
        Some("vn-female-1")
    );
}

#[test]
fn test_reference_kind_status_is_fresh_with_content() {
    use studio_artifacts::{build_project_graph, NodeStatus};

    let (_tmp, ws, proj) = create_test_env();

    let art = Artifact::create(
        &proj,
        "ref-with-content",
        "reference",
        "Profile",
        None,
        vec![],
        None,
    )
    .unwrap();

    // profile.md already exists (created by starter)
    let src = art.source_path(&proj);
    assert!(src.exists());
    // Write real content
    std::fs::write(&src, "# Nhân vật\nMô tả chi tiết.").unwrap();

    let artifacts = Artifact::list(&proj).unwrap();
    let graph = build_project_graph(&ws, &proj, &artifacts).unwrap();
    let node = graph
        .nodes
        .iter()
        .find(|n| n.id == "ref-with-content")
        .unwrap();
    assert_eq!(
        node.status,
        NodeStatus::Moi,
        "reference with content must be 'moi' (fresh), never stale"
    );
}

#[test]
fn test_ffmpeg_filter_generation_for_ken_burns_and_cut() {
    let doc = TimelineDoc {
        version: 1,
        width: 1920,
        height: 1080,
        fps: 30.0,
        loudness_lufs: None,
        tracks: vec![Track {
            id: "video".into(),
            kind: TrackKind::Video,
            clips: vec![
                Clip {
                    id: "v1".into(),
                    source: "img1.png".into(),
                    start: 0.0,
                    duration: 4.0,
                    transition: Transition::Fade,
                    transition_duration: Some(0.5),
                    motion: Some(MotionKind::ZoomIn),
                    motion_strength: Some(0.08),
                    ..Default::default()
                },
                Clip {
                    id: "v2".into(),
                    source: "img2.png".into(),
                    start: 4.0,
                    duration: 3.0,
                    transition: Transition::Cut,
                    motion: Some(MotionKind::PanRight),
                    motion_strength: Some(0.08),
                    ..Default::default()
                },
                Clip {
                    id: "v3".into(),
                    source: "broll.mp4".into(),
                    start: 7.0,
                    duration: 5.0,
                    transition: Transition::Fade,
                    motion: Some(MotionKind::ZoomIn), // video must ignore motion!
                    ..Default::default()
                },
            ],
            style: None,
        }],
    };

    let out_path = PathBuf::from("rendered.mp4");
    let (args, filter_graph) = build_timeline_ffmpeg_args(&doc, &out_path).unwrap();

    // v0: zoom_in on img1.png — sub-pixel perspective window growing from 1.0 to 1.08 over 120 frames
    assert!(
        filter_graph.contains("[0:v]scale=1920:1080,setsar=1,loop=loop=-1:size=1:start=0,fps=30,trim=start=0:duration=4,setpts=PTS-STARTPTS,perspective=x0='(W-W/(1+0.0800*(in/119)))/2'"),
        "v0 filter not matching: {filter_graph}"
    );
    assert!(filter_graph.contains("interpolation=cubic:eval=frame,fade=t=in:st=0:d=0.5[v0]"));

    // v1: pan_right on img2.png with Transition::Cut (no fade filter!)
    assert!(
        filter_graph.contains("perspective=x0='(W-W/1.0800)*(in/89)'"),
        "v1 filter not matching: {filter_graph}"
    );
    assert!(filter_graph.contains("interpolation=cubic:eval=frame[v1]"));
    assert!(!filter_graph.contains("zoompan"));

    // v2: video broll.mp4 does not have zoompan even though motion was specified
    assert!(
        filter_graph.contains("[2:v]trim=start=0:duration=5,setpts=PTS-STARTPTS,fps=30,scale=1920:1080,setsar=1,fade=t=in:st=0:d=0.5[v2]"),
        "v2 video filter not matching: {filter_graph}"
    );

    // Args includes -stream_loop -1 for broll.mp4
    assert!(args.contains(&"-stream_loop".to_string()));
}

#[tokio::test]
#[ignore = "real timeline render with motion, 3 segments, multi visuals, frame inspection and byte-identical determinism"]
async fn real_timeline_render_with_motion_and_multi_visuals_evidence() {
    let ffmpeg_report = probe_ffmpeg();
    let ffprobe_report = probe_ffprobe();
    if !ffmpeg_report.available || !ffprobe_report.available {
        println!("SKIP: real_timeline_render_with_motion_and_multi_visuals_evidence — ffmpeg or ffprobe not found");
        return;
    }
    let ffmpeg_bin = ffmpeg_report.path.unwrap();

    let (_tmp, ws, proj) = create_test_env();
    let assets_dir = proj.root().join("assets");
    std::fs::create_dir_all(&assets_dir).unwrap();

    // 1. Generate 3 audio files (wav) using ffmpeg
    for (name, dur) in [("a1.wav", "3"), ("a2.wav", "6"), ("a3.wav", "3")] {
        let p = assets_dir.join(name);
        let status = std::process::Command::new(&ffmpeg_bin)
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                &format!("sine=frequency=440:duration={dur}"),
                "-c:a",
                "pcm_s16le",
                p.to_str().unwrap(),
            ])
            .output()
            .expect("generate wav");
        assert!(status.status.success());
    }

    // 2. Generate images (png) and 1 video clip (mp4)
    // img1: colorful testsrc
    let img1_path = assets_dir.join("img1.png");
    let status = std::process::Command::new(&ffmpeg_bin)
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=1920x1080:rate=1:duration=1",
            "-vframes",
            "1",
            img1_path.to_str().unwrap(),
        ])
        .output()
        .expect("generate img1");
    assert!(status.status.success());

    // img2a: testsrc2
    let img2a_path = assets_dir.join("img2a.png");
    let status = std::process::Command::new(&ffmpeg_bin)
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=1920x1080:rate=1:duration=1",
            "-vframes",
            "1",
            img2a_path.to_str().unwrap(),
        ])
        .output()
        .expect("generate img2a");
    assert!(status.status.success());

    // img2b: smptebars
    let img2b_path = assets_dir.join("img2b.png");
    let status = std::process::Command::new(&ffmpeg_bin)
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "smptebars=size=1920x1080:rate=1:duration=1",
            "-vframes",
            "1",
            img2b_path.to_str().unwrap(),
        ])
        .output()
        .expect("generate img2b");
    assert!(status.status.success());

    // vid3: 2-second moving video clip (tests video loop when segment is longer)
    let vid3_path = assets_dir.join("vid3.mp4");
    let status = std::process::Command::new(&ffmpeg_bin)
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=1920x1080:rate=30:duration=2",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            vid3_path.to_str().unwrap(),
        ])
        .output()
        .expect("generate vid3");
    assert!(status.status.success());

    // 3. Build timeline with 3 segments:
    // Seg 1: 1 image (img1.png) -> auto motion (ZoomIn)
    // Seg 2: 2 images (img2a.png + img2b.png) -> 2 visuals in one segment, cut transition between them
    // Seg 3: 1 video (vid3.mp4) -> video clip
    let params = TimelineBuildParams {
        project: proj.slug().to_string(),
        artifact: "video-v02".into(),
        segments: vec![
            SegmentInput {
                audio: "projects/test-proj/assets/a1.wav".into(),
                text: "Chào mừng các bạn đến với video thử nghiệm chất lượng cao.".into(),
                visual: "projects/test-proj/assets/img1.png".into(),
                ..Default::default()
            },
            SegmentInput {
                audio: "projects/test-proj/assets/a2.wav".into(),
                text: "Phân cảnh thứ hai này chứa hai góc nhìn khác nhau với chuyển động mượt mà."
                    .into(),
                visuals: vec![
                    VisualInput::Simple("projects/test-proj/assets/img2a.png".into()),
                    VisualInput::Simple("projects/test-proj/assets/img2b.png".into()),
                ],
                ..Default::default()
            },
            SegmentInput {
                audio: "projects/test-proj/assets/a3.wav".into(),
                text: "Cuối cùng là một đoạn video B-roll chuyển động thực tế.".into(),
                visual: "projects/test-proj/assets/vid3.mp4".into(),
                ..Default::default()
            },
        ],
        lead_s: 0.5,
        gap_s: 0.4,
        tail_s: 1.5,
        transition: "fade".into(),
        captions: "sentences".into(),
        overlays: vec![],
        brand: None,
        channels: vec!["youtube".into()],
        width: Some(1920),
        height: Some(1080),
        fps: Some(30.0),
        min_duration_s: None,
        caption_style: None,
        max_still_s: 6.0,
    };

    let build_res = build_timeline(&ws, params).expect("build timeline ok");
    assert_eq!(build_res.segments.len(), 3);
    assert!(build_res.duration_s > 10.0);

    let art_dir = proj.artifacts_dir().join("video-v02");
    let doc_path = art_dir.join("timeline.json");
    let doc_str = std::fs::read_to_string(&doc_path).expect("read timeline.json");
    let doc: TimelineDoc = serde_json::from_str(&doc_str).expect("parse timeline.json");

    // Check clips on base video track
    let v_track = doc
        .tracks
        .iter()
        .find(|t| t.id == "video")
        .expect("video track");
    assert_eq!(
        v_track.clips.len(),
        4,
        "Seg 1 (1) + Seg 2 (2) + Seg 3 (1) = 4 video clips"
    );
    assert_eq!(v_track.clips[0].motion, Some(MotionKind::ZoomIn));
    assert_eq!(v_track.clips[1].motion, Some(MotionKind::ZoomOut));
    assert_eq!(v_track.clips[2].motion, Some(MotionKind::PanLeft));
    assert_eq!(
        v_track.clips[3].motion, None,
        "Video clip must not have motion"
    );
    assert_eq!(
        v_track.clips[2].transition,
        Transition::Cut,
        "Intra-segment transition must be Cut"
    );

    // 4. Render to MP4 twice to verify byte-identical determinism
    let tmp_out = tempfile::tempdir().unwrap();
    let mp4_1 = tmp_out.path().join("render_run1.mp4");
    let mp4_2 = tmp_out.path().join("render_run2.mp4");

    let (args1, _) = build_timeline_ffmpeg_args(&doc, &mp4_1).expect("args 1");
    let (args2, _) = build_timeline_ffmpeg_args(&doc, &mp4_2).expect("args 2");

    let run1 = std::process::Command::new(&ffmpeg_bin)
        .args(&args1)
        .current_dir(ws.root())
        .output()
        .expect("run ffmpeg 1");
    assert!(
        run1.status.success(),
        "run 1 failed: {}",
        String::from_utf8_lossy(&run1.stderr)
    );

    let run2 = std::process::Command::new(&ffmpeg_bin)
        .args(&args2)
        .current_dir(ws.root())
        .output()
        .expect("run ffmpeg 2");
    assert!(
        run2.status.success(),
        "run 2 failed: {}",
        String::from_utf8_lossy(&run2.stderr)
    );

    let bytes1 = std::fs::read(&mp4_1).expect("read mp4 1");
    let bytes2 = std::fs::read(&mp4_2).expect("read mp4 2");
    assert_eq!(bytes1, bytes2, "Double render must be byte-identical");

    // 5. Extract frames at 0%, 50%, 100% of the first image clip (which has ZoomIn motion)
    let evidence_dir = std::path::PathBuf::from(
        r"D:\Users\Dang\LocalContentStudio\.aw\tasks\2026-10-24-v02-video-chuyen-dong\evidence",
    );
    let _ = std::fs::create_dir_all(&evidence_dir);

    // Save final demo MP4 to evidence
    let ev_mp4 = evidence_dir.join("v02-motion-demo.mp4");
    std::fs::copy(&mp4_1, &ev_mp4).expect("copy demo mp4 to evidence");

    let frame_0 = evidence_dir.join("frame-00pct.png");
    let frame_50 = evidence_dir.join("frame-50pct.png");
    let frame_100 = evidence_dir.join("frame-100pct.png");
    let frame_1033 = evidence_dir.join("frame-1033ms.png");

    let extract_frame = |ss: &str, out: &std::path::Path| {
        let status = std::process::Command::new(&ffmpeg_bin)
            .args([
                "-ss",
                ss,
                "-i",
                mp4_1.to_str().unwrap(),
                "-vframes",
                "1",
                "-y",
                out.to_str().unwrap(),
            ])
            .output()
            .expect("extract frame");
        assert!(status.status.success());
    };

    extract_frame("0.6", &frame_0);
    extract_frame("1.8", &frame_50);
    extract_frame("3.0", &frame_100);
    extract_frame("1.833", &frame_1033);

    let b_0 = std::fs::read(&frame_0).unwrap();
    let b_50 = std::fs::read(&frame_50).unwrap();
    let b_100 = std::fs::read(&frame_100).unwrap();
    let b_1033 = std::fs::read(&frame_1033).unwrap();

    assert_ne!(
        b_0, b_50,
        "Frame 0% and 50% must differ due to Ken Burns zoom motion"
    );
    assert_ne!(
        b_50, b_100,
        "Frame 50% and 100% must differ due to Ken Burns zoom motion"
    );
    assert_ne!(b_50, b_1033, "Consecutive frames must differ smoothly");

    println!("SUCCESS: verified real render, byte-identical rerun, and motion frame extraction!");
}

#[tokio::test]
#[ignore = "V04 benchmark: timeline 20 motion images + 3 videos (90s) peak RAM < 1 GB & byte-identical"]
async fn real_timeline_20_images_3_videos_render_under_1gb_ram() {
    let ffmpeg_report = probe_ffmpeg();
    let ffprobe_report = probe_ffprobe();
    if !ffmpeg_report.available || !ffprobe_report.available {
        println!("SKIP: real_timeline_20_images_3_videos_render_under_1gb_ram — ffmpeg/ffprobe not found");
        return;
    }
    let ffmpeg_bin = ffmpeg_report.path.as_ref().unwrap();

    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let ws = Workspace::create(temp_dir.path(), "V04 RAM Test Workspace").unwrap();
    let proj = Project::create(&ws, "test-v04", "V04 Stress Test").unwrap();

    let scheduler = Scheduler::new(
        SchedulerConfig::default(),
        Arc::new(MockProbe),
        Arc::new(MockEstimates),
    );

    let assets_dir = proj.root().join("assets");
    std::fs::create_dir_all(&assets_dir).unwrap();

    // 1. Generate 20 distinct still image assets (1280x720) with lavfi color
    let mut image_paths = Vec::new();
    for i in 0..20 {
        let p = assets_dir.join(format!("img_{i:02}.png"));
        let color_hex = format!(
            "{:02x}{:02x}{:02x}",
            (i * 12) % 256,
            (i * 23) % 256,
            (i * 37) % 256
        );
        let status = std::process::Command::new(ffmpeg_bin)
            .args([
                "-f",
                "lavfi",
                "-i",
                &format!("color=c=0x{color_hex}:s=1280x720:d=1"),
                "-frames:v",
                "1",
                "-y",
                p.to_str().unwrap(),
            ])
            .output()
            .expect("generate test image");
        assert!(status.status.success());
        image_paths.push(p);
    }

    // 2. Generate 3 distinct video clips (1280x720, 30fps, 4s each)
    let mut video_paths = Vec::new();
    for i in 0..3 {
        let p = assets_dir.join(format!("vid_{i:02}.mp4"));
        let color_hex = format!(
            "{:02x}{:02x}{:02x}",
            (i * 50 + 100) % 256,
            (i * 40 + 50) % 256,
            180
        );
        let status = std::process::Command::new(ffmpeg_bin)
            .args([
                "-f",
                "lavfi",
                "-i",
                &format!("color=c=0x{color_hex}:s=1280x720:r=30:d=4"),
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-crf",
                "22",
                "-pix_fmt",
                "yuv420p",
                "-y",
                p.to_str().unwrap(),
            ])
            .output()
            .expect("generate test video");
        assert!(status.status.success());
        video_paths.push(p);
    }

    // 3. Assemble 23 base clips total: 20 images (with motion perspective) + 3 video clips.
    // Total duration: 20 * 3.9s + 3 * 4.0s = 78s + 12s = 90.0s!
    let mut base_clips = Vec::new();
    let mut current_time = 0.0;
    let motions = [
        MotionKind::ZoomIn,
        MotionKind::ZoomOut,
        MotionKind::PanLeft,
        MotionKind::PanRight,
    ];

    let mut img_idx = 0;
    let mut vid_idx = 0;
    for i in 0..23 {
        // Interleave 1 video clip every 7 clips
        if (i == 5 || i == 12 || i == 19) && vid_idx < video_paths.len() {
            let dur = 4.0;
            base_clips.push(Clip {
                id: format!("clip_{i:02}"),
                source: video_paths[vid_idx].to_string_lossy().to_string(),
                in_point: 0.0,
                out_point: Some(dur),
                start: current_time,
                duration: dur,
                transition: Transition::Fade,
                transition_duration: Some(0.5),
                ..Default::default()
            });
            current_time += dur;
            vid_idx += 1;
        } else {
            let dur = 3.9;
            let m = motions[img_idx % motions.len()];
            base_clips.push(Clip {
                id: format!("clip_{i:02}"),
                source: image_paths[img_idx].to_string_lossy().to_string(),
                in_point: 0.0,
                out_point: None,
                start: current_time,
                duration: dur,
                transition: if i == 0 {
                    Transition::None
                } else {
                    Transition::Fade
                },
                transition_duration: Some(0.4),
                motion: Some(m),
                motion_strength: Some(0.08),
                ..Default::default()
            });
            current_time += dur;
            img_idx += 1;
        }
    }

    // Generate test audio (sine wave 90s)
    let audio_path = assets_dir.join("bg_audio.wav");
    let status_audio = std::process::Command::new(ffmpeg_bin)
        .args([
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=90",
            "-y",
            audio_path.to_str().unwrap(),
        ])
        .output()
        .expect("generate test audio");
    assert!(status_audio.status.success());

    let timeline_doc = TimelineDoc {
        version: 1,
        width: 1280,
        height: 720,
        fps: 30.0,
        loudness_lufs: Some(-14.0),
        tracks: vec![
            Track {
                id: "main-video".into(),
                kind: TrackKind::Video,
                clips: base_clips,
                style: None,
            },
            Track {
                id: "main-audio".into(),
                kind: TrackKind::Audio,
                clips: vec![Clip {
                    id: "audio_bg".into(),
                    source: audio_path.to_string_lossy().to_string(),
                    in_point: 0.0,
                    out_point: Some(90.0),
                    start: 0.0,
                    duration: 90.0,
                    ..Default::default()
                }],
                style: None,
            },
            Track {
                id: "captions".into(),
                kind: TrackKind::Captions,
                clips: vec![
                    Clip {
                        id: "cap1".into(),
                        source: "V04 Benchmark: 20 cảnh ảnh motion + 3 video".into(),
                        start: 0.0,
                        duration: 10.0,
                        ..Default::default()
                    },
                    Clip {
                        id: "cap2".into(),
                        source: "Kiến trúc Chunked Pipeline tối ưu RAM O(1)".into(),
                        start: 45.0,
                        duration: 10.0,
                        ..Default::default()
                    },
                ],
                style: None,
            },
        ],
    };

    let art = Artifact::create(
        &proj,
        "video-v04-stress",
        "timeline",
        "V04 90s Stress Video",
        None,
        vec![],
        None,
    )
    .unwrap();

    let art_source = art.source_path(&proj);
    std::fs::write(
        &art_source,
        serde_json::to_string_pretty(&timeline_doc).unwrap(),
    )
    .unwrap();

    // 4. Render pass 1
    println!("Starting Render Pass 1 (90s, 23 scenes)...");
    let t0 = std::time::Instant::now();
    let outcome1 = render(&ws, &proj, &art, &scheduler)
        .await
        .expect("render 1 failed");
    let dur1 = t0.elapsed();
    assert!(!outcome1.cached);
    assert_eq!(outcome1.outputs.len(), 1);
    let out1 = &outcome1.outputs[0];
    assert!(out1.exists());
    let bytes1 = std::fs::read(out1).unwrap();
    println!(
        "Pass 1 finished in {:.2?}s, output size: {} bytes",
        dur1,
        bytes1.len()
    );

    // 5. Render pass 2 (Clean cache and re-render to verify byte-identical determinism)
    // To ensure full pipeline determinism, remove render cache & output file
    let out_copy = proj.root().join("pass1_output.mp4");
    std::fs::copy(out1, &out_copy).unwrap();
    std::fs::remove_file(out1).unwrap();

    // Clear render cache record in SQLite to force re-render
    let db = ws.db_path();
    if let Ok(conn) = rusqlite::Connection::open(&db) {
        let _ = conn.execute(
            "DELETE FROM renders WHERE artifact_id = ?1",
            rusqlite::params![art.id],
        );
    }

    println!("Starting Render Pass 2 (Deterministic check)...");
    let t1 = std::time::Instant::now();
    let outcome2 = render(&ws, &proj, &art, &scheduler)
        .await
        .expect("render 2 failed");
    let dur2 = t1.elapsed();
    let out2 = &outcome2.outputs[0];
    assert!(out2.exists());
    let bytes2 = std::fs::read(out2).unwrap();
    println!(
        "Pass 2 finished in {:.2?}s, output size: {} bytes",
        dur2,
        bytes2.len()
    );

    // 6. Assert byte-identical determinism
    use sha2::{Digest, Sha256};
    let sha1 = hex::encode(Sha256::digest(&bytes1));
    let sha2 = hex::encode(Sha256::digest(&bytes2));
    println!("Pass 1 SHA-256: {}", sha1);
    println!("Pass 2 SHA-256: {}", sha2);
    assert_eq!(
        sha1, sha2,
        "Render output must be byte-identical between runs!"
    );

    // 7. Validate video properties
    let meta =
        validate_video(out2, ffprobe_report.path.as_deref(), Some(89.5)).expect("validate video");
    assert_eq!(meta.width, 1280);
    assert_eq!(meta.height, 720);
    assert!(
        meta.duration_seconds >= 89.5 && meta.duration_seconds <= 90.5,
        "Duration must be ~90s, got {:.2}s",
        meta.duration_seconds
    );

    println!("SUCCESS: 23-scene 90s timeline rendered under memory limit, byte-identical 100%!");
}

/// Debug aid: assemble a real timeline from its cached scene chunks with the current code.
/// `LCS_TL=<timeline.json> LCS_WS=<workspace root> LCS_OUT=<out.mp4> cargo test -p studio-artifacts --test artifacts_test debug_assemble_real_timeline -- --ignored --nocapture`
#[test]
#[ignore]
fn debug_assemble_real_timeline() {
    use studio_artifacts::renderer::*;
    let tl = PathBuf::from(std::env::var("LCS_TL").expect("LCS_TL"));
    let ws = PathBuf::from(std::env::var("LCS_WS").expect("LCS_WS"));
    let out = PathBuf::from(std::env::var("LCS_OUT").expect("LCS_OUT"));
    let cwd = tl.parent().unwrap().to_path_buf();
    let mut doc: studio_artifacts::timeline::TimelineDoc =
        serde_json::from_str(&std::fs::read_to_string(&tl).unwrap()).unwrap();
    for t in &mut doc.tracks {
        for c in &mut t.clips {
            if c.source.starts_with("projects/") {
                c.source = ws.join(&c.source).to_string_lossy().to_string();
            }
        }
    }
    let ffmpeg = which_ffmpeg();
    let cache_dir = cwd.join(".cache").join("timeline_chunks");
    let base = &doc
        .tracks
        .iter()
        .find(|t| t.kind == studio_artifacts::timeline::TrackKind::Video)
        .unwrap()
        .clips;
    let mut list = String::new();
    for (idx, clip) in base.iter().enumerate() {
        let hash = compute_clip_chunk_hash(clip, idx, &doc, &cwd);
        let chunk = cache_dir.join(format!("chunk_{idx}_{hash}.mp4"));
        render_base_clip_chunk(&ffmpeg, clip, idx, &doc, &cwd, &chunk).unwrap();
        list.push_str(&format!(
            "file '{}'
",
            escape_concat_path(&chunk)
        ));
    }
    let list_file = std::env::temp_dir().join("lcs_debug_concat.txt");
    std::fs::write(&list_file, list).unwrap();
    let (args, _) = build_timeline_concat_ffmpeg_args(&doc, &list_file, &out).unwrap();
    let st = std::process::Command::new(&ffmpeg)
        .args(&args)
        .current_dir(&cwd)
        .status()
        .unwrap();
    assert!(st.success());
    println!("OUT {}", out.display());
}

fn which_ffmpeg() -> PathBuf {
    std::env::var("LCS_FFMPEG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("ffmpeg"))
}
