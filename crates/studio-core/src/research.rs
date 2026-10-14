use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceSpan {
    pub text: String,
    pub section: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchSnapshot {
    pub title: String,
    pub description: Option<String>,
    pub canonical_url: String,
    pub main_text: String,
    pub headings: Vec<String>,
    pub evidence_spans: Vec<EvidenceSpan>,
    pub source_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchSource {
    pub id: String,
    pub workspace_id: String,
    pub canonical_url: String,
    pub domain: String,
    pub fetch_status: String,
    pub snapshot: Option<ResearchSnapshot>,
    pub fetched_at: DateTime<Utc>,
}

impl ResearchSource {
    pub fn new(
        workspace_id: impl Into<String>,
        canonical_url: impl Into<String>,
        domain: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: workspace_id.into(),
            canonical_url: canonical_url.into(),
            domain: domain.into(),
            fetch_status: "PENDING".to_string(),
            snapshot: None,
            fetched_at: Utc::now(),
        }
    }
}
