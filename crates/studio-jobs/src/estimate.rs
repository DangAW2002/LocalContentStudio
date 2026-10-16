//! Exponentially Weighted Moving Average (EWMA) store for tool resource estimates.

use std::collections::HashMap;
use std::sync::RwLock;

/// Trait for storing and updating resource estimates per tool.
pub trait EstimateStore: Send + Sync {
    /// Retrieve the current estimated peak RAM consumption in megabytes for a tool.
    fn get_peak_ram_mb(&self, tool_id: &str) -> Option<u64>;

    /// Update the estimated peak RAM consumption using the latest observed peak.
    fn update_peak_ram_mb(&self, tool_id: &str, observed_peak_mb: u64);
}

/// In-memory implementation of `EstimateStore` using EWMA.
///
/// New estimate = alpha * observed + (1 - alpha) * old_estimate.
/// Alpha defaults to 0.3.
pub struct MemoryEstimateStore {
    estimates: RwLock<HashMap<String, f64>>,
    alpha: f64,
}

impl MemoryEstimateStore {
    /// Create a new in-memory estimate store with default alpha = 0.3.
    pub fn new() -> Self {
        Self::with_alpha(0.3)
    }

    /// Create a new in-memory estimate store with a custom smoothing factor alpha in (0.0, 1.0].
    pub fn with_alpha(alpha: f64) -> Self {
        Self {
            estimates: RwLock::new(HashMap::new()),
            alpha: alpha.clamp(0.01, 1.0),
        }
    }
}

impl Default for MemoryEstimateStore {
    fn default() -> Self {
        Self::new()
    }
}

impl EstimateStore for MemoryEstimateStore {
    fn get_peak_ram_mb(&self, tool_id: &str) -> Option<u64> {
        let lock = self.estimates.read().ok()?;
        lock.get(tool_id).map(|val| val.round() as u64)
    }

    fn update_peak_ram_mb(&self, tool_id: &str, observed_peak_mb: u64) {
        if let Ok(mut lock) = self.estimates.write() {
            let entry = lock
                .entry(tool_id.to_string())
                .or_insert(observed_peak_mb as f64);
            *entry = self.alpha * (observed_peak_mb as f64) + (1.0 - self.alpha) * (*entry);
        }
    }
}
