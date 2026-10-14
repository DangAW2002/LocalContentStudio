use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SideEffectClass {
    Read,
    Draft,
    Reversible,
    Destructive,
    External,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationEnvelope<T> {
    pub operation: String,
    pub api_version: String,
    pub request_id: String,
    pub idempotency_key: Option<String>,
    pub expected_revision_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub human_proof_token: Option<String>,
    pub input: T,
}

impl<T> OperationEnvelope<T> {
    pub const API_VERSION_V1: &'static str = "studio.operations/v1";

    pub fn new(operation: impl Into<String>, input: T) -> Self {
        Self {
            operation: operation.into(),
            api_version: Self::API_VERSION_V1.to_string(),
            request_id: uuid::Uuid::new_v4().to_string(),
            idempotency_key: None,
            expected_revision_id: None,
            human_proof_token: None,
            input,
        }
    }

    pub fn with_human_proof(mut self, proof_token: impl Into<String>) -> Self {
        self.human_proof_token = Some(proof_token.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult<R> {
    pub request_id: String,
    pub aggregate_id: Option<String>,
    pub new_revision_id: Option<String>,
    pub result: R,
    pub warnings: Vec<String>,
    pub audit_id: String,
    pub capability_snapshot: Option<serde_json::Value>,
}
