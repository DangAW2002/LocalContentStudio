//! Server-Sent Events (SSE) broadcaster for studio events, with replay on reconnect.
//!
//! Modeled on open-design's run event journal (https://github.com/nexu-io/open-design,
//! apps/daemon/src/runtimes/runs.ts — Apache-2.0): every event gets a monotonically increasing
//! id and is kept in a bounded ring. A reconnecting `EventSource` sends `Last-Event-ID`; the
//! stream first replays what the client missed, then continues live. Taking the replay snapshot
//! and subscribing to live events happen under one lock, so nothing is duplicated or lost in
//! between. When the gap can no longer be replayed (ring overflowed, server restarted, slow
//! client lagged) a `studio-resync` event tells the UI to reload from the database instead of
//! silently missing events.

use std::collections::VecDeque;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::broadcast;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

/// SSE event name telling the client its view may be stale and must be reloaded.
pub const RESYNC_EVENT: &str = "studio-resync";

/// Events kept for replay. A turn streams many text deltas, so size for minutes of output.
pub const JOURNAL_CAPACITY: usize = 2048;

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

/// One journaled event.
#[derive(Debug)]
pub struct JournalEntry {
    pub id: u64,
    pub event: WebEvent,
}

struct JournalInner {
    next_id: u64,
    ring: VecDeque<Arc<JournalEntry>>,
}

/// Bounded, id-ordered event journal with a live fan-out.
pub struct EventJournal {
    inner: Mutex<JournalInner>,
    capacity: usize,
    live: broadcast::Sender<Arc<JournalEntry>>,
}

/// What a (re)connecting client receives before going live.
pub struct Subscription {
    /// Missed events, oldest first.
    pub replay: Vec<Arc<JournalEntry>>,
    /// The requested cursor could not be fully replayed; the client must resync.
    pub gap: bool,
    pub live: broadcast::Receiver<Arc<JournalEntry>>,
}

impl EventJournal {
    pub fn new(capacity: usize) -> Arc<Self> {
        let (live, _) = broadcast::channel(capacity.max(16));
        Arc::new(Self {
            inner: Mutex::new(JournalInner {
                next_id: 1,
                ring: VecDeque::with_capacity(capacity),
            }),
            capacity,
            live,
        })
    }

    /// Journal fed from the app's event channel by a background task (needs a Tokio runtime).
    pub fn attach(events_tx: &broadcast::Sender<WebEvent>) -> Arc<Self> {
        let journal = Self::new(JOURNAL_CAPACITY);
        let mut rx = events_tx.subscribe();
        let weak = Arc::downgrade(&journal);
        tokio::spawn(async move {
            loop {
                let item = rx.recv().await;
                let Some(journal) = weak.upgrade() else { break };
                match item {
                    Ok(ev) => journal.publish(ev),
                    // The forwarder fell behind: record the hole so clients resync.
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        journal.publish(WebEvent::new(RESYNC_EVENT, Value::Null))
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        journal
    }

    /// Append an event and fan it out. Published under the lock so ids reach subscribers in order.
    pub fn publish(&self, event: WebEvent) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        let entry = Arc::new(JournalEntry {
            id: inner.next_id,
            event,
        });
        inner.next_id += 1;
        if inner.ring.len() == self.capacity {
            inner.ring.pop_front();
        }
        inner.ring.push_back(entry.clone());
        let _ = self.live.send(entry);
    }

    /// Snapshot the events after `after` and subscribe to live ones atomically.
    pub fn subscribe_from(&self, after: Option<u64>) -> Subscription {
        let inner = match self.inner.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let live = self.live.subscribe();
        let Some(after) = after else {
            return Subscription {
                replay: Vec::new(),
                gap: false,
                live,
            };
        };
        let last_id = inner.next_id - 1;
        let oldest = inner.ring.front().map(|e| e.id).unwrap_or(inner.next_id);
        // A cursor ahead of us belongs to a previous server process; one older than the ring
        // start means events were dropped.
        let gap = after > last_id || after + 1 < oldest;
        let replay = if gap {
            Vec::new()
        } else {
            inner
                .ring
                .iter()
                .filter(|e| e.id > after)
                .cloned()
                .collect()
        };
        Subscription { replay, gap, live }
    }
}

fn to_sse(entry: &JournalEntry) -> Event {
    let data = serde_json::to_string(&entry.event.payload).unwrap_or_default();
    Event::default()
        .id(entry.id.to_string())
        .event(&entry.event.event)
        .data(data)
}

fn resync_event() -> Event {
    Event::default().event(RESYNC_EVENT).data("{}")
}

/// Parse an SSE `Last-Event-ID` header value.
pub fn parse_last_event_id(value: Option<&str>) -> Option<u64> {
    value.and_then(|v| v.trim().parse().ok())
}

/// SSE stream for one client: replay after `last_event_id`, then live events.
pub fn create_event_stream(
    journal: &EventJournal,
    last_event_id: Option<u64>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let Subscription { replay, gap, live } = journal.subscribe_from(last_event_id);
    let mut last_sent = replay.last().map(|e| e.id).or(last_event_id).unwrap_or(0);
    if gap {
        last_sent = 0;
    }

    let head: Vec<Result<Event, Infallible>> = gap
        .then(resync_event)
        .into_iter()
        .chain(replay.iter().map(|e| to_sse(e)))
        .map(Ok)
        .collect();

    let tail = BroadcastStream::new(live).filter_map(move |item| match item {
        Ok(entry) => {
            // Defensive: never resend something already replayed.
            if entry.id <= last_sent {
                return None;
            }
            last_sent = entry.id;
            if entry.event.event == RESYNC_EVENT {
                Some(Ok(resync_event()))
            } else {
                Some(Ok(to_sse(&entry)))
            }
        }
        // This client lagged behind the live channel: it missed events, so it must resync.
        Err(BroadcastStreamRecvError::Lagged(_)) => Some(Ok(resync_event())),
    });

    Sse::new(stream::iter(head).chain(tail)).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(n: u64) -> WebEvent {
        WebEvent::new("turn-event", serde_json::json!({ "n": n }))
    }

    #[test]
    fn fresh_connection_gets_no_replay() {
        let j = EventJournal::new(8);
        j.publish(ev(1));
        let s = j.subscribe_from(None);
        assert!(s.replay.is_empty());
        assert!(!s.gap);
    }

    #[test]
    fn reconnect_replays_only_missed_events() {
        let j = EventJournal::new(8);
        for n in 1..=5 {
            j.publish(ev(n));
        }
        let s = j.subscribe_from(Some(3));
        assert_eq!(
            s.replay.iter().map(|e| e.id).collect::<Vec<_>>(),
            vec![4, 5]
        );
        assert!(!s.gap);
        // Up to date: nothing to replay, no gap.
        let s = j.subscribe_from(Some(5));
        assert!(s.replay.is_empty());
        assert!(!s.gap);
    }

    #[test]
    fn overflowed_ring_reports_gap() {
        let j = EventJournal::new(3);
        for n in 1..=6 {
            j.publish(ev(n));
        }
        // Ring holds 4..=6; resuming after 2 would skip event 3.
        let s = j.subscribe_from(Some(2));
        assert!(s.gap);
        assert!(s.replay.is_empty());
        // Resuming right before the ring start is still complete.
        let s = j.subscribe_from(Some(3));
        assert!(!s.gap);
        assert_eq!(s.replay.len(), 3);
    }

    #[test]
    fn cursor_from_previous_process_reports_gap() {
        let j = EventJournal::new(8);
        j.publish(ev(1));
        assert!(j.subscribe_from(Some(500)).gap);
    }

    #[test]
    fn live_events_after_subscribe_are_not_in_replay() {
        let j = EventJournal::new(8);
        j.publish(ev(1));
        let mut s = j.subscribe_from(Some(0));
        assert_eq!(s.replay.len(), 1);
        j.publish(ev(2));
        let live = s.live.try_recv().expect("live event");
        assert_eq!(live.id, 2);
    }

    #[test]
    fn parses_last_event_id() {
        assert_eq!(parse_last_event_id(Some(" 42 ")), Some(42));
        assert_eq!(parse_last_event_id(Some("x")), None);
        assert_eq!(parse_last_event_id(None), None);
    }
}
