//! Web mode settings, TLS status, and configuration management.

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::{CmdError, CmdResult};
use crate::state::AppState;

/// Request payload for configuring web mode.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebConfigureRequest {
    pub enabled: Option<bool>,
    pub lan_allowed: Option<bool>,
    pub port: Option<u16>,
    pub password: Option<String>,
    pub upload_limit_mb: Option<usize>,
    pub json_limit_mb: Option<usize>,
}

/// Retrieve current web server status, configuration, and TLS fingerprint.
pub fn web_status_impl(state: &AppState) -> CmdResult<Value> {
    let db = state.app_paths.open_app_db()?;
    let running = state
        .web_server
        .lock()
        .map(|s| s.is_some())
        .unwrap_or(false);
    let lan_allowed = db
        .get_setting("web:lan_allowed")?
        .map(|v| v == "true")
        .unwrap_or(false);
    let port: u16 = db
        .get_setting("web:port")?
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000);
    let has_password = db.get_setting("web:password_hash")?.is_some();
    let lan_ip = studio_web::lan::detect_lan_ip();

    let upload_limit_mb: usize = db
        .get_setting("web:upload_limit_mb")?
        .and_then(|v| v.parse().ok())
        .unwrap_or(200);
    let json_limit_mb: usize = db
        .get_setting("web:json_limit_mb")?
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);

    // If LAN is allowed, we use HTTPS, otherwise HTTP
    let (lan_url, local_url) = if lan_allowed {
        (
            lan_ip.as_ref().map(|ip| format!("https://{ip}:{port}")),
            Some(format!("https://localhost:{port}")),
        )
    } else {
        (
            lan_ip.as_ref().map(|ip| format!("http://{ip}:{port}")),
            Some(format!("http://localhost:{port}")),
        )
    };

    let qr_code_svg = if lan_allowed && has_password {
        lan_url
            .as_ref()
            .and_then(|url| studio_web::qr::generate_qr_svg(url).ok())
    } else {
        None
    };

    let (tls_fingerprint, active_sessions) = match state.web_server.lock() {
        Ok(slot) => (
            slot.as_ref().and_then(|s| s.tls_fingerprint.clone()),
            slot.as_ref()
                .map(|s| s.sessions.active_count())
                .unwrap_or(0),
        ),
        Err(_) => (None, 0),
    };

    // If not running or not in slot, we can also check if a cert exists on disk
    let tls_fingerprint = tls_fingerprint.or_else(|| {
        if lan_allowed {
            let cert_dir = state.app_paths.data_dir().join("tls");
            studio_web::ensure_tls_certificate(&cert_dir, None)
                .ok()
                .map(|t| t.fingerprint_sha256)
        } else {
            None
        }
    });

    Ok(json!({
        "enabled": running,
        "lanAllowed": lan_allowed,
        "port": port,
        "hasPassword": has_password,
        "lanIp": lan_ip,
        "lanUrl": lan_url,
        "localUrl": local_url,
        "qrCodeSvg": qr_code_svg,
        "tlsFingerprint": tls_fingerprint,
        "uploadLimitMb": upload_limit_mb,
        "jsonLimitMb": json_limit_mb,
        "activeSessions": active_sessions,
    }))
}

/// Configure web mode options, limits, or password, restarting the server if needed.
pub async fn web_configure_impl(state: &AppState, req: WebConfigureRequest) -> CmdResult<Value> {
    let db = state.app_paths.open_app_db()?;

    if let Some(pw) = req.password {
        let trimmed = pw.trim();
        if !trimmed.is_empty() {
            let hash = studio_web::auth::hash_password(trimmed)
                .map_err(|e| CmdError::msg(format!("Không thể tạo băm mật khẩu: {e}")))?;
            db.set_setting("web:password_hash", &hash)?;
        }
    }

    let mut needs_restart = false;

    if let Some(lan) = req.lan_allowed {
        db.set_setting("web:lan_allowed", if lan { "true" } else { "false" })?;
        needs_restart = true;
    }

    if let Some(port) = req.port {
        db.set_setting("web:port", &port.to_string())?;
        needs_restart = true;
    }

    if let Some(upload_mb) = req.upload_limit_mb {
        // Hard ceiling: max 1024 MB (1 GB), min 1 MB
        let clamped = upload_mb.clamp(1, 1024);
        db.set_setting("web:upload_limit_mb", &clamped.to_string())?;
        needs_restart = true;
    }

    if let Some(json_mb) = req.json_limit_mb {
        // Hard ceiling: max 10 MB, min 1 MB
        let clamped = json_mb.clamp(1, 10);
        db.set_setting("web:json_limit_mb", &clamped.to_string())?;
        needs_restart = true;
    }

    let is_currently_running = state
        .web_server
        .lock()
        .map(|s| s.is_some())
        .unwrap_or(false);

    if let Some(enabled) = req.enabled {
        db.set_setting("web:enabled", if enabled { "true" } else { "false" })?;
        if enabled {
            let has_pw = db.get_setting("web:password_hash")?.is_some();
            if !has_pw {
                return Err(CmdError::msg(
                    "Phải thiết lập mật khẩu chủ sở hữu trước khi bật chế độ web.",
                ));
            }
            crate::web_dispatch::start_web_server(state).await?;
        } else {
            crate::web_dispatch::stop_web_server(state);
        }
    } else if needs_restart && is_currently_running {
        crate::web_dispatch::start_web_server(state).await?;
    }

    web_status_impl(state)
}

#[cfg(test)]
mod tests {
    /// The app links rustls with both ring and aws-lc-rs; building the TLS config used to panic
    /// here (no default provider), which hung `web_configure`.
    #[tokio::test]
    async fn tls_config_loads_in_the_app_dependency_graph() {
        let dir = tempfile::tempdir().unwrap();
        let cert = studio_web::ensure_tls_certificate(dir.path(), None).unwrap();
        studio_web::load_rustls_config(&cert).await.unwrap();
    }
}
