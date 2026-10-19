//! The builtin `ffmpeg.probe` tool reads `asset` from the JSON input (it used to pass no file to
//! ffprobe and always failed). Needs ffmpeg/ffprobe on PATH; skipped otherwise.
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

fn has_ffmpeg() -> bool {
    std::process::Command::new("ffprobe")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tokio::test]
async fn ffmpeg_probe_reads_asset_from_input_and_rejects_escapes() {
    if !has_ffmpeg() {
        eprintln!("ffprobe not on PATH; skipping");
        return;
    }
    let tmp = tempdir().unwrap();
    let ws = Workspace::create(tmp.path().join("ws"), "ws").unwrap();
    let wav = ws.root().join("assets").join("tone.wav");
    std::fs::create_dir_all(wav.parent().unwrap()).unwrap();
    let made = std::process::Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
        ])
        .arg(&wav)
        .status()
        .unwrap();
    assert!(made.success());

    let reg = Registry::load(&ws);
    let scheduler = test_scheduler();
    let secret_store = MemoryStore::new();
    let ctx = ToolRunContext {
        workspace: &ws,
        turn_id: Some("turn-probe".into()),
        budget: None,
        approved: false,
        scheduler: &scheduler,
        secret_store: &secret_store,
        endpoints: std::collections::HashMap::new(),
        pricing: std::collections::HashMap::new(),
    };

    let ok = run_tool(
        &reg,
        "ffmpeg.probe",
        serde_json::json!({ "asset": "assets/tone.wav" }),
        &ctx,
    )
    .await
    .expect("probe should run");
    match ok {
        ToolRun::Success { output, .. } => {
            let duration: f64 = output["format"]["duration"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap();
            assert!((duration - 1.0).abs() < 0.1, "duration {duration}");
            assert_eq!(output["streams"][0]["codec_type"], "audio");
        }
        other => panic!("unexpected {other:?}"),
    }

    for bad in ["../outside.wav", "C:/Windows/win.ini", "assets/missing.wav"] {
        let res = run_tool(
            &reg,
            "ffmpeg.probe",
            serde_json::json!({ "asset": bad }),
            &ctx,
        )
        .await;
        assert!(
            !matches!(res, Ok(ToolRun::Success { .. })),
            "{bad} must not succeed"
        );
    }
}
