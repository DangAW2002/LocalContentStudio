//! OpenAI Videos provider implementation via `POST {base}/videos` and `GET {base}/videos/{id}`.

use crate::error::{Result, ToolError};
use crate::providers::http::{
    create_http_client, same_origin, send_with_retries, stream_download_to_file, MediaKind,
};
use crate::providers::imagen::to_workspace_relative;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Method;
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;
use studio_workspace::Workspace;

/// Run OpenAI video generation (`video.generate@1`).
pub async fn run_openai_videos(
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

    let model = input
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("sora-2");

    let seconds = input
        .get("duration_s")
        .and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)))
        .or_else(|| input.get("seconds").and_then(Value::as_u64))
        .unwrap_or(5);

    let size = input
        .get("size")
        .and_then(Value::as_str)
        .or_else(|| {
            input
                .get("aspect")
                .and_then(Value::as_str)
                .map(|aspect| match aspect {
                    "9:16" => "720x1280",
                    "1:1" => "1024x1024",
                    _ => "1280x720",
                })
        })
        .unwrap_or("1280x720");

    // Never from tool input: the agent must not choose where the API key is sent.
    let base_url = base_override
        .map(String::from)
        .or_else(|| std::env::var("OPENAI_API_BASE_URL").ok())
        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

    let endpoint = format!("{}/videos", base_url.trim_end_matches('/'));

    let mut request_body = json!({
        "model": model,
        "prompt": prompt,
        "seconds": seconds,
        "size": size
    });

    // Also support passing duration_s if required by some gateways
    if let Some(obj) = request_body.as_object_mut() {
        obj.insert("duration_s".to_string(), json!(seconds));
    }

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

    // Step 1: POST {base}/videos
    let resp = send_with_retries("provider:openai", "openai.videos", || {
        client
            .request(Method::POST, &endpoint)
            .headers(headers.clone())
            .body(body_bytes.clone())
    })
    .await?;

    let resp_bytes = resp.bytes().await.map_err(|e| ToolError::ExecutionFailed {
        tool_id: "openai.videos".into(),
        reason: format!("Lỗi đọc phản hồi từ OpenAI Videos API: {e}"),
    })?;

    let resp_json: Value =
        serde_json::from_slice(&resp_bytes).map_err(|e| ToolError::ExecutionFailed {
            tool_id: "openai.videos".into(),
            reason: format!("Phản hồi từ OpenAI Videos API không phải JSON hợp lệ: {e}"),
        })?;

    // `"error": null` is normal in OpenAI Videos responses; only a non-null value is a failure.
    if let Some(err) = resp_json.get("error").filter(|e| !e.is_null()) {
        return Err(ToolError::ExecutionFailed {
            tool_id: "openai.videos".into(),
            reason: format!("OpenAI Videos API báo lỗi: {}", err),
        });
    }

    // Check if response directly contains a video url / content
    if let Some(direct_url) = extract_direct_video_url(&resp_json) {
        return download_and_finish(VideoDownloadContext {
            ws,
            client: &client,
            out_dir,
            download_url: &direct_url,
            base_url: &base_url,
            auth_headers: &headers,
            model,
            seconds,
            pricing,
        })
        .await;
    }

    // Otherwise, expect id and poll GET {base}/videos/{id}
    let video_id =
        resp_json
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::ExecutionFailed {
                tool_id: "openai.videos".into(),
                reason: format!(
                    "Phản hồi từ OpenAI Videos API không chứa 'id' hoặc video URL: {}",
                    resp_json
                ),
            })?;

    let poll_url = format!("{}/videos/{}", base_url.trim_end_matches('/'), video_id);

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
    // Against real APIs (no host override), never poll faster than every 2 s.
    let poll_interval_ms = if base_override.is_none() {
        poll_interval_ms.max(2000)
    } else {
        poll_interval_ms
    };

    // Step 2: Poll GET {base}/videos/{id} until completed / failed
    let max_polls = 90;
    let mut final_status_json = None;

    for _ in 0..max_polls {
        tokio::time::sleep(Duration::from_millis(poll_interval_ms)).await;

        let poll_resp = send_with_retries("provider:openai", "openai.videos", || {
            client
                .request(Method::GET, &poll_url)
                .headers(headers.clone())
        })
        .await?;

        let poll_bytes = poll_resp
            .bytes()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool_id: "openai.videos".into(),
                reason: format!("Lỗi đọc trạng thái tác vụ video: {e}"),
            })?;

        let poll_json: Value =
            serde_json::from_slice(&poll_bytes).map_err(|e| ToolError::ExecutionFailed {
                tool_id: "openai.videos".into(),
                reason: format!("Phản hồi thăm dò video không phải JSON hợp lệ: {e}"),
            })?;

        if let Some(err) = poll_json.get("error").filter(|e| !e.is_null()) {
            return Err(ToolError::ExecutionFailed {
                tool_id: "openai.videos".into(),
                reason: format!("Tác vụ tạo video OpenAI báo lỗi: {}", err),
            });
        }

        let status = poll_json
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("");

        if status == "completed" || status == "succeeded" {
            final_status_json = Some(poll_json);
            break;
        } else if status == "failed" || status == "cancelled" {
            let reason = poll_json
                .get("error")
                .map(|e| e.to_string())
                .unwrap_or_else(|| format!("Trạng thái kết thúc: {}", status));
            return Err(ToolError::ExecutionFailed {
                tool_id: "openai.videos".into(),
                reason: format!("Tạo video thất bại: {}", reason),
            });
        }
    }

    let final_json = final_status_json.ok_or_else(|| ToolError::ExecutionFailed {
        tool_id: "openai.videos".into(),
        reason: "Quá thời gian chờ tạo video OpenAI (polling timeout)".into(),
    })?;

    // Check if the final status response contains a video URL directly
    if let Some(video_url) = extract_direct_video_url(&final_json) {
        download_and_finish(VideoDownloadContext {
            ws,
            client: &client,
            out_dir,
            download_url: &video_url,
            base_url: &base_url,
            auth_headers: &headers,
            model,
            seconds,
            pricing,
        })
        .await
    } else {
        // Standard OpenAI Videos: download GET {base}/videos/{id}/content with bearer key
        let content_url = format!(
            "{}/videos/{}/content",
            base_url.trim_end_matches('/'),
            video_id
        );
        download_and_finish(VideoDownloadContext {
            ws,
            client: &client,
            out_dir,
            download_url: &content_url,
            base_url: &base_url,
            auth_headers: &headers,
            model,
            seconds,
            pricing,
        })
        .await
    }
}

fn extract_direct_video_url(val: &Value) -> Option<String> {
    if let Some(u) = val.get("url").and_then(Value::as_str) {
        return Some(u.to_string());
    }
    if let Some(u) = val
        .get("output")
        .and_then(Value::as_array)
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("url").or_else(|| item.as_str().map(|_| item)))
        .and_then(Value::as_str)
    {
        return Some(u.to_string());
    }
    if let Some(u) = val
        .get("data")
        .and_then(Value::as_array)
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("url").or_else(|| item.as_str().map(|_| item)))
        .and_then(Value::as_str)
    {
        return Some(u.to_string());
    }
    None
}

struct VideoDownloadContext<'a> {
    ws: &'a Workspace,
    client: &'a reqwest::Client,
    out_dir: &'a Path,
    download_url: &'a str,
    base_url: &'a str,
    auth_headers: &'a HeaderMap,
    model: &'a str,
    seconds: u64,
    pricing: Option<&'a crate::pricing::ProviderPricing>,
}

async fn download_and_finish(ctx: VideoDownloadContext<'_>) -> Result<Value> {
    let dest = ctx.out_dir.join("openai-video-0.mp4");

    // Download with bearer key if same origin ONLY, without key if non-same-origin
    let mut download_headers = HeaderMap::new();
    if same_origin(ctx.download_url, ctx.base_url) {
        download_headers = ctx.auth_headers.clone();
    }

    let (_, ext) = stream_download_to_file(
        ctx.client,
        "provider:openai",
        "openai.videos",
        ctx.download_url,
        download_headers,
        &dest,
        MediaKind::Video,
    )
    .await?;

    let final_path = if ext != "mp4" {
        let renamed = ctx.out_dir.join(format!("openai-video-0.{}", ext));
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

    let rel_path = to_workspace_relative(ctx.ws, &final_path);
    let (unit_cost_per_s, is_default) = ctx
        .pricing
        .map(|p| p.resolve_video_cost_per_s(Some(ctx.model)))
        .unwrap_or((crate::pricing::DEFAULT_VIDEO_USD_PER_S, true));
    let estimated_cost = unit_cost_per_s * (ctx.seconds.max(1) as f64);

    Ok(json!({
        "files": [rel_path.clone()],
        "video_path": rel_path,
        "provider_metadata": {
            "provider": "openai.videos",
            "model": ctx.model,
            "seconds": ctx.seconds,
            "estimated_cost_usd": estimated_cost,
            "cost_is_default": is_default
        }
    }))
}
