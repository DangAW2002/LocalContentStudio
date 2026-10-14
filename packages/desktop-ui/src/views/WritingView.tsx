import React from "react";
import { Badge, Button, Input } from "@studio/design-system";

export const WritingView: React.FC = () => {
  const [selectedPlatform, setSelectedPlatform] = React.useState<"facebook" | "instagram">("facebook");

  return (
    <div style={{ padding: "var(--cs-space-4)", display: "flex", flexDirection: "column", gap: "var(--cs-space-4)", height: "100%", overflowY: "auto" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <div>
          <h1 style={{ margin: 0, fontSize: "var(--cs-text-lg)", fontWeight: 700 }}>
            Biên soạn nội dung & Biến thể kênh (Writing & Variants)
          </h1>
          <span style={{ fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-secondary)" }}>
            ContentBrief có trích dẫn số liệu, tạo biến thể bài đăng Facebook / Instagram
          </span>
        </div>
        <Button variant="primary">Lưu phiên bản (Save Revision)</Button>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--cs-space-4)" }}>
        {/* Left: Content Brief */}
        <div style={{ padding: "var(--cs-space-4)", backgroundColor: "var(--cs-bg-panel)", borderRadius: "var(--cs-radius-md)", border: "1px solid var(--cs-border-subtle)", display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
          <h2 style={{ margin: 0, fontSize: "var(--cs-text-md)", fontWeight: 600 }}>Bản tóm tắt nội dung (Content Brief)</h2>
          <Input label="Mục tiêu chính:" value="Giới thiệu gói giải pháp tự động hóa Q4" readOnly />
          <Input label="Đối tượng độc giả mục tiêu:" value="Chủ doanh nghiệp nhỏ & Quản lý Marketing" readOnly />
          <div>
            <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>Bằng chứng đính kèm (Evidence Citations):</span>
            <div style={{ padding: "var(--cs-space-2)", backgroundColor: "var(--cs-bg-raised)", borderRadius: "var(--cs-radius-sm)", marginTop: "var(--cs-space-1)", fontSize: "var(--cs-text-xs)" }}>
              ✓ Trích dẫn: "Tăng 42% hiệu suất vận hành nội dung theo báo cáo Gartner Q3" (Nguồn: Research #8f4a21)
            </div>
          </div>
        </div>

        {/* Right: Channel Variant */}
        <div style={{ padding: "var(--cs-space-4)", backgroundColor: "var(--cs-bg-panel)", borderRadius: "var(--cs-radius-md)", border: "1px solid var(--cs-border-subtle)", display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <h2 style={{ margin: 0, fontSize: "var(--cs-text-md)", fontWeight: 600 }}>Biến thể kênh đăng (Channel Variant)</h2>
            <div style={{ display: "flex", gap: "var(--cs-space-1)" }}>
              <button
                onClick={() => setSelectedPlatform("facebook")}
                style={{
                  padding: "4px 8px",
                  borderRadius: "var(--cs-radius-sm)",
                  border: "1px solid",
                  borderColor: selectedPlatform === "facebook" ? "var(--cs-accent)" : "var(--cs-border-subtle)",
                  backgroundColor: selectedPlatform === "facebook" ? "var(--cs-selection)" : "transparent",
                  color: selectedPlatform === "facebook" ? "var(--cs-accent)" : "var(--cs-fg-primary)",
                  fontSize: "var(--cs-text-xs)",
                  cursor: "pointer",
                }}
              >
                Facebook
              </button>
              <button
                onClick={() => setSelectedPlatform("instagram")}
                style={{
                  padding: "4px 8px",
                  borderRadius: "var(--cs-radius-sm)",
                  border: "1px solid",
                  borderColor: selectedPlatform === "instagram" ? "var(--cs-accent)" : "var(--cs-border-subtle)",
                  backgroundColor: selectedPlatform === "instagram" ? "var(--cs-selection)" : "transparent",
                  color: selectedPlatform === "instagram" ? "var(--cs-accent)" : "var(--cs-fg-primary)",
                  fontSize: "var(--cs-text-xs)",
                  cursor: "pointer",
                }}
              >
                Instagram
              </button>
            </div>
          </div>

          <div>
            <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>Nội dung bài viết (Caption):</span>
            <textarea
              style={{
                width: "100%",
                height: "140px",
                padding: "var(--cs-space-2)",
                borderRadius: "var(--cs-radius-md)",
                border: "1px solid var(--cs-border-subtle)",
                backgroundColor: "var(--cs-bg-panel)",
                color: "var(--cs-fg-primary)",
                fontFamily: "var(--cs-font-ui)",
                fontSize: "var(--cs-text-sm)",
                boxSizing: "border-box",
                marginTop: "var(--cs-space-1)",
              }}
              defaultValue={
                selectedPlatform === "facebook"
                  ? "🚀 Đột phá hiệu suất sáng tạo nội dung với Local Content Studio! Giải pháp độc lập, an toàn và tối ưu cho đội ngũ của bạn. Khám phá ngay chi tiết tại bài viết dưới đây."
                  : "✨ Tự do sáng tạo, bảo toàn bản quyền với Local Content Studio. Swipe xem ngay 6 trang bí quyết! 👉 #contentstudio #creative"
              }
            />
          </div>
        </div>
      </div>
    </div>
  );
};
