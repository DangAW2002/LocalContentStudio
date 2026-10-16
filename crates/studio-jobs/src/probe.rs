//! System probe traits and implementations for measuring host resources.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use sysinfo::System;

/// Trait to probe available system resources (RAM, VRAM, CPU cores).
pub trait SystemProbe: Send + Sync {
    /// Available free RAM on the host in megabytes.
    fn free_ram_mb(&self) -> u64;

    /// Available free GPU VRAM on the host in megabytes.
    fn free_vram_mb(&self) -> u64;

    /// Number of logical CPU cores.
    fn cores(&self) -> usize;
}

/// Real system probe using the `sysinfo` crate.
pub struct SysinfoProbe {
    system: std::sync::Mutex<System>,
    cores: usize,
}

impl SysinfoProbe {
    /// Creates a new `SysinfoProbe`.
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_memory();
        let cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self {
            system: std::sync::Mutex::new(sys),
            cores,
        }
    }
}

impl Default for SysinfoProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemProbe for SysinfoProbe {
    fn free_ram_mb(&self) -> u64 {
        if let Ok(mut sys) = self.system.lock() {
            sys.refresh_memory();
            sys.available_memory() / (1024 * 1024)
        } else {
            1024
        }
    }

    fn free_vram_mb(&self) -> u64 {
        // Fallback for systems without direct NVML / DXGI querying:
        // Assume large VRAM or equal to RAM for admission unless specialized GPU probe is attached.
        8192
    }

    fn cores(&self) -> usize {
        self.cores
    }
}

/// Controllable fake system probe for deterministic testing.
#[derive(Debug, Clone)]
pub struct FakeProbe {
    free_ram: Arc<AtomicU64>,
    free_vram: Arc<AtomicU64>,
    cores: Arc<AtomicUsize>,
}

impl FakeProbe {
    /// Create a new `FakeProbe` with the specified initial values.
    pub fn new(ram_mb: u64, vram_mb: u64, cores: usize) -> Self {
        Self {
            free_ram: Arc::new(AtomicU64::new(ram_mb)),
            free_vram: Arc::new(AtomicU64::new(vram_mb)),
            cores: Arc::new(AtomicUsize::new(cores)),
        }
    }

    /// Set the free RAM in megabytes.
    pub fn set_free_ram_mb(&self, ram_mb: u64) {
        self.free_ram.store(ram_mb, Ordering::SeqCst);
    }

    /// Set the free VRAM in megabytes.
    pub fn set_free_vram_mb(&self, vram_mb: u64) {
        self.free_vram.store(vram_mb, Ordering::SeqCst);
    }

    /// Set the number of logical cores.
    pub fn set_cores(&self, cores: usize) {
        self.cores.store(cores, Ordering::SeqCst);
    }
}

impl SystemProbe for FakeProbe {
    fn free_ram_mb(&self) -> u64 {
        self.free_ram.load(Ordering::SeqCst)
    }

    fn free_vram_mb(&self) -> u64 {
        self.free_vram.load(Ordering::SeqCst)
    }

    fn cores(&self) -> usize {
        self.cores.load(Ordering::SeqCst)
    }
}
