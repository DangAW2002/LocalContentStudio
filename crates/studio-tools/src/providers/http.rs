//! HTTP client utilities: timeouts, 429/5xx exponential backoff retries,
//! rate-key concurrency limits, streaming download with 200 MB size cap, and magic bytes validation.

use crate::error::{Result, ToolError};
use reqwest::header::{HeaderMap, HeaderValue, RETRY_AFTER};
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;

/// Hard size cap for streamed media downloads: 200 MB.
pub const MAX_DOWNLOAD_SIZE_BYTES: usize = 200 * 1024 * 1024;

/// Media category for magic-byte validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    /// Image formats: PNG, JPEG, WEBP, GIF.
    Image,
    /// Video formats: MP4, WebM, MKV.
    Video,
}

static RATE_LIMITERS: OnceLock<Mutex<HashMap<String, Arc<Semaphore>>>> = OnceLock::new();

/// Acquire a concurrency permit for a specific rate key (default limit 4).
pub async fn acquire_rate_permit(rate_key: &str) -> tokio::sync::OwnedSemaphorePermit {
    let map = RATE_LIMITERS.get_or_init(|| Mutex::new(HashMap::new()));
    let sem = {
        let mut guard = map.lock().unwrap_or_else(|e| e.into_inner());
        guard
            .entry(rate_key.to_string())
            .or_insert_with(|| Arc::new(Semaphore::new(4)))
            .clone()
    };
    sem.acquire_owned()
        .await
        .expect("rate limiter semaphore should never be closed")
}

/// Verify that raw bytes match expected magic bytes for `MediaKind`.
/// Returns the detected extension (e.g. "png", "jpg", "mp4").
pub fn check_magic_bytes(bytes: &[u8], kind: MediaKind) -> Result<&'static str> {
    match kind {
        MediaKind::Image => {
            // PNG: 89 50 4E 47 0D 0A 1A 0A
            if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
                return Ok("png");
            }
            // JPEG: FF D8 FF
            if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
                return Ok("jpg");
            }
            // WEBP: RIFF....WEBP
            if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
                return Ok("webp");
            }
            // GIF: GIF87a or GIF89a
            if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
                return Ok("gif");
            }
            Err(ToolError::ExecutionFailed {
                tool_id: "image_validator".into(),
                reason: "Tệp hình ảnh không hợp lệ: magic bytes không khớp PNG, JPEG, WEBP hay GIF"
                    .into(),
            })
        }
        MediaKind::Video => {
            // MP4 / QuickTime: bytes 4..8 == ftyp, moov, or mdat
            if bytes.len() >= 8
                && (&bytes[4..8] == b"ftyp" || &bytes[4..8] == b"moov" || &bytes[4..8] == b"mdat")
            {
                return Ok("mp4");
            }
            // WebM / MKV: 1A 45 DF A3 (EBML)
            if bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
                return Ok("webm");
            }
            // AVI: RIFF....AVI
            if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"AVI " {
                return Ok("avi");
            }
            Err(ToolError::ExecutionFailed {
                tool_id: "video_validator".into(),
                reason: "Tệp video không hợp lệ: magic bytes không khớp MP4, WebM hay MKV".into(),
            })
        }
    }
}

/// Create a reusable reqwest Client with reasonable timeouts.
pub fn create_http_client() -> Client {
    Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(120))
        .build()
        .unwrap_or_else(|_| Client::new())
}

/// Execute an HTTP request with exponential backoff on 429 and 5xx errors (up to 3 retries).
pub async fn send_with_retries<F>(
    rate_key: &str,
    tool_id: &str,
    make_request: F,
) -> Result<Response>
where
    F: Fn() -> RequestBuilder,
{
    let _permit = acquire_rate_permit(rate_key).await;

    let max_retries = 3;
    let mut attempt = 0;
    let fast_retry = std::env::var("STUDIO_TEST_FAST_RETRY").is_ok() || cfg!(test);
    let mut backoff = if fast_retry {
        Duration::from_millis(20)
    } else {
        Duration::from_millis(500)
    };

    loop {
        let req = make_request();
        let res = req.send().await;

        match res {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    return Ok(resp);
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

                // Non-retriable or retries exhausted
                let body_text = resp.text().await.unwrap_or_default();
                let error_msg = parse_api_error(&body_text).unwrap_or_else(|| {
                    format!("Yêu cầu HTTP thất bại với mã {}: {}", status, body_text)
                });

                return Err(ToolError::ExecutionFailed {
                    tool_id: tool_id.to_string(),
                    reason: error_msg,
                });
            }
            Err(e) if attempt < max_retries && (e.is_timeout() || e.is_connect()) => {
                attempt += 1;
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(30));
            }
            Err(e) => {
                return Err(ToolError::ExecutionFailed {
                    tool_id: tool_id.to_string(),
                    reason: format!("Lỗi kết nối mạng: {e}"),
                });
            }
        }
    }
}

fn parse_retry_after(header: &HeaderValue) -> Option<Duration> {
    if let Ok(s) = header.to_str() {
        if let Ok(secs) = s.parse::<u64>() {
            return Some(Duration::from_secs(secs.min(30)));
        }
    }
    None
}

fn parse_api_error(body: &str) -> Option<String> {
    let val: serde_json::Value = serde_json::from_str(body).ok()?;
    if let Some(msg) = val
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
    {
        return Some(msg.to_string());
    }
    if let Some(msg) = val.get("message").and_then(|m| m.as_str()) {
        return Some(msg.to_string());
    }
    None
}

/// Stream download from a URL directly to disk, enforcing size limit and magic bytes.
pub async fn stream_download_to_file(
    client: &Client,
    rate_key: &str,
    tool_id: &str,
    url: &str,
    headers: HeaderMap,
    dest: &Path,
    expected_kind: MediaKind,
) -> Result<(usize, &'static str)> {
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| ToolError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
    }

    let url_clone = url.to_string();
    let headers_clone = headers.clone();
    let mut resp = send_with_retries(rate_key, tool_id, || {
        let mut builder = client.request(Method::GET, &url_clone);
        for (k, v) in headers_clone.iter() {
            builder = builder.header(k, v);
        }
        builder
    })
    .await?;

    let mut file = File::create(dest).await.map_err(|e| ToolError::Io {
        path: dest.to_path_buf(),
        source: e,
    })?;

    let mut downloaded = 0usize;
    let mut header_buf = Vec::new();

    while let Some(chunk) = resp.chunk().await.map_err(|e| ToolError::ExecutionFailed {
        tool_id: tool_id.to_string(),
        reason: format!("Lỗi tải dữ liệu dòng (streaming): {e}"),
    })? {
        downloaded += chunk.len();
        if downloaded > MAX_DOWNLOAD_SIZE_BYTES {
            drop(file);
            let _ = tokio::fs::remove_file(dest).await;
            return Err(ToolError::ExecutionFailed {
                tool_id: tool_id.to_string(),
                reason: format!(
                    "Kích thước tệp tải về vượt quá giới hạn 200 MB (đã nhận {} bytes)",
                    downloaded
                ),
            });
        }

        if header_buf.len() < 32 {
            let needed = 32 - header_buf.len();
            let take = needed.min(chunk.len());
            header_buf.extend_from_slice(&chunk[..take]);
        }

        file.write_all(&chunk).await.map_err(|e| ToolError::Io {
            path: dest.to_path_buf(),
            source: e,
        })?;
    }

    file.flush().await.map_err(|e| ToolError::Io {
        path: dest.to_path_buf(),
        source: e,
    })?;
    drop(file);

    let ext = match check_magic_bytes(&header_buf, expected_kind) {
        Ok(ext) => ext,
        Err(err) => {
            let _ = tokio::fs::remove_file(dest).await;
            return Err(err);
        }
    };

    Ok((downloaded, ext))
}

/// Decode Base64 payload, enforce size limit and magic bytes, and write to disk.
pub async fn save_base64_to_file(
    tool_id: &str,
    b64_str: &str,
    dest: &Path,
    expected_kind: MediaKind,
) -> Result<(usize, &'static str)> {
    use base64::prelude::*;

    let bytes = BASE64_STANDARD
        .decode(b64_str.trim())
        .map_err(|e| ToolError::ExecutionFailed {
            tool_id: tool_id.to_string(),
            reason: format!("Lỗi giải mã Base64: {e}"),
        })?;

    if bytes.len() > MAX_DOWNLOAD_SIZE_BYTES {
        return Err(ToolError::ExecutionFailed {
            tool_id: tool_id.to_string(),
            reason: format!(
                "Kích thước tệp vượt quá giới hạn 200 MB (kích thước: {} bytes)",
                bytes.len()
            ),
        });
    }

    let ext = check_magic_bytes(&bytes, expected_kind)?;

    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| ToolError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
    }

    tokio::fs::write(dest, &bytes)
        .await
        .map_err(|e| ToolError::Io {
            path: dest.to_path_buf(),
            source: e,
        })?;

    Ok((bytes.len(), ext))
}

/// True when `uri` has exactly the same scheme, host and port as `base`.
pub fn same_origin(uri: &str, base: &str) -> bool {
    match (reqwest::Url::parse(uri), reqwest::Url::parse(base)) {
        (Ok(a), Ok(b)) => {
            a.scheme() == b.scheme()
                && a.host_str() == b.host_str()
                && a.port_or_known_default() == b.port_or_known_default()
        }
        _ => false,
    }
}

#[cfg(test)]
mod origin_tests {
    use super::same_origin;

    #[test]
    fn exact_origin_only() {
        let base = "https://generativelanguage.googleapis.com";
        assert!(same_origin(
            "https://generativelanguage.googleapis.com/v1beta/files/x:download",
            base
        ));
        assert!(!same_origin(
            "https://evil.example/?generativelanguage.googleapis.com",
            base
        ));
        assert!(!same_origin(
            "https://generativelanguage.googleapis.com.evil.example/x",
            base
        ));
        assert!(!same_origin(
            "http://generativelanguage.googleapis.com/x",
            base
        ));
    }
}
