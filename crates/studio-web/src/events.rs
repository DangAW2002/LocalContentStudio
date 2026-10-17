//! Server-Sent Events (SSE) broadcaster for studio events.

use std::convert::Infallible;
use std::time::Duration;

use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::stream::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

/// Normalized event broadcast to web clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebEvent {
    pub event: String,
    pub payload: Value,
}

impl WebEvent {
    pub fn new(event: impl Into<String>, payload: Value) -> Self {
        Self {
            event: event.into(),
            payload,
        }
    }
}

/// Create an SSE stream from a broadcast receiver.
pub fn create_event_stream(
    rx: broadcast::Receiver<WebEvent>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let broadcast_stream = BroadcastStream::new(rx);

    let stream = broadcast_stream.filter_map(|item| match item {
        Ok(web_event) => {
            let json_str = serde_json::to_string(&web_event.payload).unwrap_or_default();
            Some(Ok(Event::default().event(web_event.event).data(json_str)))
        }
        // Dropped / lagged events are skipped; slow clients continue receiving latest events
        Err(_) => None,
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    )
}
