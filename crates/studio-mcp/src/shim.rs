//! stdio ⇄ HTTP shim for engines that only support globally configured stdio MCP servers (agy, ADR-008 §4).

use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::runtime::RuntimeInfo;

fn rpc_error(id: Value, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32000, "message": message } })
}

/// Obtain the token of the active agy turn whose workspace contains one of `dirs`.
pub async fn fetch_token(
    client: &reqwest::Client,
    info: &RuntimeInfo,
    dirs: &[PathBuf],
) -> Result<String, String> {
    let url = format!("http://127.0.0.1:{}/shim/token", info.port);
    let dirs: Vec<String> = dirs
        .iter()
        .map(|d| d.to_string_lossy().to_string())
        .collect();
    let resp = client
        .post(url)
        .json(&json!({ "secret": info.shim_secret, "dirs": dirs }))
        .send()
        .await
        .map_err(|_| "Studio chưa chạy.".to_string())?;
    let body: Value = resp.json().await.map_err(|e| e.to_string())?;
    body.get("token")
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| "Không có lượt agy đang hoạt động trong workspace này.".to_string())
}

/// Discover running Studio instance and fetch active token using provided search directories and liveness function.
/// Order of discovery:
/// 1. `explicit_path` (if provided) and/or `STUDIO_APP_DIR` runtime.json
/// 2. default runtime.json (`%APPDATA%/LocalContentStudio/runtime.json`)
/// 3. every `runtime.d` entry whose process is alive. Stale entries are ignored/cleaned.
///
/// Returns the first instance that returns a valid token for `dirs`.
pub async fn discover_and_fetch_token_in<F>(
    client: &reqwest::Client,
    explicit_path: Option<&Path>,
    studio_app_dir: Option<&Path>,
    default_runtime_path: Option<&Path>,
    runtime_d_dir: Option<&Path>,
    dirs: &[PathBuf],
    is_alive: F,
) -> Result<(u16, String), String>
where
    F: Fn(u32) -> bool,
{
    let mut attempted_paths: Vec<PathBuf> = Vec::new();
    let mut last_error: Option<String> = None;

    // 1. Explicit path (e.g. CLI argument)
    if let Some(path) = explicit_path {
        attempted_paths.push(path.to_path_buf());
        if let Ok(info) = crate::runtime::read_entry(path) {
            match fetch_token(client, &info, dirs).await {
                Ok(tok) => return Ok((info.port, tok)),
                Err(e) => last_error = Some(e),
            }
        }
    }

    // 1b. STUDIO_APP_DIR runtime.json
    if let Some(app_dir) = studio_app_dir {
        let path = app_dir.join("runtime.json");
        if !attempted_paths.contains(&path) {
            attempted_paths.push(path.clone());
            if let Ok(info) = crate::runtime::read_entry(&path) {
                match fetch_token(client, &info, dirs).await {
                    Ok(tok) => return Ok((info.port, tok)),
                    Err(e) => last_error = Some(e),
                }
            }
        }
    }

    // 2. Default runtime.json
    if let Some(def_path) = default_runtime_path {
        if !attempted_paths.contains(&def_path.to_path_buf()) {
            attempted_paths.push(def_path.to_path_buf());
            if let Ok(info) = crate::runtime::read_entry(def_path) {
                match fetch_token(client, &info, dirs).await {
                    Ok(tok) => return Ok((info.port, tok)),
                    Err(e) => last_error = Some(e),
                }
            }
        }
    }

    // 3. runtime.d directory entries
    if let Some(d_dir) = runtime_d_dir {
        if d_dir.exists() && d_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(d_dir) {
                let mut files: Vec<PathBuf> = entries
                    .flatten()
                    .map(|e| e.path())
                    .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
                    .collect();
                files.sort();

                let mut alive_entries = Vec::new();
                for file_path in files {
                    let file_pid = file_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .and_then(|s| s.parse::<u32>().ok());

                    match crate::runtime::read_entry(&file_path) {
                        Ok(info) => {
                            let pid = info.pid;
                            if !is_alive(pid) {
                                // Stale entry: clean it up
                                let _ = std::fs::remove_file(&file_path);
                            } else {
                                alive_entries.push(info);
                            }
                        }
                        Err(_) => {
                            // If unreadable, check if file_pid is dead to clean it
                            if let Some(pid) = file_pid {
                                if !is_alive(pid) {
                                    let _ = std::fs::remove_file(&file_path);
                                }
                            }
                        }
                    }
                }

                for info in alive_entries {
                    match fetch_token(client, &info, dirs).await {
                        Ok(tok) => return Ok((info.port, tok)),
                        Err(e) => last_error = Some(e),
                    }
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| "Studio chưa chạy.".to_string()))
}

/// Discover running Studio instance and fetch active token using system defaults.
pub async fn discover_and_fetch_token<F>(
    client: &reqwest::Client,
    explicit_path: Option<&Path>,
    dirs: &[PathBuf],
    is_alive: F,
) -> Result<(u16, String), String>
where
    F: Fn(u32) -> bool,
{
    let studio_app_dir = std::env::var("STUDIO_APP_DIR").ok().map(PathBuf::from);
    let def_path = crate::runtime::fixed_default_path();
    let d_dir = crate::runtime::runtime_d_dir();
    discover_and_fetch_token_in(
        client,
        explicit_path,
        studio_app_dir.as_deref(),
        Some(&def_path),
        Some(&d_dir),
        dirs,
        is_alive,
    )
    .await
}

/// Forward one JSON-RPC message; `None` for notifications.
pub async fn forward(
    client: &reqwest::Client,
    port: u16,
    token: &str,
    msg: &Value,
) -> Option<Value> {
    let id = msg.get("id").cloned();
    let resp = client
        .post(format!("http://127.0.0.1:{port}/mcp"))
        .bearer_auth(token)
        .json(msg)
        .send()
        .await;
    let id = id?;
    match resp {
        Ok(r) if r.status().is_success() => r
            .json()
            .await
            .ok()
            .or_else(|| Some(rpc_error(id, "Phản hồi không hợp lệ từ Studio."))),
        Ok(r) => Some(rpc_error(
            id,
            &format!("Studio từ chối yêu cầu ({}).", r.status()),
        )),
        Err(_) => Some(rpc_error(id, "Studio chưa chạy.")),
    }
}

/// Run the shim on stdin/stdout until EOF.
pub async fn run(explicit_path: impl Into<Option<PathBuf>>) -> std::io::Result<()> {
    let explicit_path = explicit_path.into();
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(std::io::Error::other)?;
    let mut dirs = vec![std::env::current_dir()?];
    let mut token: Option<(u16, String)> = None;
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    let mut stdout = tokio::io::stdout();

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(msg) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        // Workspace roots announced by the client take precedence over cwd.
        if msg.get("method").and_then(Value::as_str) == Some("initialize") {
            if let Some(roots) = msg.pointer("/params/roots").and_then(Value::as_array) {
                let mut announced: Vec<PathBuf> = roots
                    .iter()
                    .filter_map(|r| r.get("uri").and_then(Value::as_str))
                    .map(|u| {
                        PathBuf::from(
                            u.trim_start_matches("file:///")
                                .trim_start_matches("file://"),
                        )
                    })
                    .collect();
                announced.append(&mut dirs);
                dirs = announced;
            }
        }
        if token.is_none() {
            let resolved = discover_and_fetch_token(
                &client,
                explicit_path.as_deref(),
                &dirs,
                crate::runtime::is_process_alive,
            )
            .await;
            match resolved {
                Ok(t) => token = Some(t),
                Err(e) => {
                    if let Some(id) = msg.get("id").cloned() {
                        let out = rpc_error(id, &e).to_string() + "\n";
                        stdout.write_all(out.as_bytes()).await?;
                        stdout.flush().await?;
                    }
                    continue;
                }
            }
        }
        if let Some((port, t)) = &token {
            if let Some(reply) = forward(&client, *port, t, &msg).await {
                stdout
                    .write_all((reply.to_string() + "\n").as_bytes())
                    .await?;
                stdout.flush().await?;
            }
        }
    }
    Ok(())
}
