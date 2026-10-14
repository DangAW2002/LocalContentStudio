import React from "react";
import { Badge, Button, Input } from "@studio/design-system";

export const BrandView: React.FC = () => {
  return (
    <div style={{ padding: "var(--cs-space-4)", display: "flex", flexDirection: "column", gap: "var(--cs-space-4)", height: "100%", overflowY: "auto" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <div>
          <h1 style={{ margin: 0, fontSize: "var(--cs-text-lg)", fontWeight: 700 }}>
            Hệ thống nhận diện (Brand Kit)
          </h1>
          <span style={{ fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-secondary)" }}>
            Quy chuẩn màu sắc, phông chữ, giọng điệu, từ cấm và phiên bản bất biến BrandVersion
          </span>
        </div>
        <div style={{ display: "flex", gap: "var(--cs-space-2)" }}>
          <Button variant="secondary">Tạo bản nháp v3</Button>
          <Button variant="primary">Xuất bản BrandVersion (Human)</Button>
        </div>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--cs-space-4)" }}>
        <div style={{ padding: "var(--cs-space-4)", backgroundColor: "var(--cs-bg-panel)", borderRadius: "var(--cs-radius-md)", border: "1px solid var(--cs-border-subtle)", display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <h2 style={{ margin: 0, fontSize: "var(--cs-text-md)", fontWeight: 600 }}>Visual Tokens (Màu & Font)</h2>
            <Badge variant="success">Phiên bản hiện tại: v2</Badge>
          </div>

          <div style={{ display: "flex", gap: "var(--cs-space-3)" }}>
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-1)" }}>
              <span style={{ fontSize: "var(--cs-text-xs)" }}>Màu chủ đạo (Primary)</span>
              <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-2)" }}>
                <div style={{ width: "24px", height: "24px", borderRadius: "4px", backgroundColor: "#334bd1" }} />
                <span style={{ fontFamily: "var(--cs-font-mono)", fontSize: "var(--cs-text-sm)" }}>#334bd1</span>
              </div>
            </div>

            <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-1)" }}>
              <span style={{ fontSize: "var(--cs-text-xs)" }}>Màu nền (Surface)</span>
              <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-2)" }}>
                <div style={{ width: "24px", height: "24px", borderRadius: "4px", backgroundColor: "#ffffff", border: "1px solid var(--cs-border-subtle)" }} />
                <span style={{ fontFamily: "var(--cs-font-mono)", fontSize: "var(--cs-text-sm)" }}>#ffffff</span>
              </div>
            </div>
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-1)" }}>
            <span style={{ fontSize: "var(--cs-text-xs)" }}>Phông chữ tiêu đề (Heading Font):</span>
            <div style={{ fontWeight: 600 }}>Inter, Segoe UI, sans-serif</div>
          </div>
        </div>

        <div style={{ padding: "var(--cs-space-4)", backgroundColor: "var(--cs-bg-panel)", borderRadius: "var(--cs-radius-md)", border: "1px solid var(--cs-border-subtle)", display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
          <h2 style={{ margin: 0, fontSize: "var(--cs-text-md)", fontWeight: 600 }}>Giọng điệu & Từ ngữ cấm (Voice Guidelines)</h2>
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-1)" }}>
            <span style={{ fontSize: "var(--cs-text-xs)" }}>Giọng điệu (Tone):</span>
            <div>Chuyên nghiệp, tin cậy, thúc đẩy hành động (Professional & Empowering)</div>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-1)" }}>
            <span style={{ fontSize: "var(--cs-text-xs)" }}>Danh sách từ ngữ cấm (Banned vocabulary):</span>
            <div style={{ display: "flex", gap: "var(--cs-space-1)", flexWrap: "wrap" }}>
              <Badge variant="danger">cam kết 100%</Badge>
              <Badge variant="danger">chữa dứt điểm</Badge>
              <Badge variant="danger">rẻ nhất thị trường</Badge>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
