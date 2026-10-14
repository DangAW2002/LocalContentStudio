use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CapabilityStatus {
    Verified,
    Proposed,
    Blocked,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityItem {
    pub name: String,
    pub status: CapabilityStatus,
    pub human_only: bool,
    pub description: String,
    pub evidence_or_blocker: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRegistry {
    pub capabilities: HashMap<String, CapabilityItem>,
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        let mut reg = Self {
            capabilities: HashMap::new(),
        };
        reg.register(
            "render.ffmpeg.mp4",
            CapabilityStatus::Verified,
            false,
            "Render MP4 H.264/AAC qua FFmpeg worker",
            "FFmpeg 8.1.1 có sẵn libx264 và aac",
        );
        reg.register(
            "export.canvas.png",
            CapabilityStatus::Verified,
            false,
            "Xuất ảnh PNG từng artboard cho Poster/Carousel",
            "Offscreen canvas adapter đạt benchmark T00",
        );
        reg.register(
            "ai.byok.openai",
            CapabilityStatus::Proposed,
            false,
            "AI Assistant qua OpenAI-compatible API endpoint",
            "Chưa cấu hình API key/endpoint thực tế; hệ thống duy trì chế độ No-AI/Manual",
        );
        reg.register(
            "rights.attest",
            CapabilityStatus::Verified,
            true, // HUMAN ONLY
            "Phê duyệt và chuyển trạng thái bản quyền tài nguyên sang READY",
            "Bắt buộc người dùng thực hiện theo rules.md",
        );
        reg.register(
            "rights.revoke",
            CapabilityStatus::Verified,
            true, // HUMAN ONLY
            "Thu hồi trạng thái bản quyền tài nguyên về REVOKED",
            "Bắt buộc người dùng thực hiện theo rules.md",
        );
        reg.register(
            "asset.transfer_scope",
            CapabilityStatus::Verified,
            true, // HUMAN ONLY
            "Chuyển đổi phạm vi Workspace <-> Global và reset bản quyền về QUARANTINED",
            "Bảo đảm an toàn bản quyền khi chia sẻ tài nguyên",
        );
        reg.register(
            "asset.delete",
            CapabilityStatus::Verified,
            true, // HUMAN ONLY
            "Xóa vĩnh viễn tài nguyên khỏi hệ thống",
            "Thao tác không thể hoàn tác",
        );
        reg.register(
            "brand.publish",
            CapabilityStatus::Verified,
            true, // HUMAN ONLY
            "Phát hành phiên bản thương hiệu chính thức (BrandVersion) sau khi vượt qua bộ kiểm tra đối soát",
            "Chỉ con người có thẩm quyền phê duyệt thương hiệu theo rules.md",
        );
        reg.register(
            "publish.confirm",
            CapabilityStatus::Verified,
            true, // HUMAN ONLY
            "Xác nhận bàn giao bài đăng sang Social hoặc lệnh phát hành",
            "Bắt buộc người dùng thực hiện theo rules.md",
        );
        reg.register(
            "plugin.external_sandbox",
            CapabilityStatus::Blocked,
            false,
            "Cơ chế cách ly an toàn (Sandbox) cho plugin bên ngoài",
            "Chưa có sandbox process OS/WASM; chỉ cho phép built-in plugin nội bộ",
        );
        reg.register(
            "social.facebook.video_auto",
            CapabilityStatus::Blocked,
            true,
            "Tự động xuất bản video lên Facebook Graph API",
            "Gated mặc định: chờ live canary có quyền thực tế",
        );
        reg
    }
}

impl CapabilityRegistry {
    pub fn register(
        &mut self,
        name: impl Into<String>,
        status: CapabilityStatus,
        human_only: bool,
        desc: impl Into<String>,
        evidence: impl Into<String>,
    ) {
        let name_str = name.into();
        self.capabilities.insert(
            name_str.clone(),
            CapabilityItem {
                name: name_str,
                status,
                human_only,
                description: desc.into(),
                evidence_or_blocker: evidence.into(),
            },
        );
    }

    pub fn is_available(&self, name: &str) -> bool {
        if let Some(item) = self.capabilities.get(name) {
            item.status == CapabilityStatus::Verified
        } else {
            false
        }
    }

    pub fn is_human_only(&self, name: &str) -> bool {
        self.capabilities
            .get(name)
            .map(|i| i.human_only)
            .unwrap_or(false)
    }
}
