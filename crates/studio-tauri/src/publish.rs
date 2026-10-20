//! IPC commands for publishing workflows: list, get, update, confirm, cancel, and YouTube connect.

use crate::error::{CmdError, CmdResult};
use crate::state::AppState;
use std::path::PathBuf;
use studio_tools::publish::{
    connect_youtube, export_bundle, extract_client_id_last6, publish_facebook_complete,
    publish_youtube_complete, resolve_google_client_credentials, FacebookPublishParams,
    PublishMetadata, PublishRequest, PublishStatus, PublishTarget, YouTubeChannelAccount,
    YouTubePublishParams, KEY_FACEBOOK_PAGE_TOKEN, KEY_YOUTUBE_CLIENT_ID,
    KEY_YOUTUBE_CLIENT_SECRET, KEY_YOUTUBE_REFRESH_TOKEN,
};
use studio_workspace::WorkspaceDb;

/// Helper to emit `job-event` matching the scheduler contract.
fn emit_publish_event(
    state: &AppState,
    job_id: &str,
    state_str: &str,
    progress: Option<f32>,
    message: &str,
    error: Option<&str>,
) {
    let snapshot_state = match state_str {
        "running" => "RUNNING",
        "succeeded" => "DONE",
        "failed" => "FAILED",
        "cancelled" => "CANCELLED",
        _ => "PENDING",
    };

    let payload = serde_json::json!({
        "jobId": job_id,
        "kind": "publish",
        "pool": "network",
        "state": state_str,
        "progress": progress,
        "message": message,
        "peakMemoryBytes": None::<u64>,
        "event": {
            "id": job_id,
            "title": "Xuất bản & Đăng tải",
            "kind": "publish",
            "state": snapshot_state,
            "progress": progress,
            "stageName": message,
            "createdAt": chrono::Utc::now().to_rfc3339(),
            "error": error,
        }
    });

    state.emit("job-event", payload);
}

/// Prepare a draft publish request and validate against channel limits.
pub fn publish_prepare_impl(
    state: &AppState,
    project: Option<String>,
    artifact: String,
    channel: String,
    target: String,
    metadata: PublishMetadata,
) -> CmdResult<studio_tools::publish::PrepareOutcome> {
    let ws = state.workspace()?;
    let db = WorkspaceDb::open(&ws)?;
    let parsed_target = target.parse::<PublishTarget>().map_err(CmdError::from)?;
    let outcome = studio_tools::publish::prepare_publish(
        &ws,
        db.conn(),
        &artifact,
        project.as_deref(),
        &channel,
        parsed_target,
        metadata,
    )
    .map_err(CmdError::from)?;
    Ok(outcome)
}

#[tauri::command]
pub fn publish_prepare(
    state: tauri::State<'_, AppState>,
    project: Option<String>,
    artifact: String,
    channel: String,
    target: String,
    metadata: PublishMetadata,
) -> CmdResult<studio_tools::publish::PrepareOutcome> {
    publish_prepare_impl(&state, project, artifact, channel, target, metadata)
}

/// List publish requests in the current workspace, optionally filtered by project slug.
pub fn publish_list_impl(
    state: &AppState,
    project: Option<String>,
) -> CmdResult<Vec<PublishRequest>> {
    let ws = state.workspace()?;
    let db = WorkspaceDb::open(&ws)?;
    let list = PublishRequest::list(db.conn(), project.as_deref())?;
    Ok(list)
}

#[tauri::command]
pub fn publish_list(
    state: tauri::State<'_, AppState>,
    project: Option<String>,
) -> CmdResult<Vec<PublishRequest>> {
    publish_list_impl(&state, project)
}

/// Get a specific publish request by its ID.
pub fn publish_get_impl(state: &AppState, id: String) -> CmdResult<PublishRequest> {
    let ws = state.workspace()?;
    let db = WorkspaceDb::open(&ws)?;
    let req = PublishRequest::get(db.conn(), &id)?
        .ok_or_else(|| CmdError::msg(format!("Không tìm thấy bản đăng với mã: \"{id}\"")))?;
    Ok(req)
}

#[tauri::command]
pub fn publish_get(state: tauri::State<'_, AppState>, id: String) -> CmdResult<PublishRequest> {
    publish_get_impl(&state, id)
}

/// Update metadata for an existing draft or awaiting-confirmation publish request.
pub fn publish_update_metadata_impl(
    state: &AppState,
    id: String,
    metadata: PublishMetadata,
) -> CmdResult<PublishRequest> {
    let ws = state.workspace()?;
    let db = WorkspaceDb::open(&ws)?;
    let mut req = PublishRequest::get(db.conn(), &id)?
        .ok_or_else(|| CmdError::msg(format!("Không tìm thấy bản đăng với mã: \"{id}\"")))?;

    if req.status == PublishStatus::Published {
        return Err(CmdError::msg(
            "Bản đăng đã xuất bản thành công, không thể thay đổi thông tin.",
        ));
    }
    if req.status == PublishStatus::Publishing {
        return Err(CmdError::msg(
            "Bản đăng đang trong tiến trình xuất bản, không thể thay đổi thông tin.",
        ));
    }

    req.metadata = metadata;
    req.updated_at = chrono::Utc::now().to_rfc3339();
    req.save(db.conn())?;

    Ok(req)
}

#[tauri::command]
pub fn publish_update_metadata(
    state: tauri::State<'_, AppState>,
    id: String,
    metadata: PublishMetadata,
) -> CmdResult<PublishRequest> {
    publish_update_metadata_impl(&state, id, metadata)
}

/// Confirm and execute a publish request.
/// Runs upload as a job with progress events.
/// Enforces idempotency: published requests can never be sent again.
/// Failed requests require explicit user confirmation to retry.
pub async fn publish_confirm_impl(
    state: &AppState,
    id: String,
    channel_id: Option<String>,
) -> CmdResult<PublishRequest> {
    let ws = state.workspace()?;
    let db = WorkspaceDb::open(&ws)?;
    let mut req = PublishRequest::get(db.conn(), &id)?
        .ok_or_else(|| CmdError::msg(format!("Không tìm thấy bản đăng với mã: \"{id}\"")))?;

    // Idempotency: published requests can NEVER be sent again
    if req.status == PublishStatus::Published {
        return Err(CmdError::msg(
            "Bản đăng đã được xuất bản thành công, không thể đăng lại.",
        ));
    }

    // Atomic claim: refuses concurrent confirmations and anything not awaiting/failed.
    let now = chrono::Utc::now().to_rfc3339();
    if !PublishRequest::claim_for_publishing(db.conn(), &req.id, &now)? {
        return Err(CmdError::msg(
            "Bản đăng đang được đăng tải hoặc đã xử lý, không thể xác nhận lại.",
        ));
    }
    req.status = PublishStatus::Publishing;
    req.error = None;
    req.updated_at = now;

    let job_id = format!("pub-job-{}", req.id);
    emit_publish_event(
        state,
        &job_id,
        "running",
        Some(0.05),
        "Bắt đầu quy trình xuất bản...",
        None,
    );

    match req.target {
        PublishTarget::Youtube => {
            // Locate primary video file and optional thumbnail
            let mut video_path: Option<PathBuf> = None;
            let mut thumb_path: Option<PathBuf> = None;

            for f in &req.files {
                // Only workspace files may be uploaded (no absolute paths, `..` or junction escapes).
                let Ok(p) = ws.scoped_path(f) else { continue };

                let ext = p
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if ["mp4", "mov", "webm", "mkv"].contains(&ext.as_str()) && video_path.is_none() {
                    video_path = Some(p);
                } else if ["png", "jpg", "jpeg", "webp"].contains(&ext.as_str())
                    && thumb_path.is_none()
                {
                    thumb_path = Some(p);
                }
            }

            // Fallback: if first file is not video, use first file
            if video_path.is_none() {
                video_path = req.files.first().and_then(|f| ws.scoped_path(f).ok());
            }

            let video_file = match video_path {
                Some(v) if v.exists() => v,
                _ => {
                    let err = "Không tìm thấy tệp video để đăng lên YouTube.".to_string();
                    req.status = PublishStatus::Failed;
                    req.error = Some(err.clone());
                    req.updated_at = chrono::Utc::now().to_rfc3339();
                    let _ = req.save(db.conn());
                    emit_publish_event(state, &job_id, "failed", None, "Thất bại", Some(&err));
                    return Err(CmdError::msg(err));
                }
            };

            let state_clone = state.clone();
            let j_id_clone = job_id.clone();
            let progress_cb = move |frac: f32, msg: &str| {
                emit_publish_event(&state_clone, &j_id_clone, "running", Some(frac), msg, None);
            };

            let org_name = ws.org().name();
            let ws_name = ws.name();

            let publish_params = YouTubePublishParams {
                channel_id: channel_id.as_deref(),
                video_path: &video_file,
                thumbnail_path: thumb_path.as_deref(),
                metadata: &req.metadata,
            };

            match publish_youtube_complete(
                state.secrets.as_ref(),
                org_name,
                Some(ws_name),
                publish_params,
                progress_cb,
            )
            .await
            {
                Ok(url) => {
                    req.status = PublishStatus::Published;
                    req.result_url = Some(url.clone());
                    req.error = None;
                    req.updated_at = chrono::Utc::now().to_rfc3339();
                    req.save(db.conn())?;

                    if let Some(ref cid) = channel_id {
                        if let Ok(app_db) = state.app_paths.open_app_db() {
                            let _ = app_db.set_setting("publish:last_used_channel", cid);
                        }
                    }

                    emit_publish_event(
                        state,
                        &job_id,
                        "succeeded",
                        Some(1.0),
                        "Đã đăng tải video lên YouTube thành công!",
                        None,
                    );
                    Ok(req)
                }
                Err(err) => {
                    let err_msg = err.to_string();
                    req.status = PublishStatus::Failed;
                    req.error = Some(err_msg.clone());
                    req.updated_at = chrono::Utc::now().to_rfc3339();
                    let _ = req.save(db.conn());
                    emit_publish_event(
                        state,
                        &job_id,
                        "failed",
                        None,
                        "Đăng video YouTube thất bại",
                        Some(&err_msg),
                    );
                    Err(err.into())
                }
            }
        }
        PublishTarget::Facebook => {
            // Facebook Page publisher
            let target_page_id = if let Some(ref cid) = channel_id {
                cid.clone()
            } else if let Ok(app_db) = state.app_paths.open_app_db() {
                app_db
                    .get_setting("publish:last_used_facebook_page")
                    .ok()
                    .flatten()
                    .unwrap_or_default()
            } else {
                String::new()
            };

            let org_name = ws.org().name();
            let ws_name = ws.name();
            let has_token = if !target_page_id.is_empty() {
                state
                    .secrets
                    .get(
                        org_name,
                        Some(ws_name),
                        &format!("{KEY_FACEBOOK_PAGE_TOKEN}:{target_page_id}"),
                    )
                    .is_some()
            } else {
                false
            };

            if !has_token {
                // Not connected -> Fall back to Export bundle as required by specs
                emit_publish_event(
                    state,
                    &job_id,
                    "running",
                    Some(0.5),
                    "Chưa kết nối Facebook Page — Đang tạo gói tệp xuất bản...",
                    None,
                );

                match export_bundle(&ws, &req) {
                    Ok(bundle_path) => {
                        let bundle_str = bundle_path
                            .strip_prefix(ws.root())
                            .unwrap_or(&bundle_path)
                            .to_string_lossy()
                            .replace('\\', "/");

                        req.status = PublishStatus::Published;
                        req.result_url = Some("exported".into());
                        req.error = None;
                        req.updated_at = chrono::Utc::now().to_rfc3339();
                        req.save(db.conn())?;

                        emit_publish_event(
                            state,
                            &job_id,
                            "succeeded",
                            Some(1.0),
                            &format!("Đã xuất gói thành công vào thư mục: {bundle_str}"),
                            None,
                        );

                        return Ok(req);
                    }
                    Err(err) => {
                        let err_msg = err.to_string();
                        req.status = PublishStatus::Failed;
                        req.error = Some(err_msg.clone());
                        req.updated_at = chrono::Utc::now().to_rfc3339();
                        let _ = req.save(db.conn());

                        emit_publish_event(
                            state,
                            &job_id,
                            "failed",
                            None,
                            "Xuất gói thất bại",
                            Some(&err_msg),
                        );

                        return Err(CmdError::msg(err_msg));
                    }
                }
            }

            // Target file to publish (video or image)
            let mut file_to_publish = None;
            for rel in &req.files {
                if let Ok(p) = ws.scoped_path(rel) {
                    if p.exists() {
                        file_to_publish = Some(p);
                        break;
                    }
                }
            }

            let upload_file = match file_to_publish {
                Some(f) => f,
                None => {
                    let err =
                        "Không tìm thấy tệp nội dung (ảnh hoặc video) để đăng lên Facebook Page."
                            .to_string();
                    req.status = PublishStatus::Failed;
                    req.error = Some(err.clone());
                    req.updated_at = chrono::Utc::now().to_rfc3339();
                    let _ = req.save(db.conn());
                    emit_publish_event(state, &job_id, "failed", None, "Thất bại", Some(&err));
                    return Err(CmdError::msg(err));
                }
            };

            let state_clone = state.clone();
            let j_id_clone = job_id.clone();
            let progress_cb = move |frac: f32, msg: &str| {
                emit_publish_event(&state_clone, &j_id_clone, "running", Some(frac), msg, None);
            };

            let fb_params = FacebookPublishParams {
                page_id: &target_page_id,
                file_path: &upload_file,
                metadata: &req.metadata,
            };

            match publish_facebook_complete(
                state.secrets.as_ref(),
                org_name,
                Some(ws_name),
                fb_params,
                progress_cb,
            )
            .await
            {
                Ok(url) => {
                    req.status = PublishStatus::Published;
                    req.result_url = Some(url.clone());
                    req.error = None;
                    req.updated_at = chrono::Utc::now().to_rfc3339();
                    req.save(db.conn())?;

                    if let Ok(app_db) = state.app_paths.open_app_db() {
                        let _ =
                            app_db.set_setting("publish:last_used_facebook_page", &target_page_id);
                    }

                    emit_publish_event(
                        state,
                        &job_id,
                        "succeeded",
                        Some(1.0),
                        "Đã đăng tải bài viết lên Facebook Page thành công!",
                        None,
                    );
                    Ok(req)
                }
                Err(err) => {
                    let err_msg = err.to_string();
                    req.status = PublishStatus::Failed;
                    req.error = Some(err_msg.clone());
                    req.updated_at = chrono::Utc::now().to_rfc3339();
                    let _ = req.save(db.conn());
                    emit_publish_event(
                        state,
                        &job_id,
                        "failed",
                        None,
                        "Đăng bài Facebook Page thất bại",
                        Some(&err_msg),
                    );
                    Err(err.into())
                }
            }
        }
        PublishTarget::Tiktok | PublishTarget::Instagram | PublishTarget::Export => {
            // Generate export bundle folder and open folder on desktop
            emit_publish_event(
                state,
                &job_id,
                "running",
                Some(0.5),
                "Đang xuất gói tệp và tạo siêu dữ liệu...",
                None,
            );

            match export_bundle(&ws, &req) {
                Ok(bundle_path) => {
                    let bundle_str = bundle_path
                        .strip_prefix(ws.root())
                        .unwrap_or(&bundle_path)
                        .to_string_lossy()
                        .replace('\\', "/");

                    req.status = PublishStatus::Published;
                    req.result_url = Some("exported".into());
                    req.error = None;
                    req.updated_at = chrono::Utc::now().to_rfc3339();
                    req.save(db.conn())?;

                    emit_publish_event(
                        state,
                        &job_id,
                        "succeeded",
                        Some(1.0),
                        &format!("Đã xuất gói thành công vào thư mục: {bundle_str}"),
                        None,
                    );

                    Ok(req)
                }
                Err(err) => {
                    let err_msg = err.to_string();
                    req.status = PublishStatus::Failed;
                    req.error = Some(err_msg.clone());
                    req.updated_at = chrono::Utc::now().to_rfc3339();
                    let _ = req.save(db.conn());

                    emit_publish_event(
                        state,
                        &job_id,
                        "failed",
                        None,
                        "Xuất gói thất bại",
                        Some(&err_msg),
                    );

                    Err(CmdError::msg(err_msg))
                }
            }
        }
    }
}

#[tauri::command]
pub async fn publish_confirm(
    state: tauri::State<'_, AppState>,
    id: String,
    channel_id: Option<String>,
) -> CmdResult<PublishRequest> {
    publish_confirm_impl(&state, id, channel_id).await
}

/// Cancel an in-progress or awaiting-confirmation publish request.
pub fn publish_cancel_impl(state: &AppState, id: String) -> CmdResult<PublishRequest> {
    let ws = state.workspace()?;
    let db = WorkspaceDb::open(&ws)?;
    let mut req = PublishRequest::get(db.conn(), &id)?
        .ok_or_else(|| CmdError::msg(format!("Không tìm thấy bản đăng với mã: \"{id}\"")))?;

    if req.status == PublishStatus::Published {
        return Err(CmdError::msg(
            "Không thể hủy bản đăng đã xuất bản thành công.",
        ));
    }

    let job_id = format!("pub-job-{}", req.id);
    if req.status == PublishStatus::Publishing {
        req.status = PublishStatus::Failed;
        req.error = Some("Người dùng đã hủy quá trình xuất bản.".into());
        emit_publish_event(
            state,
            &job_id,
            "cancelled",
            None,
            "Người dùng đã hủy quá trình xuất bản.",
            None,
        );
    } else {
        req.status = PublishStatus::Draft;
    }

    req.updated_at = chrono::Utc::now().to_rfc3339();
    req.save(db.conn())?;

    Ok(req)
}

#[tauri::command]
pub fn publish_cancel(state: tauri::State<'_, AppState>, id: String) -> CmdResult<PublishRequest> {
    publish_cancel_impl(&state, id)
}

/// Result of importing a YouTube OAuth client.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YoutubeClientImportResult {
    pub client_id_last6: String,
    pub message: String,
}

/// Status of the configured YouTube OAuth client.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YoutubeClientStatus {
    pub configured: bool,
    pub client_id_last6: Option<String>,
}

/// Import a Google OAuth client JSON or raw Client ID + Secret into Windows Credential Manager.
/// Never echoes secret values back, never logs them.
pub async fn youtube_client_import_impl(
    state: &AppState,
    json: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
) -> CmdResult<YoutubeClientImportResult> {
    let ws = state.workspace().ok();
    let org_name = ws.as_ref().map(|w| w.org().name()).unwrap_or("");
    let ws_name = ws.as_ref().map(|w| w.name());

    let (cid, csec) = resolve_google_client_credentials(
        json.as_deref(),
        client_id.as_deref(),
        client_secret.as_deref(),
    )?;

    // Store in Credential Manager
    state
        .secrets
        .set(org_name, ws_name, KEY_YOUTUBE_CLIENT_ID, &cid)
        .map_err(|e| CmdError::msg(format!("Lỗi lưu YOUTUBE_CLIENT_ID: {e}")))?;
    state
        .secrets
        .set(org_name, ws_name, KEY_YOUTUBE_CLIENT_SECRET, &csec)
        .map_err(|e| CmdError::msg(format!("Lỗi lưu YOUTUBE_CLIENT_SECRET: {e}")))?;

    if ws_name.is_some() {
        let _ = state.secrets.set("", None, KEY_YOUTUBE_CLIENT_ID, &cid);
        let _ = state
            .secrets
            .set("", None, KEY_YOUTUBE_CLIENT_SECRET, &csec);
    }

    let last6 = extract_client_id_last6(&cid);
    Ok(YoutubeClientImportResult {
        client_id_last6: last6.clone(),
        message: format!("Đã nạp (…{last6})"),
    })
}

#[tauri::command]
pub async fn youtube_client_import(
    state: tauri::State<'_, AppState>,
    json: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
) -> CmdResult<YoutubeClientImportResult> {
    youtube_client_import_impl(&state, json, client_id, client_secret).await
}

/// Retrieve status of the YouTube OAuth client (configured boolean and masked last 6 characters).
pub fn youtube_client_status_impl(state: &AppState) -> CmdResult<YoutubeClientStatus> {
    let ws = state.workspace().ok();
    let org_name = ws.as_ref().map(|w| w.org().name()).unwrap_or("");
    let ws_name = ws.as_ref().map(|w| w.name());

    if let Some(client_id) = state.secrets.get(org_name, ws_name, KEY_YOUTUBE_CLIENT_ID) {
        let last6 = extract_client_id_last6(&client_id);
        Ok(YoutubeClientStatus {
            configured: true,
            client_id_last6: Some(last6),
        })
    } else {
        Ok(YoutubeClientStatus {
            configured: false,
            client_id_last6: None,
        })
    }
}

#[tauri::command]
pub async fn youtube_client_status(
    state: tauri::State<'_, AppState>,
) -> CmdResult<YoutubeClientStatus> {
    youtube_client_status_impl(&state)
}

/// Connect YouTube via OAuth 2.0 PKCE, retrieve channel details, store non-secret metadata in app.db,
/// and store refresh token keyed by channel ID.
#[tauri::command]
pub async fn youtube_connect(
    state: tauri::State<'_, AppState>,
) -> CmdResult<YouTubeChannelAccount> {
    let ws = state.workspace().ok();
    let org_name = ws.as_ref().map(|w| w.org().name()).unwrap_or("");
    let ws_name = ws.as_ref().map(|w| w.name());

    let channel = connect_youtube(state.secrets.as_ref(), org_name, ws_name, 300).await?;

    // Store non-secret channel info in app.db
    if let Ok(db) = state.app_paths.open_app_db() {
        let mut channels: Vec<YouTubeChannelAccount> = db
            .get_setting("publish:youtube_channels")
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        if let Some(pos) = channels.iter().position(|c| c.id == channel.id) {
            channels[pos] = channel.clone();
        } else {
            channels.push(channel.clone());
        }

        if let Ok(json_str) = serde_json::to_string(&channels) {
            let _ = db.set_setting("publish:youtube_channels", &json_str);
        }
        let _ = db.set_setting("publish:last_used_channel", &channel.id);
    }

    Ok(channel)
}

/// List all connected YouTube channels from app.db.
/// Provides automatic migration if a legacy single `YOUTUBE_REFRESH_TOKEN` exists in Credential Manager.
pub async fn youtube_channels_list_impl(state: &AppState) -> CmdResult<Vec<YouTubeChannelAccount>> {
    let db = state.app_paths.open_app_db().map_err(CmdError::from)?;
    let mut channels: Vec<YouTubeChannelAccount> = db
        .get_setting("publish:youtube_channels")
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    // Backward compatibility: If no channels stored in app.db, but legacy YOUTUBE_REFRESH_TOKEN exists:
    // attempt migration on first query
    if channels.is_empty() {
        let ws = state.workspace().ok();
        let org_name = ws.as_ref().map(|w| w.org().name()).unwrap_or("");
        let ws_name = ws.as_ref().map(|w| w.name());

        if let Ok(Some(channel)) = studio_tools::publish::migrate_legacy_youtube_account(
            state.secrets.as_ref(),
            org_name,
            ws_name,
        )
        .await
        {
            channels.push(channel.clone());
            if let Ok(json_str) = serde_json::to_string(&channels) {
                let _ = db.set_setting("publish:youtube_channels", &json_str);
            }
            let _ = db.set_setting("publish:last_used_channel", &channel.id);
        }
    }

    Ok(channels)
}

#[tauri::command]
pub async fn youtube_channels_list(
    state: tauri::State<'_, AppState>,
) -> CmdResult<Vec<YouTubeChannelAccount>> {
    youtube_channels_list_impl(&state).await
}

/// Result of disconnecting a YouTube channel.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YoutubeDisconnectResult {
    pub success: bool,
    pub revoke_url: String,
}

/// Disconnect a YouTube channel: deletes its refresh token from Credential Manager,
/// removes its record from app.db, and returns the Google revocation link.
pub fn youtube_channel_disconnect_impl(
    state: &AppState,
    channel_id: String,
) -> CmdResult<YoutubeDisconnectResult> {
    let ws = state.workspace().ok();
    let org_name = ws.as_ref().map(|w| w.org().name()).unwrap_or("");
    let ws_name = ws.as_ref().map(|w| w.name());

    // Delete refresh token
    let _ = state.secrets.delete(
        org_name,
        ws_name,
        &format!("YOUTUBE_REFRESH_TOKEN:{channel_id}"),
    );
    if ws_name.is_some() {
        let _ = state
            .secrets
            .delete("", None, &format!("YOUTUBE_REFRESH_TOKEN:{channel_id}"));
    }

    // Update app.db stored channels
    if let Ok(db) = state.app_paths.open_app_db() {
        let mut channels: Vec<YouTubeChannelAccount> = db
            .get_setting("publish:youtube_channels")
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        channels.retain(|c| c.id != channel_id);

        if channels.is_empty() {
            let _ = db.delete_setting("publish:youtube_channels");
            let _ = db.delete_setting("publish:last_used_channel");
            // Delete legacy single token if no channels remain
            let _ = state
                .secrets
                .delete(org_name, ws_name, KEY_YOUTUBE_REFRESH_TOKEN);
            if ws_name.is_some() {
                let _ = state.secrets.delete("", None, KEY_YOUTUBE_REFRESH_TOKEN);
            }
        } else {
            if let Ok(json_str) = serde_json::to_string(&channels) {
                let _ = db.set_setting("publish:youtube_channels", &json_str);
            }
            let last = db.get_setting("publish:last_used_channel").ok().flatten();
            if last.as_deref() == Some(&channel_id) {
                if let Some(first) = channels.first() {
                    let _ = db.set_setting("publish:last_used_channel", &first.id);
                }
            }
        }
    }

    Ok(YoutubeDisconnectResult {
        success: true,
        revoke_url: "https://myaccount.google.com/permissions".into(),
    })
}

#[tauri::command]
pub async fn youtube_channel_disconnect(
    state: tauri::State<'_, AppState>,
    channel_id: String,
) -> CmdResult<YoutubeDisconnectResult> {
    youtube_channel_disconnect_impl(&state, channel_id)
}
