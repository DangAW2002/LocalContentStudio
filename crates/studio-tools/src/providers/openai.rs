//! OpenAI Images provider implementation via `api.openai.com/v1/images/generations`
//! and `api.openai.com/v1/images/edits` (for reference images support).

use crate::error::{Result, ToolError};
use crate::providers::http::{
    acquire_rate_permit, create_http_client, parse_api_error, parse_retry_after,
    save_base64_to_file, send_with_retries, stream_download_to_file, MediaKind,
};
use crate::providers::imagen::to_workspace_relative;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, RETRY_AFTER};
use reqwest::{Client, Method, Response, StatusCode};
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;
use studio_workspace::Workspace;

/// File attachment for multipart/form-data request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultipartFile {
    /// Form field name (e.g. "image[]")
    pub field_name: String,
    /// Filename sent in Content-Disposition header
    pub filename: String,
    /// MIME type sent in Content-Type header
    pub mime_type: String,
    /// Raw binary contents
    pub bytes: Vec<u8>,
}

/// Builds a multipart/form-data payload body using given boundary.
pub fn build_openai_edits_multipart(
    boundary: &str,
    model: &str,
    prompt: &str,
    size: &str,
    n: u64,
    files: &[MultipartFile],
) -> Vec<u8> {
    let mut body = Vec::new();

    let text_fields = [("model", model), ("prompt", prompt), ("size", size)];

    for (name, val) in text_fields {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n{val}\r\n").as_bytes(),
        );
    }

    let n_str = n.to_string();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"n\"\r\n\r\n{n_str}\r\n").as_bytes(),
    );

    for file in files {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\nContent-Type: {}\r\n\r\n",
                file.field_name, file.filename, file.mime_type
            )
            .as_bytes(),
        );
        body.extend_from_slice(&file.bytes);
        body.extend_from_slice(b"\r\n");
    }

    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    body
}

/// Run OpenAI images generation or edit (`gpt-image-1` / DALL-E).
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

    let mut warnings = Vec::new();
    if input.get("seed").is_some() {
        warnings.push(
            "Provider 'openai.images' không hỗ trợ seed — tham số seed sẽ bị bỏ qua.".to_string(),
        );
    }

    // Never from tool input: the agent must not choose where the API key is sent.
    let base_url = base_override
        .map(String::from)
        .or_else(|| std::env::var("OPENAI_API_BASE_URL").ok())
        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

    let client = create_http_client();

    // Check for reference images
    let mut ref_paths = Vec::new();
    if let Some(arr) = input.get("reference_images").and_then(Value::as_array) {
        for v in arr {
            if let Some(s) = v.as_str() {
                let scoped = ws.scoped_path(s).map_err(ToolError::WorkspaceError)?;
                if !scoped.is_file() {
                    return Err(ToolError::ExecutionFailed {
                        tool_id: "openai.images".into(),
                        reason: format!("Không tìm thấy tệp ảnh tham chiếu: {s}"),
                    });
                }
                ref_paths.push(scoped);
            }
        }
    }

    let mut resp_opt: Option<Response> = None;

    if !ref_paths.is_empty() {
        let selected_refs: Vec<_> = ref_paths.into_iter().take(4).collect();
        let mut multipart_files = Vec::new();
        for path in &selected_refs {
            let bytes = tokio::fs::read(path).await.map_err(|e| ToolError::Io {
                path: path.clone(),
                source: e,
            })?;
            let filename = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "image.png".to_string());
            let ext = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("png")
                .to_lowercase();
            let mime = match ext.as_str() {
                "jpg" | "jpeg" => "image/jpeg",
                "webp" => "image/webp",
                _ => "image/png",
            };
            multipart_files.push(MultipartFile {
                field_name: "image[]".to_string(),
                filename,
                mime_type: mime.to_string(),
                bytes,
            });
        }

        let edits_endpoint = format!("{}/images/edits", base_url.trim_end_matches('/'));
        let boundary = format!("----StudioBoundary{}", uuid::Uuid::new_v4().simple());
        let body_bytes = build_openai_edits_multipart(
            &boundary,
            model,
            prompt,
            size,
            sample_count,
            &multipart_files,
        );

        let mut edits_headers = HeaderMap::new();
        edits_headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_str(&format!("multipart/form-data; boundary={boundary}")).map_err(
                |_| ToolError::ParseError("Lỗi tạo header Content-Type multipart".into()),
            )?,
        );
        edits_headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key.trim())).map_err(|_| {
                ToolError::SecretStoreError("Khóa OPENAI_API_KEY chứa ký tự không hợp lệ".into())
            })?,
        );

        match send_edits_request(&client, &edits_endpoint, edits_headers, body_bytes).await? {
            EditsResult::Success(r) => {
                resp_opt = Some(r);
            }
            EditsResult::ClientError(status, err_text) => {
                tracing::warn!(
                    "Gateway không hỗ trợ /images/edits (HTTP {status}): {err_text}. Chuyển về /images/generations."
                );
                warnings.push(format!(
                    "Cổng API không hỗ trợ /images/edits với ảnh tham chiếu (mã lỗi {status}); đã tự động chuyển về /images/generations không dùng ảnh tham chiếu."
                ));
            }
        }
    }

    let resp = match resp_opt {
        Some(r) => r,
        None => {
            let gen_endpoint = format!("{}/images/generations", base_url.trim_end_matches('/'));
            let request_body = json!({
                "model": model,
                "prompt": prompt,
                "n": sample_count,
                "size": size,
                "response_format": "b64_json"
            });

            let mut gen_headers = HeaderMap::new();
            gen_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            gen_headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", api_key.trim())).map_err(|_| {
                    ToolError::SecretStoreError(
                        "Khóa OPENAI_API_KEY chứa ký tự không hợp lệ".into(),
                    )
                })?,
            );

            let body_bytes = serde_json::to_vec(&request_body)
                .map_err(|e| ToolError::ParseError(e.to_string()))?;

            send_with_retries("provider:openai", "openai.images", || {
                client
                    .request(Method::POST, &gen_endpoint)
                    .headers(gen_headers.clone())
                    .body(body_bytes.clone())
            })
            .await?
        }
    };

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

    let mut ret = json!({
        "files": saved_files,
        "provider_metadata": {
            "provider": "openai.images",
            "model": model,
            "count": saved_files.len(),
            "size": size,
            "estimated_cost_usd": estimated_cost,
            "cost_is_default": is_default
        }
    });

    if !warnings.is_empty() {
        ret["warnings"] = json!(warnings);
    }

    Ok(ret)
}

enum EditsResult {
    Success(Response),
    ClientError(StatusCode, String),
}

async fn send_edits_request(
    client: &Client,
    endpoint: &str,
    headers: HeaderMap,
    body: Vec<u8>,
) -> Result<EditsResult> {
    let _permit = acquire_rate_permit("provider:openai").await;

    let max_retries = 3;
    let mut attempt = 0;
    let fast_retry = std::env::var("STUDIO_TEST_FAST_RETRY").is_ok() || cfg!(test);
    let mut backoff = if fast_retry {
        Duration::from_millis(20)
    } else {
        Duration::from_millis(500)
    };

    loop {
        let req = client
            .request(Method::POST, endpoint)
            .headers(headers.clone())
            .body(body.clone());

        let res = req.send().await;

        match res {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    return Ok(EditsResult::Success(resp));
                }

                // Fall back only when the gateway lacks /images/edits. Auth, moderation or
                // bad-input errors must surface: a silent fallback still costs money but
                // drops the reference images.
                if (status.is_client_error() && status != StatusCode::TOO_MANY_REQUESTS)
                    || status == StatusCode::NOT_IMPLEMENTED
                {
                    let err_text = resp.text().await.unwrap_or_default();
                    if edits_unsupported(status, &err_text) {
                        return Ok(EditsResult::ClientError(status, err_text));
                    }
                    let error_msg = parse_api_error(&err_text).unwrap_or_else(|| {
                        format!("Yêu cầu HTTP thất bại với mã {}: {}", status, err_text)
                    });
                    return Err(ToolError::ExecutionFailed {
                        tool_id: "openai.images".into(),
                        reason: error_msg,
                    });
                }

                if (status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error())
                    && attempt < max_retries
                {
                    attempt += 1;
                    let wait_dur = if let Some(retry_after) = resp.headers().get(RETRY_AFTER) {
                        parse_retry_after(retry_after).unwrap_or(backoff)
                    } else {
                        backoff
                    };

                    tokio::time::sleep(wait_dur).await;
                    backoff = (backoff * 2).min(Duration::from_secs(30));
                    continue;
                }

                let body_text = resp.text().await.unwrap_or_default();
                let error_msg = parse_api_error(&body_text).unwrap_or_else(|| {
                    format!("Yêu cầu HTTP thất bại với mã {}: {}", status, body_text)
                });

                return Err(ToolError::ExecutionFailed {
                    tool_id: "openai.images".into(),
                    reason: error_msg,
                });
            }
            Err(e) if attempt < max_retries && e.is_connect() => {
                attempt += 1;
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(30));
            }
            Err(e) => {
                return Err(ToolError::ExecutionFailed {
                    tool_id: "openai.images".into(),
                    reason: format!("Lỗi kết nối mạng: {e}"),
                });
            }
        }
    }
}

/// Whether an error from `/images/edits` means the gateway does not offer the endpoint,
/// as opposed to a real request error (bad key, moderation, invalid input).
fn edits_unsupported(status: StatusCode, body: &str) -> bool {
    if matches!(
        status,
        StatusCode::NOT_FOUND
            | StatusCode::METHOD_NOT_ALLOWED
            | StatusCode::UNSUPPORTED_MEDIA_TYPE
            | StatusCode::NOT_IMPLEMENTED
    ) {
        return true;
    }
    if status != StatusCode::BAD_REQUEST {
        return false;
    }
    let lower = body.to_lowercase();
    [
        "not supported",
        "unsupported",
        "unknown endpoint",
        "invalid url",
        "no route",
    ]
    .iter()
    .any(|k| lower.contains(k))
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
