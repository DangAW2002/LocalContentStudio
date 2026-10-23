//! Facebook Graph API client for Page photo and video publishing, OAuth authentication,
//! and Windows Credential Manager token management.

use crate::error::{Result, ToolError};
use crate::publish::model::{PublishMetadata, PublishPrivacy};
use crate::publish::YouTubeErrorDetail;
use crate::secret::SecretStore;
use reqwest::header::{CONTENT_LENGTH, CONTENT_TYPE};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub const DEFAULT_FACEBOOK_GRAPH_BASE: &str = "https://graph.facebook.com/v25.0";
pub const DEFAULT_FACEBOOK_OAUTH_DIALOG_URL: &str = "https://www.facebook.com/v25.0/dialog/oauth";
pub const DEFAULT_FACEBOOK_LOOPBACK_PORT: u16 = 8911;

/// Secret keys used for Facebook integration.
pub const KEY_FACEBOOK_APP_ID: &str = "FACEBOOK_APP_ID";
pub const KEY_FACEBOOK_APP_SECRET: &str = "FACEBOOK_APP_SECRET";
pub const KEY_FACEBOOK_PAGE_TOKEN: &str = "FACEBOOK_PAGE_TOKEN";

/// Non-secret Facebook Page metadata stored in app.db and displayed in UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FacebookPageAccount {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    pub connected_at: String,
}

/// Raw page item returned from Meta Graph API `/me/accounts`.
#[derive(Debug, Clone, Deserialize)]
pub struct FacebookAccountsData {
    pub data: Vec<FacebookPageItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FacebookPageItem {
    pub id: String,
    pub name: String,
    pub access_token: String,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub picture: Option<FacebookPictureDataWrapper>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FacebookPictureDataWrapper {
    pub data: FacebookPictureData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FacebookPictureData {
    pub url: String,
}

/// Token response from Graph API oauth/access_token.
#[derive(Debug, Deserialize)]
pub struct FacebookTokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub token_type: Option<String>,
    #[serde(default)]
    pub expires_in: Option<u64>,
}

/// Response from video upload start phase.
#[derive(Debug, Deserialize)]
pub struct FacebookVideoStartResponse {
    #[serde(default)]
    pub upload_session_id: Option<String>,
    #[serde(default)]
    pub video_id: Option<String>,
    #[serde(default)]
    pub start_offset: Option<String>,
    #[serde(default)]
    pub end_offset: Option<String>,
}

/// Response from video upload transfer phase.
#[derive(Debug, Deserialize)]
pub struct FacebookVideoTransferResponse {
    #[serde(default)]
    pub start_offset: Option<String>,
    #[serde(default)]
    pub end_offset: Option<String>,
}

/// Response from photo post or finish video.
#[derive(Debug, Deserialize)]
pub struct FacebookPostResponse {
    pub id: String,
    #[serde(default)]
    pub post_id: Option<String>,
}

/// Map Meta Graph API and OAuth errors to Vietnamese user guidance.
pub fn map_facebook_error(status: Option<StatusCode>, raw_body_or_msg: &str) -> YouTubeErrorDetail {
    let lower = raw_body_or_msg.to_lowercase();

    // 1. Session / token expired (Code 190)
    if lower.contains("\"code\": 190")
        || lower.contains("\"code\":190")
        || lower.contains("error validating access token")
        || lower.contains("session has expired")
        || lower.contains("token was revoked")
    {
        return YouTubeErrorDetail {
            code: "facebook_token_expired".into(),
            message_vi: "Phiên kết nối Facebook đã hết hạn hoặc bị thu hồi".into(),
            fix_vi: "Vào Cài đặt → Tài khoản đăng bài → kết nối lại Facebook Page.".into(),
        };
    }

    // 2. Permissions error (Code 200, 298, or missing pages_manage_posts)
    if lower.contains("\"code\": 200")
        || lower.contains("\"code\":200")
        || lower.contains("permission")
        || lower.contains("pages_manage_posts")
        || lower.contains("pages_read_engagement")
        || lower.contains("pages_show_list")
    {
        return YouTubeErrorDetail {
            code: "facebook_permission_denied".into(),
            message_vi: "Thiếu quyền quản lý hoặc đăng bài trên Facebook Page".into(),
            fix_vi: "Đảm bảo tài khoản Facebook của bạn là Quản trị viên (Admin) hoặc Biên tập viên của Page và cấp đủ quyền khi đăng nhập.".into(),
        };
    }

    // 3. Redirect URI mismatch
    if lower.contains("redirect_uri")
        || lower.contains("url blocked")
        || lower.contains("not whitelisted")
    {
        return YouTubeErrorDetail {
            code: "facebook_redirect_uri_mismatch".into(),
            message_vi: "Redirect URI chưa được thêm vào Meta App".into(),
            fix_vi: "Giữ Meta App ở chế độ Phát triển (Meta tự cho phép http://localhost:8911/callback) hoặc thêm URI đó vào Valid OAuth Redirect URIs nếu Meta cho phép.".into(),
        };
    }

    // 4. Rate limit (Code 4, 17, 32, 613)
    if lower.contains("\"code\": 4")
        || lower.contains("\"code\": 17")
        || lower.contains("\"code\": 32")
        || lower.contains("\"code\": 613")
        || lower.contains("rate limit")
        || lower.contains("calls have been reached")
    {
        return YouTubeErrorDetail {
            code: "facebook_rate_limit".into(),
            message_vi: "Đã vượt hạn mức gọi API Facebook Page trong ngày".into(),
            fix_vi: "Facebook giới hạn tần suất đăng bài. Vui lòng thử lại sau vài giờ.".into(),
        };
    }

    // 5. File size / invalid format error
    if lower.contains("file size")
        || lower.contains("maximum size")
        || lower.contains("1363019")
        || lower.contains("6000")
    {
        return YouTubeErrorDetail {
            code: "facebook_file_limit".into(),
            message_vi: "Tệp tin vượt quá dung lượng cho phép hoặc định dạng video lỗi".into(),
            fix_vi:
                "Video Facebook tối đa 4GB (chuẩn H.264/AAC, moov atom ở đầu). Ảnh tối đa 100MB."
                    .into(),
        };
    }

    // 6. Timeout
    if lower.contains("timeout") || lower.contains("hết thời gian chờ") {
        return YouTubeErrorDetail {
            code: "timeout".into(),
            message_vi: "Hết thời gian chờ xác thực từ trình duyệt (timeout)".into(),
            fix_vi: "Vui lòng bấm 'Kết nối Facebook Page' và hoàn tất đăng nhập trong vòng 5 phút."
                .into(),
        };
    }

    // 7. Network error
    if lower.contains("connection refused")
        || lower.contains("dns")
        || lower.contains("error sending request")
    {
        return YouTubeErrorDetail {
            code: "network".into(),
            message_vi: "Lỗi kết nối mạng đến máy chủ Facebook".into(),
            fix_vi: "Vui lòng kiểm tra đường truyền Internet hoặc VPN rồi thử lại.".into(),
        };
    }

    // Fallback status codes
    if status == Some(StatusCode::FORBIDDEN) {
        return YouTubeErrorDetail {
            code: "facebook_forbidden".into(),
            message_vi: "Máy chủ Facebook từ chối yêu cầu (403 Forbidden)".into(),
            fix_vi: "Kiểm tra quyền tài khoản hoặc trạng thái ứng dụng trên Meta Developer Portal."
                .into(),
        };
    }

    YouTubeErrorDetail {
        code: "facebook_error".into(),
        message_vi: format!("Lỗi Facebook API: {raw_body_or_msg}"),
        fix_vi: "Kiểm tra lại cấu hình App ID/Secret và thử lại.".into(),
    }
}

/// Build Meta OAuth dialog URL.
pub fn build_facebook_oauth_url(app_id: &str, redirect_uri: &str, state: &str) -> String {
    format!(
        "{}?client_id={}&redirect_uri={}&scope=pages_show_list,business_management,pages_manage_posts,pages_read_engagement&response_type=code&state={}",
        DEFAULT_FACEBOOK_OAUTH_DIALOG_URL,
        urlencoding(app_id),
        urlencoding(redirect_uri),
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

/// Helper to build manual multipart form-data payload with boundary.
fn build_multipart_form(
    boundary: &str,
    text_fields: &[(&str, &str)],
    file_field: (&str, &str, &str, &[u8]), // (field_name, filename, mime, bytes)
) -> Vec<u8> {
    let mut body = Vec::new();

    for (name, val) in text_fields {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n{val}\r\n").as_bytes(),
        );
    }

    let (name, filename, mime, bytes) = file_field;
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"{name}\"; filename=\"{filename}\"\r\nContent-Type: {mime}\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(bytes);
    body.extend_from_slice(b"\r\n");

    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    body
}

/// Exchange code for short-lived token, exchange for long-lived token, and query pages with Page tokens.
pub async fn exchange_facebook_code_and_fetch_pages(
    client: &Client,
    graph_base: &str,
    app_id: &str,
    app_secret: &str,
    code: &str,
    redirect_uri: &str,
) -> Result<Vec<(FacebookPageAccount, String)>> {
    // 1. Exchange code -> Short-lived user token
    let token_url = format!("{graph_base}/oauth/access_token");
    let params_code = [
        ("client_id", app_id),
        ("client_secret", app_secret),
        ("redirect_uri", redirect_uri),
        ("code", code),
    ];

    let res1 = client
        .get(&token_url)
        .query(&params_code)
        .send()
        .await
        .map_err(|e| ToolError::PublishError(map_facebook_error(None, &e.to_string())))?;

    if !res1.status().is_success() {
        let status = res1.status();
        let body = res1.text().await.unwrap_or_default();
        return Err(ToolError::PublishError(map_facebook_error(
            Some(status),
            &body,
        )));
    }

    let token_resp1: FacebookTokenResponse = res1
        .json()
        .await
        .map_err(|e| ToolError::ParseError(format!("Lỗi phân tích mã token người dùng: {e}")))?;

    // 2. Exchange short-lived -> Long-lived user token (~60 days)
    let params_exchange = [
        ("grant_type", "fb_exchange_token"),
        ("client_id", app_id),
        ("client_secret", app_secret),
        ("fb_exchange_token", &token_resp1.access_token),
    ];

    let res2 = client
        .get(&token_url)
        .query(&params_exchange)
        .send()
        .await
        .map_err(|e| ToolError::PublishError(map_facebook_error(None, &e.to_string())))?;

    let long_lived_user_token = if res2.status().is_success() {
        let token_resp2: FacebookTokenResponse = res2
            .json()
            .await
            .map_err(|e| ToolError::ParseError(format!("Lỗi phân tích token dài hạn: {e}")))?;
        token_resp2.access_token
    } else {
        token_resp1.access_token
    };

    // 3. Fetch Pages via /me/accounts?fields=id,name,access_token,category,picture
    let accounts_url = format!("{graph_base}/me/accounts");
    let res3 = client
        .get(&accounts_url)
        .query(&[
            ("access_token", &long_lived_user_token),
            (
                "fields",
                &"id,name,access_token,category,picture".to_string(),
            ),
        ])
        .send()
        .await
        .map_err(|e| ToolError::PublishError(map_facebook_error(None, &e.to_string())))?;

    if !res3.status().is_success() {
        let status = res3.status();
        let body = res3.text().await.unwrap_or_default();
        return Err(ToolError::PublishError(map_facebook_error(
            Some(status),
            &body,
        )));
    }

    let accounts: FacebookAccountsData = res3.json().await.map_err(|e| {
        ToolError::ParseError(format!("Lỗi phân tích danh sách Facebook Page: {e}"))
    })?;

    let now = chrono::Utc::now().to_rfc3339();
    let mut result = Vec::new();

    for item in accounts.data {
        let account = FacebookPageAccount {
            id: item.id,
            name: item.name,
            category: item.category,
            avatar_url: item.picture.map(|p| p.data.url),
            connected_at: now.clone(),
        };
        result.push((account, item.access_token));
    }

    Ok(result)
}

/// Connect Facebook Pages via OAuth loopback redirect on localhost:8911 (Meta only accepts plain http for localhost).
/// Stores page access tokens into SecretStore and returns all retrieved accounts.
pub async fn connect_facebook(
    secrets: &dyn SecretStore,
    org: &str,
    ws_name: Option<&str>,
    timeout_secs: u64,
) -> Result<Vec<FacebookPageAccount>> {
    let app_id = secrets
        .get(org, ws_name, KEY_FACEBOOK_APP_ID)
        .ok_or_else(|| {
            ToolError::PublishError(YouTubeErrorDetail {
                code: "missing_facebook_app_id".into(),
                message_vi: "Chưa cấu hình FACEBOOK_APP_ID".into(),
                fix_vi: "Hãy nhập Facebook App ID trong Cài đặt → Tài khoản đăng bài.".into(),
            })
        })?;

    let app_secret = secrets
        .get(org, ws_name, KEY_FACEBOOK_APP_SECRET)
        .ok_or_else(|| {
            ToolError::PublishError(YouTubeErrorDetail {
                code: "missing_facebook_app_secret".into(),
                message_vi: "Chưa cấu hình FACEBOOK_APP_SECRET".into(),
                fix_vi: "Hãy nhập Facebook App Secret trong Cài đặt → Tài khoản đăng bài.".into(),
            })
        })?;

    let port = DEFAULT_FACEBOOK_LOOPBACK_PORT;
    let redirect_uri = format!("http://localhost:{port}/callback");

    let listener = TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .map_err(|e| {
            ToolError::ExecutionError(format!(
                "Không thể mở cổng loopback {port} (có thể đang có ứng dụng khác chiếm dụng): {e}"
            ))
        })?;

    let state_expected = format!("state-{}", uuid::Uuid::new_v4().simple());
    let auth_url = build_facebook_oauth_url(&app_id, &redirect_uri, &state_expected);

    // Open system browser
    #[cfg(windows)]
    let _ = std::process::Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", &auth_url])
        .spawn();

    #[cfg(not(windows))]
    let _ = std::process::Command::new("xdg-open")
        .arg(&auth_url)
        .spawn();

    // Wait for the browser redirect (preconnects and favicon requests are skipped).
    let (mut stream, request_str) = crate::publish::loopback::accept_oauth_redirect(
        &listener,
        Duration::from_secs(timeout_secs),
    )
    .await
    .map_err(|e| match e {
        crate::publish::loopback::LoopbackError::Timeout => {
            ToolError::PublishError(map_facebook_error(
                None,
                "Hết thời gian chờ xác thực Facebook từ trình duyệt (timeout).",
            ))
        }
        crate::publish::loopback::LoopbackError::Io(e) => {
            ToolError::ExecutionError(format!("Lỗi nhận kết nối loopback: {e}"))
        }
    })?;

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
        } else if k == "error" || k == "error_description" {
            error_found = Some(v.to_string());
        }
    }

    if let Some(err_code) = error_found {
        let mapped = map_facebook_error(None, &err_code);
        let error_response = format!(
            "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n\
            <html><body style=\"font-family:sans-serif;padding:2rem;\">\
            <h2 style=\"color:#c62828;\">Xác thực Facebook không thành công</h2>\
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
                <h2>Xác thực Facebook thất bại</h2>\
                <p>Mã trạng thái không khớp (state mismatch) hoặc thiếu mã ủy quyền.</p>\
                </body></html>";
            let _ = stream.write_all(error_response.as_bytes()).await;
            let _ = stream.flush().await;
            return Err(ToolError::ExecutionError(
                "Xác thực thất bại: state không khớp hoặc thiếu mã xác thực.".into(),
            ));
        }
    };

    let client = Client::new();
    let pages = exchange_facebook_code_and_fetch_pages(
        &client,
        DEFAULT_FACEBOOK_GRAPH_BASE,
        &app_id,
        &app_secret,
        &code,
        &redirect_uri,
    )
    .await?;

    let success_response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n\
        <html><body style=\"font-family:sans-serif;padding:2rem;\">\
        <h2 style=\"color:#2e7d32;\">Kết nối Facebook thành công!</h2>\
        <p>Đã tìm thấy {} trang Facebook. Bạn có thể đóng cửa sổ này và quay lại Local Content Studio.</p>\
        </body></html>",
        pages.len()
    );
    let _ = stream.write_all(success_response.as_bytes()).await;
    let _ = stream.flush().await;

    let mut accounts = Vec::new();
    for (acc, token) in pages {
        let key = format!("{KEY_FACEBOOK_PAGE_TOKEN}:{}", acc.id);
        secrets.set(org, ws_name, &key, &token)?;
        if ws_name.is_some() {
            let _ = secrets.set("", None, &key, &token);
        }
        accounts.push(acc);
    }

    Ok(accounts)
}

/// Parameters for uploading content to a Facebook Page.
#[derive(Debug, Clone)]
pub struct FacebookPublishParams<'a> {
    pub page_id: &'a str,
    pub file_path: &'a Path,
    pub metadata: &'a PublishMetadata,
}

/// Upload photo to a Facebook Page (single image upload via multipart/form-data).
pub async fn upload_photo_to_page(
    client: &Client,
    graph_base: &str,
    page_token: &str,
    page_id: &str,
    photo_path: &Path,
    caption: &str,
    published: bool,
) -> Result<String> {
    if !photo_path.exists() {
        return Err(ToolError::ExecutionError(format!(
            "Tệp ảnh không tồn tại: {}",
            photo_path.display()
        )));
    }

    let file_bytes = tokio::fs::read(photo_path)
        .await
        .map_err(|e| ToolError::Io {
            path: photo_path.to_path_buf(),
            source: e,
        })?;

    let file_name = photo_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("image.png")
        .to_string();

    let mime = if file_name.ends_with(".jpg") || file_name.ends_with(".jpeg") {
        "image/jpeg"
    } else {
        "image/png"
    };

    let boundary = format!("----LcsBoundary{}", uuid::Uuid::new_v4().simple());
    let pub_str = published.to_string();
    let text_fields = [
        ("access_token", page_token),
        ("caption", caption),
        ("published", &pub_str),
    ];
    let file_field = ("source", file_name.as_str(), mime, file_bytes.as_slice());
    let body_bytes = build_multipart_form(&boundary, &text_fields, file_field);

    let url = format!("{graph_base}/{page_id}/photos");
    let res = client
        .post(&url)
        .header(
            CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .header(CONTENT_LENGTH, body_bytes.len().to_string())
        .body(body_bytes)
        .send()
        .await
        .map_err(|e| ToolError::PublishError(map_facebook_error(None, &e.to_string())))?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(ToolError::PublishError(map_facebook_error(
            Some(status),
            &body,
        )));
    }

    let post_resp: FacebookPostResponse = res
        .json()
        .await
        .map_err(|e| ToolError::ParseError(format!("Lỗi phân tích phản hồi bài đăng ảnh: {e}")))?;

    Ok(post_resp.post_id.unwrap_or(post_resp.id))
}

/// Upload video to Facebook Page via 3-phase Resumable Upload API.
#[allow(clippy::too_many_arguments)]
pub async fn upload_video_resumable_to_page(
    client: &Client,
    graph_base: &str,
    page_token: &str,
    page_id: &str,
    video_path: &Path,
    metadata: &PublishMetadata,
    published: bool,
    chunk_size: usize,
    mut progress_cb: impl FnMut(f32, &str),
) -> Result<String> {
    if !video_path.exists() {
        return Err(ToolError::ExecutionError(format!(
            "Tệp video không tồn tại: {}",
            video_path.display()
        )));
    }

    let total_size = video_path
        .metadata()
        .map_err(|e| ToolError::Io {
            path: video_path.to_path_buf(),
            source: e,
        })?
        .len();

    // Max 4GB
    if total_size > 4 * 1024 * 1024 * 1024 {
        return Err(ToolError::PublishError(YouTubeErrorDetail {
            code: "facebook_file_too_large".into(),
            message_vi: "Video vượt quá giới hạn 4GB của Facebook".into(),
            fix_vi: "Vui lòng nén video hoặc chọn bản render có dung lượng dưới 4GB.".into(),
        }));
    }

    let videos_endpoint = format!("{graph_base}/{page_id}/videos");

    // Phase 1: Start
    progress_cb(0.05, "Khởi tạo phiên tải lên video Facebook...");
    let start_res = client
        .post(&videos_endpoint)
        .form(&[
            ("upload_phase", "start"),
            ("file_size", &total_size.to_string()),
            ("access_token", page_token),
        ])
        .send()
        .await
        .map_err(|e| ToolError::PublishError(map_facebook_error(None, &e.to_string())))?;

    if !start_res.status().is_success() {
        let status = start_res.status();
        let body = start_res.text().await.unwrap_or_default();
        return Err(ToolError::PublishError(map_facebook_error(
            Some(status),
            &body,
        )));
    }

    let start_data: FacebookVideoStartResponse = start_res.json().await.map_err(|e| {
        ToolError::ParseError(format!("Lỗi phân tích phiên bắt đầu tải video: {e}"))
    })?;

    let upload_session_id = start_data.upload_session_id.ok_or_else(|| {
        ToolError::ExecutionError("Facebook không trả về upload_session_id".into())
    })?;

    // Phase 2: Transfer chunks
    let mut file = tokio::fs::File::open(video_path)
        .await
        .map_err(|e| ToolError::Io {
            path: video_path.to_path_buf(),
            source: e,
        })?;

    let mut current_offset: u64 = 0;
    while current_offset < total_size {
        let remaining = (total_size - current_offset) as usize;
        let bytes_to_read = std::cmp::min(chunk_size, remaining);
        let mut buffer = vec![0u8; bytes_to_read];
        file.read_exact(&mut buffer)
            .await
            .map_err(|e| ToolError::Io {
                path: video_path.to_path_buf(),
                source: e,
            })?;

        let fraction = (current_offset as f32 / total_size as f32) * 0.8 + 0.1;
        progress_cb(
            fraction,
            &format!(
                "Đang tải video lên Facebook: {}%",
                (fraction * 100.0) as u32
            ),
        );

        let boundary = format!("----LcsChunkBoundary{}", uuid::Uuid::new_v4().simple());
        let offset_str = current_offset.to_string();
        let text_fields = [
            ("upload_phase", "transfer"),
            ("upload_session_id", upload_session_id.as_str()),
            ("start_offset", offset_str.as_str()),
            ("access_token", page_token),
        ];
        let file_field = (
            "video_file_chunk",
            "chunk.mp4",
            "video/mp4",
            buffer.as_slice(),
        );
        let body_bytes = build_multipart_form(&boundary, &text_fields, file_field);

        let transfer_res = client
            .post(&videos_endpoint)
            .header(
                CONTENT_TYPE,
                format!("multipart/form-data; boundary={boundary}"),
            )
            .header(CONTENT_LENGTH, body_bytes.len().to_string())
            .body(body_bytes)
            .send()
            .await
            .map_err(|e| ToolError::PublishError(map_facebook_error(None, &e.to_string())))?;

        if !transfer_res.status().is_success() {
            let status = transfer_res.status();
            let body = transfer_res.text().await.unwrap_or_default();
            return Err(ToolError::PublishError(map_facebook_error(
                Some(status),
                &body,
            )));
        }

        let trans_data: FacebookVideoTransferResponse = transfer_res.json().await.map_err(|e| {
            ToolError::ParseError(format!("Lỗi phân tích phản hồi chunk video: {e}"))
        })?;

        if let Some(next_offset) = trans_data.end_offset.and_then(|o| o.parse::<u64>().ok()) {
            if next_offset > current_offset {
                current_offset = next_offset;
            } else {
                current_offset += bytes_to_read as u64;
            }
        } else {
            current_offset += bytes_to_read as u64;
        }
    }

    // Phase 3: Finish
    progress_cb(0.92, "Hoàn tất xử lý video trên Facebook...");
    let pub_str = published.to_string();
    let finish_params = [
        ("upload_phase", "finish"),
        ("upload_session_id", upload_session_id.as_str()),
        ("title", metadata.title.as_str()),
        ("description", metadata.description.as_str()),
        ("access_token", page_token),
        ("published", pub_str.as_str()),
    ];

    let finish_res = client
        .post(&videos_endpoint)
        .form(&finish_params)
        .send()
        .await
        .map_err(|e| ToolError::PublishError(map_facebook_error(None, &e.to_string())))?;

    if !finish_res.status().is_success() {
        let status = finish_res.status();
        let body = finish_res.text().await.unwrap_or_default();
        return Err(ToolError::PublishError(map_facebook_error(
            Some(status),
            &body,
        )));
    }

    let finish_data: FacebookPostResponse = finish_res.json().await.map_err(|e| {
        ToolError::ParseError(format!("Lỗi phân tích phản hồi hoàn tất video: {e}"))
    })?;

    progress_cb(1.0, "Đăng video lên Facebook Page thành công!");
    Ok(finish_data.post_id.unwrap_or(finish_data.id))
}

/// Orchestrate complete Facebook Page publish flow using secrets, channel limits, and progress callbacks.
pub async fn publish_facebook_complete(
    secrets: &dyn SecretStore,
    org: &str,
    ws_name: Option<&str>,
    params: FacebookPublishParams<'_>,
    mut progress_cb: impl FnMut(f32, &str),
) -> Result<String> {
    let key = format!("{KEY_FACEBOOK_PAGE_TOKEN}:{}", params.page_id);
    let page_token = secrets.get(org, ws_name, &key).ok_or_else(|| {
        ToolError::PublishError(YouTubeErrorDetail {
            code: "facebook_page_not_connected".into(),
            message_vi: "Chưa kết nối Facebook Page này hoặc phiên đăng nhập đã hết hạn".into(),
            fix_vi: "Vui lòng vào Cài đặt → Tài khoản đăng bài → kết nối lại Facebook Page.".into(),
        })
    })?;

    let is_video = params
        .file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("mp4") || ext.eq_ignore_ascii_case("mov"))
        .unwrap_or(false);

    // Draft / Unpublished mode by default for testing safety
    let published = match params.metadata.privacy {
        PublishPrivacy::Public => true,
        PublishPrivacy::Private | PublishPrivacy::Unlisted => false,
    };

    let client = Client::new();

    if is_video {
        let chunk_size = 1024 * 1024; // 1 MB chunk
        let post_id = upload_video_resumable_to_page(
            &client,
            DEFAULT_FACEBOOK_GRAPH_BASE,
            &page_token,
            params.page_id,
            params.file_path,
            params.metadata,
            published,
            chunk_size,
            &mut progress_cb,
        )
        .await?;

        Ok(format!("https://www.facebook.com/{post_id}"))
    } else {
        progress_cb(0.3, "Đang tải ảnh lên Facebook Page...");
        let post_id = upload_photo_to_page(
            &client,
            DEFAULT_FACEBOOK_GRAPH_BASE,
            &page_token,
            params.page_id,
            params.file_path,
            &params.metadata.description,
            published,
        )
        .await?;
        progress_cb(1.0, "Đăng ảnh lên Facebook Page thành công!");

        Ok(format!("https://www.facebook.com/{post_id}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_map_facebook_errors() {
        let err1 = map_facebook_error(
            Some(StatusCode::UNAUTHORIZED),
            r#"{"error": {"code": 190, "message": "Error validating access token: Session has expired."}}"#,
        );
        assert_eq!(err1.code, "facebook_token_expired");
        assert!(err1.message_vi.contains("hết hạn"));

        let err2 = map_facebook_error(
            Some(StatusCode::FORBIDDEN),
            r#"{"error": {"code": 200, "message": "(#200) Requires pages_manage_posts permission"}}"#,
        );
        assert_eq!(err2.code, "facebook_permission_denied");
        assert!(err2.message_vi.contains("Thiếu quyền"));

        let err3 = map_facebook_error(
            None,
            "URL Blocked: This redirect failed because the redirect URI is not whitelisted",
        );
        assert_eq!(err3.code, "facebook_redirect_uri_mismatch");
        assert!(err3.fix_vi.contains("localhost:8911"));

        let err4 = map_facebook_error(
            Some(StatusCode::BAD_REQUEST),
            r#"{"error": {"code": 6000, "message": "There was a problem uploading your video file."}}"#,
        );
        assert_eq!(err4.code, "facebook_file_limit");
    }

    #[tokio::test]
    async fn test_mock_facebook_photo_upload_success() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://127.0.0.1:{}", addr.port());

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf).await.unwrap();

            let resp_body = r#"{"id": "photo_12345", "post_id": "page_post_99999"}"#;
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                resp_body.len(),
                resp_body
            );
            stream.write_all(resp.as_bytes()).await.unwrap();
            stream.flush().await.unwrap();
        });

        let mut temp_photo = NamedTempFile::new().unwrap();
        use std::io::Write;
        temp_photo.write_all(b"fake-image-bytes").unwrap();

        let client = Client::new();
        let post_id = upload_photo_to_page(
            &client,
            &base_url,
            "mock-page-token",
            "page_123",
            temp_photo.path(),
            "Mock Caption",
            false,
        )
        .await
        .unwrap();

        assert_eq!(post_id, "page_post_99999");
    }

    #[tokio::test]
    async fn test_mock_facebook_video_resumable_success() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://127.0.0.1:{}", addr.port());

        tokio::spawn(async move {
            // 1. Start Phase
            let (mut stream1, _) = listener.accept().await.unwrap();
            let mut buf1 = [0u8; 4096];
            let _ = stream1.read(&mut buf1).await.unwrap();
            let start_json = r#"{"upload_session_id": "sess_fb_123", "video_id": "vid_fb_999", "start_offset": "0", "end_offset": "2048"}"#;
            let resp1 = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                start_json.len(),
                start_json
            );
            stream1.write_all(resp1.as_bytes()).await.unwrap();
            stream1.flush().await.unwrap();

            // 2. Transfer Phase
            let (mut stream2, _) = listener.accept().await.unwrap();
            let mut buf2 = [0u8; 4096];
            let _ = stream2.read(&mut buf2).await.unwrap();
            let transfer_json = r#"{"start_offset": "2048", "end_offset": "2048"}"#;
            let resp2 = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                transfer_json.len(),
                transfer_json
            );
            stream2.write_all(resp2.as_bytes()).await.unwrap();
            stream2.flush().await.unwrap();

            // 3. Finish Phase
            let (mut stream3, _) = listener.accept().await.unwrap();
            let mut buf3 = [0u8; 4096];
            let _ = stream3.read(&mut buf3).await.unwrap();
            let finish_json = r#"{"id": "vid_fb_999", "post_id": "page_post_8888"}"#;
            let resp3 = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                finish_json.len(),
                finish_json
            );
            stream3.write_all(resp3.as_bytes()).await.unwrap();
            stream3.flush().await.unwrap();
        });

        let mut temp_video = NamedTempFile::new().unwrap();
        use std::io::Write;
        let dummy = vec![0x33u8; 2048];
        temp_video.write_all(&dummy).unwrap();

        let client = Client::new();
        let metadata = PublishMetadata {
            title: "Test FB Video".into(),
            description: "Test FB Description".into(),
            tags: vec![],
            privacy: PublishPrivacy::Private,
            schedule: None,
            thumbnail: None,
        };

        let mut progress = Vec::new();
        let post_id = upload_video_resumable_to_page(
            &client,
            &base_url,
            "mock-token",
            "page_123",
            temp_video.path(),
            &metadata,
            false,
            2048,
            |p, msg| progress.push((p, msg.to_string())),
        )
        .await
        .unwrap();

        assert_eq!(post_id, "page_post_8888");
        assert!(!progress.is_empty());
    }

    #[tokio::test]
    async fn test_mock_facebook_permission_denied() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://127.0.0.1:{}", addr.port());

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf).await.unwrap();

            let err_body = r#"{"error": {"code": 200, "message": "(#200) Requires pages_manage_posts permission"}}"#;
            let resp = format!(
                "HTTP/1.1 403 Forbidden\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                err_body.len(),
                err_body
            );
            stream.write_all(resp.as_bytes()).await.unwrap();
            stream.flush().await.unwrap();
        });

        let mut temp_photo = NamedTempFile::new().unwrap();
        use std::io::Write;
        temp_photo.write_all(b"fake-bytes").unwrap();

        let client = Client::new();
        let err = upload_photo_to_page(
            &client,
            &base_url,
            "bad-token",
            "page_123",
            temp_photo.path(),
            "Caption",
            false,
        )
        .await
        .unwrap_err();

        if let ToolError::PublishError(detail) = err {
            assert_eq!(detail.code, "facebook_permission_denied");
        } else {
            panic!("Expected ToolError::PublishError");
        }
    }
}
