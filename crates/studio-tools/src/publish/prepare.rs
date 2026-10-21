//! Prepare draft publish request and run initial channel compliance validation.

use crate::error::{Result, ToolError};
use crate::publish::model::{PublishMetadata, PublishRequest, PublishStatus, PublishTarget};
use crate::publish::validation::validate_publish;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use studio_brand::checkers::Finding;
use studio_workspace::{Project, Workspace};

/// Outcome of preparing a publish request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareOutcome {
    /// Created publish request ID.
    pub id: String,
    /// Status (always `awaiting_confirmation` upon successful prepare).
    pub status: PublishStatus,
    /// Findings from validation checks.
    pub findings: Vec<Finding>,
    /// Resolved files for publishing.
    pub files: Vec<String>,
    /// Full request object.
    pub request: PublishRequest,
}

/// Resolve project slug and artifact ID from given input strings.
pub fn resolve_artifact_identity(
    ws: &Workspace,
    artifact_raw: &str,
    project_raw: Option<&str>,
) -> Result<(String, String)> {
    let raw = artifact_raw.trim().replace('\\', "/");
    if raw.is_empty() {
        return Err(ToolError::ExecutionError(
            "Tham số \"artifact\" không được để trống.".into(),
        ));
    }
    if raw.contains("..")
        || raw.starts_with('/')
        || (raw.len() >= 2 && raw.chars().nth(1) == Some(':'))
    {
        return Err(ToolError::ExecutionError(
            "Đường dẫn artifact không hợp lệ (path traversal).".into(),
        ));
    }

    let parts: Vec<&str> = raw.split('/').filter(|p| !p.is_empty()).collect();
    let (inferred_proj, art_id) = if parts.first() == Some(&"projects") {
        if parts.len() < 3 {
            return Err(ToolError::ExecutionError(format!(
                "Đường dẫn artifact không hợp lệ: \"{artifact_raw}\"."
            )));
        }
        let proj = parts[1].to_string();
        let id = if parts.len() >= 4 && parts[2] == "artifacts" {
            parts[3].trim_end_matches(".yaml").to_string()
        } else {
            parts[2].trim_end_matches(".yaml").to_string()
        };
        (Some(proj), id)
    } else if parts.first() == Some(&"artifacts") {
        if parts.len() < 2 {
            return Err(ToolError::ExecutionError(format!(
                "Đường dẫn artifact không hợp lệ: \"{artifact_raw}\"."
            )));
        }
        let id = parts[1].trim_end_matches(".yaml").to_string();
        (None, id)
    } else {
        let id = parts[0].trim_end_matches(".yaml").to_string();
        (None, id)
    };

    let target_proj = match (project_raw, inferred_proj) {
        (Some(p_arg), Some(p_inf)) => {
            if p_arg != p_inf {
                return Err(ToolError::ExecutionError(format!(
                    "Slug project được truyền (\"{p_arg}\") không khớp với đường dẫn artifact (\"{p_inf}\")."
                )));
            }
            p_arg.to_string()
        }
        (Some(p_arg), None) => p_arg.to_string(),
        (None, Some(p_inf)) => p_inf,
        (None, None) => {
            let projects = ws.list_projects()?;
            let matching: Vec<_> = projects
                .into_iter()
                .filter(|p| p.artifacts_dir().join(&art_id).exists())
                .collect();
            if matching.len() == 1 {
                matching[0].slug().to_string()
            } else if matching.is_empty() {
                return Err(ToolError::ExecutionError(format!(
                    "Không tìm thấy artifact \"{art_id}\" trong project nào. Hãy chỉ định tham số \"project\"."
                )));
            } else {
                return Err(ToolError::ExecutionError(format!(
                    "Artifact \"{art_id}\" xuất hiện ở nhiều project. Hãy chỉ định rõ tham số \"project\"."
                )));
            }
        }
    };

    Ok((target_proj, art_id))
}

/// Prepare a publish draft and save into database.
pub fn prepare_publish(
    ws: &Workspace,
    conn: &Connection,
    artifact_raw: &str,
    project_raw: Option<&str>,
    channel: &str,
    target: PublishTarget,
    metadata: PublishMetadata,
) -> Result<PrepareOutcome> {
    let (project_slug, artifact_id) = resolve_artifact_identity(ws, artifact_raw, project_raw)?;

    // Verify artifact directory exists
    let proj = Project::open(ws, &project_slug)?;
    let art_dir = proj.artifacts_dir().join(&artifact_id);
    if !art_dir.exists() {
        return Err(ToolError::ExecutionError(format!(
            "Không tìm thấy artifact \"{artifact_id}\" trong dự án \"{project_slug}\"."
        )));
    }

    // Run compliance checks against channel and renders
    let validation = validate_publish(ws, conn, &project_slug, &artifact_id, channel, &metadata)?;

    // Preparing again (e.g. after fixing metadata) updates the open draft for the same artifact,
    // channel and target instead of leaving one stale draft per attempt.
    let open = PublishRequest::list(conn, Some(&project_slug))?
        .into_iter()
        .find(|r| {
            r.artifact == artifact_id
                && r.channel == channel
                && r.target == target
                && matches!(
                    r.status,
                    PublishStatus::Draft | PublishStatus::AwaitingConfirmation
                )
        });
    let now = chrono::Utc::now().to_rfc3339();
    let (id, created_at) = match open {
        Some(r) => (r.id, r.created_at),
        None => (uuid::Uuid::new_v4().to_string(), now.clone()),
    };

    let request = PublishRequest {
        id: id.clone(),
        artifact: artifact_id,
        project: project_slug,
        channel: channel.to_string(),
        target,
        files: validation.files.clone(),
        metadata,
        status: PublishStatus::AwaitingConfirmation,
        result_url: None,
        error: None,
        created_at,
        updated_at: now,
    };

    request.save(conn)?;

    Ok(PrepareOutcome {
        id,
        status: PublishStatus::AwaitingConfirmation,
        findings: validation.findings,
        files: validation.files,
        request,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::publish::model::PublishPrivacy;
    use studio_workspace::WorkspaceDb;

    #[test]
    fn preparing_again_updates_the_open_draft() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = Workspace::create(tmp.path(), "Test").unwrap();
        let proj = Project::create(&ws, "p1", "P1").unwrap();
        std::fs::create_dir_all(proj.artifacts_dir().join("video")).unwrap();
        let db = WorkspaceDb::open(&ws).unwrap();
        let meta = |title: &str| PublishMetadata {
            title: title.into(),
            description: String::new(),
            tags: vec![],
            privacy: PublishPrivacy::Private,
            schedule: None,
            thumbnail: None,
        };
        let first = prepare_publish(
            &ws,
            db.conn(),
            "video",
            Some("p1"),
            "youtube",
            PublishTarget::Youtube,
            meta("A"),
        )
        .unwrap();
        let second = prepare_publish(
            &ws,
            db.conn(),
            "video",
            Some("p1"),
            "youtube",
            PublishTarget::Youtube,
            meta("B"),
        )
        .unwrap();
        assert_eq!(first.id, second.id);
        let all = PublishRequest::list(db.conn(), Some("p1")).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].metadata.title, "B");

        // A request that already left the draft stage is not reused.
        let mut done = all[0].clone();
        done.status = PublishStatus::Published;
        done.save(db.conn()).unwrap();
        let third = prepare_publish(
            &ws,
            db.conn(),
            "video",
            Some("p1"),
            "youtube",
            PublishTarget::Youtube,
            meta("C"),
        )
        .unwrap();
        assert_ne!(third.id, first.id);
    }
}
