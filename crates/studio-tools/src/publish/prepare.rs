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

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

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
        created_at: now.clone(),
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
