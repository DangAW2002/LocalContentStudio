//! Local Content Studio desktop shell (Tauri 2): IPC commands, Studio MCP server, agent turns.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod broker;
mod commands;
mod defaults;
mod error;
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
            if let Err(e) = defaults::install(&state.app_paths.shared_dir(), &state.catalog_path) {
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
        ])
        .run(tauri::generate_context!())
        .expect("không khởi động được Local Content Studio");
}
