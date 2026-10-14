use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RenderJobState {
    Pending,
    Running,
    Verifying,
    Ready,
    Failed,
    Cancelled,
    NeedsReconciliation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryboardScene {
    pub id: String,
    pub title: String,
    pub duration_seconds: f64,
    pub background_asset_ref: Option<String>,
    pub text_overlays: Vec<String>,
    pub audio_asset_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timeline {
    pub id: String,
    pub project_id: String,
    pub scenes: Vec<StoryboardScene>,
    pub total_duration_seconds: f64,
    pub fps: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderJob {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub revision_id: String,
    pub state: RenderJobState,
    pub progress_percent: u32,
    pub input_snapshot_hash: String,
    pub output_asset_ref: Option<String>,
    pub output_hash: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RenderJob {
    pub fn new(
        workspace_id: impl Into<String>,
        project_id: impl Into<String>,
        revision_id: impl Into<String>,
        input_hash: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: workspace_id.into(),
            project_id: project_id.into(),
            revision_id: revision_id.into(),
            state: RenderJobState::Pending,
            progress_percent: 0,
            input_snapshot_hash: input_hash.into(),
            output_asset_ref: None,
            output_hash: None,
            error_message: None,
            created_at: now,
            updated_at: now,
        }
    }
}
