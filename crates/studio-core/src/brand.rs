use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BrandVersionState {
    Draft,
    Published,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualTokens {
    pub primary_color: String,
    pub secondary_color: String,
    pub surface_color: String,
    pub heading_font: String,
    pub body_font: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceGuidelines {
    pub tone: String,
    pub prohibited_terms: Vec<String>,
    pub required_disclaimers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandVersion {
    pub id: String,
    pub brand_kit_id: String,
    pub version_number: u32,
    pub state: BrandVersionState,
    pub visual_tokens: VisualTokens,
    pub voice: VoiceGuidelines,
    pub logos: Vec<String>, // asset IDs
    pub provenance: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandKit {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub active_version_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl BrandKit {
    pub fn new(workspace_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: workspace_id.into(),
            name: name.into(),
            active_version_id: None,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBrief {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    pub objective: String,
    pub target_audience: String,
    pub key_messages: Vec<String>,
    pub citation_sources: Vec<String>, // research source IDs
    pub brand_version_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelVariant {
    pub id: String,
    pub brief_id: String,
    pub platform: String, // "facebook", "instagram", etc.
    pub body_copy: String,
    pub hashtags: Vec<String>,
    pub call_to_action: String,
    pub is_stale: bool,
    pub last_reviewed_at: DateTime<Utc>,
}
