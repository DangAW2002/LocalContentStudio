//! Local Content Studio desktop shell (Tauri 2): IPC commands, Studio MCP server, agent turns.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod assets;
mod bootstrap;
mod broker;
pub mod budget;
mod commands;
mod defaults;
mod engines;
mod error;
mod headless;
mod publish;
mod publish_facebook;
mod state;
mod turns;
pub mod web_dispatch;
mod web_settings;

use tauri::Manager;

use crate::state::AppState;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--set-web-password") {
        if let Err(e) = headless::handle_set_web_password(&mut std::io::stdin()) {
            eprintln!("[studio-tauri] Lỗi: {e}");
            std::process::exit(1);
        }
        return;
    }

    if args.iter().any(|a| a == "--headless") {
        let config = headless::parse_headless_args(&args);
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("không khởi tạo được tokio runtime");
        let _guard = rt.enter();
        let state = AppState::new();
        if let Err(e) = rt.block_on(headless::run_headless(state, config, None)) {
            eprintln!("[studio-tauri] Lỗi chạy headless: {e}");
            std::process::exit(1);
        }
        return;
    }

    // The job scheduler spawns its loop on creation: enter Tauri's tokio runtime first.
    let runtime = tauri::async_runtime::handle();
    let _guard = runtime.inner().enter();
    tauri::Builder::default()
        .manage(AppState::new())
        .setup(|app| {
            let handle = app.handle().clone();
            let state = app.state::<AppState>();
            let _ = state.app_handle.set(handle.clone());
            state.broker.attach(handle.clone());

            tauri::async_runtime::block_on(bootstrap::bootstrap(&state))
                .map_err(|e| e as Box<dyn std::error::Error>)?;

            // Check if personal web mode is enabled and password is set. If so, start the web server.
            let web_enabled = state
                .app_paths
                .open_app_db()
                .ok()
                .and_then(|db| db.get_setting("web:enabled").ok().flatten())
                .map(|v| v == "true")
                .unwrap_or(false);
            let has_pw = state
                .app_paths
                .open_app_db()
                .ok()
                .and_then(|db| db.get_setting("web:password_hash").ok().flatten())
                .is_some();
            if web_enabled && has_pw {
                let state_web = (*state).clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = web_dispatch::start_web_server(&state_web).await {
                        eprintln!("Không thể tự động khởi động máy chủ web: {e}");
                    }
                });
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                bootstrap::teardown();
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
            budget::thread_set_model,
            commands::turn_start,
            commands::turn_cancel,
            commands::turn_undo,
            commands::permission_respond,
            commands::question_respond,
            commands::permission_pending,
            budget::budget_get,
            budget::budget_set,
            commands::checkpoint_list,
            commands::checkpoint_restore,
            commands::engines_probe,
            engines::engine_models,
            commands::catalog_kits,
            commands::catalog_modules,
            commands::catalog_suggest,
            commands::project_modules_set,
            commands::scope_list,
            commands::scope_override,
            commands::scope_create,
            commands::scope_validate,
            commands::scope_remove_override,
            assets::asset_list,
            assets::asset_import,
            assets::asset_delete,
            assets::asset_open_folder,
            commands::jobs_snapshot,
            commands::disk_usage,
            commands::agy_connect,
            commands::agy_disconnect,
            commands::file_write,
            commands::checkpoint_create,
            commands::artifact_list,
            commands::graph_get,
            commands::graph_link,
            commands::graph_unlink,
            commands::graph_set_pinned,
            commands::graph_set_notes,
            commands::graph_create_variant,
            commands::graph_update_downstream_plan,
            commands::graph_update_downstream_run,
            commands::restore_pinned_artifact,
            commands::artifact_render,
            commands::artifact_check,
            commands::artifact_channel_matrix,
            commands::secret_status,
            commands::secret_set,
            commands::secret_delete,
            commands::provider_endpoint_get,
            commands::provider_endpoint_set,
            commands::provider_pricing_get,
            commands::provider_pricing_set,
            commands::env_probe,
            commands::tool_install_preview,
            commands::tool_install_confirm,
            commands::tool_list_installed,
            commands::tool_uninstall,
            commands::web_status,
            commands::web_configure,
            commands::tts_status,
            commands::tts_model_preview,
            commands::tts_model_install,
            commands::tts_synthesize_preview,
            commands::tts_get_config,
            commands::tts_set_config,
            commands::engine_settings_get,
            commands::engine_settings_set,
            commands::dialog_pick_folder,
            commands::fs_list_dirs,
            commands::series_list,
            commands::series_get,
            commands::series_create,
            commands::series_create_episode,
            commands::graph_make_reference,
            publish::publish_prepare,
            publish::publish_list,
            publish::publish_get,
            publish::publish_update_metadata,
            publish::publish_confirm,
            publish::publish_cancel,
            publish::youtube_client_import,
            publish::youtube_client_status,
            publish::youtube_connect,
            publish::youtube_channels_list,
            publish::youtube_channel_disconnect,
            publish_facebook::facebook_client_config,
            publish_facebook::facebook_client_status,
            publish_facebook::facebook_connect,
            publish_facebook::facebook_pages_list,
            publish_facebook::facebook_page_disconnect,
        ])
        .run(tauri::generate_context!())
        .expect("không khởi động được Local Content Studio");
}
