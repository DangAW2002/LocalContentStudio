//! Tests for builtin media.inspect@1 capability.
#![cfg(windows)]

use std::sync::Arc;
use studio_jobs::{EstimateStore, MemoryEstimateStore, Scheduler, SchedulerConfig, SysinfoProbe};
use studio_tools::runner::{run_tool, ToolRun, ToolRunContext};
use studio_tools::secret::MemoryStore;
use studio_tools::Registry;
use studio_workspace::Workspace;
use tempfile::tempdir;

fn test_scheduler() -> Arc<Scheduler> {
    let probe = Arc::new(SysinfoProbe::new());
    let estimates: Arc<dyn EstimateStore> = Arc::new(MemoryEstimateStore::new());
    Scheduler::new(SchedulerConfig::default(), probe, estimates)
}

fn has_ffmpeg_and_ffprobe() -> bool {
    let ff = std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    let fp = std::process::Command::new("ffprobe")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    ff && fp
}

#[tokio::test]
async fn test_media_inspect_on_generated_clip() {
    if !has_ffmpeg_and_ffprobe() {
        eprintln!("ffmpeg/ffprobe missing; skipping test_media_inspect_on_generated_clip");
        return;
    }

    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("ws"), "ws").unwrap();
    let project_dir = ws.root().join("projects").join("demo-inspect");
    let assets_dir = project_dir.join("assets");
    std::fs::create_dir_all(&assets_dir).unwrap();

    let clip_path = assets_dir.join("sample.mp4");

    // Generate a 2-second 320x240 video clip with 440Hz sine audio via ffmpeg
    let status = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=2:size=320x240:rate=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=2",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            clip_path.to_str().unwrap(),
        ])
        .output()
        .expect("ffmpeg clip creation failed");
    assert!(status.status.success(), "Failed to create test mp4 clip");

    let reg = Registry::load(&ws);
    let scheduler = test_scheduler();
    let secret_store = MemoryStore::new();
    let ctx = ToolRunContext {
        workspace: &ws,
        turn_id: Some("turn-inspect".into()),
        budget: None,
        approved: false,
        scheduler: &scheduler,
        secret_store: &secret_store,
        endpoints: std::collections::HashMap::new(),
        pricing: std::collections::HashMap::new(),
    };

    let run_res = run_tool(
        &reg,
        "media.inspect",
        serde_json::json!({ "path": "projects/demo-inspect/assets/sample.mp4" }),
        &ctx,
    )
    .await
    .expect("media.inspect should run successfully");

    match run_res {
        ToolRun::Success { output, .. } => {
            let dur = output["duration_seconds"]
                .as_f64()
                .expect("duration_seconds");
            assert!(
                (dur - 2.0).abs() < 0.25,
                "duration should be ~2.0s, got {dur}"
            );

            assert_eq!(output["width"], 320);
            assert_eq!(output["height"], 240);
            let fps = output["fps"].as_f64().expect("fps");
            assert!((fps - 30.0).abs() < 1.0, "fps should be ~30, got {fps}");

            assert_eq!(output["has_audio"], true);

            let lufs = output["loudness_lufs"].as_f64();
            assert!(
                lufs.is_some(),
                "loudness_lufs should be present for clip with audio"
            );

            let frames = output["sample_frames"].as_array().expect("sample_frames");
            assert_eq!(frames.len(), 3, "should extract 3 sample frames");

            for f_val in frames {
                let rel = f_val.as_str().expect("frame path str");
                assert!(rel.starts_with("projects/demo-inspect/.cache/media_inspect/"));
                let full = ws.root().join(rel);
                assert!(
                    full.is_file(),
                    "Extracted frame file {rel} must exist on disk"
                );
            }
        }
        other => panic!("Unexpected outcome: {other:?}"),
    }

    // Rejection of invalid/escaping paths
    for bad in [
        "../outside.mp4",
        "C:/Windows/win.ini",
        "projects/demo-inspect/assets/missing.mp4",
    ] {
        let res = run_tool(
            &reg,
            "media.inspect",
            serde_json::json!({ "path": bad }),
            &ctx,
        )
        .await;
        assert!(
            !matches!(res, Ok(ToolRun::Success { .. })),
            "{bad} must not succeed"
        );
    }
}

#[tokio::test]
async fn test_media_inspect_audio_only() {
    if !has_ffmpeg_and_ffprobe() {
        return;
    }

    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("ws"), "ws").unwrap();
    let wav_path = ws.root().join("tone.wav");

    let status = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=1000:duration=1",
            wav_path.to_str().unwrap(),
        ])
        .output()
        .expect("ffmpeg wav creation failed");
    assert!(status.status.success());

    let reg = Registry::load(&ws);
    let scheduler = test_scheduler();
    let secret_store = MemoryStore::new();
    let ctx = ToolRunContext {
        workspace: &ws,
        turn_id: None,
        budget: None,
        approved: false,
        scheduler: &scheduler,
        secret_store: &secret_store,
        endpoints: std::collections::HashMap::new(),
        pricing: std::collections::HashMap::new(),
    };

    let run_res = run_tool(
        &reg,
        "media.inspect",
        serde_json::json!({ "path": "tone.wav" }),
        &ctx,
    )
    .await
    .expect("media.inspect should run successfully on wav");

    if let ToolRun::Success { output, .. } = run_res {
        assert_eq!(output["has_audio"], true);
        assert_eq!(output["width"], serde_json::Value::Null);
        assert_eq!(output["height"], serde_json::Value::Null);
        assert_eq!(output["fps"], serde_json::Value::Null);
        assert_eq!(output["sample_frames"], serde_json::json!([]));
        assert!(output["loudness_lufs"].as_f64().is_some());
    } else {
        panic!("Expected Success");
    }
}
