//! Resource requirements and scheduler pool limits.

use serde::{Deserialize, Serialize};

/// CPU classification for admission and concurrency pools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CpuClass {
    /// Light CPU task (e.g. quick probe, token check, metadata extraction).
    Light,
    /// Heavy CPU task (e.g. software encoding, compilation, heavy parsing).
    Heavy,
}

/// Resource requirements declared for a job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceSpec {
    /// Declared initial RAM requirement in megabytes.
    pub ram_mb: u64,
    /// CPU class (Light or Heavy).
    pub cpu: CpuClass,
    /// GPU VRAM requirement in megabytes.
    pub gpu_vram_mb: u64,
    /// Exclusive resource names that cannot run concurrently with other jobs requesting the same resource.
    /// E.g. `vec!["gpu".to_string()]` or specific hardware adapters.
    pub exclusive: Vec<String>,
    /// Optional rate key for network / provider throttling (e.g. `"provider:elevenlabs"`).
    pub rate_key: Option<String>,
}

impl Default for ResourceSpec {
    fn default() -> Self {
        Self {
            ram_mb: 256,
            cpu: CpuClass::Light,
            gpu_vram_mb: 0,
            exclusive: Vec::new(),
            rate_key: None,
        }
    }
}

/// Global scheduler configuration and pool capacities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// Maximum concurrent engine turns (default 2).
    pub engine_pool: usize,
    /// Maximum concurrent GPU-heavy jobs (default 1).
    pub gpu_pool: usize,
    /// Maximum concurrent CPU-heavy jobs (default max(1, cores / 4)).
    pub cpu_heavy_pool: usize,
    /// Maximum concurrent light jobs (default 8).
    pub light_pool: usize,
    /// Maximum concurrent requests per specific `rate_key` (default 4).
    pub per_rate_key_limit: usize,
    /// RAM headroom kept free on the host in megabytes (default 2048 MB).
    pub ram_headroom_mb: u64,
    /// VRAM headroom kept free on the host in megabytes (default 0 MB).
    pub vram_headroom_mb: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        let cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        let cpu_heavy = (cores / 4).max(1);

        Self {
            engine_pool: 2,
            gpu_pool: 1,
            cpu_heavy_pool: cpu_heavy,
            light_pool: 8,
            per_rate_key_limit: 4,
            ram_headroom_mb: 2048,
            vram_headroom_mb: 0,
        }
    }
}
