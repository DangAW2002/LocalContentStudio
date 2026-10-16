//! stdio ⇄ HTTP shim for engines that only support globally configured stdio MCP servers (agy, ADR-008 §4).

use std::path::PathBuf;

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
pub async fn run(runtime_path: PathBuf) -> std::io::Result<()> {
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
            let resolved = match crate::runtime::read(&runtime_path) {
                Ok(info) => fetch_token(&client, &info, &dirs)
                    .await
                    .map(|t| (info.port, t)),
                Err(_) => Err("Studio chưa chạy.".to_string()),
            };
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
