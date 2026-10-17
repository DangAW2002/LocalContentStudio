//! Export bundle generator for social platforms and manual export.

use crate::error::{Result, ToolError};
use crate::publish::model::PublishRequest;
use std::fs;
use std::path::{Path, PathBuf};
use studio_workspace::Workspace;

/// Generate export bundle directory under `projects/<slug>/publish/<id>/`.
pub fn export_bundle(ws: &Workspace, req: &PublishRequest) -> Result<PathBuf> {
    let bundle_dir = ws
        .root()
        .join("projects")
        .join(&req.project)
        .join("publish")
        .join(&req.id);

    fs::create_dir_all(&bundle_dir).map_err(|e| ToolError::Io {
        path: bundle_dir.clone(),
        source: e,
    })?;

    let mut copied_files = Vec::new();

    // 1. Copy media files and thumbnail
    for file_str in &req.files {
        // Only workspace files are exported (no absolute paths, `..` or junction escapes).
        let Ok(src_path) = ws.scoped_path(file_str) else {
            continue;
        };

        if src_path.exists() && src_path.is_file() {
            if let Some(file_name) = src_path.file_name() {
                let dest_path = bundle_dir.join(file_name);
                if let Err(e) = fs::copy(&src_path, &dest_path) {
                    tracing::warn!("Không thể sao chép tệp {:?}: {}", src_path, e);
                } else {
                    copied_files.push(file_name.to_string_lossy().to_string());
                }
            }
        }
    }

    // 2. Write metadata.json
    let metadata_path = bundle_dir.join("metadata.json");
    let export_meta = serde_json::json!({
        "id": req.id,
        "artifact": req.artifact,
        "project": req.project,
        "channel": req.channel,
        "target": req.target,
        "title": req.metadata.title,
        "description": req.metadata.description,
        "tags": req.metadata.tags,
        "privacy": req.metadata.privacy,
        "schedule": req.metadata.schedule,
        "files": copied_files,
        "exported_at": chrono::Utc::now().to_rfc3339(),
    });

    let meta_bytes = serde_json::to_vec_pretty(&export_meta)
        .map_err(|e| ToolError::ParseError(e.to_string()))?;
    fs::write(&metadata_path, meta_bytes).map_err(|e| ToolError::Io {
        path: metadata_path.clone(),
        source: e,
    })?;

    // 3. Write caption.txt formatted ready to paste
    let mut caption_text = String::new();
    if !req.metadata.title.trim().is_empty() {
        caption_text.push_str(req.metadata.title.trim());
        caption_text.push_str("\n\n");
    }
    if !req.metadata.description.trim().is_empty() {
        caption_text.push_str(req.metadata.description.trim());
        caption_text.push_str("\n\n");
    }
    if !req.metadata.tags.is_empty() {
        let tags_formatted: Vec<String> = req
            .metadata
            .tags
            .iter()
            .map(|t| {
                let clean = t.trim().trim_start_matches('#');
                format!("#{clean}")
            })
            .collect();
        caption_text.push_str(&tags_formatted.join(" "));
        caption_text.push('\n');
    }

    let caption_path = bundle_dir.join("caption.txt");
    fs::write(&caption_path, caption_text.as_bytes()).map_err(|e| ToolError::Io {
        path: caption_path.clone(),
        source: e,
    })?;

    // 4. Open folder in system file manager (skip in tests)
    if !cfg!(test) {
        open_folder_in_file_manager(&bundle_dir);
    }

    Ok(bundle_dir)
}

/// Open folder using system file explorer.
pub fn open_folder_in_file_manager(path: &Path) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer").arg(path).spawn();
    }

    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(path).spawn();
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::publish::model::{PublishMetadata, PublishPrivacy, PublishStatus, PublishTarget};
    use tempfile::tempdir;

    #[test]
    fn test_export_bundle_generation() {
        let temp = tempdir().unwrap();
        let ws = Workspace::create(temp.path(), "test-ws").unwrap();
        let proj_dir = temp.path().join("projects").join("demo");
        fs::create_dir_all(&proj_dir).unwrap();

        // Create dummy video file
        let video_file = temp.path().join("video.mp4");
        fs::write(&video_file, b"sample-video-content").unwrap();

        let req = PublishRequest {
            id: "pub-123".into(),
            artifact: "video-01".into(),
            project: "demo".into(),
            channel: "facebook".into(),
            target: PublishTarget::Facebook,
            files: vec!["video.mp4".into()],
            metadata: PublishMetadata {
                title: "Bài viết Facebook mới".into(),
                description: "Nội dung bài viết Facebook đầy đủ".into(),
                tags: vec!["studio".into(), "content".into()],
                privacy: PublishPrivacy::Public,
                schedule: None,
                thumbnail: None,
            },
            status: PublishStatus::Publishing,
            result_url: None,
            error: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        let bundle_dir = export_bundle(&ws, &req).unwrap();
        assert!(bundle_dir.exists());
        assert!(bundle_dir.join("video.mp4").exists());
        assert!(bundle_dir.join("metadata.json").exists());
        assert!(bundle_dir.join("caption.txt").exists());

        let caption = fs::read_to_string(bundle_dir.join("caption.txt")).unwrap();
        assert!(caption.contains("Bài viết Facebook mới"));
        assert!(caption.contains("#studio #content"));
    }
}
