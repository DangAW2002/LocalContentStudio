use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PreviewFidelity {
    Exact,
    Estimate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewReceipt {
    pub revision_id: String,
    pub source_hash: String,
    pub renderer_id: String,
    pub renderer_version: String,
    pub fidelity: PreviewFidelity,
    pub is_draft: bool,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub output_hash: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportReceipt {
    pub project_id: String,
    pub revision_id: String,
    pub source_hash: String,
    pub export_format: String, // "png", "html", "svg"
    pub files: Vec<ExportedFileItem>,
    pub total_bytes: u64,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedFileItem {
    pub artboard_id: Option<String>,
    pub file_name: String,
    pub mime_type: String,
    pub sha256: String,
    pub size_bytes: u64,
}
