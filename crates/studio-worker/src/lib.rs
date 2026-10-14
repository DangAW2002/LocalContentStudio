pub mod ffmpeg;

pub use ffmpeg::{FFmpegProbeResult, FFmpegRenderWorker};

pub struct SupervisedWorker {
    pub worker_id: String,
}

impl SupervisedWorker {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            worker_id: id.into(),
        }
    }

    pub async fn execute_probe(&self) -> bool {
        true
    }
}
