//! Axum web server implementation for Local Content Studio web mode.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{ConnectInfo, Path as AxumPath, Query, State};
use axum::http::header::{self, HeaderMap};
use axum::http::{Method, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use include_dir::{include_dir, Dir};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::broadcast;

use crate::allowlist;
use crate::auth::{verify_password, LoginRateLimiter, SessionStore};
use crate::dispatcher::WebDispatcher;
use crate::events::{create_event_stream, WebEvent};
use crate::file::{self, FileQuery};
use crate::lan::format_lan_url;
use crate::qr::generate_qr_svg;
use crate::security::{
    forbidden_response, sudo_required_response, verify_csrf_header, verify_origin_and_host,
};
use crate::upload::handle_upload;

static UI_DIST: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../packages/desktop-ui/dist");

/// Configuration for starting the web server.
#[derive(Debug, Clone)]
pub struct WebServerConfig {
    pub bind_host: String,
    pub port: u16,
    pub lan_allowed: bool,
}

impl Default for WebServerConfig {
    fn default() -> Self {
        Self {
            bind_host: "127.0.0.1".into(),
            port: 3000,
            lan_allowed: false,
        }
    }
}

/// Shared state of the web server.
pub struct WebServerState {
    pub dispatcher: Arc<dyn WebDispatcher>,
    pub sessions: Arc<SessionStore>,
    pub rate_limiter: Arc<LoginRateLimiter>,
    pub events_tx: broadcast::Sender<WebEvent>,
    pub lan_allowed: bool,
    pub port: u16,
}

/// Running server handle.
pub struct RunningWebServer {
    pub addr: SocketAddr,
    pub shutdown: tokio::sync::oneshot::Sender<()>,
}

/// Start the web server.
pub async fn start(
    config: WebServerConfig,
    dispatcher: Arc<dyn WebDispatcher>,
    events_tx: broadcast::Sender<WebEvent>,
) -> std::io::Result<RunningWebServer> {
    let host = if config.lan_allowed {
        "0.0.0.0"
    } else {
        config.bind_host.as_str()
    };

    let state = Arc::new(WebServerState {
        dispatcher,
        sessions: Arc::new(SessionStore::new()),
        rate_limiter: Arc::new(LoginRateLimiter::new()),
        events_tx,
        lan_allowed: config.lan_allowed,
        port: config.port,
    });

    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind((host, config.port)).await?;
    let addr = listener.local_addr()?;
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    tokio::spawn(async move {
        let _ = axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async {
            let _ = rx.await;
        })
        .await;
    });

    Ok(RunningWebServer { addr, shutdown: tx })
}

/// Build the router with all web endpoints and static UI fallback.
pub fn build_router(state: Arc<WebServerState>) -> Router {
    Router::new()
        .route("/api/auth/status", get(auth_status))
        .route("/api/auth/login", post(auth_login))
        .route("/api/auth/logout", post(auth_logout))
        .route("/api/auth/sudo", post(auth_sudo))
        .route("/api/invoke/:command", post(invoke_handler))
        .route("/api/file", get(file_handler))
        .route("/api/events", get(events_handler))
        .route("/api/upload", post(upload_handler))
        .route("/api/qr", get(qr_handler))
        .fallback(static_or_spa_fallback)
        .layer(axum::middleware::map_response(security_headers))
        .with_state(state)
}

/// Defence-in-depth headers on every response: no framing, no MIME sniffing, no referrer leaks.
async fn security_headers(mut res: Response) -> Response {
    let h = res.headers_mut();
    h.insert(
        "x-frame-options",
        axum::http::HeaderValue::from_static("DENY"),
    );
    h.insert(
        "x-content-type-options",
        axum::http::HeaderValue::from_static("nosniff"),
    );
    h.insert(
        "referrer-policy",
        axum::http::HeaderValue::from_static("no-referrer"),
    );
    h.insert(
        "content-security-policy",
        axum::http::HeaderValue::from_static("frame-ancestors 'none'"),
    );
    res
}

fn extract_session_token(headers: &HeaderMap) -> Option<String> {
    let cookie_hdr = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in cookie_hdr.split(';') {
        let mut pair = part.trim().splitn(2, '=');
        if let (Some(name), Some(val)) = (pair.next(), pair.next()) {
            if name == "studio_session" {
                return Some(val.to_string());
            }
        }
    }
    None
}

fn is_https_request(headers: &HeaderMap) -> bool {
    if let Some(proto) = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
    {
        if proto.trim().eq_ignore_ascii_case("https") {
            return true;
        }
    }
    if let Some(origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) {
        if origin.starts_with("https://") {
            return true;
        }
    }
    if let Some(referer) = headers.get(header::REFERER).and_then(|v| v.to_str().ok()) {
        if referer.starts_with("https://") {
            return true;
        }
    }
    false
}

fn make_cookie(token: &str, max_age: i64, is_secure: bool) -> String {
    if is_secure {
        format!(
            "studio_session={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={max_age}; Secure"
        )
    } else {
        format!("studio_session={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={max_age}")
    }
}

// ──────────────────────────────── Auth Routes ────────────────────────────────

#[derive(Deserialize)]
struct LoginRequest {
    password: String,
}

#[derive(Serialize)]
struct AuthStatusResponse {
    authenticated: bool,
    #[serde(rename = "hasPassword")]
    has_password: bool,
    #[serde(rename = "isSudo")]
    is_sudo: bool,
    #[serde(rename = "sudoUntil")]
    sudo_until: Option<String>,
}

async fn auth_status(State(state): State<Arc<WebServerState>>, headers: HeaderMap) -> Response {
    if !verify_origin_and_host(&headers, state.lan_allowed) {
        return forbidden_response("Origin/Host không hợp lệ.");
    }

    let has_password = state
        .dispatcher
        .get_password_hash()
        .ok()
        .flatten()
        .is_some();

    let session = extract_session_token(&headers).and_then(|t| state.sessions.validate(&t));
    let authenticated = session.is_some();
    let is_sudo = session.as_ref().map(|s| s.is_sudo()).unwrap_or(false);
    let sudo_until = session
        .as_ref()
        .and_then(|s| s.last_sudo)
        .map(|t| (t + chrono::Duration::seconds(600)).to_rfc3339());

    Json(AuthStatusResponse {
        authenticated,
        has_password,
        is_sudo,
        sudo_until,
    })
    .into_response()
}

async fn auth_login(
    State(state): State<Arc<WebServerState>>,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    headers: HeaderMap,
    Json(body): Json<LoginRequest>,
) -> Response {
    if !verify_origin_and_host(&headers, state.lan_allowed) {
        return forbidden_response("Origin/Host không hợp lệ.");
    }
    if !verify_csrf_header(&Method::POST, &headers) {
        return forbidden_response("Yêu cầu header CSRF (x-studio-csrf).");
    }

    let ip = connect_info
        .map(|c| c.ip())
        .unwrap_or_else(|| std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));

    // Check rate limiter lockout
    if let Some(rem_secs) = state.rate_limiter.check_lockout(ip) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "error": format!("Quá nhiều lần thử đăng nhập sai. Vui lòng đợi {rem_secs} giây."),
                "code": "RATE_LIMITED",
                "retryAfter": rem_secs,
            })),
        )
            .into_response();
    }

    let stored_hash = match state.dispatcher.get_password_hash() {
        Ok(Some(h)) => h,
        Ok(None) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Chưa thiết lập mật khẩu chủ sở hữu." })),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Lỗi đọc mật khẩu: {e}") })),
            )
                .into_response();
        }
    };

    if verify_password(&body.password, &stored_hash) {
        state.rate_limiter.record_success(ip);
        let session = state.sessions.create_session();
        let cookie = make_cookie(&session.token, 86400, is_https_request(&headers));

        Response::builder()
            .status(StatusCode::OK)
            .header(header::SET_COOKIE, cookie)
            .header(header::CONTENT_TYPE, "application/json")
            .body(
                json!({
                    "authenticated": true,
                    "isSudo": true,
                    "sudoUntil": session.last_sudo.map(|t| (t + chrono::Duration::seconds(600)).to_rfc3339()),
                })
                .to_string()
                .into(),
            )
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
    } else {
        let (locked, rem) = state.rate_limiter.record_failure(ip);
        if locked {
            (
                StatusCode::TOO_MANY_REQUESTS,
                Json(json!({
                    "error": format!("Mật khẩu sai 5 lần. Khóa truy cập trong {rem} giây."),
                    "code": "RATE_LIMITED",
                    "retryAfter": rem,
                })),
            )
                .into_response()
        } else {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Mật khẩu không chính xác." })),
            )
                .into_response()
        }
    }
}

async fn auth_logout(State(state): State<Arc<WebServerState>>, headers: HeaderMap) -> Response {
    if !verify_origin_and_host(&headers, state.lan_allowed) {
        return forbidden_response("Origin/Host không hợp lệ.");
    }
    if !verify_csrf_header(&Method::POST, &headers) {
        return forbidden_response("Yêu cầu header CSRF.");
    }

    if let Some(token) = extract_session_token(&headers) {
        state.sessions.remove(&token);
    }
    let expired_cookie = make_cookie("", 0, is_https_request(&headers));

    Response::builder()
        .status(StatusCode::OK)
        .header(header::SET_COOKIE, expired_cookie)
        .header(header::CONTENT_TYPE, "application/json")
        .body(json!({ "ok": true }).to_string().into())
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

async fn auth_sudo(
    State(state): State<Arc<WebServerState>>,
    headers: HeaderMap,
    Json(body): Json<LoginRequest>,
) -> Response {
    if !verify_origin_and_host(&headers, state.lan_allowed) {
        return forbidden_response("Origin/Host không hợp lệ.");
    }
    if !verify_csrf_header(&Method::POST, &headers) {
        return forbidden_response("Yêu cầu header CSRF.");
    }

    let token = match extract_session_token(&headers) {
        Some(t) => t,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };

    let stored_hash = match state.dispatcher.get_password_hash() {
        Ok(Some(h)) => h,
        _ => return StatusCode::UNAUTHORIZED.into_response(),
    };

    if verify_password(&body.password, &stored_hash) {
        if state.sessions.refresh_sudo(&token) {
            Json(json!({
                "isSudo": true,
                "sudoUntil": (chrono::Utc::now() + chrono::Duration::seconds(600)).to_rfc3339()
            }))
            .into_response()
        } else {
            StatusCode::UNAUTHORIZED.into_response()
        }
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Mật khẩu không đúng." })),
        )
            .into_response()
    }
}

// ──────────────────────────────── Invoke Route ───────────────────────────────

async fn invoke_handler(
    State(state): State<Arc<WebServerState>>,
    AxumPath(command): AxumPath<String>,
    headers: HeaderMap,
    body: Option<Json<Value>>,
) -> Response {
    if !verify_origin_and_host(&headers, state.lan_allowed) {
        return forbidden_response("Origin/Host không hợp lệ.");
    }
    if !verify_csrf_header(&Method::POST, &headers) {
        return forbidden_response("Yêu cầu header CSRF (x-studio-csrf).");
    }

    let token = match extract_session_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Chưa đăng nhập." })),
            )
                .into_response()
        }
    };

    let session = match state.sessions.validate(&token) {
        Some(s) => s,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Phiên đăng nhập hết hạn hoặc không hợp lệ." })),
            )
                .into_response()
        }
    };

    // Allowlist check
    if !allowlist::is_command_allowed(&command) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "code": "COMMAND_NOT_ALLOWED",
                "error": format!("Lệnh '{command}' không được phép thực thi qua giao diện web.")
            })),
        )
            .into_response();
    }

    // Sudo check
    if allowlist::requires_sudo(&command) && !session.is_sudo() {
        return sudo_required_response();
    }

    let args = body.map(|Json(v)| v).unwrap_or(Value::Null);

    match state
        .dispatcher
        .invoke(&command, args, session.is_sudo())
        .await
    {
        Ok(result) => (StatusCode::OK, Json(result)).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": err })),
        )
            .into_response(),
    }
}

// ──────────────────────────────── File Route ─────────────────────────────────

async fn file_handler(
    State(state): State<Arc<WebServerState>>,
    headers: HeaderMap,
    Query(query): Query<FileQuery>,
) -> Response {
    if !verify_origin_and_host(&headers, state.lan_allowed) {
        return forbidden_response("Origin/Host không hợp lệ.");
    }

    let token = match extract_session_token(&headers) {
        Some(t) => t,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };

    if state.sessions.validate(&token).is_none() {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    file::serve_file(state.dispatcher.clone(), headers, query).await
}

// ──────────────────────────────── Events Route ───────────────────────────────

async fn events_handler(State(state): State<Arc<WebServerState>>, headers: HeaderMap) -> Response {
    if !verify_origin_and_host(&headers, state.lan_allowed) {
        return forbidden_response("Origin/Host không hợp lệ.");
    }

    let token = match extract_session_token(&headers) {
        Some(t) => t,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };

    if state.sessions.validate(&token).is_none() {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    create_event_stream(state.events_tx.subscribe()).into_response()
}

// ──────────────────────────────── Upload Route ───────────────────────────────

async fn upload_handler(
    State(state): State<Arc<WebServerState>>,
    headers: HeaderMap,
    multipart: axum::extract::Multipart,
) -> Response {
    if !verify_origin_and_host(&headers, state.lan_allowed) {
        return forbidden_response("Origin/Host không hợp lệ.");
    }
    if !verify_csrf_header(&Method::POST, &headers) {
        return forbidden_response("Yêu cầu header CSRF.");
    }

    let token = match extract_session_token(&headers) {
        Some(t) => t,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };

    if state.sessions.validate(&token).is_none() {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    handle_upload(multipart).await
}

// ──────────────────────────────── QR Route ───────────────────────────────────

async fn qr_handler(State(state): State<Arc<WebServerState>>) -> Response {
    let url = format_lan_url(state.port);
    match generate_qr_svg(&url) {
        Ok(svg) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "image/svg+xml")
            .body(svg.into())
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e })),
        )
            .into_response(),
    }
}

// ──────────────────────────── Static & SPA Fallback ──────────────────────────

async fn static_or_spa_fallback(
    _headers: HeaderMap,
    axum::extract::OriginalUri(uri): axum::extract::OriginalUri,
) -> Response {
    let path = uri.path().trim_start_matches('/');

    // Try serving exact file
    if !path.is_empty() {
        if let Some(file) = UI_DIST.get_file(path) {
            let mime = file::guess_mime(std::path::Path::new(path));
            return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime)
                .body(file.contents().to_vec().into())
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
    }

    // SPA fallback: index.html
    if let Some(index) = UI_DIST.get_file("index.html") {
        let content_str = String::from_utf8_lossy(index.contents());
        let is_placeholder = !content_str.contains("<script")
            || content_str.trim()
                == "<!DOCTYPE html><html><head><title>Local Content Studio</title></head><body><div id=\"root\"></div></body></html>";
        if is_placeholder {
            return (
                StatusCode::OK,
                Html(
                    "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>Local Content Studio</title></head><body style=\"font-family: sans-serif; display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; background: #0f172a; color: #f8fafc;\"><div style=\"text-align: center; padding: 2rem; border: 1px solid #334155; border-radius: 8px; background: #1e293b;\"><h1 style=\"font-size: 1.5rem; margin-bottom: 0.5rem;\">Chưa build giao diện web — chạy pnpm build</h1><p style=\"color: #94a3b8;\">Vui lòng chạy lệnh <code>pnpm --filter @studio/desktop-ui build</code> để tạo bản build cho giao diện.</p></div></body></html>",
                ),
            )
                .into_response();
        }

        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(index.contents().to_vec().into())
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
    } else {
        (
            StatusCode::NOT_FOUND,
            Html(
                "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>Local Content Studio</title></head><body style=\"font-family: sans-serif; display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; background: #0f172a; color: #f8fafc;\"><div style=\"text-align: center; padding: 2rem; border: 1px solid #334155; border-radius: 8px; background: #1e293b;\"><h1 style=\"font-size: 1.5rem; margin-bottom: 0.5rem;\">Chưa build giao diện web — chạy pnpm build</h1></div></body></html>",
            ),
        )
            .into_response()
    }
}
