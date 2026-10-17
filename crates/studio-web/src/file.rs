//! Scoped workspace file streaming endpoint with Range support for video and media previews.

use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::http::header::{self, HeaderMap};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use http_range::HttpRange;
use serde::Deserialize;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
use tokio_util::io::ReaderStream;

use crate::dispatcher::WebDispatcher;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct FileQuery {
    pub project: Option<String>,
    pub path: String,
}

/// Guess MIME type from file extension or content.
pub fn guess_mime(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "md" | "markdown" => "text/markdown; charset=utf-8",
        "txt" => "text/plain; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "m4a" => "audio/mp4",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

/// Serve a scoped file from the workspace with HTTP Range support.
pub async fn serve_file(
    dispatcher: Arc<dyn WebDispatcher>,
    headers: HeaderMap,
    query: FileQuery,
) -> Response {
    // Path traversal check
    if query.path.contains("..") {
        return (
            StatusCode::FORBIDDEN,
            "Từ chối truy cập đường dẫn chứa traversal '..'",
        )
            .into_response();
    }

    let file_path: PathBuf = match dispatcher.resolve_file(query.project.as_deref(), &query.path) {
        Ok(p) => p,
        Err(e) => {
            return (StatusCode::NOT_FOUND, format!("Không tìm thấy tệp: {e}")).into_response()
        }
    };

    let meta = match tokio::fs::metadata(&file_path).await {
        Ok(m) => m,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    if !meta.is_file() {
        return (StatusCode::BAD_REQUEST, "Đường dẫn không phải là tệp.").into_response();
    }

    let file_size = meta.len();
    let mime = guess_mime(&file_path);

    let mut file = match File::open(&file_path).await {
        Ok(f) => f,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    // Range support for audio/video playback
    if let Some(range_header) = headers.get(header::RANGE).and_then(|v| v.to_str().ok()) {
        match HttpRange::parse(range_header, file_size) {
            Ok(ranges) if !ranges.is_empty() => {
                let range = &ranges[0];
                let start = range.start;
                let length = range.length;

                if file.seek(SeekFrom::Start(start)).await.is_err() {
                    return StatusCode::RANGE_NOT_SATISFIABLE.into_response();
                }

                let stream = ReaderStream::new(file.take(length));
                let body = Body::from_stream(stream);

                let content_range = format!("bytes {}-{}/{}", start, start + length - 1, file_size);

                Response::builder()
                    .status(StatusCode::PARTIAL_CONTENT)
                    .header(header::CONTENT_TYPE, mime)
                    .header(header::CONTENT_LENGTH, length.to_string())
                    .header(header::CONTENT_RANGE, content_range)
                    .header(header::ACCEPT_RANGES, "bytes")
                    .body(body)
                    .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
            }
            _ => Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header(header::CONTENT_RANGE, format!("bytes */{file_size}"))
                .body(Body::empty())
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()),
        }
    } else {
        let stream = ReaderStream::new(file);
        let body = Body::from_stream(stream);

        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime)
            .header(header::CONTENT_LENGTH, file_size.to_string())
            .header(header::ACCEPT_RANGES, "bytes")
            .body(body)
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
    }
}
