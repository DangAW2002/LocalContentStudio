//! Warm worker pool keeping long-lived processes in memory with idle timeouts.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Cached warm entry metadata.
#[derive(Debug, Clone)]
pub struct WarmEntry {
    /// Tool ID.
    pub tool_id: String,
    /// Last used timestamp.
    pub last_used: Instant,
    /// Inactivity timeout.
    pub idle_timeout: Duration,
    /// Estimated or recorded RAM footprint in megabytes.
    pub estimated_ram_mb: u64,
}

/// Pool managing warm worker processes.
///
/// Keeps track of warm processes keyed by `tool_id`, supports idle cleanup
/// and LRU eviction when host memory is needed for admission of queued jobs.
pub struct WarmPool {
    entries: Arc<Mutex<HashMap<String, WarmEntry>>>,
}

impl WarmPool {
    /// Create a new empty warm pool.
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register or refresh a warm entry.
    pub async fn touch(&self, tool_id: &str, idle_timeout: Duration, estimated_ram_mb: u64) {
        let mut entries = self.entries.lock().await;
        entries.insert(
            tool_id.to_string(),
            WarmEntry {
                tool_id: tool_id.to_string(),
                last_used: Instant::now(),
                idle_timeout,
                estimated_ram_mb,
            },
        );
    }

    /// Check if a warm worker is currently registered and not expired.
    pub async fn is_warm(&self, tool_id: &str) -> bool {
        let mut entries = self.entries.lock().await;
        if let Some(entry) = entries.get(tool_id) {
            if entry.last_used.elapsed() <= entry.idle_timeout {
                return true;
            } else {
                entries.remove(tool_id);
            }
        }
        false
    }

    /// Remove expired warm entries whose idle timeout has elapsed.
    pub async fn reap_idle(&self) -> Vec<String> {
        let mut entries = self.entries.lock().await;
        let mut reaped = Vec::new();
        entries.retain(|tool_id, entry| {
            if entry.last_used.elapsed() > entry.idle_timeout {
                reaped.push(tool_id.clone());
                false
            } else {
                true
            }
        });
        reaped
    }

    /// Evict least-recently-used (LRU) warm entries until `needed_mb` of RAM is reclaimed.
    /// Returns the list of evicted `tool_id`s.
    pub async fn evict_lru(&self, needed_mb: u64) -> Vec<String> {
        let mut entries = self.entries.lock().await;
        let mut sorted: Vec<(String, Instant, u64)> = entries
            .iter()
            .map(|(k, v)| (k.clone(), v.last_used, v.estimated_ram_mb))
            .collect();

        // Sort ascending by last_used (oldest first)
        sorted.sort_by_key(|(_, last_used, _)| *last_used);

        let mut evicted = Vec::new();
        let mut reclaimed_mb = 0u64;

        for (id, _, ram) in sorted {
            entries.remove(&id);
            evicted.push(id);
            reclaimed_mb += ram;
            if reclaimed_mb >= needed_mb {
                break;
            }
        }

        evicted
    }

    /// Total RAM currently occupied by warm entries in megabytes.
    pub async fn total_warm_ram_mb(&self) -> u64 {
        let entries = self.entries.lock().await;
        entries.values().map(|e| e.estimated_ram_mb).sum()
    }
}

impl Default for WarmPool {
    fn default() -> Self {
        Self::new()
    }
}
