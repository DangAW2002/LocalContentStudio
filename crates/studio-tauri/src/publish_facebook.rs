//! Facebook publishing IPC commands and account integration.

use crate::error::{CmdError, CmdResult};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use studio_tools::publish::{
    connect_facebook, FacebookPageAccount, KEY_FACEBOOK_APP_ID, KEY_FACEBOOK_APP_SECRET,
    KEY_FACEBOOK_PAGE_TOKEN,
};

/// Result of configuring Facebook App ID and App Secret.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FacebookClientConfigResult {
    pub app_id_last4: String,
    pub message: String,
}

/// Status of Facebook client configuration.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FacebookClientStatus {
    pub configured: bool,
    pub app_id_last4: Option<String>,
}

/// Extract last 4 characters of Facebook App ID for display.
pub fn extract_app_id_last4(app_id: &str) -> String {
    let trimmed = app_id.trim();
    if trimmed.len() <= 4 {
        trimmed.to_string()
    } else {
        trimmed[trimmed.len() - 4..].to_string()
    }
}

/// Save Facebook App ID and App Secret to Credential Manager.
pub async fn facebook_client_config_impl(
    state: &AppState,
    app_id: String,
    app_secret: String,
) -> CmdResult<FacebookClientConfigResult> {
    let trimmed_id = app_id.trim();
    let trimmed_secret = app_secret.trim();

    if trimmed_id.is_empty() || trimmed_secret.is_empty() {
        return Err(CmdError::msg("Vui lòng nhập cả App ID và App Secret."));
    }

    let ws = state.workspace().ok();
    let org_name = ws.as_ref().map(|w| w.org().name()).unwrap_or("");
    let ws_name = ws.as_ref().map(|w| w.name());

    state
        .secrets
        .set(org_name, ws_name, KEY_FACEBOOK_APP_ID, trimmed_id)
        .map_err(|e| CmdError::msg(format!("Lỗi lưu FACEBOOK_APP_ID: {e}")))?;
    state
        .secrets
        .set(org_name, ws_name, KEY_FACEBOOK_APP_SECRET, trimmed_secret)
        .map_err(|e| CmdError::msg(format!("Lỗi lưu FACEBOOK_APP_SECRET: {e}")))?;

    if ws_name.is_some() {
        let _ = state.secrets.set("", None, KEY_FACEBOOK_APP_ID, trimmed_id);
        let _ = state
            .secrets
            .set("", None, KEY_FACEBOOK_APP_SECRET, trimmed_secret);
    }

    let last4 = extract_app_id_last4(trimmed_id);
    Ok(FacebookClientConfigResult {
        app_id_last4: last4.clone(),
        message: format!("Đã lưu thông tin Meta App (…{last4})"),
    })
}

#[tauri::command]
pub async fn facebook_client_config(
    state: tauri::State<'_, AppState>,
    app_id: String,
    app_secret: String,
) -> CmdResult<FacebookClientConfigResult> {
    facebook_client_config_impl(&state, app_id, app_secret).await
}

/// Retrieve status of configured Facebook App.
pub fn facebook_client_status_impl(state: &AppState) -> CmdResult<FacebookClientStatus> {
    let ws = state.workspace().ok();
    let org_name = ws.as_ref().map(|w| w.org().name()).unwrap_or("");
    let ws_name = ws.as_ref().map(|w| w.name());

    if let Some(app_id) = state.secrets.get(org_name, ws_name, KEY_FACEBOOK_APP_ID) {
        let last4 = extract_app_id_last4(&app_id);
        Ok(FacebookClientStatus {
            configured: true,
            app_id_last4: Some(last4),
        })
    } else {
        Ok(FacebookClientStatus {
            configured: false,
            app_id_last4: None,
        })
    }
}

#[tauri::command]
pub async fn facebook_client_status(
    state: tauri::State<'_, AppState>,
) -> CmdResult<FacebookClientStatus> {
    facebook_client_status_impl(&state)
}

/// Connect Facebook Pages via OAuth loopback on 127.0.0.1:8911.
#[tauri::command]
pub async fn facebook_connect(
    state: tauri::State<'_, AppState>,
) -> CmdResult<Vec<FacebookPageAccount>> {
    let ws = state.workspace().ok();
    let org_name = ws.as_ref().map(|w| w.org().name()).unwrap_or("");
    let ws_name = ws.as_ref().map(|w| w.name());

    let pages = connect_facebook(state.secrets.as_ref(), org_name, ws_name, 300).await?;

    if let Ok(db) = state.app_paths.open_app_db() {
        let mut existing_pages: Vec<FacebookPageAccount> = db
            .get_setting("publish:facebook_pages")
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        for new_p in &pages {
            if let Some(pos) = existing_pages.iter().position(|p| p.id == new_p.id) {
                existing_pages[pos] = new_p.clone();
            } else {
                existing_pages.push(new_p.clone());
            }
        }

        if let Ok(json_str) = serde_json::to_string(&existing_pages) {
            let _ = db.set_setting("publish:facebook_pages", &json_str);
        }
        if let Some(first) = pages.first() {
            let _ = db.set_setting("publish:last_used_facebook_page", &first.id);
        }
    }

    Ok(pages)
}

/// List all connected Facebook Pages from app.db.
pub fn facebook_pages_list_impl(state: &AppState) -> CmdResult<Vec<FacebookPageAccount>> {
    let db = state.app_paths.open_app_db().map_err(CmdError::from)?;
    let pages: Vec<FacebookPageAccount> = db
        .get_setting("publish:facebook_pages")
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    Ok(pages)
}

#[tauri::command]
pub async fn facebook_pages_list(
    state: tauri::State<'_, AppState>,
) -> CmdResult<Vec<FacebookPageAccount>> {
    facebook_pages_list_impl(&state)
}

/// Result of disconnecting a Facebook Page.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FacebookDisconnectResult {
    pub success: bool,
    pub message: String,
}

/// Disconnect a Facebook Page from Studio.
pub fn facebook_page_disconnect_impl(
    state: &AppState,
    page_id: String,
) -> CmdResult<FacebookDisconnectResult> {
    let ws = state.workspace().ok();
    let org_name = ws.as_ref().map(|w| w.org().name()).unwrap_or("");
    let ws_name = ws.as_ref().map(|w| w.name());

    let token_key = format!("{KEY_FACEBOOK_PAGE_TOKEN}:{page_id}");
    let _ = state.secrets.delete(org_name, ws_name, &token_key);
    if ws_name.is_some() {
        let _ = state.secrets.delete("", None, &token_key);
    }

    if let Ok(db) = state.app_paths.open_app_db() {
        let mut pages: Vec<FacebookPageAccount> = db
            .get_setting("publish:facebook_pages")
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        pages.retain(|p| p.id != page_id);

        if pages.is_empty() {
            let _ = db.delete_setting("publish:facebook_pages");
            let _ = db.delete_setting("publish:last_used_facebook_page");
        } else {
            if let Ok(json_str) = serde_json::to_string(&pages) {
                let _ = db.set_setting("publish:facebook_pages", &json_str);
            }
            let last = db
                .get_setting("publish:last_used_facebook_page")
                .ok()
                .flatten();
            if last.as_deref() == Some(&page_id) {
                if let Some(first) = pages.first() {
                    let _ = db.set_setting("publish:last_used_facebook_page", &first.id);
                }
            }
        }
    }

    Ok(FacebookDisconnectResult {
        success: true,
        message: "Đã ngắt kết nối Facebook Page thành công.".into(),
    })
}

#[tauri::command]
pub async fn facebook_page_disconnect(
    state: tauri::State<'_, AppState>,
    page_id: String,
) -> CmdResult<FacebookDisconnectResult> {
    facebook_page_disconnect_impl(&state, page_id)
}
