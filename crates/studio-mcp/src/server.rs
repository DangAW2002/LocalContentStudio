//! Streamable HTTP MCP server (JSON responses) on loopback, bearer token per turn (ADR-008 §4).

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{json, Value};
use studio_jobs::Scheduler;
use studio_tools::SecretStore;

use crate::permission::PermissionBroker;
use crate::tokens::{TokenRegistry, TurnContext};

/// MCP protocol version announced when the client does not request one.
pub const DEFAULT_PROTOCOL: &str = "2025-06-18";

/// Shared state of the server.
pub struct ServerState {
    /// Active turn tokens.
    pub tokens: Arc<TokenRegistry>,
    /// Machine-wide job scheduler.
    pub scheduler: Arc<Scheduler>,
    /// Secret store (Credential Manager in the app).
    pub secrets: Arc<dyn SecretStore>,
    /// Permission broker (UI dialog in the app).
    pub broker: Arc<dyn PermissionBroker>,
    /// Catalog layer files (app → org → workspace).
    pub catalog_paths: Vec<PathBuf>,
    /// Secret the stdio shim must present to obtain a turn token.
    pub shim_secret: String,
    /// Host-set API endpoint overrides by tool id.
    pub endpoints: Arc<tokio::sync::RwLock<std::collections::HashMap<String, String>>>,
}

/// Handle of a running server.
pub struct RunningServer {
    /// Bound address (127.0.0.1:port).
    pub addr: SocketAddr,
    /// Stops the server when sent or dropped.
    pub shutdown: tokio::sync::oneshot::Sender<()>,
}

/// Start the server on `127.0.0.1:port` (0 = random port).
pub async fn start(state: Arc<ServerState>, port: u16) -> std::io::Result<RunningServer> {
    let app = Router::new()
        .route("/mcp", post(mcp_handler).get(method_not_allowed))
        .route("/shim/token", post(shim_token))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    let addr = listener.local_addr()?;
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = rx.await;
            })
            .await;
    });
    Ok(RunningServer { addr, shutdown: tx })
}

async fn method_not_allowed() -> StatusCode {
    StatusCode::METHOD_NOT_ALLOWED
}

/// Reject browsers on other origins (DNS rebinding / CSRF) and non-loopback Host headers.
fn origin_ok(headers: &HeaderMap) -> bool {
    let local = |v: &str| {
        let v = v
            .trim_start_matches("http://")
            .trim_start_matches("https://");
        let host = v.split(['/', ':']).next().unwrap_or("");
        host == "127.0.0.1" || host == "localhost"
    };
    let origin_ok = headers
        .get("origin")
        .and_then(|v| v.to_str().ok())
        .is_none_or(local);
    let host_ok = headers
        .get("host")
        .and_then(|v| v.to_str().ok())
        .is_none_or(local);
    origin_ok && host_ok
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}

async fn mcp_handler(
    State(state): State<Arc<ServerState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    if !origin_ok(&headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Some(ctx) = bearer(&headers).and_then(|t| state.tokens.get(t)) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let out = match body {
        Value::Array(batch) => {
            let mut results = Vec::new();
            for msg in batch {
                if let Some(r) = handle_message(&state, &ctx, msg).await {
                    results.push(r);
                }
            }
            if results.is_empty() {
                return StatusCode::ACCEPTED.into_response();
            }
            Value::Array(results)
        }
        msg => match handle_message(&state, &ctx, msg).await {
            Some(r) => r,
            None => return StatusCode::ACCEPTED.into_response(),
        },
    };
    Json(out).into_response()
}

/// Handle one JSON-RPC message; `None` for notifications.
pub async fn handle_message(state: &ServerState, ctx: &TurnContext, msg: Value) -> Option<Value> {
    let id = msg.get("id").cloned()?;
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    let ok = |result: Value| json!({ "jsonrpc": "2.0", "id": id, "result": result });
    Some(match method {
        "initialize" => ok(json!({
            "protocolVersion": params.get("protocolVersion").and_then(Value::as_str).unwrap_or(DEFAULT_PROTOCOL),
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "studio", "version": env!("CARGO_PKG_VERSION") },
            "instructions": "Local Content Studio: dùng các tool này để đọc Brand/kênh/playbook, chạy capability và kiểm sản phẩm. Tệp nằm trong workspace hiện tại."
        })),
        "ping" => ok(json!({})),
        "tools/list" => ok(json!({ "tools": crate::tools::definitions() })),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            match crate::tools::call(state, ctx, name, &args).await {
                Ok(v) => ok(json!({ "content": [{ "type": "text", "text": v.to_string() }] })),
                Err(e) => {
                    ok(json!({ "content": [{ "type": "text", "text": e }], "isError": true }))
                }
            }
        }
        other => {
            json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32601, "message": format!("method not found: {other}") } })
        }
    })
}

async fn shim_token(
    State(state): State<Arc<ServerState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    if !origin_ok(&headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    if body.get("secret").and_then(Value::as_str) != Some(state.shim_secret.as_str()) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let dirs: Vec<PathBuf> = body
        .get("dirs")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(PathBuf::from)
                .collect()
        })
        .unwrap_or_default();
    match dirs
        .iter()
        .find_map(|d| state.tokens.active_agy_token_for(d))
    {
        Some(token) => Json(json!({ "token": token })).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Không có lượt agy đang hoạt động cho workspace này." })),
        )
            .into_response(),
    }
}
