use studio_core::media::{RenderJob, RenderJobState};
use studio_worker::FFmpegRenderWorker;
use tempfile::tempdir;

#[test]
fn test_ffmpeg_probe_and_render_execution() {
    // 1. Probe FFmpeg and codecs
    let probe = FFmpegRenderWorker::probe_ffmpeg();
    assert!(probe.has_ffmpeg);
    assert!(probe.supports_libx264);
    assert!(probe.supports_aac);
    assert!(probe.version.contains("ffmpeg"));

    // 2. Execute render job
    let temp = tempdir().unwrap();
    let worker = FFmpegRenderWorker::new("worker-media-1");

    let mut job = RenderJob::new(
        "ws-video",
        "proj-motion-1",
        "rev-video-1",
        "hash-input-abc-123",
    );
    assert_eq!(job.state, RenderJobState::Pending);

    let output_path = worker.execute_render_job(&mut job, temp.path()).unwrap();
    assert!(output_path.exists());
    assert_eq!(job.state, RenderJobState::Ready);
    assert_eq!(job.progress_percent, 100);
    assert!(job.output_hash.is_some());
    assert_eq!(job.output_hash.as_ref().unwrap().len(), 64);
    assert!(job.output_asset_ref.is_some());
}
