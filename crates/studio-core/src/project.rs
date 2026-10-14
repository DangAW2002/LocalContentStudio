use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    Poster,
    Carousel,
    Report,
    Motion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectStatus {
    Active,
    Archived,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    pub project_type: ProjectType,
    pub status: ProjectStatus,
    pub manifest_version: String,
    pub brand_ref: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRevision {
    pub id: String,
    pub project_id: String,
    pub parent_id: Option<String>,
    pub source_hash: String,
    pub author_actor_id: String,
    pub document_id: String,
    pub created_at: DateTime<Utc>,
    pub summary: String,
}

impl Project {
    pub fn new(
        workspace_id: impl Into<String>,
        title: impl Into<String>,
        project_type: ProjectType,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: workspace_id.into(),
            title: title.into(),
            project_type,
            status: ProjectStatus::Active,
            manifest_version: "project-manifest/v1".to_string(),
            brand_ref: None,
            created_at: now,
            updated_at: now,
        }
    }
}
