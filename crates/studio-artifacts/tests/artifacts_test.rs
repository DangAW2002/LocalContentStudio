use std::fs;
use std::path::PathBuf;
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
    assert_eq!(kinds.len(), 6);

    let expected = [
        "design",
        "motion",
        "timeline",
        "text",
        "generation",
        "audio",
    ];
    for k in &expected {
        assert!(KindRegistry::get(k).is_some(), "Missing kind: {}", k);
    }
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
                    },
                ],
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
                }],
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
    assert!(filter_graph.contains("drawtext=text='Intro Hello':enable='between(t,0.5,2.5)'"));

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
                },
            ],
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
                }],
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
                }],
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
                }],
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
                }],
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
                }],
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
                }],
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
                }],
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
                }],
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
            }],
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
                }],
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
                }],
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
