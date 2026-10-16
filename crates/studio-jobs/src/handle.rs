//! Handle to an active or scheduled job yielding events and cancellation.

use crate::event::JobEvent;
use tokio::sync::mpsc;

/// Handle returned upon submitting a job to the scheduler.
pub struct JobHandle {
    pub(crate) id: String,
    pub(crate) rx: mpsc::Receiver<JobEvent>,
    pub(crate) cancel_tx: mpsc::Sender<()>,
}

impl JobHandle {
    /// Return the job ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Receive the next event from the job.
    ///
    /// Returns `None` once all events have completed and the channel is closed.
    pub async fn next_event(&mut self) -> Option<JobEvent> {
        self.rx.recv().await
    }

    /// Request explicit cancellation of the job.
    pub async fn cancel(&self) {
        let _ = self.cancel_tx.send(()).await;
    }
}
