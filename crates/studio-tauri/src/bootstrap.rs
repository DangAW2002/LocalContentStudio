//! Shared bootstrap and teardown routines for both desktop Tauri shell and headless mode.

use std::sync::Arc;

use studio_mcp::{RuntimeInfo, ServerState};

use crate::commands;
use crate::defaults;
use crate::state::AppState;

/// Shared application initialization:
/// 1. Installs default assets/catalog data.
/// 2. Starts the Studio MCP server (loopback + token) and writes runtime lock files.
/// 3. Reopens the most recent workspace.
/// 4. Subscribes to the job scheduler and broadcasts "job-event" to frontend SSE and Tauri.
pub async fn bootstrap(state: &AppState) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Err(e) = defaults::install(
        &state.app_paths.shared_dir(),
        &state.catalog_path,
        &state.sources_dir,
    ) {
        eprintln!("Không cài được dữ liệu mặc định: {e}");
    }

    // Studio MCP server for engine turns (loopback + per-turn tokens) and the agy shim lockfile.
    let shim_secret = uuid::Uuid::new_v4().simple().to_string();
    let server_state = Arc::new(ServerState {
        tokens: state.tokens.clone(),
        scheduler: state.scheduler.clone(),
        secrets: state.secrets.clone(),
        broker: state.broker.clone(),
        catalog_paths: vec![state.catalog_path.clone()],
        shim_secret: shim_secret.clone(),
        endpoints: state.endpoints.clone(),
        pricing: state.pricing.clone(),
    });

    let running = studio_mcp::start(server_state, 0).await?;
    let port = running.addr.port();
    let _ = state.mcp_port.set(port);

    let runtime_path = studio_mcp::runtime::default_path();
    let runtime_info = RuntimeInfo {
        port,
        shim_secret,
        pid: std::process::id(),
        runtime_path: Some(runtime_path.clone()),
    };
    studio_mcp::runtime::write(&runtime_path, &runtime_info)?;
    if let Err(e) = studio_mcp::runtime::write_runtime_d(&runtime_info) {
        eprintln!("Không ghi được runtime.d: {e}");
    }
    // Keep MCP server alive for process lifetime
    std::mem::forget(running);

    // Auto-register Studio MCP shim with agy if studio-mcp executable is found alongside app
    if let Ok(current) = std::env::current_exe() {
        let mcp_exe = current.with_file_name(if cfg!(windows) {
            "studio-mcp.exe"
        } else {
            "studio-mcp"
        });
        if mcp_exe.exists() {
            if let Err(e) = studio_tools::setup_agy(&mcp_exe) {
                eprintln!("[studio-tauri] Không thể tự động kết nối agy với Studio MCP: {e}");
            }
        }
    }

    // Reopen the most recent workspace
    let recent = state
        .app_paths
        .open_app_db()
        .ok()
        .and_then(|db| db.list_recent_workspaces().ok());
    if let Some(path) = recent.and_then(|r| r.into_iter().next()).map(|r| r.path) {
        let _ = commands::workspace_open_impl(state, path);
    }

    // Subscribe to scheduler events and emit "job-event" to the frontend (Tauri + SSE)
    let mut job_rx = state.scheduler.subscribe();
    let state_events = state.clone();
    tokio::spawn(async move {
        use serde_json::json;

        loop {
            let evt = match job_rx.recv().await {
                Ok(evt) => evt,
                // A slow UI only loses old progress ticks; keep listening.
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            };
            let state_str = match evt.state {
                studio_jobs::event::JobState::Queued => "queued",
                studio_jobs::event::JobState::Admitted => "admitted",
                studio_jobs::event::JobState::Running => "running",
                studio_jobs::event::JobState::Succeeded => "succeeded",
                studio_jobs::event::JobState::Failed => "failed",
                studio_jobs::event::JobState::Cancelled => "cancelled",
            };
            let snapshot_state = match evt.state {
                studio_jobs::event::JobState::Queued | studio_jobs::event::JobState::Admitted => {
                    "PENDING"
                }
                studio_jobs::event::JobState::Running => "RUNNING",
                studio_jobs::event::JobState::Succeeded => "DONE",
                studio_jobs::event::JobState::Failed => "FAILED",
                studio_jobs::event::JobState::Cancelled => "CANCELLED",
            };
            let payload = json!({
                "jobId": evt.job_id,
                "kind": evt.kind,
                "pool": evt.pool,
                "state": state_str,
                "progress": evt.progress,
                "message": evt.message,
                "peakMemoryBytes": evt.peak_memory_bytes,
                "event": {
                    "id": evt.job_id,
                    "title": evt.kind,
                    "kind": evt.kind,
                    "state": snapshot_state,
                    "progress": evt.progress,
                    "stageName": evt.message,
                    "createdAt": chrono::Utc::now().to_rfc3339(),
                    "error": if evt.state == studio_jobs::event::JobState::Failed { evt.message.clone() } else { None },
                }
            });
            state_events.emit("job-event", payload);
        }
    });

    Ok(())
}

/// Clean up MCP runtime lock files on application shutdown.
pub fn teardown() {
    studio_mcp::runtime::remove(&studio_mcp::runtime::default_path());
    studio_mcp::runtime::remove_runtime_d(std::process::id());
}
