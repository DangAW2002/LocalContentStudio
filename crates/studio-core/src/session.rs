use crate::actor::ActorRole;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthenticatedSession {
    pub session_id: String,
    pub subject_id: String,
    pub os_profile_id: String,
    pub active_workspace_id: String,
    pub allowed_workspaces: Vec<String>,
    pub role: ActorRole,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl AuthenticatedSession {
    pub fn new(
        subject_id: impl Into<String>,
        os_profile_id: impl Into<String>,
        active_workspace_id: impl Into<String>,
        allowed_workspaces: Vec<String>,
        role: ActorRole,
        ttl_seconds: i64,
    ) -> Self {
        let now = Utc::now();
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            subject_id: subject_id.into(),
            os_profile_id: os_profile_id.into(),
            active_workspace_id: active_workspace_id.into(),
            allowed_workspaces,
            role,
            issued_at: now,
            expires_at: now + chrono::Duration::seconds(ttl_seconds),
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn can_access_workspace(&self, workspace_id: &str) -> bool {
        self.allowed_workspaces.iter().any(|ws| ws == workspace_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HumanConfirmationProof {
    pub proof_id: String,
    pub subject_id: String,
    pub target_workspace_id: String,
    pub operation: String,
    pub action_payload_hash: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub single_use: bool,
    pub consumed: bool,
}

impl HumanConfirmationProof {
    pub fn new(
        subject_id: impl Into<String>,
        target_workspace_id: impl Into<String>,
        operation: impl Into<String>,
        action_payload_hash: impl Into<String>,
        ttl_seconds: i64,
    ) -> Self {
        let now = Utc::now();
        Self {
            proof_id: format!("proof_{}", uuid::Uuid::new_v4()),
            subject_id: subject_id.into(),
            target_workspace_id: target_workspace_id.into(),
            operation: operation.into(),
            action_payload_hash: action_payload_hash.into(),
            issued_at: now,
            expires_at: now + chrono::Duration::seconds(ttl_seconds),
            single_use: true,
            consumed: false,
        }
    }

    pub fn is_valid_for(
        &self,
        subject_id: &str,
        workspace_id: &str,
        operation: &str,
        payload_hash: &str,
    ) -> bool {
        !self.consumed
            && Utc::now() <= self.expires_at
            && self.subject_id == subject_id
            && self.target_workspace_id == workspace_id
            && self.operation == operation
            && self.action_payload_hash == payload_hash
    }
}
