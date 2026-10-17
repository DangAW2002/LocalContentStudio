//! Local Content Studio desktop shell (Tauri 2): IPC commands, Studio MCP server, agent turns.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod broker;
mod commands;
mod defaults;
mod engines;
mod error;
mod publish;
mod state;
mod turns;

use std::sync::Arc;

use studio_mcp::{RuntimeInfo, ServerState};
use tauri::Manager;

use crate::state::AppState;

fn main() {
    // The job scheduler spawns its loop on creation: enter Tauri's tokio runtime first.
    let runtime = tauri::async_runtime::handle();
    let _guard = runtime.inner().enter();
    tauri::Builder::default()
        .manage(AppState::new())
        .setup(|app| {
            let handle = app.handle().clone();
            let state = app.state::<AppState>();
            state.broker.attach(handle.clone());
            if let Err(e) = defaults::install(&state.app_paths.shared_dir(), &state.catalog_path, &state.sources_dir) {
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
            let running = tauri::async_runtime::block_on(studio_mcp::start(server_state, 0))?;
            let port = running.addr.port();
            let _ = state.mcp_port.set(port);
            let runtime_path = studio_mcp::runtime::default_path();
            studio_mcp::runtime::write(
                &runtime_path,
                &RuntimeInfo {
                    port,
                    shim_secret,
                    pid: std::process::id(),
                },
            )?;
            // Keep the server alive for the whole process lifetime.
            std::mem::forget(running);

            // Reopen the most recent workspace.
            let recent = state
                .app_paths
                .open_app_db()
                .ok()
                .and_then(|db| db.list_recent_workspaces().ok());
            if let Some(path) = recent.and_then(|r| r.into_iter().next()).map(|r| r.path) {
                let _ = commands::workspace_open(handle.clone(), app.state::<AppState>(), path);
            }

            // Subscribe to scheduler events and emit "job-event" to the frontend.
            let mut job_rx = state.scheduler.subscribe();
            let job_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                use serde_json::json;
                use tauri::Emitter;

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
                        studio_jobs::event::JobState::Queued | studio_jobs::event::JobState::Admitted => "PENDING",
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
                    let _ = job_handle.emit("job-event", payload);
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                studio_mcp::runtime::remove(&studio_mcp::runtime::default_path());
                let _ = window;
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_recent_workspaces,
            commands::workspace_create,
            commands::workspace_open,
            commands::workspace_current,
            commands::project_list,
            commands::project_create,
            commands::project_files,
            commands::file_read,
            commands::thread_list,
            commands::thread_create,
            commands::thread_events,
            commands::turn_start,
            commands::turn_cancel,
            commands::turn_undo,
            commands::permission_respond,
            commands::checkpoint_list,
            commands::checkpoint_restore,
            commands::engines_probe,
            engines::engine_models,
            commands::catalog_kits,
            commands::catalog_modules,
            commands::catalog_suggest,
            commands::project_modules_set,
            commands::scope_list,
            commands::asset_list,
            commands::jobs_snapshot,
            commands::disk_usage,
            commands::agy_connect,
            commands::agy_disconnect,
            commands::file_write,
            commands::checkpoint_create,
            commands::artifact_list,
            commands::artifact_render,
            commands::artifact_check,
            commands::secret_status,
            commands::secret_set,
            commands::secret_delete,
            commands::provider_endpoint_get,
            commands::provider_endpoint_set,
            commands::provider_pricing_get,
            commands::provider_pricing_set,
            commands::env_probe,
            publish::publish_prepare,
            publish::publish_list,
            publish::publish_get,
            publish::publish_update_metadata,
            publish::publish_confirm,
            publish::publish_cancel,
            publish::youtube_connect,
        ])
        .run(tauri::generate_context!())
        .expect("không khởi động được Local Content Studio");
}
