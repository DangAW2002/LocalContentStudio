use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetScope {
    Workspace,
    Global,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RightsState {
    Quarantined,
    Ready,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetKind {
    Image,
    Video,
    Audio,
    Document,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RightsAttestation {
    pub source: String,
    pub license_basis: String,
    pub attribution_text: Option<String>,
    pub allowed_uses: Vec<String>,
    pub attested_by_human_actor_id: String,
    pub attested_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub scope: AssetScope,
    pub owner_workspace_id: Option<String>,
    pub kind: AssetKind,
    pub original_name: String,
    pub declared_mime: String,
    pub verified_mime: String,
    pub blob_sha256: String,
    pub file_size_bytes: u64,
    pub provenance: String,
    pub rights_state: RightsState,
    pub rights_attestation: Option<RightsAttestation>,
    pub created_at: DateTime<Utc>,
}

impl Asset {
    #[allow(clippy::too_many_arguments)]
    pub fn new_quarantined(
        scope: AssetScope,
        workspace_id: Option<String>,
        kind: AssetKind,
        original_name: impl Into<String>,
        declared_mime: impl Into<String>,
        verified_mime: impl Into<String>,
        blob_sha256: impl Into<String>,
        size_bytes: u64,
        provenance: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            scope,
            owner_workspace_id: workspace_id,
            kind,
            original_name: original_name.into(),
            declared_mime: declared_mime.into(),
            verified_mime: verified_mime.into(),
            blob_sha256: blob_sha256.into(),
            file_size_bytes: size_bytes,
            provenance: provenance.into(),
            rights_state: RightsState::Quarantined,
            rights_attestation: None,
            created_at: Utc::now(),
        }
    }
}
