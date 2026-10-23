//! Headless CLI mode and password setup for headless Linux server deployment.

use std::io::BufRead;
use std::sync::Arc;

use studio_web::WebServerConfig;

use crate::bootstrap;
use crate::state::AppState;
use crate::web_dispatch;

/// Headless execution configuration parsed from CLI arguments.
#[derive(Debug, Clone, Default)]
pub struct HeadlessConfig {
    /// Port override (does not modify database).
    pub port: Option<u16>,
    /// LAN/TLS mode flag (bind 0.0.0.0 + self-signed TLS).
    pub lan: bool,
    /// Trusted hosts for Host/Origin checking (e.g. Tailscale domains).
    pub allowed_hosts: Vec<String>,
    /// Set true when running behind an HTTPS reverse proxy (e.g. tailscale serve).
    pub behind_https_proxy: bool,
}

/// Parse command line arguments for headless mode.
pub fn parse_headless_args(args: &[String]) -> HeadlessConfig {
    let mut config = HeadlessConfig::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--port" if i + 1 < args.len() => {
                config.port = args[i + 1].parse().ok();
                i += 1;
            }
            "--lan" => {
                config.lan = true;
            }
            "--allowed-host" if i + 1 < args.len() => {
                config.allowed_hosts.push(args[i + 1].clone());
                i += 1;
            }
            "--behind-https-proxy" => {
                config.behind_https_proxy = true;
            }
            _ => {}
        }
        i += 1;
    }
    config
}

/// Handle `--set-web-password` by reading a password from stdin, validating length >= 8,
/// hashing with argon2, and persisting to app.db.
pub fn handle_set_web_password<R: std::io::Read>(reader: &mut R) -> Result<(), String> {
    let mut buf = String::new();
    let mut reader = std::io::BufReader::new(reader);
    reader
        .read_line(&mut buf)
        .map_err(|e| format!("Lỗi đọc stdin: {e}"))?;

    let trimmed = buf.trim_end_matches(['\r', '\n']);
    if trimmed.len() < 8 {
        return Err("Mật khẩu web phải có ít nhất 8 ký tự.".to_string());
    }

    let hash = studio_web::hash_password(trimmed)
        .map_err(|e| format!("Không thể tạo băm mật khẩu: {e}"))?;

    let app_paths = studio_workspace::AppPaths::from_env();
    let db = app_paths
        .open_app_db()
        .map_err(|e| format!("Không thể mở cơ sở dữ liệu app.db: {e}"))?;

    db.set_setting("web:password_hash", &hash)
        .map_err(|e| format!("Lỗi ghi mật khẩu: {e}"))?;
    db.set_setting("web:enabled", "true")
        .map_err(|e| format!("Lỗi bật chế độ web: {e}"))?;

    eprintln!("Đã đặt mật khẩu web thành công.");
    Ok(())
}

/// Run Studio in headless mode.
/// Starts core bootstrap, runs the web server (without altering database configuration),
/// and gracefully shuts down upon signal or shutdown receiver.
pub async fn run_headless(
    state: AppState,
    config: HeadlessConfig,
    shutdown_rx: Option<tokio::sync::oneshot::Receiver<()>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    bootstrap::bootstrap(&state).await?;

    let db = state.app_paths.open_app_db()?;
    let port = config.port.unwrap_or_else(|| {
        db.get_setting("web:port")
            .ok()
            .flatten()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3000)
    });

    let lan_allowed = if config.lan {
        true
    } else {
        db.get_setting("web:lan_allowed")
            .ok()
            .flatten()
            .map(|v| v == "true")
            .unwrap_or(false)
    };

    let bind_host = if lan_allowed { "0.0.0.0" } else { "127.0.0.1" };
    let behind_https_proxy =
        config.behind_https_proxy || config.allowed_hosts.iter().any(|h| h.contains(".ts.net"));

    let web_config = WebServerConfig {
        bind_host: bind_host.into(),
        port,
        lan_allowed,
        app_data_dir: state.app_paths.data_dir().to_path_buf(),
        redirect_http_to_https: false,
        limits: studio_web::WebLimitsConfig::default(),
        allowed_hosts: config.allowed_hosts,
        behind_https_proxy,
    };

    let dispatcher = Arc::new(web_dispatch::TauriWebDispatcher::new(state.clone()));
    let running = studio_web::start(web_config, dispatcher, state.events_tx.clone()).await?;
    let bound_addr = running.addr;

    if let Ok(mut slot) = state.web_server.lock() {
        *slot = Some(running);
    }

    eprintln!(
        "[studio-tauri] Chế độ headless đã khởi động tại {} ({})",
        bound_addr,
        if lan_allowed { "HTTPS/TLS" } else { "HTTP" }
    );

    let has_password = db.get_setting("web:password_hash").ok().flatten().is_some();
    if !has_password {
        eprintln!(
            "[studio-tauri] Cảnh báo: Chưa đặt mật khẩu web. Hãy chạy 'studio-tauri --set-web-password' để đặt mật khẩu."
        );
    }

    // Wait for shutdown signal or trigger
    if let Some(rx) = shutdown_rx {
        let _ = rx.await;
    } else {
        wait_for_shutdown_signal().await;
    }

    eprintln!("[studio-tauri] Đang dừng máy chủ web và dọn dẹp...");
    web_dispatch::stop_web_server(&state);
    bootstrap::teardown();
    eprintln!("[studio-tauri] Đã tắt gọn.");

    Ok(())
}

/// Wait for OS termination signal (SIGINT or SIGTERM on Unix, Ctrl+C on Windows).
pub async fn wait_for_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigint =
            signal(SignalKind::interrupt()).expect("failed to install SIGINT signal handler");
        let mut sigterm =
            signal(SignalKind::terminate()).expect("failed to install SIGTERM signal handler");
        tokio::select! {
            _ = sigint.recv() => {
                eprintln!("[studio-tauri] Nhận tín hiệu SIGINT.");
            }
            _ = sigterm.recv() => {
                eprintln!("[studio-tauri] Nhận tín hiệu SIGTERM.");
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
        eprintln!("[studio-tauri] Nhận tín hiệu Ctrl+C.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_parse_headless_args_full() {
        let args = vec![
            "studio-tauri".to_string(),
            "--headless".to_string(),
            "--port".to_string(),
            "8080".to_string(),
            "--lan".to_string(),
            "--allowed-host".to_string(),
            "server.tailnet.ts.net".to_string(),
            "--allowed-host".to_string(),
            "*.custom.org".to_string(),
            "--behind-https-proxy".to_string(),
        ];
        let cfg = parse_headless_args(&args);
        assert_eq!(cfg.port, Some(8080));
        assert!(cfg.lan);
        assert_eq!(
            cfg.allowed_hosts,
            vec![
                "server.tailnet.ts.net".to_string(),
                "*.custom.org".to_string()
            ]
        );
        assert!(cfg.behind_https_proxy);
    }

    #[test]
    fn test_set_web_password_reject_short() {
        let _lock = crate::commands::tests::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().expect("tempdir");
        std::env::set_var("STUDIO_APP_DIR", tmp.path());

        // Less than 8 characters -> rejected
        let mut input = Cursor::new(b"short\n");
        let res = handle_set_web_password(&mut input);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("ít nhất 8 ký tự"));

        // Verify nothing was saved
        let app_paths = studio_workspace::AppPaths::from_env();
        let db = app_paths.open_app_db().expect("open db");
        assert!(db.get_setting("web:password_hash").unwrap().is_none());
    }

    #[test]
    fn test_set_web_password_success() {
        let _lock = crate::commands::tests::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().expect("tempdir");
        std::env::set_var("STUDIO_APP_DIR", tmp.path());

        let mut input = Cursor::new(b"MatKhauDaiHon8KyTu123!\r\n");
        let res = handle_set_web_password(&mut input);
        assert!(res.is_ok());

        let app_paths = studio_workspace::AppPaths::from_env();
        let db = app_paths.open_app_db().expect("open db");
        let hash = db
            .get_setting("web:password_hash")
            .unwrap()
            .expect("hash exists");
        assert!(studio_web::verify_password("MatKhauDaiHon8KyTu123!", &hash));
        assert_eq!(
            db.get_setting("web:enabled").unwrap(),
            Some("true".to_string())
        );
    }

    #[tokio::test]
    // The env lock must span the whole run: STUDIO_APP_DIR is read again at teardown.
    #[allow(clippy::await_holding_lock)]
    async fn test_headless_run_and_clean_teardown() {
        let _lock = crate::commands::tests::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().expect("tempdir");
        std::env::set_var("STUDIO_APP_DIR", tmp.path());

        let state = AppState::new();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        let config = HeadlessConfig {
            port: Some(0), // Port 0 for random port in test
            lan: false,
            allowed_hosts: vec!["test.ts.net".to_string()],
            behind_https_proxy: false,
        };

        let headless_handle =
            tokio::spawn(async move { run_headless(state, config, Some(shutdown_rx)).await });

        // Give it a brief moment to initialize bootstrap and MCP
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        let runtime_path = studio_mcp::runtime::default_path();
        assert!(
            runtime_path.exists(),
            "MCP runtime file must exist while headless is running"
        );

        // Send shutdown
        let _ = shutdown_tx.send(());
        let res = headless_handle.await.expect("join handle");
        assert!(res.is_ok(), "run_headless must return Ok");

        // Teardown must remove runtime files cleanly
        assert!(
            !runtime_path.exists(),
            "MCP runtime file must be deleted after teardown"
        );
    }
}
