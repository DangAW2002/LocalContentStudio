import React from "react";
import type { ScopeItem } from "../../api/types";
import { SafeZonePreview } from "./SafeZonePreview";

export interface ChannelDetailProps {
  selectedItem: ScopeItem;
  rawText: string | null;
}

export const ChannelDetail: React.FC<ChannelDetailProps> = ({
  selectedItem,
  rawText,
}) => {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-4)" }}>
      <div style={cardSectionStyle}>
        <div style={sectionTitleStyle}>QUY CHUẨN KHUNG HÌNH & KHU VỰC AN TOÀN (SAFE ZONE)</div>
        <p style={{ margin: "0 0 12px 0", fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
          Khu vực an toàn đảm bảo phụ đề, logo và nội dung cốt lõi không bị che bởi giao diện người dùng (nút thả tim, thanh điều hướng, mô tả).
        </p>

        {selectedItem.formats && selectedItem.formats.length > 0 ? (
          <div style={{ display: "flex", gap: 24, flexWrap: "wrap", justifyContent: "center", alignItems: "flex-end" }}>
            {selectedItem.formats.map((f) => (
              <SafeZonePreview key={f.name} format={f} />
            ))}
          </div>
        ) : (
          <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
            Kênh này chưa khai báo định dạng khung hình.
          </span>
        )}
      </div>

      {/* Raw Channel Definition Text */}
      <div style={cardSectionStyle}>
        <div style={sectionTitleStyle}>CẤU HÌNH KÊNH (YAML)</div>
        <pre
          style={{
            margin: 0,
            padding: "var(--cs-space-3)",
            backgroundColor: "var(--cs-bg-raised)",
            border: "1px solid var(--cs-border-subtle)",
            borderRadius: "var(--cs-radius-control)",
            fontFamily: "var(--cs-font-mono)",
            fontSize: 11,
            lineHeight: 1.5,
            overflowX: "auto",
          }}
        >
          {rawText || "Đang tải cấu hình..."}
        </pre>
      </div>
    </div>
  );
};

const cardSectionStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 8,
  padding: "var(--cs-space-4)",
  backgroundColor: "var(--cs-bg-panel)",
  border: "1px solid var(--cs-border-subtle)",
  borderRadius: "var(--cs-radius-control)",
};

const sectionTitleStyle: React.CSSProperties = {
  fontSize: "var(--cs-text-xs)",
  fontWeight: 700,
  color: "var(--cs-fg-secondary)",
  letterSpacing: 0.5,
};
