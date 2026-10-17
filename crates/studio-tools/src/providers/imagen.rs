//! Google Imagen provider implementation via Gemini API (`generativelanguage.googleapis.com`).

use crate::error::{Result, ToolError};
use crate::providers::http::{
    create_http_client, save_base64_to_file, send_with_retries, MediaKind,
};
use base64::prelude::*;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::Method;
use serde_json::{json, Value};
use std::path::Path;
use studio_workspace::Workspace;

/// Run Google Imagen generation.
pub async fn run_google_imagen(
    ws: &Workspace,
    out_dir: &Path,
    input: &Value,
    api_key: &str,
    base_override: Option<&str>,
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
        .clamp(1, 8);
    let negative_prompt = input.get("negative_prompt").and_then(Value::as_str);
    let seed = input.get("seed").and_then(Value::as_u64);

    let aspect_ratio = calculate_aspect_ratio(width, height);

    // Build base URL
    // Never from tool input: the agent must not choose where the API key is sent.
    let base_url = base_override
        .map(String::from)
        .or_else(|| std::env::var("GEMINI_API_BASE_URL").ok())
        .unwrap_or_else(|| "https://generativelanguage.googleapis.com".to_string());

    let endpoint = format!(
        "{}/v1beta/models/imagen-3.0-generate-002:predict",
        base_url.trim_end_matches('/')
    );

    // Build instance
    let mut instance = json!({
        "prompt": prompt
    });

    // Reference images (if any)
    if let Some(ref_images) = input.get("reference_images").and_then(Value::as_array) {
        if let Some(first_ref) = ref_images.first().and_then(Value::as_str) {
            let ref_path = ws
                .scoped_path(first_ref)
                .map_err(ToolError::WorkspaceError)?;
            if ref_path.is_file() {
                let bytes = tokio::fs::read(&ref_path)
                    .await
                    .map_err(|e| ToolError::Io {
                        path: ref_path.clone(),
                        source: e,
                    })?;
                let b64 = BASE64_STANDARD.encode(&bytes);
                instance["image"] = json!({ "bytesBase64Encoded": b64 });
            }
        }
    }

    let mut parameters = json!({
        "sampleCount": sample_count,
        "aspectRatio": aspect_ratio,
        "outputMimeType": "image/jpeg"
    });

    if let Some(neg) = negative_prompt {
        parameters["negativePrompt"] = json!(neg);
    }
    if let Some(s) = seed {
        parameters["seed"] = json!(s);
    }

    let request_body = json!({
        "instances": [instance],
        "parameters": parameters
    });

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        "x-goog-api-key",
        HeaderValue::from_str(api_key).map_err(|_| {
            ToolError::SecretStoreError("Khóa GEMINI_API_KEY chứa ký tự không hợp lệ".into())
        })?,
    );

    let client = create_http_client();
    let body_bytes =
        serde_json::to_vec(&request_body).map_err(|e| ToolError::ParseError(e.to_string()))?;

    let resp = send_with_retries("provider:google", "google.imagen", || {
        client
            .request(Method::POST, &endpoint)
            .headers(headers.clone())
            .body(body_bytes.clone())
    })
    .await?;

    let resp_bytes = resp.bytes().await.map_err(|e| ToolError::ExecutionFailed {
        tool_id: "google.imagen".into(),
        reason: format!("Lỗi đọc phản hồi từ Gemini API: {e}"),
    })?;

    let resp_json: Value =
        serde_json::from_slice(&resp_bytes).map_err(|e| ToolError::ExecutionFailed {
            tool_id: "google.imagen".into(),
            reason: format!("Phản hồi từ Gemini API không phải JSON hợp lệ: {e}"),
        })?;

    let predictions = resp_json
        .get("predictions")
        .and_then(Value::as_array)
        .ok_or_else(|| ToolError::ExecutionFailed {
            tool_id: "google.imagen".into(),
            reason: format!(
                "Phản hồi từ Gemini API không chứa danh sách 'predictions': {}",
                resp_json
            ),
        })?;

    let mut saved_files = Vec::new();
    for (i, pred) in predictions.iter().enumerate() {
        let b64 = pred
            .get("bytesBase64Encoded")
            .or_else(|| {
                pred.get("image")
                    .and_then(|img| img.get("bytesBase64Encoded"))
            })
            .or_else(|| pred.get("b64_json"))
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::ExecutionFailed {
                tool_id: "google.imagen".into(),
                reason: "Không tìm thấy dữ liệu ảnh Base64 trong prediction".into(),
            })?;

        let dest = out_dir.join(format!("imagen-{}.jpg", i));
        let (_, ext) = save_base64_to_file("google.imagen", b64, &dest, MediaKind::Image).await?;

        let final_path = if ext != "jpg" {
            let renamed = out_dir.join(format!("imagen-{}.{}", i, ext));
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

        let rel_path = to_workspace_relative(ws, &final_path);
        saved_files.push(rel_path);
    }

    let estimated_cost = 0.04 * (saved_files.len().max(1) as f64);

    Ok(json!({
        "files": saved_files,
        "provider_metadata": {
            "provider": "google.imagen",
            "model": "imagen-3.0-generate-002",
            "count": saved_files.len(),
            "aspect_ratio": aspect_ratio,
            "estimated_cost_usd": estimated_cost
        }
    }))
}

fn calculate_aspect_ratio(width: u64, height: u64) -> &'static str {
    if width == height {
        return "1:1";
    }

    let ratio = (width as f64) / (height as f64);
    if ratio > 1.0 {
        if ratio >= 1.55 {
            "16:9"
        } else {
            "4:3"
        }
    } else {
        let inv = (height as f64) / (width as f64);
        if inv >= 1.55 {
            "9:16"
        } else {
            "3:4"
        }
    }
}

pub fn to_workspace_relative(ws: &Workspace, path: &Path) -> String {
    path.strip_prefix(ws.root())
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
