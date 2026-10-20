//! File upload handler for browser mode (fallback when native file dialog is unavailable).

use axum::extract::Multipart;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use tokio::io::AsyncWriteExt;

/// Handle file upload from browser fallback input.
pub async fn handle_upload(mut multipart: Multipart) -> Response {
    let mut uploaded_files = Vec::new();
    let temp_dir = std::env::temp_dir().join("studio_uploads");
    if let Err(e) = tokio::fs::create_dir_all(&temp_dir).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Không tạo được thư mục tạm: {e}") })),
        )
            .into_response();
    }

    loop {
        let field_result = multipart.next_field().await;
        let mut field = match field_result {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(err) => {
                let status = err.status();
                if status == StatusCode::PAYLOAD_TOO_LARGE {
                    return (
                        StatusCode::PAYLOAD_TOO_LARGE,
                        Json(json!({ "error": "Kích thước tệp tải lên vượt quá giới hạn cho phép." })),
                    )
                        .into_response();
                }
                return (
                    status,
                    Json(json!({ "error": format!("Lỗi tải lên: {err}") })),
                )
                    .into_response();
            }
        };

        let file_name = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("upload_{}.bin", uuid::Uuid::new_v4().simple()));

        // Sanitize file name to prevent directory traversal
        let safe_name = std::path::Path::new(&file_name)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "file.bin".to_string());

        let target_path = temp_dir.join(format!("{}_{}", uuid::Uuid::new_v4().simple(), safe_name));

        let mut dest = match tokio::fs::File::create(&target_path).await {
            Ok(f) => f,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": format!("Lỗi tạo tệp lưu trữ: {e}") })),
                )
                    .into_response();
            }
        };

        let mut size = 0u64;
        loop {
            match field.chunk().await {
                Ok(Some(chunk)) => {
                    size += chunk.len() as u64;
                    if let Err(e) = dest.write_all(&chunk).await {
                        drop(dest);
                        let _ = tokio::fs::remove_file(&target_path).await;
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({ "error": format!("Lỗi ghi tệp tải lên: {e}") })),
                        )
                            .into_response();
                    }
                }
                Ok(None) => break,
                Err(err) => {
                    // Do not leave a partial upload behind (e.g. over the size limit).
                    drop(dest);
                    let _ = tokio::fs::remove_file(&target_path).await;
                    let status = err.status();
                    if status == StatusCode::PAYLOAD_TOO_LARGE {
                        return (
                            StatusCode::PAYLOAD_TOO_LARGE,
                            Json(json!({ "error": "Kích thước tệp tải lên vượt quá giới hạn cho phép." })),
                        )
                            .into_response();
                    }
                    return (
                        status,
                        Json(json!({ "error": format!("Lỗi đọc khối dữ liệu: {err}") })),
                    )
                        .into_response();
                }
            }
        }

        let _ = dest.flush().await;

        uploaded_files.push(json!({
            "path": target_path.to_string_lossy(),
            "name": safe_name,
            "size": size,
        }));
    }

    if uploaded_files.is_empty() {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Không có tệp nào được tải lên." })),
        )
            .into_response()
    } else {
        (StatusCode::OK, Json(json!({ "files": uploaded_files }))).into_response()
    }
}
