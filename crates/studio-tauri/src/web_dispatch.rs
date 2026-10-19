//! Web command dispatcher and web server lifecycle management for Studio Tauri.

use std::path::PathBuf;
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use studio_web::dispatcher::{BoxFuture, WebDispatcher};
use studio_web::WebServerConfig;

use crate::commands;
use crate::error::{CmdError, CmdResult};
use crate::publish;
use crate::state::AppState;
use crate::turns;

/// Dispatcher connecting `studio-web` requests to the core application commands.
pub struct TauriWebDispatcher {
    state: AppState,
}

impl TauriWebDispatcher {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

impl WebDispatcher for TauriWebDispatcher {
    fn invoke<'a>(
        &'a self,
        command: &'a str,
        args: Value,
        is_sudo: bool,
    ) -> BoxFuture<'a, Result<Value, String>> {
        Box::pin(async move {
            dispatch_invoke(&self.state, command, args, is_sudo)
                .await
                .map_err(|e| e.to_string())
        })
    }

    fn resolve_file(&self, project: Option<&str>, path: &str) -> Result<PathBuf, String> {
        let ws = self.state.workspace().map_err(|e| e.to_string())?;
        commands::resolve_path(&ws, project, path).map_err(|e| e.to_string())
    }

    fn get_password_hash(&self) -> Result<Option<String>, String> {
        let db = self
            .state
            .app_paths
            .open_app_db()
            .map_err(|e| e.to_string())?;
        db.get_setting("web:password_hash")
            .map_err(|e| e.to_string())
    }

    fn set_password_hash(&self, hash: &str) -> Result<(), String> {
        let db = self
            .state
            .app_paths
            .open_app_db()
            .map_err(|e| e.to_string())?;
        db.set_setting("web:password_hash", hash)
            .map_err(|e| e.to_string())
    }
}

/// Start or restart the web server using configuration stored in app.db.
pub async fn start_web_server(state: &AppState) -> CmdResult<()> {
    stop_web_server(state);

    let db = state.app_paths.open_app_db()?;
    let lan_allowed = db
        .get_setting("web:lan_allowed")?
        .map(|v| v == "true")
        .unwrap_or(false);
    let port: u16 = db
        .get_setting("web:port")?
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000);

    let config = WebServerConfig {
        bind_host: "127.0.0.1".into(),
        port,
        lan_allowed,
    };

    let dispatcher = Arc::new(TauriWebDispatcher::new(state.clone()));
    let running = studio_web::start(config, dispatcher, state.events_tx.clone())
        .await
        .map_err(|e| CmdError::msg(format!("Không khởi động được máy chủ web: {e}")))?;

    if let Ok(mut slot) = state.web_server.lock() {
        *slot = Some(running);
    }
    Ok(())
}

/// Stop the running web server if active.
pub fn stop_web_server(state: &AppState) {
    if let Ok(mut slot) = state.web_server.lock() {
        if let Some(running) = slot.take() {
            let _ = running.shutdown.send(());
        }
    }
}

fn get_arg<T: DeserializeOwned>(args: &Value, name: &str) -> CmdResult<T> {
    match args.get(name) {
        Some(v) => serde_json::from_value(v.clone())
            .map_err(|e| CmdError::msg(format!("Tham số '{name}' không đúng định dạng: {e}"))),
        None => serde_json::from_value(Value::Null)
            .map_err(|_| CmdError::msg(format!("Thiếu tham số bắt buộc '{name}'."))),
    }
}

fn get_opt_arg<T: DeserializeOwned>(args: &Value, name: &str) -> Option<T> {
    args.get(name)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
}

/// Dispatch a web invoke request to the corresponding command implementation.
pub async fn dispatch_invoke(
    state: &AppState,
    command: &str,
    args: Value,
    _is_sudo: bool,
) -> CmdResult<Value> {
    match command {
        "app_recent_workspaces" => {
            let res = commands::app_recent_workspaces_impl(state)?;
            Ok(serde_json::to_value(res)?)
        }
        "workspace_create" => {
            let path: String = get_arg(&args, "path")?;
            let name: String = get_arg(&args, "name")?;
            commands::workspace_create_impl(state, path, name)
        }
        "workspace_open" => {
            let path: String = get_arg(&args, "path")?;
            commands::workspace_open_impl(state, path)
        }
        "workspace_current" => commands::workspace_current_impl(state),
        "project_list" => {
            let res = commands::project_list_impl(state)?;
            Ok(serde_json::to_value(res)?)
        }
        "project_create" => {
            let slug: String = get_arg(&args, "slug")?;
            let title: String = get_arg(&args, "title")?;
            let kit: Option<String> = get_opt_arg(&args, "kit");
            let modules: Option<Vec<String>> = get_opt_arg(&args, "modules");
            commands::project_create_impl(state, slug, title, kit, modules)
        }
        "project_files" => {
            let project: String = get_arg(&args, "project")?;
            let res = commands::project_files_impl(state, project)?;
            Ok(serde_json::to_value(res)?)
        }
        "file_read" => {
            let project: Option<String> = get_opt_arg(&args, "project");
            let path: String = get_arg(&args, "path")?;
            commands::file_read_impl(state, project, path)
        }
        "file_write" => {
            let project: Option<String> = get_opt_arg(&args, "project");
            let path: String = get_arg(&args, "path")?;
            let content: String = get_arg(&args, "content")?;
            commands::file_write_impl(state, project, path, content)
        }
        "thread_list" => {
            let project: String = get_arg(&args, "project")?;
            let res = commands::thread_list_impl(state, project)?;
            Ok(serde_json::to_value(res)?)
        }
        "thread_create" => {
            let project: String = get_arg(&args, "project")?;
            let engine: String = get_arg(&args, "engine")?;
            let model: Option<String> = get_opt_arg(&args, "model");
            let title: Option<String> = get_opt_arg(&args, "title");
            commands::thread_create_impl(state, project, engine, model, title)
        }
        "thread_events" => {
            let thread_id: String =
                get_arg(&args, "threadId").or_else(|_| get_arg(&args, "thread_id"))?;
            let res = commands::thread_events_impl(state, thread_id)?;
            Ok(serde_json::to_value(res)?)
        }
        "turn_start" => {
            let thread_id: String =
                get_arg(&args, "threadId").or_else(|_| get_arg(&args, "thread_id"))?;
            let prompt: String = get_arg(&args, "prompt")?;
            let id = turns::start_turn(state.clone(), thread_id, prompt).await?;
            Ok(json!({ "id": id }))
        }
        "turn_cancel" => {
            let turn_id: String =
                get_arg(&args, "turnId").or_else(|_| get_arg(&args, "turn_id"))?;
            commands::turn_cancel_impl(state, turn_id)
        }
        "turn_undo" => {
            let turn_id: String =
                get_arg(&args, "turnId").or_else(|_| get_arg(&args, "turn_id"))?;
            commands::turn_undo_impl(state, turn_id)
        }
        "permission_respond" => {
            let request_id: String =
                get_arg(&args, "requestId").or_else(|_| get_arg(&args, "request_id"))?;
            let allow: bool = get_arg(&args, "allow")?;
            commands::permission_respond_impl(state, request_id, allow)
        }
        "checkpoint_list" => {
            let res = commands::checkpoint_list_impl(state)?;
            Ok(serde_json::to_value(res)?)
        }
        "checkpoint_create" => {
            let label: Option<String> = get_opt_arg(&args, "label");
            commands::checkpoint_create_impl(state, label)
        }
        "checkpoint_restore" => {
            let id: String = get_arg(&args, "id")?;
            commands::checkpoint_restore_impl(state, id)
        }
        "engines_probe" => {
            let res = commands::engines_probe_impl(state).await?;
            Ok(serde_json::to_value(res)?)
        }
        "engine_models" => {
            let engine: String = get_arg(&args, "engine")?;
            let res = crate::engines::engine_models_impl(state, engine).await?;
            Ok(serde_json::to_value(res)?)
        }
        "catalog_kits" => {
            let res = commands::catalog_kits_impl(state)?;
            Ok(serde_json::to_value(res)?)
        }
        "catalog_modules" => {
            let res = commands::catalog_modules_impl(state)?;
            Ok(serde_json::to_value(res)?)
        }
        "catalog_suggest" => {
            let text: String = get_arg(&args, "text")?;
            commands::catalog_suggest_impl(state, text)
        }
        "project_modules_set" => {
            let project: String = get_arg(&args, "project")?;
            let modules: Vec<String> = get_arg(&args, "modules")?;
            commands::project_modules_set_impl(state, project, modules)
        }
        "scope_list" => {
            let kind: String = get_arg(&args, "kind")?;
            let res = commands::scope_list_impl(state, kind)?;
            Ok(serde_json::to_value(res)?)
        }
        "asset_list" => {
            let filter: Option<commands::AssetFilter> = get_opt_arg(&args, "filter");
            let res = commands::asset_list_impl(state, filter)?;
            Ok(serde_json::to_value(res)?)
        }
        "jobs_snapshot" => {
            let res = commands::jobs_snapshot_impl(state).await?;
            Ok(serde_json::to_value(res)?)
        }
        "disk_usage" => commands::disk_usage_impl(state),
        "artifact_list" => {
            let project: String = get_arg(&args, "project")?;
            let res = commands::artifact_list_impl(state, project)?;
            Ok(serde_json::to_value(res)?)
        }
        "artifact_render" => {
            let project: String = get_arg(&args, "project")?;
            let id: String = get_arg(&args, "id").or_else(|_| get_arg(&args, "path"))?;
            commands::artifact_render_impl(state, project, id).await
        }
        "artifact_check" => {
            let project: String = get_arg(&args, "project")?;
            let id: String = get_arg(&args, "id").or_else(|_| get_arg(&args, "path"))?;
            let channels: Option<Vec<String>> =
                get_opt_arg(&args, "channels").or_else(|| get_opt_arg(&args, "checks"));
            commands::artifact_check_impl(state, project, id, channels)
        }
        "secret_status" => {
            let res = commands::secret_status_impl(state).await?;
            Ok(serde_json::to_value(res)?)
        }
        "secret_set" => {
            let name: String = get_arg(&args, "name")?;
            let value: String = get_arg(&args, "value")?;
            commands::secret_set_impl(state, name, value).await?;
            Ok(json!({ "ok": true }))
        }
        "secret_delete" => {
            let name: String = get_arg(&args, "name")?;
            commands::secret_delete_impl(state, name).await?;
            Ok(json!({ "ok": true }))
        }
        "provider_endpoint_get" => {
            let res = commands::provider_endpoint_get_impl(state).await?;
            Ok(serde_json::to_value(res)?)
        }
        "provider_endpoint_set" => {
            let tool_id: String =
                get_arg(&args, "toolId").or_else(|_| get_arg(&args, "tool_id"))?;
            let endpoint: Option<String> =
                get_opt_arg(&args, "endpoint").or_else(|| get_opt_arg(&args, "baseUrl"));
            commands::provider_endpoint_set_impl(state, tool_id, endpoint).await?;
            Ok(json!({ "ok": true }))
        }
        "provider_pricing_get" => {
            let res = commands::provider_pricing_get_impl(state).await?;
            Ok(serde_json::to_value(res)?)
        }
        "provider_pricing_set" => {
            let tool_id: String =
                get_arg(&args, "toolId").or_else(|_| get_arg(&args, "tool_id"))?;
            let pricing: Option<studio_tools::ProviderPricing> = get_opt_arg(&args, "pricing");
            commands::provider_pricing_set_impl(state, tool_id, pricing).await?;
            Ok(json!({ "ok": true }))
        }
        "env_probe" => {
            let res = commands::env_probe_impl(state).await?;
            Ok(serde_json::to_value(res)?)
        }
        "tool_install_preview" => {
            let req = get_arg(&args, "req").or_else(|_| {
                serde_json::from_value(args.clone()).map_err(|e| CmdError::msg(e.to_string()))
            })?;
            let res = commands::tool_install_preview_impl(state, req).await?;
            Ok(serde_json::to_value(res)?)
        }
        "tool_install_confirm" => {
            let req = get_arg(&args, "req").or_else(|_| {
                serde_json::from_value(args.clone()).map_err(|e| CmdError::msg(e.to_string()))
            })?;
            let res = commands::tool_install_confirm_impl(state, req).await?;
            Ok(serde_json::to_value(res)?)
        }
        "tool_list_installed" => {
            let res = commands::tool_list_installed_impl(state).await?;
            Ok(serde_json::to_value(res)?)
        }
        "tool_uninstall" => {
            let req = get_arg(&args, "req").or_else(|_| {
                serde_json::from_value(args.clone()).map_err(|e| CmdError::msg(e.to_string()))
            })?;
            let res = commands::tool_uninstall_impl(state, req).await?;
            Ok(json!(res))
        }
        "publish_prepare" => {
            let project: Option<String> = get_opt_arg(&args, "project");
            let artifact: String = get_arg(&args, "artifact")?;
            let channel: String = get_arg(&args, "channel")?;
            let target: String = get_arg(&args, "target")?;
            let metadata = get_arg(&args, "metadata")?;
            let res =
                publish::publish_prepare_impl(state, project, artifact, channel, target, metadata)?;
            Ok(serde_json::to_value(res)?)
        }
        "publish_list" => {
            let project: Option<String> = get_opt_arg(&args, "project");
            let res = publish::publish_list_impl(state, project)?;
            Ok(serde_json::to_value(res)?)
        }
        "publish_get" => {
            let id: String = get_arg(&args, "id")?;
            let res = publish::publish_get_impl(state, id)?;
            Ok(serde_json::to_value(res)?)
        }
        "publish_update_metadata" => {
            let id: String = get_arg(&args, "id")?;
            let metadata = get_arg(&args, "metadata")?;
            let res = publish::publish_update_metadata_impl(state, id, metadata)?;
            Ok(serde_json::to_value(res)?)
        }
        "publish_confirm" => {
            let id: String = get_arg(&args, "id")?;
            let channel_id: Option<String> =
                get_opt_arg(&args, "channelId").or_else(|| get_opt_arg(&args, "channel_id"));
            let res = publish::publish_confirm_impl(state, id, channel_id).await?;
            Ok(serde_json::to_value(res)?)
        }
        "publish_cancel" => {
            let id: String = get_arg(&args, "id")?;
            let res = publish::publish_cancel_impl(state, id)?;
            Ok(serde_json::to_value(res)?)
        }
        "youtube_client_import" => {
            let json_val: Option<String> = get_opt_arg(&args, "json");
            let client_id: Option<String> =
                get_opt_arg(&args, "clientId").or_else(|| get_opt_arg(&args, "client_id"));
            let client_secret: Option<String> =
                get_opt_arg(&args, "clientSecret").or_else(|| get_opt_arg(&args, "client_secret"));
            let res =
                publish::youtube_client_import_impl(state, json_val, client_id, client_secret)
                    .await?;
            Ok(serde_json::to_value(res)?)
        }
        "youtube_client_status" => {
            let res = publish::youtube_client_status_impl(state)?;
            Ok(serde_json::to_value(res)?)
        }
        "youtube_channels_list" => {
            let res = publish::youtube_channels_list_impl(state).await?;
            Ok(serde_json::to_value(res)?)
        }
        "youtube_channel_disconnect" => {
            let channel_id: String =
                get_arg(&args, "channelId").or_else(|_| get_arg(&args, "channel_id"))?;
            let res = publish::youtube_channel_disconnect_impl(state, channel_id)?;
            Ok(serde_json::to_value(res)?)
        }
        "web_status" => {
            let res = commands::web_status_impl(state)?;
            Ok(serde_json::to_value(res)?)
        }
        "web_configure" => {
            let req: commands::WebConfigureRequest = if let Some(nested) = args.get("req") {
                serde_json::from_value(nested.clone()).map_err(|e| CmdError::msg(e.to_string()))?
            } else {
                serde_json::from_value(args.clone()).map_err(|e| CmdError::msg(e.to_string()))?
            };
            let res = commands::web_configure_impl(state, req).await?;
            Ok(serde_json::to_value(res)?)
        }
        "tts_status" => {
            let res = commands::tts_status_impl(state).await?;
            Ok(serde_json::to_value(res)?)
        }
        "tts_model_preview" => {
            let provider: String = get_arg(&args, "provider")?;
            let voice: String = get_arg(&args, "voice")?;
            let res = commands::tts_model_preview_impl(state, provider, voice).await?;
            Ok(serde_json::to_value(res)?)
        }
        "tts_model_install" => {
            let provider: String = get_arg(&args, "provider")?;
            let voice: String = get_arg(&args, "voice")?;
            let res = commands::tts_model_install_impl(state, provider, voice).await?;
            Ok(serde_json::to_value(res)?)
        }
        "tts_synthesize_preview" => {
            let req: commands::TtsPreviewRequest = get_arg(&args, "req").or_else(|_| {
                serde_json::from_value(args.clone()).map_err(|e| CmdError::msg(e.to_string()))
            })?;
            let res = commands::tts_synthesize_preview_impl(state, req).await?;
            Ok(serde_json::to_value(res)?)
        }
        "tts_get_config" => {
            let res = commands::tts_get_config_impl(state)?;
            Ok(serde_json::to_value(res)?)
        }
        "tts_set_config" => {
            let req: commands::TtsSetConfigRequest = get_arg(&args, "req").or_else(|_| {
                serde_json::from_value(args.clone()).map_err(|e| CmdError::msg(e.to_string()))
            })?;
            commands::tts_set_config_impl(state, req)?;
            Ok(json!({ "ok": true }))
        }
        "dialog_pick_folder" => {
            let res = commands::dialog_pick_folder_impl().await?;
            Ok(serde_json::to_value(res)?)
        }
        unhandled => Err(CmdError::msg(format!(
            "Lệnh '{unhandled}' chưa được định tuyến trong bộ điều phối web."
        ))),
    }
}
