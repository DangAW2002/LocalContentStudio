use crate::error::Result;
use crate::workspace::Workspace;
use serde::{Deserialize, Serialize};
use std::fs;

use walkdir::WalkDir;

/// Disk usage breakdown by area in bytes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiskUsage {
    /// Bytes in source files (brands, channels, playbooks, skills, tools, capabilities, projects source)
    pub sources_bytes: u64,
    /// Bytes in `library/`
    pub library_bytes: u64,
    /// Bytes in `projects/*/renders/` and any renders
    pub renders_bytes: u64,
    /// Bytes in `.studio/cache/`
    pub cache_bytes: u64,
    /// Bytes in `.studio/blobs/`
    pub blobs_bytes: u64,
    /// Total bytes across entire workspace
    pub total_bytes: u64,
}

/// Compute disk usage of workspace by area (sources, library, renders, cache, blobs).
pub fn disk_usage(ws: &Workspace) -> Result<DiskUsage> {
    let ws_root = ws.root();
    let mut usage = DiskUsage::default();

    for entry in WalkDir::new(ws_root).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }

        let size = match fs::metadata(p) {
            Ok(m) => m.len(),
            Err(_) => 0,
        };

        usage.total_bytes += size;

        let rel = match p.strip_prefix(ws_root) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };

        if rel.starts_with(".studio/blobs/") {
            usage.blobs_bytes += size;
        } else if rel.starts_with(".studio/cache/") {
            usage.cache_bytes += size;
        } else if rel.starts_with("library/") {
            usage.library_bytes += size;
        } else if rel.contains("/renders/") || rel.starts_with("renders/") {
            usage.renders_bytes += size;
        } else if !rel.starts_with(".studio/") && !rel.starts_with(".git/") {
            usage.sources_bytes += size;
        }
    }

    Ok(usage)
}
