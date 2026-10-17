//! YouTube Data API v3 resumable upload client and OAuth 2.0 PKCE installed-app authentication.

use crate::error::{Result, ToolError};
use crate::publish::model::PublishMetadata;
use crate::secret::SecretStore;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub const DEFAULT_GOOGLE_UPLOAD_BASE: &str = "https://www.googleapis.com";
pub const DEFAULT_GOOGLE_OAUTH_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

/// Secret keys used for YouTube integration.
pub const KEY_YOUTUBE_CLIENT_ID: &str = "YOUTUBE_CLIENT_ID";
pub const KEY_YOUTUBE_CLIENT_SECRET: &str = "YOUTUBE_CLIENT_SECRET";
pub const KEY_YOUTUBE_REFRESH_TOKEN: &str = "YOUTUBE_REFRESH_TOKEN";

/// OAuth 2.0 token response from Google.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub token_type: Option<String>,
    pub expires_in: Option<u64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

/// Generate PKCE code_verifier (43-128 chars) and code_challenge (S256).
pub fn generate_pkce_codes() -> (String, String) {
    let mut random_bytes = [0u8; 32];
    let u1 = uuid::Uuid::new_v4();
    let u2 = uuid::Uuid::new_v4();
    random_bytes[0..16].copy_from_slice(u1.as_bytes());
    random_bytes[16..32].copy_from_slice(u2.as_bytes());
    let verifier = URL_SAFE_NO_PAD.encode(random_bytes);

    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    let challenge = URL_SAFE_NO_PAD.encode(hash);

    (verifier, challenge)
}

/// Construct Google OAuth 2.0 authorization URL for installed applications.
pub fn build_oauth_url(
    client_id: &str,
    redirect_uri: &str,
    code_challenge: &str,
    state: &str,
) -> String {
    format!(
        "https://accounts.google.com/o/oauth2/v2/auth?\
client_id={}&\
redirect_uri={}&\
response_type=code&\
scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fyoutube.upload%20https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fyoutube&\
code_challenge={}&\
code_challenge_method=S256&\
state={}&\
access_type=offline&\
prompt=consent",
        urlencoding(client_id),
        urlencoding(redirect_uri),
        urlencoding(code_challenge),
        urlencoding(state),
    )
}

fn urlencoding(s: &str) -> String {
    let mut encoded = String::new();
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
    }
    encoded
}

/// Exchange authorization code for tokens via Google OAuth token endpoint.
pub async fn exchange_code_for_tokens(
    client: &Client,
    token_endpoint: &str,
    client_id: &str,
    client_secret: &str,
    code: &str,
    code_verifier: &str,
    redirect_uri: &str,
) -> Result<OAuthTokenResponse> {
    let params = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("code", code),
        ("code_verifier", code_verifier),
        ("grant_type", "authorization_code"),
        ("redirect_uri", redirect_uri),
    ];

    let res = client
        .post(token_endpoint)
        .form(&params)
        .send()
        .await
        .map_err(|e| ToolError::ExecutionError(format!("Lỗi kết nối OAuth Google: {e}")))?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err(ToolError::ExecutionError(format!(
            "Không thể đổi mã xác thực YouTube: {err_text}"
        )));
    }

    res.json::<OAuthTokenResponse>()
        .await
        .map_err(|e| ToolError::ParseError(format!("Lỗi phân tích phản hồi token OAuth: {e}")))
}

/// Refresh an access token using a stored refresh token.
pub async fn refresh_access_token(
    client: &Client,
    token_endpoint: &str,
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<String> {
    let params = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
    ];

    let res = client
        .post(token_endpoint)
        .form(&params)
        .send()
        .await
        .map_err(|e| ToolError::ExecutionError(format!("Lỗi làm mới token YouTube: {e}")))?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err(ToolError::ExecutionError(format!(
            "Làm mới token YouTube thất bại: {err_text}"
        )));
    }

    let parsed: OAuthTokenResponse = res
        .json()
        .await
        .map_err(|e| ToolError::ParseError(format!("Lỗi đọc token mới: {e}")))?;

    Ok(parsed.access_token)
}

/// Run YouTube OAuth 2.0 loopback server on a random port, open system browser, and save refresh token.
pub async fn connect_youtube(
    secrets: &dyn SecretStore,
    org: &str,
    ws_name: Option<&str>,
    timeout_secs: u64,
) -> Result<String> {
    let client_id = secrets
        .get(org, ws_name, KEY_YOUTUBE_CLIENT_ID)
        .ok_or_else(|| {
            ToolError::ExecutionError(
                "Chưa cấu hình YOUTUBE_CLIENT_ID trong Credential Manager. Hãy thêm khóa trong Cài đặt."
                    .into(),
            )
        })?;

    let client_secret = secrets
        .get(org, ws_name, KEY_YOUTUBE_CLIENT_SECRET)
        .ok_or_else(|| {
            ToolError::ExecutionError(
                "Chưa cấu hình YOUTUBE_CLIENT_SECRET trong Credential Manager. Hãy thêm khóa trong Cài đặt."
                    .into(),
            )
        })?;

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| ToolError::ExecutionError(format!("Không thể mở cổng loopback: {e}")))?;

    let local_addr = listener
        .local_addr()
        .map_err(|e| ToolError::ExecutionError(e.to_string()))?;
    let port = local_addr.port();
    let redirect_uri = format!("http://127.0.0.1:{port}");

    let (code_verifier, code_challenge) = generate_pkce_codes();
    let state_expected = format!("state-{}", uuid::Uuid::new_v4().simple());

    let auth_url = build_oauth_url(&client_id, &redirect_uri, &code_challenge, &state_expected);

    // Open browser
    #[cfg(windows)]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", &auth_url])
        .spawn();

    #[cfg(not(windows))]
    let _ = std::process::Command::new("xdg-open")
        .arg(&auth_url)
        .spawn();

    // Wait on loopback listener with timeout
    let (mut stream, _) =
        tokio::time::timeout(Duration::from_secs(timeout_secs), listener.accept())
            .await
            .map_err(|_| {
                ToolError::ExecutionError(
                    "Hết thời gian chờ xác thực YouTube từ trình duyệt (timeout).".into(),
                )
            })?
            .map_err(|e| ToolError::ExecutionError(format!("Lỗi nhận kết nối loopback: {e}")))?;

    let mut buf = [0u8; 4096];
    let n = stream
        .read(&mut buf)
        .await
        .map_err(|e| ToolError::ExecutionError(format!("Lỗi đọc yêu cầu loopback: {e}")))?;
    let request_str = String::from_utf8_lossy(&buf[..n]);

    // Parse HTTP GET line: "GET /?code=...&state=... HTTP/1.1"
    let first_line = request_str.lines().next().unwrap_or_default();
    let query_path = first_line.split_whitespace().nth(1).unwrap_or("/");
    let query_str = query_path.split('?').nth(1).unwrap_or("");

    let mut code_found = None;
    let mut state_found = None;
    for pair in query_str.split('&') {
        let mut kv = pair.splitn(2, '=');
        let k = kv.next().unwrap_or("");
        let v = kv.next().unwrap_or("");
        if k == "code" {
            code_found = Some(v.to_string());
        } else if k == "state" {
            state_found = Some(v.to_string());
        }
    }

    let code = match (code_found, state_found) {
        (Some(c), Some(s)) if s == state_expected => c,
        _ => {
            let error_response =
                "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
                <html><body style=\"font-family:sans-serif;padding:2rem;\">\
                <h2>Xác thực YouTube thất bại</h2>\
                <p>Mã trạng thái không khớp (state mismatch) hoặc thiếu mã ủy quyền.</p>\
                </body></html>";
            let _ = stream.write_all(error_response.as_bytes()).await;
            return Err(ToolError::ExecutionError(
                "Xác thực thất bại: state không khớp hoặc thiếu mã xác thực.".into(),
            ));
        }
    };

    // Respond success HTML to browser
    let ok_response =
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n\
        <html><body style=\"font-family:sans-serif;padding:2rem;text-align:center;\">\
        <h2 style=\"color:#2e7d32;\">Kết nối YouTube thành công!</h2>\
        <p>Local Content Studio đã nhận được mã ủy quyền. Bạn có thể đóng tab này.</p>\
        </body></html>";
    let _ = stream.write_all(ok_response.as_bytes()).await;
    let _ = stream.flush().await;

    // Exchange code for tokens
    let http = Client::new();
    let token_resp = exchange_code_for_tokens(
        &http,
        DEFAULT_GOOGLE_OAUTH_TOKEN_URL,
        &client_id,
        &client_secret,
        &code,
        &code_verifier,
        &redirect_uri,
    )
    .await?;

    let refresh_token = token_resp.refresh_token.ok_or_else(|| {
        ToolError::ExecutionError(
            "Google không trả về refresh token. Hãy thu hồi quyền ứng dụng trên Google Account và kết nối lại."
                .into(),
        )
    })?;

    // Store in SecretStore
    secrets.set(org, ws_name, KEY_YOUTUBE_REFRESH_TOKEN, &refresh_token)?;

    Ok("Đã kết nối tài khoản YouTube và lưu khóa bảo mật thành công.".into())
}

/// Video upload JSON metadata payload for YouTube Data API v3.
#[derive(Debug, Serialize, Deserialize)]
struct VideoInsertPayload {
    snippet: VideoSnippet,
    status: VideoStatus,
}

#[derive(Debug, Serialize, Deserialize)]
struct VideoSnippet {
    title: String,
    description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct VideoStatus {
    #[serde(rename = "privacyStatus")]
    privacy_status: String,
    #[serde(rename = "publishAt", skip_serializing_if = "Option::is_none")]
    publish_at: Option<String>,
}

/// Execute resumable video upload against YouTube Data API v3 endpoint.
pub async fn upload_video_resumable(
    client: &Client,
    base_url: &str,
    access_token: &str,
    video_path: &Path,
    metadata: &PublishMetadata,
    chunk_size_bytes: usize,
    mut progress_cb: impl FnMut(f32, &str),
) -> Result<String> {
    let video_bytes = tokio::fs::read(video_path)
        .await
        .map_err(|e| ToolError::Io {
            path: video_path.to_path_buf(),
            source: e,
        })?;

    let total_size = video_bytes.len();
    if total_size == 0 {
        return Err(ToolError::ExecutionError(
            "Tệp video rỗng (0 bytes).".into(),
        ));
    }

    // Step 1: Initiate Resumable Session
    let init_url =
        format!("{base_url}/upload/youtube/v3/videos?uploadType=resumable&part=snippet,status");
    let payload = VideoInsertPayload {
        snippet: VideoSnippet {
            title: metadata.title.clone(),
            description: metadata.description.clone(),
            tags: metadata.tags.clone(),
        },
        status: VideoStatus {
            privacy_status: metadata.privacy.as_str().to_string(),
            publish_at: metadata.schedule.clone(),
        },
    };

    let mut init_headers = HeaderMap::new();
    init_headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {access_token}"))
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?,
    );
    init_headers.insert("X-Upload-Content-Type", HeaderValue::from_static("video/*"));
    init_headers.insert(
        "X-Upload-Content-Length",
        HeaderValue::from_str(&total_size.to_string())
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?,
    );

    progress_cb(0.1, "Khởi tạo phiên tải lên YouTube...");

    let init_res = client
        .post(&init_url)
        .headers(init_headers)
        .json(&payload)
        .send()
        .await
        .map_err(|e| ToolError::ExecutionError(format!("Lỗi kết nối phiên upload YouTube: {e}")))?;

    if !init_res.status().is_success() {
        let err_text = init_res.text().await.unwrap_or_default();
        return Err(ToolError::ExecutionError(format!(
            "Không thể tạo phiên tải lên YouTube: {err_text}"
        )));
    }

    let session_url = init_res
        .headers()
        .get("Location")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            ToolError::ExecutionError(
                "YouTube không trả về Location header cho phiên resumable.".into(),
            )
        })?
        .to_string();

    // Step 2: Upload chunks
    let mut uploaded_bytes = 0usize;
    let chunk_size = chunk_size_bytes.max(256 * 1024); // Recommended minimum 256KB

    let mut final_video_id: Option<String> = None;

    while uploaded_bytes < total_size {
        let end_byte = (uploaded_bytes + chunk_size).min(total_size) - 1;
        let chunk_data = &video_bytes[uploaded_bytes..=end_byte];
        let chunk_len = chunk_data.len();

        let content_range = format!("bytes {uploaded_bytes}-{end_byte}/{total_size}");
        let progress_frac = 0.1 + 0.8 * ((uploaded_bytes + chunk_len) as f32 / total_size as f32);
        progress_cb(
            progress_frac,
            &format!(
                "Đang tải dữ liệu video ({}/{} KB)...",
                (end_byte + 1) / 1024,
                total_size / 1024
            ),
        );

        let res = client
            .put(&session_url)
            .header(CONTENT_LENGTH, chunk_len.to_string())
            .header("Content-Range", &content_range)
            .header(CONTENT_TYPE, "video/*")
            .body(chunk_data.to_vec())
            .send()
            .await
            .map_err(|e| ToolError::ExecutionError(format!("Lỗi truyền chunk video: {e}")))?;

        let status = res.status();
        if status == StatusCode::from_u16(308).unwrap() {
            // Resume incomplete: read Range header
            if let Some(range_hdr) = res.headers().get("Range").and_then(|r| r.to_str().ok()) {
                if let Some(last_str) = range_hdr.split('-').nth(1) {
                    if let Ok(last_byte) = last_str.trim().parse::<usize>() {
                        uploaded_bytes = last_byte + 1;
                        continue;
                    }
                }
            }
            uploaded_bytes = end_byte + 1;
        } else if status.is_success() {
            // Completed! Parse video ID
            let body: serde_json::Value = res
                .json()
                .await
                .map_err(|e| ToolError::ParseError(format!("Lỗi đọc kết quả đăng YouTube: {e}")))?;
            let id = body
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ToolError::ExecutionError(
                        "Không tìm thấy video ID trong kết quả YouTube.".into(),
                    )
                })?
                .to_string();
            final_video_id = Some(id);
            break;
        } else {
            let err_text = res.text().await.unwrap_or_default();
            return Err(ToolError::ExecutionError(format!(
                "Tải lên video thất bại (HTTP {status}): {err_text}"
            )));
        }
    }

    let video_id = final_video_id.ok_or_else(|| {
        ToolError::ExecutionError(
            "Hoàn tất upload nhưng không nhận được video ID từ YouTube.".into(),
        )
    })?;

    Ok(video_id)
}

/// Set video thumbnail via `thumbnails.set`.
pub async fn upload_thumbnail(
    client: &Client,
    base_url: &str,
    access_token: &str,
    video_id: &str,
    thumb_path: &Path,
) -> Result<()> {
    let thumb_bytes = tokio::fs::read(thumb_path)
        .await
        .map_err(|e| ToolError::Io {
            path: thumb_path.to_path_buf(),
            source: e,
        })?;

    let url =
        format!("{base_url}/upload/youtube/v3/thumbnails/set?videoId={video_id}&uploadType=media");

    let mime = if thumb_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("png"))
        .unwrap_or(false)
    {
        "image/png"
    } else {
        "image/jpeg"
    };

    let res = client
        .post(&url)
        .header(AUTHORIZATION, format!("Bearer {access_token}"))
        .header(CONTENT_TYPE, mime)
        .body(thumb_bytes)
        .send()
        .await
        .map_err(|e| ToolError::ExecutionError(format!("Lỗi tải ảnh thu nhỏ lên YouTube: {e}")))?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err(ToolError::ExecutionError(format!(
            "Cập nhật ảnh thu nhỏ thất bại: {err_text}"
        )));
    }

    Ok(())
}

/// Orchestrate complete YouTube publish using secrets and progress updates.
pub async fn publish_youtube_complete(
    secrets: &dyn SecretStore,
    org: &str,
    ws_name: Option<&str>,
    video_path: &Path,
    thumbnail_path: Option<&Path>,
    metadata: &PublishMetadata,
    mut progress_cb: impl FnMut(f32, &str),
) -> Result<String> {
    let client_id = secrets
        .get(org, ws_name, KEY_YOUTUBE_CLIENT_ID)
        .ok_or_else(|| {
            ToolError::ExecutionError(
                "Chưa cấu hình YOUTUBE_CLIENT_ID. Hãy thêm khóa trong Cài đặt.".into(),
            )
        })?;

    let client_secret = secrets
        .get(org, ws_name, KEY_YOUTUBE_CLIENT_SECRET)
        .ok_or_else(|| {
            ToolError::ExecutionError(
                "Chưa cấu hình YOUTUBE_CLIENT_SECRET. Hãy thêm khóa trong Cài đặt.".into(),
            )
        })?;

    let refresh_token = secrets
        .get(org, ws_name, KEY_YOUTUBE_REFRESH_TOKEN)
        .ok_or_else(|| {
            ToolError::ExecutionError(
                "Chưa kết nối YouTube. Vui lòng nhấn \"Kết nối YouTube\" trước khi đăng video."
                    .into(),
            )
        })?;

    progress_cb(0.05, "Làm mới mã truy cập YouTube...");
    let http = Client::new();
    let access_token = refresh_access_token(
        &http,
        DEFAULT_GOOGLE_OAUTH_TOKEN_URL,
        &client_id,
        &client_secret,
        &refresh_token,
    )
    .await?;

    let chunk_size = 1024 * 1024; // 1 MB chunks
    let video_id = upload_video_resumable(
        &http,
        DEFAULT_GOOGLE_UPLOAD_BASE,
        &access_token,
        video_path,
        metadata,
        chunk_size,
        &mut progress_cb,
    )
    .await?;

    if let Some(thumb) = thumbnail_path {
        if thumb.exists() {
            progress_cb(0.95, "Đang tải ảnh thu nhỏ lên YouTube...");
            let _ = upload_thumbnail(
                &http,
                DEFAULT_GOOGLE_UPLOAD_BASE,
                &access_token,
                &video_id,
                thumb,
            )
            .await;
        }
    }

    progress_cb(1.0, "Đăng video YouTube thành công!");
    Ok(format!("https://www.youtube.com/watch?v={video_id}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_pkce_generation() {
        let (verifier, challenge) = generate_pkce_codes();
        assert!(!verifier.is_empty());
        assert!(!challenge.is_empty());
        assert_ne!(verifier, challenge);

        // Verify challenge is sha256 of verifier
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let expected = URL_SAFE_NO_PAD.encode(hasher.finalize());
        assert_eq!(challenge, expected);
    }

    #[tokio::test]
    async fn test_mock_resumable_upload_and_308_resume() {
        // Spawn mock HTTP server on 127.0.0.1:0
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let base_url = format!("http://127.0.0.1:{port}");

        // Handle server requests in background task
        tokio::spawn(async move {
            // 1. Session initiation POST
            let (mut stream1, _) = listener.accept().await.unwrap();
            let mut buf1 = [0u8; 4096];
            let _ = stream1.read(&mut buf1).await.unwrap();
            let session_url = format!("http://127.0.0.1:{port}/mock-session-789");
            let resp1 = format!(
                "HTTP/1.1 200 OK\r\nLocation: {session_url}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            stream1.write_all(resp1.as_bytes()).await.unwrap();
            stream1.flush().await.unwrap();

            // 2. Chunk 1 PUT -> respond with 308 Resume Incomplete
            let (mut stream2, _) = listener.accept().await.unwrap();
            let mut buf2 = [0u8; 4096];
            let _ = stream2.read(&mut buf2).await.unwrap();
            let resp2 = "HTTP/1.1 308 Resume Incomplete\r\nRange: bytes=0-262143\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            stream2.write_all(resp2.as_bytes()).await.unwrap();
            stream2.flush().await.unwrap();

            // 3. Chunk 2 PUT -> respond with 200 OK + video id
            let (mut stream3, _) = listener.accept().await.unwrap();
            let mut buf3 = [0u8; 4096];
            let _ = stream3.read(&mut buf3).await.unwrap();
            let body = r#"{"id": "mock_yt_vid_test_123"}"#;
            let resp3 = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream3.write_all(resp3.as_bytes()).await.unwrap();
            stream3.flush().await.unwrap();

            // 4. Thumbnail POST -> respond with 200 OK
            let (mut stream4, _) = listener.accept().await.unwrap();
            let mut buf4 = [0u8; 4096];
            let _ = stream4.read(&mut buf4).await.unwrap();
            let resp4 = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            stream4.write_all(resp4.as_bytes()).await.unwrap();
            stream4.flush().await.unwrap();
        });

        // Create temporary dummy video file (size 500 KB)
        let mut temp_video = NamedTempFile::new().unwrap();
        use std::io::Write;
        let dummy_data = vec![0x42u8; 500 * 1024];
        temp_video.write_all(&dummy_data).unwrap();

        let mut temp_thumb = NamedTempFile::new().unwrap();
        temp_thumb.write_all(b"fake-thumbnail-data").unwrap();

        let client = Client::new();
        let metadata = PublishMetadata {
            title: "Test Video".into(),
            description: "Test Desc".into(),
            tags: vec!["test".into()],
            privacy: crate::publish::model::PublishPrivacy::Private,
            schedule: None,
            thumbnail: None,
        };

        let mut progress_history = Vec::new();
        let chunk_size = 256 * 1024; // 256 KB chunk
        let video_id = upload_video_resumable(
            &client,
            &base_url,
            "mock-access-token",
            temp_video.path(),
            &metadata,
            chunk_size,
            |p, msg| {
                progress_history.push((p, msg.to_string()));
            },
        )
        .await
        .unwrap();

        assert_eq!(video_id, "mock_yt_vid_test_123");
        assert!(!progress_history.is_empty());

        // Test thumbnail upload
        let thumb_res = upload_thumbnail(
            &client,
            &base_url,
            "mock-access-token",
            &video_id,
            temp_thumb.path(),
        )
        .await;
        assert!(thumb_res.is_ok());
    }
}
