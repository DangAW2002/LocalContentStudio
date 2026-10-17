//! IPC commands for publishing workflows: list, get, update, confirm, cancel, and YouTube connect.

use crate::error::{CmdError, CmdResult};
use crate::state::AppState;
use std::path::PathBuf;
use studio_tools::publish::{
    connect_youtube, export_bundle, publish_youtube_complete, PublishMetadata, PublishRequest,
    PublishStatus, PublishTarget,
};
use studio_workspace::WorkspaceDb;
use tauri::Emitter;

/// Helper to emit `job-event` to the frontend matching the scheduler contract.
fn emit_publish_event(
    handle: &tauri::AppHandle,
    job_id: &str,
    state: &str,
    progress: Option<f32>,
    message: &str,
    error: Option<&str>,
) {
    let snapshot_state = match state {
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
        "state": state,
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

    let _ = handle.emit("job-event", payload);
}

/// Prepare a draft publish request and validate against channel limits.
#[tauri::command]
pub fn publish_prepare(
    state: tauri::State<'_, AppState>,
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

/// List publish requests in the current workspace, optionally filtered by project slug.
#[tauri::command]
pub fn publish_list(
    state: tauri::State<'_, AppState>,
    project: Option<String>,
) -> CmdResult<Vec<PublishRequest>> {
    let ws = state.workspace()?;
    let db = WorkspaceDb::open(&ws)?;
    let list = PublishRequest::list(db.conn(), project.as_deref())?;
    Ok(list)
}

/// Get a specific publish request by its ID.
#[tauri::command]
pub fn publish_get(state: tauri::State<'_, AppState>, id: String) -> CmdResult<PublishRequest> {
    let ws = state.workspace()?;
    let db = WorkspaceDb::open(&ws)?;
    let req = PublishRequest::get(db.conn(), &id)?
        .ok_or_else(|| CmdError::msg(format!("Không tìm thấy bản đăng với mã: \"{id}\"")))?;
    Ok(req)
}

/// Update metadata for an existing draft or awaiting-confirmation publish request.
#[tauri::command]
pub fn publish_update_metadata(
    state: tauri::State<'_, AppState>,
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

/// Confirm and execute a publish request.
/// Runs upload as a job with progress events.
/// Enforces idempotency: published requests can never be sent again.
/// Failed requests require explicit user confirmation to retry.
#[tauri::command]
pub async fn publish_confirm(
    handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
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
        &handle,
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
                    emit_publish_event(&handle, &job_id, "failed", None, "Thất bại", Some(&err));
                    return Err(CmdError::msg(err));
                }
            };

            let h_clone = handle.clone();
            let j_id_clone = job_id.clone();
            let progress_cb = move |frac: f32, msg: &str| {
                emit_publish_event(&h_clone, &j_id_clone, "running", Some(frac), msg, None);
            };

            let org_name = ws.org().name();
            let ws_name = ws.name();

            match publish_youtube_complete(
                state.secrets.as_ref(),
                org_name,
                Some(ws_name),
                &video_file,
                thumb_path.as_deref(),
                &req.metadata,
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
                    emit_publish_event(
                        &handle,
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
                        &handle,
                        &job_id,
                        "failed",
                        None,
                        "Đăng video YouTube thất bại",
                        Some(&err_msg),
                    );
                    Err(CmdError::msg(err_msg))
                }
            }
        }
        PublishTarget::Facebook
        | PublishTarget::Tiktok
        | PublishTarget::Instagram
        | PublishTarget::Export => {
            // Generate export bundle folder and open folder on desktop
            emit_publish_event(
                &handle,
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
                        &handle,
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
                        &handle,
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

/// Cancel an in-progress or awaiting-confirmation publish request.
#[tauri::command]
pub fn publish_cancel(
    handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
) -> CmdResult<PublishRequest> {
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
            &handle,
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

/// Connect YouTube via OAuth 2.0 PKCE and loopback redirect server.
#[tauri::command]
pub async fn youtube_connect(state: tauri::State<'_, AppState>) -> CmdResult<String> {
    let ws = state.workspace()?;
    let org_name = ws.org().name();
    let ws_name = ws.name();

    let msg = connect_youtube(state.secrets.as_ref(), org_name, Some(ws_name), 300).await?;
    Ok(msg)
}
