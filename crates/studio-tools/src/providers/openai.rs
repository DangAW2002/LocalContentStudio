//! OpenAI Images provider implementation via `api.openai.com/v1/images/generations`.

use crate::error::{Result, ToolError};
use crate::providers::http::{
    create_http_client, save_base64_to_file, send_with_retries, stream_download_to_file, MediaKind,
};
use crate::providers::imagen::to_workspace_relative;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Method;
use serde_json::{json, Value};
use std::path::Path;
use studio_workspace::Workspace;

/// Run OpenAI images generation (`gpt-image-1` / DALL-E).
pub async fn run_openai_images(
    ws: &Workspace,
    out_dir: &Path,
    input: &Value,
    api_key: &str,
    base_override: Option<&str>,
    pricing: Option<&crate::pricing::ProviderPricing>,
) -> Result<Value> {
    let prompt = input
        .get("prompt")
        .and_then(Value::as_str)
        .ok_or_else(|| ToolError::ParseError("Thiếu tham số 'prompt'".into()))?;

    let width = input.get("width").and_then(Value::as_u64).unwrap_or(1024);
    let height = input.get("height").and_then(Value::as_u64).unwrap_or(1024);
    let sample_count = input
        .get("n")
        .and_then(Value::as_u64)
        .unwrap_or(1)
        .clamp(1, 10);
    let model = input
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("gpt-image-1");

    let size = select_openai_size(width, height);

    // Never from tool input: the agent must not choose where the API key is sent.
    let base_url = base_override
        .map(String::from)
        .or_else(|| std::env::var("OPENAI_API_BASE_URL").ok())
        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

    let endpoint = format!("{}/images/generations", base_url.trim_end_matches('/'));

    let request_body = json!({
        "model": model,
        "prompt": prompt,
        "n": sample_count,
        "size": size,
        "response_format": "b64_json"
    });

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key.trim())).map_err(|_| {
            ToolError::SecretStoreError("Khóa OPENAI_API_KEY chứa ký tự không hợp lệ".into())
        })?,
    );

    let client = create_http_client();
    let body_bytes =
        serde_json::to_vec(&request_body).map_err(|e| ToolError::ParseError(e.to_string()))?;

    let resp = send_with_retries("provider:openai", "openai.images", || {
        client
            .request(Method::POST, &endpoint)
            .headers(headers.clone())
            .body(body_bytes.clone())
    })
    .await?;

    let resp_bytes = resp.bytes().await.map_err(|e| ToolError::ExecutionFailed {
        tool_id: "openai.images".into(),
        reason: format!("Lỗi đọc phản hồi từ OpenAI API: {e}"),
    })?;

    let resp_json: Value =
        serde_json::from_slice(&resp_bytes).map_err(|e| ToolError::ExecutionFailed {
            tool_id: "openai.images".into(),
            reason: format!("Phản hồi từ OpenAI API không phải JSON hợp lệ: {e}"),
        })?;

    let data = resp_json
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| ToolError::ExecutionFailed {
            tool_id: "openai.images".into(),
            reason: format!(
                "Phản hồi từ OpenAI API không chứa trường 'data': {}",
                resp_json
            ),
        })?;

    let mut saved_files = Vec::new();
    for (i, item) in data.iter().enumerate() {
        if let Some(b64) = item.get("b64_json").and_then(Value::as_str) {
            let dest = out_dir.join(format!("openai-{}.png", i));
            let (_, ext) =
                save_base64_to_file("openai.images", b64, &dest, MediaKind::Image).await?;

            let final_path = if ext != "png" {
                let renamed = out_dir.join(format!("openai-{}.{}", i, ext));
                tokio::fs::rename(&dest, &renamed)
                    .await
                    .map_err(|e| ToolError::Io {
                        path: renamed.clone(),
                        source: e,
                    })?;
                renamed
            } else {
                dest
            };

            saved_files.push(to_workspace_relative(ws, &final_path));
        } else if let Some(url) = item.get("url").and_then(Value::as_str) {
            let dest = out_dir.join(format!("openai-{}.png", i));
            let mut download_headers = HeaderMap::new();
            if crate::providers::http::same_origin(url, &base_url) {
                download_headers.insert(
                    AUTHORIZATION,
                    HeaderValue::from_str(&format!("Bearer {}", api_key.trim())).map_err(|_| {
                        ToolError::SecretStoreError(
                            "Khóa OPENAI_API_KEY chứa ký tự không hợp lệ".into(),
                        )
                    })?,
                );
            }
            let (_, ext) = stream_download_to_file(
                &client,
                "provider:openai",
                "openai.images",
                url,
                download_headers,
                &dest,
                MediaKind::Image,
            )
            .await?;

            let final_path = if ext != "png" {
                let renamed = out_dir.join(format!("openai-{}.{}", i, ext));
                tokio::fs::rename(&dest, &renamed)
                    .await
                    .map_err(|e| ToolError::Io {
                        path: renamed.clone(),
                        source: e,
                    })?;
                renamed
            } else {
                dest
            };

            saved_files.push(to_workspace_relative(ws, &final_path));
        } else {
            return Err(ToolError::ExecutionFailed {
                tool_id: "openai.images".into(),
                reason: "Mục trong 'data' không chứa trường 'b64_json' hoặc 'url'".into(),
            });
        }
    }

    let (unit_cost, is_default) = pricing
        .map(|p| p.resolve_image_cost(Some(model)))
        .unwrap_or((crate::pricing::DEFAULT_IMAGE_USD, true));
    let estimated_cost = unit_cost * (saved_files.len().max(1) as f64);

    Ok(json!({
        "files": saved_files,
        "provider_metadata": {
            "provider": "openai.images",
            "model": model,
            "count": saved_files.len(),
            "size": size,
            "estimated_cost_usd": estimated_cost,
            "cost_is_default": is_default
        }
    }))
}

fn select_openai_size(width: u64, height: u64) -> &'static str {
    if width == height {
        "1024x1024"
    } else if width > height {
        "1792x1024"
    } else {
        "1024x1792"
    }
}
