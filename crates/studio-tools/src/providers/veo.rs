//! Google Veo video generation provider via Gemini API (`generativelanguage.googleapis.com`).

use crate::error::{Result, ToolError};
use crate::providers::http::{
    create_http_client, save_base64_to_file, send_with_retries, stream_download_to_file, MediaKind,
};
use crate::providers::imagen::to_workspace_relative;
use base64::prelude::*;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::Method;
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;
use studio_workspace::Workspace;

/// Run Google Veo video generation (`video.generate@1`).
pub async fn run_google_veo(
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

    let duration_s = input
        .get("duration_s")
        .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
        .unwrap_or(5.0);

    let aspect = input
        .get("aspect")
        .and_then(Value::as_str)
        .unwrap_or("16:9");

    // Never from tool input: the agent must not choose where the API key is sent.
    let base_url = base_override
        .map(String::from)
        .or_else(|| std::env::var("GEMINI_API_BASE_URL").ok())
        .unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta".to_string());

    let base_trimmed = base_url.trim_end_matches('/');
    let endpoint = if base_trimmed.ends_with("/v1beta") || base_trimmed.ends_with("/v1") {
        format!(
            "{}/models/veo-2.0-generate-001:predictLongRunning",
            base_trimmed
        )
    } else {
        format!(
            "{}/v1beta/models/veo-2.0-generate-001:predictLongRunning",
            base_trimmed
        )
    };

    let mut instance = json!({
        "prompt": prompt
    });

    if let Some(ref_img) = input.get("reference_image").and_then(Value::as_str) {
        let ref_path = ws.scoped_path(ref_img).map_err(ToolError::WorkspaceError)?;
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

    let request_body = json!({
        "instances": [instance],
        "parameters": {
            "aspectRatio": aspect,
            "durationSeconds": duration_s
        }
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

    // Step 1: Submit predictLongRunning
    let resp = send_with_retries("provider:google", "google.veo", || {
        client
            .request(Method::POST, &endpoint)
            .headers(headers.clone())
            .body(body_bytes.clone())
    })
    .await?;

    let resp_bytes = resp.bytes().await.map_err(|e| ToolError::ExecutionFailed {
        tool_id: "google.veo".into(),
        reason: format!("Lỗi đọc phản hồi từ Veo API: {e}"),
    })?;

    let init_json: Value =
        serde_json::from_slice(&resp_bytes).map_err(|e| ToolError::ExecutionFailed {
            tool_id: "google.veo".into(),
            reason: format!("Phản hồi khởi tạo Veo không phải JSON hợp lệ: {e}"),
        })?;

    let op_name = init_json
        .get("name")
        .or_else(|| init_json.get("operation"))
        .and_then(Value::as_str)
        .ok_or_else(|| ToolError::ExecutionFailed {
            tool_id: "google.veo".into(),
            reason: format!(
                "Phản hồi từ Veo API không chứa tên tác vụ 'name': {}",
                init_json
            ),
        })?;

    let poll_url = if op_name.starts_with("http://") || op_name.starts_with("https://") {
        op_name.to_string()
    } else if base_trimmed.ends_with("/v1beta") || base_trimmed.ends_with("/v1") {
        format!("{}/{}", base_trimmed, op_name.trim_start_matches('/'))
    } else {
        format!(
            "{}/v1beta/{}",
            base_trimmed,
            op_name.trim_start_matches('/')
        )
    };

    let poll_interval_ms = input
        .get("poll_interval_ms")
        .and_then(Value::as_u64)
        .unwrap_or(
            if cfg!(test) || std::env::var("STUDIO_TEST_FAST_RETRY").is_ok() {
                20
            } else {
                2000
            },
        );
    // Against the real API (no host override) never poll faster than every 2 s.
    let poll_interval_ms = if base_override.is_none() {
        poll_interval_ms.max(2000)
    } else {
        poll_interval_ms
    };

    // Step 2: Poll operation until done (up to 90 attempts)
    let max_polls = 90;
    let mut operation_response = None;

    for _ in 0..max_polls {
        tokio::time::sleep(Duration::from_millis(poll_interval_ms)).await;

        let poll_resp = send_with_retries("provider:google", "google.veo", || {
            client
                .request(Method::GET, &poll_url)
                .headers(headers.clone())
        })
        .await?;

        let poll_bytes = poll_resp
            .bytes()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool_id: "google.veo".into(),
                reason: format!("Lỗi đọc trạng thái tác vụ Veo: {e}"),
            })?;

        let poll_json: Value =
            serde_json::from_slice(&poll_bytes).map_err(|e| ToolError::ExecutionFailed {
                tool_id: "google.veo".into(),
                reason: format!("Phản hồi thăm dò Veo không phải JSON hợp lệ: {e}"),
            })?;

        if let Some(err) = poll_json.get("error") {
            return Err(ToolError::ExecutionFailed {
                tool_id: "google.veo".into(),
                reason: format!("Tác vụ tạo video Veo báo lỗi: {}", err),
            });
        }

        let is_done = poll_json
            .get("done")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        if is_done {
            operation_response = Some(poll_json);
            break;
        }
    }

    let final_op = operation_response.ok_or_else(|| ToolError::ExecutionFailed {
        tool_id: "google.veo".into(),
        reason: "Quá thời gian chờ tạo video Veo (polling timeout)".into(),
    })?;

    // Step 3: Extract video payload
    let resp_obj = final_op.get("response").unwrap_or(&final_op);

    let (video_uri, video_b64) =
        extract_video_target(resp_obj).ok_or_else(|| ToolError::ExecutionFailed {
            tool_id: "google.veo".into(),
            reason: format!(
                "Không tìm thấy URI hoặc dữ liệu video Base64 trong kết quả: {}",
                final_op
            ),
        })?;

    let dest = out_dir.join("veo-0.mp4");
    let final_path = if let Some(b64) = video_b64 {
        let (_, ext) = save_base64_to_file("google.veo", &b64, &dest, MediaKind::Video).await?;
        if ext != "mp4" {
            let renamed = out_dir.join(format!("veo-0.{}", ext));
            tokio::fs::rename(&dest, &renamed)
                .await
                .map_err(|e| ToolError::Io {
                    path: renamed.clone(),
                    source: e,
                })?;
            renamed
        } else {
            dest
        }
    } else if let Some(uri) = video_uri {
        let mut download_headers = HeaderMap::new();
        // Send the key only to the exact API origin (scheme + host + port), never to other hosts.
        if super::http::same_origin(&uri, &base_url) {
            download_headers = headers.clone();
        }
        let (_, ext) = stream_download_to_file(
            &client,
            "provider:google",
            "google.veo",
            &uri,
            download_headers,
            &dest,
            MediaKind::Video,
        )
        .await?;

        if ext != "mp4" {
            let renamed = out_dir.join(format!("veo-0.{}", ext));
            tokio::fs::rename(&dest, &renamed)
                .await
                .map_err(|e| ToolError::Io {
                    path: renamed.clone(),
                    source: e,
                })?;
            renamed
        } else {
            dest
        }
    } else {
        return Err(ToolError::ExecutionFailed {
            tool_id: "google.veo".into(),
            reason: "Không có dữ liệu video để lưu".into(),
        });
    };

    let rel_path = to_workspace_relative(ws, &final_path);

    Ok(json!({
        "files": [rel_path.clone()],
        "video_path": rel_path,
        "provider_metadata": {
            "provider": "google.veo",
            "model": "veo-2.0-generate-001",
            "operation_name": op_name,
            "duration_s": duration_s,
            "aspect": aspect,
            "estimated_cost_usd": 0.20
        }
    }))
}

fn extract_video_target(resp: &Value) -> Option<(Option<String>, Option<String>)> {
    // Check generateVideoResponse.generatedVideos[0].video
    if let Some(videos) = resp
        .get("generateVideoResponse")
        .and_then(|g| g.get("generatedVideos"))
        .and_then(Value::as_array)
    {
        if let Some(v) = videos.first().and_then(|item| item.get("video")) {
            if let Some(uri) = v.get("uri").and_then(Value::as_str) {
                return Some((Some(uri.to_string()), None));
            }
            if let Some(b64) = v.get("bytesBase64Encoded").and_then(Value::as_str) {
                return Some((None, Some(b64.to_string())));
            }
        }
    }

    // Check generatedVideos[0].video
    if let Some(videos) = resp.get("generatedVideos").and_then(Value::as_array) {
        if let Some(v) = videos.first().and_then(|item| item.get("video")) {
            if let Some(uri) = v.get("uri").and_then(Value::as_str) {
                return Some((Some(uri.to_string()), None));
            }
            if let Some(b64) = v.get("bytesBase64Encoded").and_then(Value::as_str) {
                return Some((None, Some(b64.to_string())));
            }
        }
    }

    // Check video.uri or video.bytesBase64Encoded
    if let Some(v) = resp.get("video") {
        if let Some(uri) = v.get("uri").and_then(Value::as_str) {
            return Some((Some(uri.to_string()), None));
        }
        if let Some(b64) = v.get("bytesBase64Encoded").and_then(Value::as_str) {
            return Some((None, Some(b64.to_string())));
        }
    }

    // Direct uri field
    if let Some(uri) = resp
        .get("uri")
        .or_else(|| resp.get("outputUri"))
        .and_then(Value::as_str)
    {
        return Some((Some(uri.to_string()), None));
    }

    None
}
