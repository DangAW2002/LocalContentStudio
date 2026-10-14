use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HandoffStatus {
    Draft,
    Prepared,
    Transferring,
    Accepted,
    Rejected,
    NeedsReconciliation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryMediaItem {
    pub asset_version_id: String,
    pub sha256: String,
    pub mime: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryVariant {
    pub platform: String,
    pub caption_text: String,
    pub caption_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryTargetSelectors {
    pub page_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryPackage {
    pub schema_version: String,
    pub delivery_id: String,
    pub studio_workspace_id: String,
    pub project_id: String,
    pub source_revision_id: String,
    pub source_hash: String,
    pub variant: DeliveryVariant,
    pub media: Vec<DeliveryMediaItem>,
    pub target_selectors: DeliveryTargetSelectors,
    pub idempotency_key: String,
    pub status: HandoffStatus,
    pub created_at: DateTime<Utc>,
}

impl DeliveryPackage {
    pub const SCHEMA_V1: &'static str = "studio.delivery/v1";

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        workspace_id: impl Into<String>,
        project_id: impl Into<String>,
        revision_id: impl Into<String>,
        source_hash: impl Into<String>,
        platform: impl Into<String>,
        caption: impl Into<String>,
        page_id: impl Into<String>,
        idempotency_key: impl Into<String>,
    ) -> Self {
        let cap = caption.into();
        let cap_hash = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(cap.as_bytes());
            hex::encode(h.finalize())
        };

        Self {
            schema_version: Self::SCHEMA_V1.to_string(),
            delivery_id: uuid::Uuid::new_v4().to_string(),
            studio_workspace_id: workspace_id.into(),
            project_id: project_id.into(),
            source_revision_id: revision_id.into(),
            source_hash: source_hash.into(),
            variant: DeliveryVariant {
                platform: platform.into(),
                caption_text: cap,
                caption_hash: cap_hash,
            },
            media: Vec::new(),
            target_selectors: DeliveryTargetSelectors {
                page_id: page_id.into(),
            },
            idempotency_key: idempotency_key.into(),
            status: HandoffStatus::Prepared,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SocialPublicationState {
    Draft,
    InReview,
    Approved,
    Scheduled,
    Queued,
    Sent,
    Unknown,
    Verified,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishReadiness {
    pub is_ready: bool,
    pub target_page_id: String,
    pub source_revision_id: String,
    pub source_hash: String,
    pub blockers: Vec<String>,
    pub allowed_actions: Vec<String>,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialPublicationProjection {
    pub delivery_id: String,
    pub state: SocialPublicationState,
    pub permalink: Option<String>,
    pub error_message: Option<String>,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OutboxState {
    Pending,
    Dispatched,
    Accepted,
    Unknown,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxItem {
    pub id: String,
    pub destination: String,
    pub payload_hash: String,
    pub idempotency_key: String,
    pub state: OutboxState,
    pub remote_ref: Option<String>,
    pub created_at: DateTime<Utc>,
}
