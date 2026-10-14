use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    Unauthorized,
    ForbiddenScope,
    ForbiddenHumanOnly,
    NotFound,
    InvalidInput,
    ConflictCas,
    PreconditionFailed,
    QuotaExceeded,
    UnsupportedCapability,
    InternalError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorEnvelope {
    pub code: ErrorCode,
    pub safe_message_key: String,
    pub safe_message: String,
    pub retryable: bool,
    pub recovery_action: String,
    pub correlation_id: String,
    pub details: Option<serde_json::Value>,
}

#[derive(Error, Debug, Clone)]
pub enum StudioError {
    #[error("Authentication required")]
    Unauthorized { correlation_id: String },

    #[error("Forbidden cross-workspace access to resource '{resource_id}'")]
    ForbiddenScope {
        resource_id: String,
        correlation_id: String,
    },

    #[error("Operation '{operation}' requires authenticated Human authority")]
    ForbiddenHumanOnly {
        operation: String,
        correlation_id: String,
    },

    #[error("Human confirmation proof required for operation '{operation}' with payload hash '{action_payload_hash}'")]
    HumanConfirmationRequired {
        operation: String,
        action_payload_hash: String,
        correlation_id: String,
    },

    #[error("Resource '{resource_type}' with id '{resource_id}' not found")]
    NotFound {
        resource_type: String,
        resource_id: String,
        correlation_id: String,
    },

    #[error("Invalid input for '{field}': {reason}")]
    InvalidInput {
        field: String,
        reason: String,
        correlation_id: String,
    },

    #[error("Conflict: expected revision '{expected}', but current is '{current}'")]
    ConflictCas {
        expected: String,
        current: String,
        correlation_id: String,
    },

    #[error("Precondition failed: {reason}")]
    PreconditionFailed {
        reason: String,
        correlation_id: String,
    },

    #[error("Quota exceeded: {reason}")]
    QuotaExceeded {
        reason: String,
        correlation_id: String,
    },

    #[error("Unsupported capability: '{capability}'")]
    UnsupportedCapability {
        capability: String,
        correlation_id: String,
    },

    #[error("Internal error: {message}")]
    Internal {
        message: String,
        correlation_id: String,
    },
}

impl StudioError {
    pub fn to_envelope(&self) -> ErrorEnvelope {
        match self {
            Self::Unauthorized { correlation_id } => ErrorEnvelope {
                code: ErrorCode::Unauthorized,
                safe_message_key: "error.auth.unauthorized".into(),
                safe_message: "Yêu cầu đăng nhập hoặc phiên làm việc hợp lệ.".into(),
                retryable: false,
                recovery_action: "Đăng nhập lại hoặc làm mới token kết nối.".into(),
                correlation_id: correlation_id.clone(),
                details: None,
            },
            Self::ForbiddenScope {
                resource_id,
                correlation_id,
            } => ErrorEnvelope {
                code: ErrorCode::ForbiddenScope,
                safe_message_key: "error.auth.forbidden_scope".into(),
                safe_message: "Không có quyền truy cập tài nguyên ngoài workspace hiện tại.".into(),
                retryable: false,
                recovery_action:
                    "Chuyển sang đúng workspace sở hữu tài nguyên hoặc yêu cầu cấp quyền.".into(),
                correlation_id: correlation_id.clone(),
                details: Some(serde_json::json!({ "resource_id": resource_id })),
            },
            Self::ForbiddenHumanOnly {
                operation,
                correlation_id,
            } => ErrorEnvelope {
                code: ErrorCode::ForbiddenHumanOnly,
                safe_message_key: "error.auth.human_only".into(),
                safe_message:
                    "Thao tác này bắt buộc phải do người dùng trực tiếp phê duyệt hoặc thực hiện."
                        .into(),
                retryable: false,
                recovery_action: "Vui lòng xác nhận qua giao diện người dùng Desktop.".into(),
                correlation_id: correlation_id.clone(),
                details: Some(serde_json::json!({ "operation": operation })),
            },
            Self::HumanConfirmationRequired {
                operation,
                action_payload_hash,
                correlation_id,
            } => ErrorEnvelope {
                code: ErrorCode::ForbiddenHumanOnly,
                safe_message_key: "error.auth.human_confirmation_required".into(),
                safe_message:
                    "Thao tác yêu cầu bằng chứng xác nhận trực tiếp từ người dùng (Human Confirmation Proof) hợp lệ."
                        .into(),
                retryable: false,
                recovery_action: "Xác nhận hành động qua giao diện người dùng Desktop đáng tin cậy.".into(),
                correlation_id: correlation_id.clone(),
                details: Some(serde_json::json!({
                    "operation": operation,
                    "action_payload_hash": action_payload_hash,
                })),
            },
            Self::NotFound {
                resource_type,
                resource_id,
                correlation_id,
            } => ErrorEnvelope {
                code: ErrorCode::NotFound,
                safe_message_key: "error.resource.not_found".into(),
                safe_message: format!("Không tìm thấy {} yêu cầu.", resource_type),
                retryable: false,
                recovery_action: "Kiểm tra lại ID hoặc đường dẫn tài nguyên.".into(),
                correlation_id: correlation_id.clone(),
                details: Some(
                    serde_json::json!({ "resource_type": resource_type, "resource_id": resource_id }),
                ),
            },
            Self::InvalidInput {
                field,
                reason,
                correlation_id,
            } => ErrorEnvelope {
                code: ErrorCode::InvalidInput,
                safe_message_key: "error.input.invalid".into(),
                safe_message: format!("Dữ liệu đầu vào '{}' không hợp lệ: {}", field, reason),
                retryable: false,
                recovery_action: "Kiểm tra và sửa dữ liệu đầu vào theo định dạng yêu cầu.".into(),
                correlation_id: correlation_id.clone(),
                details: Some(serde_json::json!({ "field": field, "reason": reason })),
            },
            Self::ConflictCas {
                expected,
                current,
                correlation_id,
            } => ErrorEnvelope {
                code: ErrorCode::ConflictCas,
                safe_message_key: "error.concurrency.cas_conflict".into(),
                safe_message:
                    "Xung đột phiên bản: tài nguyên đã được cập nhật bởi một phiên làm việc khác."
                        .into(),
                retryable: true,
                recovery_action: "Lấy phiên bản mới nhất, so sánh khác biệt (diff) và thử lại."
                    .into(),
                correlation_id: correlation_id.clone(),
                details: Some(serde_json::json!({ "expected": expected, "current": current })),
            },
            Self::PreconditionFailed {
                reason,
                correlation_id,
            } => ErrorEnvelope {
                code: ErrorCode::PreconditionFailed,
                safe_message_key: "error.precondition.failed".into(),
                safe_message: format!("Điều kiện tiên quyết không thỏa mãn: {}", reason),
                retryable: false,
                recovery_action: "Hoàn tất các bước yêu cầu trước khi thực hiện thao tác.".into(),
                correlation_id: correlation_id.clone(),
                details: Some(serde_json::json!({ "reason": reason })),
            },
            Self::QuotaExceeded {
                reason,
                correlation_id,
            } => ErrorEnvelope {
                code: ErrorCode::QuotaExceeded,
                safe_message_key: "error.quota.exceeded".into(),
                safe_message: format!("Vượt quá hạn mức lưu trữ hoặc tài nguyên: {}", reason),
                retryable: false,
                recovery_action: "Dọn dẹp bớt tệp không dùng hoặc tăng hạn mức lưu trữ.".into(),
                correlation_id: correlation_id.clone(),
                details: Some(serde_json::json!({ "reason": reason })),
            },
            Self::UnsupportedCapability {
                capability,
                correlation_id,
            } => ErrorEnvelope {
                code: ErrorCode::UnsupportedCapability,
                safe_message_key: "error.capability.unsupported".into(),
                safe_message: format!(
                    "Tính năng hoặc bộ xử lý '{}' không khả dụng trong môi trường hiện tại.",
                    capability
                ),
                retryable: false,
                recovery_action:
                    "Cài đặt bộ mã hóa/driver phụ trợ hoặc chọn định dạng xuất khả dụng khác."
                        .into(),
                correlation_id: correlation_id.clone(),
                details: Some(serde_json::json!({ "capability": capability })),
            },
            Self::Internal {
                message,
                correlation_id,
            } => ErrorEnvelope {
                code: ErrorCode::InternalError,
                safe_message_key: "error.internal".into(),
                safe_message: "Đã xảy ra lỗi hệ thống nội bộ.".into(),
                retryable: true,
                recovery_action: "Thử lại sau hoặc kiểm tra tệp nhật ký ứng dụng.".into(),
                correlation_id: correlation_id.clone(),
                details: Some(serde_json::json!({ "message": message })),
            },
        }
    }
}
