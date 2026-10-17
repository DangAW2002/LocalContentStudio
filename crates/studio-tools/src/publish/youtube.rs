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

/// Structured error for YouTube publishing and OAuth flows with Vietnamese guidance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct YouTubeErrorDetail {
    pub code: String,
    pub message_vi: String,
    pub fix_vi: String,
}

impl std::fmt::Display for YouTubeErrorDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.message_vi, self.fix_vi)
    }
}

impl std::error::Error for YouTubeErrorDetail {}

/// Non-secret YouTube channel metadata displayed in the UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct YouTubeChannelAccount {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    pub connected_at: String,
}

/// Map Google OAuth and YouTube Data API v3 errors to structured Vietnamese remediation.
pub fn map_youtube_error(status: Option<StatusCode>, raw_body_or_msg: &str) -> YouTubeErrorDetail {
    let lower = raw_body_or_msg.to_lowercase();

    // 1. youtubeSignupRequired (no YouTube channel for this Google account)
    if lower.contains("youtubesignuprequired")
        || (status == Some(StatusCode::UNAUTHORIZED)
            && lower.contains("unauthorized")
            && lower.contains("youtube"))
    {
        return YouTubeErrorDetail {
            code: "youtubeSignupRequired".into(),
            message_vi: "Tài khoản này chưa có kênh YouTube".into(),
            fix_vi: "Vào youtube.com → Tạo kênh, rồi bấm Thử lại".into(),
        };
    }

    // 2. access_denied (Google OAuth consent test users)
    if lower.contains("access_denied") {
        return YouTubeErrorDetail {
            code: "access_denied".into(),
            message_vi: "Email chưa có trong Test users của OAuth consent screen".into(),
            fix_vi: "Vào Google Cloud Console → APIs & Services → OAuth consent screen → mục Test users → thêm đúng email sở hữu kênh, rồi thử lại.".into(),
        };
    }

    // 3. redirect_uri_mismatch (OAuth client type is web rather than desktop)
    if lower.contains("redirect_uri_mismatch") {
        return YouTubeErrorDetail {
            code: "redirect_uri_mismatch".into(),
            message_vi: "Client phải là loại Desktop app".into(),
            fix_vi: "Tạo lại OAuth client trên Google Cloud Console với Application type là 'Desktop app' (không phải Web application), rồi nạp lại tệp JSON.".into(),
        };
    }

    // 4. invalid_grant / token revoked
    if lower.contains("invalid_grant")
        || lower.contains("token revoked")
        || lower.contains("expired or revoked")
    {
        return YouTubeErrorDetail {
            code: "invalid_grant".into(),
            message_vi: "Phiên đăng nhập hết hạn".into(),
            fix_vi: "Bấm Kết nối YouTube lại để cấp quyền mới.".into(),
        };
    }

    // 5. quotaExceeded
    if lower.contains("quotaexceeded") {
        return YouTubeErrorDetail {
            code: "quotaExceeded".into(),
            message_vi: "Hết hạn mức API YouTube hôm nay (mặc định ~6 lần tải/ngày)".into(),
            fix_vi: "Thử lại vào ngày mai (hạn mức đặt lại vào nửa đêm giờ Thái Bình Dương PT) hoặc yêu cầu tăng hạn mức trên Google Cloud.".into(),
        };
    }

    // 6. uploadLimitExceeded
    if lower.contains("uploadlimitexceeded") {
        return YouTubeErrorDetail {
            code: "uploadLimitExceeded".into(),
            message_vi:
                "Kênh đã vượt quá giới hạn số lượng video được tải lên trong ngày của YouTube"
                    .into(),
            fix_vi: "Chờ 24 giờ rồi thử tải lại video.".into(),
        };
    }

    // 7. forbidden (generic 403)
    if lower.contains("forbidden") || status == Some(StatusCode::FORBIDDEN) {
        return YouTubeErrorDetail {
            code: "forbidden".into(),
            message_vi: "Không có quyền truy cập YouTube Data API (HTTP 403)".into(),
            fix_vi: "Kiểm tra xem YouTube Data API v3 đã được Enable trong Google Cloud project chưa, và tài khoản đã được cấp quyền chưa.".into(),
        };
    }

    // 8. timeout of loopback
    if lower.contains("timeout") || lower.contains("hết thời gian chờ") {
        return YouTubeErrorDetail {
            code: "timeout".into(),
            message_vi: "Hết thời gian chờ xác thực YouTube từ trình duyệt".into(),
            fix_vi: "Bấm Kết nối YouTube lại và hoàn tất đăng nhập trong vòng 5 phút. Đừng tắt Studio khi đang đăng nhập.".into(),
        };
    }

    // 9. network errors
    if lower.contains("dns")
        || lower.contains("connection refused")
        || lower.contains("error sending request")
        || lower.contains("tcp")
        || lower.contains("failed to connect")
        || lower.contains("lỗi kết nối mạng")
    {
        return YouTubeErrorDetail {
            code: "network".into(),
            message_vi: "Lỗi kết nối mạng tới máy chủ Google".into(),
            fix_vi: "Kiểm tra kết nối mạng internet hoặc thiết lập proxy/tường lửa của máy tính, rồi thử lại.".into(),
        };
    }

    // Fallback
    YouTubeErrorDetail {
        code: "unknown".into(),
        message_vi: if raw_body_or_msg.trim().is_empty() {
            "Lỗi không xác định khi tương tác với YouTube API".into()
        } else {
            raw_body_or_msg.trim().to_string()
        },
        fix_vi: "Kiểm tra lại cấu hình tài khoản hoặc thử lại.".into(),
    }
}

#[derive(Debug, Deserialize)]
struct GoogleClientJsonRoot {
    installed: Option<GoogleClientSecrets>,
    web: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct GoogleClientSecrets {
    client_id: Option<String>,
    client_secret: Option<String>,
}

/// Parse Google `client_secret_*.json` file.
/// Enforces that the client is an 'installed' application (Desktop app),
/// rejecting 'web' clients with an actionable Vietnamese message.
pub fn parse_google_client_json(json_content: &str) -> Result<(String, String)> {
    let parsed: GoogleClientJsonRoot = serde_json::from_str(json_content).map_err(|e| {
        ToolError::ParseError(format!(
            "Tệp JSON không đúng định dạng client OAuth của Google: {e}"
        ))
    })?;

    if parsed.web.is_some() && parsed.installed.is_none() {
        return Err(ToolError::ExecutionError(
            "Hãy tạo client loại Desktop app".into(),
        ));
    }

    let installed = parsed
        .installed
        .ok_or_else(|| ToolError::ExecutionError("Hãy tạo client loại Desktop app".into()))?;

    let client_id = installed
        .client_id
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ToolError::ExecutionError("Tệp JSON thiếu client_id.".into()))?;

    let client_secret = installed
        .client_secret
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ToolError::ExecutionError("Tệp JSON thiếu client_secret.".into()))?;

    Ok((client_id, client_secret))
}

/// Helper to resolve client credentials from either a JSON string or raw ID & secret.
pub fn resolve_google_client_credentials(
    json: Option<&str>,
    client_id: Option<&str>,
    client_secret: Option<&str>,
) -> Result<(String, String)> {
    if let Some(j) = json.filter(|s| !s.trim().is_empty()) {
        parse_google_client_json(j)
    } else {
        let cid = client_id
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ToolError::ExecutionError("Thiếu Client ID.".into()))?;
        let csec = client_secret
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ToolError::ExecutionError("Thiếu Client Secret.".into()))?;
        Ok((cid, csec))
    }
}

/// Extract the last 6 characters of client ID for display purposes.
///
/// Every Google client id ends with `.apps.googleusercontent.com`, so that suffix is dropped first;
/// otherwise every client would display as "…nt.com".
pub fn extract_client_id_last6(client_id: &str) -> String {
    let trimmed = client_id.trim();
    let core = trimmed
        .strip_suffix(".apps.googleusercontent.com")
        .unwrap_or(trimmed);
    let chars: Vec<char> = core.chars().collect();
    chars[chars.len().saturating_sub(6)..].iter().collect()
}

/// Call `channels.list?part=snippet&mine=true` to retrieve the authenticated YouTube channel's details.
pub async fn fetch_my_channel(
    client: &Client,
    base_url: &str,
    access_token: &str,
) -> Result<YouTubeChannelAccount> {
    let url = format!("{base_url}/youtube/v3/channels?part=snippet&mine=true");
    let res = client
        .get(&url)
        .header(AUTHORIZATION, format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|e| ToolError::PublishError(map_youtube_error(None, &e.to_string())))?;

    let status = res.status();
    let body_text = res.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(ToolError::PublishError(map_youtube_error(
            Some(status),
            &body_text,
        )));
    }

    let parsed: serde_json::Value = serde_json::from_str(&body_text)
        .map_err(|e| ToolError::ParseError(format!("Lỗi đọc dữ liệu kênh YouTube: {e}")))?;

    let items = parsed
        .get("items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    if items.is_empty() {
        // Account has no YouTube channel
        return Err(ToolError::PublishError(map_youtube_error(
            Some(StatusCode::UNAUTHORIZED),
            "youtubeSignupRequired",
        )));
    }

    let channel_obj = &items[0];
    let id = channel_obj
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let snippet = channel_obj.get("snippet");
    let title = snippet
        .and_then(|s| s.get("title"))
        .and_then(|v| v.as_str())
        .unwrap_or("Kênh YouTube")
        .to_string();

    let handle = snippet
        .and_then(|s| s.get("customUrl"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let avatar_url = snippet
        .and_then(|s| s.get("thumbnails"))
        .and_then(|t| {
            t.get("medium")
                .or_else(|| t.get("default"))
                .or_else(|| t.get("high"))
        })
        .and_then(|m| m.get("url"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(YouTubeChannelAccount {
        id,
        title,
        handle,
        avatar_url,
        connected_at: chrono::Utc::now().to_rfc3339(),
    })
}

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
        .map_err(|e| ToolError::PublishError(map_youtube_error(None, &e.to_string())))?;

    if !res.status().is_success() {
        let status = res.status();
        let err_text = res.text().await.unwrap_or_default();
        return Err(ToolError::PublishError(map_youtube_error(
            Some(status),
            &err_text,
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
        .map_err(|e| ToolError::PublishError(map_youtube_error(None, &e.to_string())))?;

    if !res.status().is_success() {
        let status = res.status();
        let err_text = res.text().await.unwrap_or_default();
        return Err(ToolError::PublishError(map_youtube_error(
            Some(status),
            &err_text,
        )));
    }

    let parsed: OAuthTokenResponse = res
        .json()
        .await
        .map_err(|e| ToolError::ParseError(format!("Lỗi đọc token mới: {e}")))?;

    Ok(parsed.access_token)
}

///// Run YouTube OAuth 2.0 loopback server on a random port, open system browser, and save refresh token.
pub async fn connect_youtube(
    secrets: &dyn SecretStore,
    org: &str,
    ws_name: Option<&str>,
    timeout_secs: u64,
) -> Result<YouTubeChannelAccount> {
    let client_id = secrets
        .get(org, ws_name, KEY_YOUTUBE_CLIENT_ID)
        .ok_or_else(|| {
            ToolError::PublishError(YouTubeErrorDetail {
                code: "missing_client_id".into(),
                message_vi: "Chưa nạp OAuth client ID trong Credential Manager".into(),
                fix_vi: "Hãy nạp tệp client_secret_*.json hoặc điền Client ID và Secret trong mục Cài đặt → Tài khoản đăng bài.".into(),
            })
        })?;

    let client_secret = secrets
        .get(org, ws_name, KEY_YOUTUBE_CLIENT_SECRET)
        .ok_or_else(|| {
            ToolError::PublishError(YouTubeErrorDetail {
                code: "missing_client_secret".into(),
                message_vi: "Chưa nạp OAuth client Secret trong Credential Manager".into(),
                fix_vi: "Hãy nạp tệp client_secret_*.json hoặc điền Client ID và Secret trong mục Cài đặt → Tài khoản đăng bài.".into(),
            })
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

    // Open browser. Not via `cmd /C start`: cmd treats every `&` in the query string as a command
    // separator, so the browser would get a truncated URL (and the rest would be run as commands).
    #[cfg(windows)]
    let _ = std::process::Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", &auth_url])
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
                ToolError::PublishError(map_youtube_error(
                    None,
                    "Hết thời gian chờ xác thực YouTube từ trình duyệt (timeout).",
                ))
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
    let mut error_found = None;
    for pair in query_str.split('&') {
        let mut kv = pair.splitn(2, '=');
        let k = kv.next().unwrap_or("");
        let v = kv.next().unwrap_or("");
        if k == "code" {
            code_found = Some(v.to_string());
        } else if k == "state" {
            state_found = Some(v.to_string());
        } else if k == "error" {
            error_found = Some(v.to_string());
        }
    }

    if let Some(err_code) = error_found {
        let mapped = map_youtube_error(None, &err_code);
        let error_response = format!(
            "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n\
            <html><body style=\"font-family:sans-serif;padding:2rem;\">\
            <h2 style=\"color:#c62828;\">Xác thực YouTube không thành công</h2>\
            <p><strong>Lỗi:</strong> {}</p>\
            <p>{}</p>\
            </body></html>",
            mapped.message_vi, mapped.fix_vi
        );
        let _ = stream.write_all(error_response.as_bytes()).await;
        let _ = stream.flush().await;
        return Err(ToolError::PublishError(mapped));
    }

    let code = match (code_found, state_found) {
        (Some(c), Some(s)) if s == state_expected => c,
        _ => {
            let error_response =
                "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n\
                <html><body style=\"font-family:sans-serif;padding:2rem;\">\
                <h2>Xác thực YouTube thất bại</h2>\
                <p>Mã trạng thái không khớp (state mismatch) hoặc thiếu mã ủy quyền.</p>\
                </body></html>";
            let _ = stream.write_all(error_response.as_bytes()).await;
            let _ = stream.flush().await;
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
        ToolError::PublishError(YouTubeErrorDetail {
            code: "missing_refresh_token".into(),
            message_vi: "Google không trả về refresh token".into(),
            fix_vi: "Hãy thu hồi quyền ứng dụng trên Google Account (https://myaccount.google.com/permissions) rồi kết nối lại.".into(),
        })
    })?;

    // Retrieve channel metadata via channels.list?part=snippet&mine=true
    let channel =
        fetch_my_channel(&http, DEFAULT_GOOGLE_UPLOAD_BASE, &token_resp.access_token).await?;

    // Store in SecretStore keyed by channel ID and default single token
    let channel_token_key = format!("YOUTUBE_REFRESH_TOKEN:{}", channel.id);
    secrets.set(org, ws_name, &channel_token_key, &refresh_token)?;
    secrets.set(org, ws_name, KEY_YOUTUBE_REFRESH_TOKEN, &refresh_token)?;

    if ws_name.is_some() {
        let _ = secrets.set("", None, &channel_token_key, &refresh_token);
        let _ = secrets.set("", None, KEY_YOUTUBE_REFRESH_TOKEN, &refresh_token);
    }

    Ok(channel)
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
        .map_err(|e| ToolError::PublishError(map_youtube_error(None, &e.to_string())))?;

    if !init_res.status().is_success() {
        let status = init_res.status();
        let err_text = init_res.text().await.unwrap_or_default();
        return Err(ToolError::PublishError(map_youtube_error(
            Some(status),
            &err_text,
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
            .map_err(|e| ToolError::PublishError(map_youtube_error(None, &e.to_string())))?;

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
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(ToolError::PublishError(map_youtube_error(
                Some(status),
                &err_text,
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
        .map_err(|e| ToolError::PublishError(map_youtube_error(None, &e.to_string())))?;

    if !res.status().is_success() {
        let status = res.status();
        let err_text = res.text().await.unwrap_or_default();
        return Err(ToolError::PublishError(map_youtube_error(
            Some(status),
            &err_text,
        )));
    }

    Ok(())
}

/// Parameters for publishing a video to YouTube.
#[derive(Debug, Clone)]
pub struct YouTubePublishParams<'a> {
    /// Optional channel ID to upload with (or None for default/last-used token).
    pub channel_id: Option<&'a str>,
    /// Path to the video file on disk.
    pub video_path: &'a Path,
    /// Path to the thumbnail image on disk, if any.
    pub thumbnail_path: Option<&'a Path>,
    /// Video title, description, tags, privacy status.
    pub metadata: &'a PublishMetadata,
}

/// Orchestrate complete YouTube publish using secrets and progress updates.
pub async fn publish_youtube_complete(
    secrets: &dyn SecretStore,
    org: &str,
    ws_name: Option<&str>,
    params: YouTubePublishParams<'_>,
    mut progress_cb: impl FnMut(f32, &str),
) -> Result<String> {
    let client_id = secrets
        .get(org, ws_name, KEY_YOUTUBE_CLIENT_ID)
        .ok_or_else(|| {
            ToolError::PublishError(YouTubeErrorDetail {
                code: "missing_client_id".into(),
                message_vi: "Chưa cấu hình YOUTUBE_CLIENT_ID".into(),
                fix_vi: "Hãy nạp OAuth client trong mục Cài đặt → Tài khoản đăng bài.".into(),
            })
        })?;

    let client_secret = secrets
        .get(org, ws_name, KEY_YOUTUBE_CLIENT_SECRET)
        .ok_or_else(|| {
            ToolError::PublishError(YouTubeErrorDetail {
                code: "missing_client_secret".into(),
                message_vi: "Chưa cấu hình YOUTUBE_CLIENT_SECRET".into(),
                fix_vi: "Hãy nạp OAuth client trong mục Cài đặt → Tài khoản đăng bài.".into(),
            })
        })?;

    // A chosen channel must use its own token: falling back to the legacy single token (whichever
    // account connected last) could upload to the wrong channel.
    let refresh_token = if let Some(cid) = params.channel_id {
        secrets.get(org, ws_name, &format!("YOUTUBE_REFRESH_TOKEN:{cid}"))
    } else {
        secrets.get(org, ws_name, KEY_YOUTUBE_REFRESH_TOKEN)
    }
    .ok_or_else(|| {
        ToolError::PublishError(YouTubeErrorDetail {
            code: "not_connected".into(),
            message_vi: "Chưa kết nối YouTube".into(),
            fix_vi: "Vui lòng vào Cài đặt → Tài khoản đăng bài → nhấn 'Kết nối YouTube' trước khi đăng video.".into(),
        })
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
        params.video_path,
        params.metadata,
        chunk_size,
        &mut progress_cb,
    )
    .await?;

    if let Some(thumb) = params.thumbnail_path {
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

/// Migrate legacy single `YOUTUBE_REFRESH_TOKEN` to keyed `YOUTUBE_REFRESH_TOKEN:<channelId>` format
/// by refreshing the token and fetching the channel profile.
pub async fn migrate_legacy_youtube_account(
    secrets: &dyn SecretStore,
    org: &str,
    ws_name: Option<&str>,
) -> Result<Option<YouTubeChannelAccount>> {
    let client_id = match secrets.get(org, ws_name, KEY_YOUTUBE_CLIENT_ID) {
        Some(cid) => cid,
        None => return Ok(None),
    };
    let client_secret = match secrets.get(org, ws_name, KEY_YOUTUBE_CLIENT_SECRET) {
        Some(cs) => cs,
        None => return Ok(None),
    };
    let refresh_token = match secrets.get(org, ws_name, KEY_YOUTUBE_REFRESH_TOKEN) {
        Some(rt) => rt,
        None => return Ok(None),
    };

    let http = Client::new();
    let access_token = match refresh_access_token(
        &http,
        DEFAULT_GOOGLE_OAUTH_TOKEN_URL,
        &client_id,
        &client_secret,
        &refresh_token,
    )
    .await
    {
        Ok(t) => t,
        Err(_) => return Ok(None),
    };

    let channel = match fetch_my_channel(&http, DEFAULT_GOOGLE_UPLOAD_BASE, &access_token).await {
        Ok(ch) => ch,
        Err(_) => return Ok(None),
    };

    let keyed = format!("YOUTUBE_REFRESH_TOKEN:{}", channel.id);
    let _ = secrets.set(org, ws_name, &keyed, &refresh_token);
    if ws_name.is_some() {
        let _ = secrets.set("", None, &keyed, &refresh_token);
    }

    Ok(Some(channel))
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

    #[test]
    fn test_google_client_json_parsing() {
        let valid_installed = r#"{
            "installed": {
                "client_id": "123456789-abcdef.apps.googleusercontent.com",
                "project_id": "lcs-test",
                "auth_uri": "https://accounts.google.com/o/oauth2/auth",
                "token_uri": "https://oauth2.googleapis.com/token",
                "client_secret": "GOCSPX-secret123",
                "redirect_uris": ["http://localhost"]
            }
        }"#;

        let (cid, csec) = parse_google_client_json(valid_installed).unwrap();
        assert_eq!(cid, "123456789-abcdef.apps.googleusercontent.com");
        assert_eq!(csec, "GOCSPX-secret123");

        let web_client = r#"{
            "web": {
                "client_id": "987654321-web.apps.googleusercontent.com",
                "client_secret": "web-secret"
            }
        }"#;

        let err = parse_google_client_json(web_client).unwrap_err();
        assert!(err.to_string().contains("Desktop app"));

        let bad_json = r#"{"invalid": true}"#;
        assert!(parse_google_client_json(bad_json).is_err());
    }

    #[test]
    fn test_extract_client_id_last6() {
        assert_eq!(extract_client_id_last6("123"), "123");
        assert_eq!(extract_client_id_last6("123456"), "123456");
        assert_eq!(
            extract_client_id_last6("123456789-abcdef.apps.googleusercontent.com"),
            "abcdef"
        );
        assert_eq!(extract_client_id_last6("  short  "), "short");
    }

    #[test]
    fn test_map_youtube_error_recorded_bodies() {
        // 1. youtubeSignupRequired
        let body_signup = r#"{
            "error": {
                "errors": [
                    {
                        "domain": "youtube.header",
                        "reason": "youtubeSignupRequired",
                        "message": "Unauthorized"
                    }
                ],
                "code": 401,
                "message": "Unauthorized"
            }
        }"#;
        let err1 = map_youtube_error(Some(StatusCode::UNAUTHORIZED), body_signup);
        assert_eq!(err1.code, "youtubeSignupRequired");
        assert!(err1.message_vi.contains("chưa có kênh"));
        assert!(err1.fix_vi.contains("youtube.com → Tạo kênh"));

        // 2. access_denied
        let err2 = map_youtube_error(None, "error=access_denied&error_description=User+denied");
        assert_eq!(err2.code, "access_denied");
        assert!(err2.message_vi.contains("Test users"));

        // 3. redirect_uri_mismatch
        let body_redirect =
            r#"{"error": "redirect_uri_mismatch", "error_description": "Bad Request"}"#;
        let err3 = map_youtube_error(None, body_redirect);
        assert_eq!(err3.code, "redirect_uri_mismatch");
        assert!(err3.message_vi.contains("Desktop app"));

        // 4. invalid_grant
        let body_invalid_grant = r#"{"error": "invalid_grant", "error_description": "Token has been expired or revoked."}"#;
        let err4 = map_youtube_error(Some(StatusCode::BAD_REQUEST), body_invalid_grant);
        assert_eq!(err4.code, "invalid_grant");
        assert!(err4.message_vi.contains("hết hạn"));
        assert!(err4.fix_vi.contains("Kết nối YouTube lại"));

        // 5. quotaExceeded
        let body_quota = r#"{
            "error": {
                "errors": [
                    {
                        "domain": "youtube.quota",
                        "reason": "quotaExceeded",
                        "message": "The request cannot be completed because you have exceeded your quota."
                    }
                ],
                "code": 403,
                "message": "The request cannot be completed because you have exceeded your quota."
            }
        }"#;
        let err5 = map_youtube_error(Some(StatusCode::FORBIDDEN), body_quota);
        assert_eq!(err5.code, "quotaExceeded");
        assert!(err5.message_vi.contains("hạn mức API YouTube"));
        assert!(err5.fix_vi.contains("ngày mai"));

        // 6. uploadLimitExceeded
        let body_upload_limit = r#"{
            "error": {
                "errors": [
                    {
                        "domain": "youtube.video",
                        "reason": "uploadLimitExceeded",
                        "message": "The user has exceeded the number of videos they may upload."
                    }
                ],
                "code": 400,
                "message": "The user has exceeded the number of videos they may upload."
            }
        }"#;
        let err6 = map_youtube_error(Some(StatusCode::BAD_REQUEST), body_upload_limit);
        assert_eq!(err6.code, "uploadLimitExceeded");
        assert!(err6.message_vi.contains("giới hạn"));
        assert!(err6.fix_vi.contains("24 giờ"));

        // 7. forbidden
        let body_forbidden = r#"{
            "error": {
                "code": 403,
                "message": "The caller does not have permission",
                "status": "PERMISSION_DENIED"
            }
        }"#;
        let err7 = map_youtube_error(Some(StatusCode::FORBIDDEN), body_forbidden);
        assert_eq!(err7.code, "forbidden");
        assert!(err7.message_vi.contains("403"));

        // 8. timeout
        let err8 = map_youtube_error(
            None,
            "Hết thời gian chờ xác thực YouTube từ trình duyệt (timeout).",
        );
        assert_eq!(err8.code, "timeout");
        assert!(err8.message_vi.contains("thời gian chờ"));

        // 9. network
        let err9 = map_youtube_error(None, "error sending request for url: connection refused");
        assert_eq!(err9.code, "network");
        assert!(err9.message_vi.contains("kết nối mạng"));
    }
}
