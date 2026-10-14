import React from "react";
import { Badge, Button } from "@studio/design-system";

export interface InspectorPanelProps {
  selectedElement: {
    id: string;
    type: string;
    semanticRole: string;
    text?: string;
    box: { x: number; y: number; w: number; h: number };
  } | null;
  brandTokens: {
    headingFont: string;
    primaryColor: string;
    surfaceColor: string;
  };
}

export const InspectorPanel: React.FC<InspectorPanelProps> = ({
  selectedElement,
  brandTokens,
}) => {
  return (
    <aside
      aria-label="Bảng thuộc tính Inspector"
      style={{
        width: "280px",
        backgroundColor: "var(--cs-bg-panel)",
        borderLeft: "1px solid var(--cs-border-subtle)",
        display: "flex",
        flexDirection: "column",
        height: "100%",
        padding: "var(--cs-space-3)",
        gap: "var(--cs-space-3)",
        overflowY: "auto",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", borderBottom: "1px solid var(--cs-border-subtle)", paddingBottom: "var(--cs-space-2)" }}>
        <h3 style={{ margin: 0, fontSize: "var(--cs-text-sm)", fontWeight: 700, textTransform: "uppercase", color: "var(--cs-fg-secondary)" }}>
          Thuộc tính đối tượng
        </h3>
        {selectedElement ? <Badge variant="success">Đang chọn</Badge> : <Badge variant="info">Toàn trang</Badge>}
      </div>

      {selectedElement ? (
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
          <div>
            <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>ID phần tử:</span>
            <div style={{ fontFamily: "var(--cs-font-mono)", fontSize: "var(--cs-text-xs)" }}>{selectedElement.id}</div>
          </div>

          <div>
            <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>Loại & Vai trò ngữ nghĩa:</span>
            <div style={{ fontWeight: 600 }}>{selectedElement.type} ({selectedElement.semanticRole})</div>
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--cs-space-2)" }}>
            <div>
              <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>Tọa độ X, Y:</span>
              <div style={{ fontFamily: "var(--cs-font-mono)" }}>{selectedElement.box.x}, {selectedElement.box.y} px</div>
            </div>
            <div>
              <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>Rộng x Cao:</span>
              <div style={{ fontFamily: "var(--cs-font-mono)" }}>{selectedElement.box.w} x {selectedElement.box.h} px</div>
            </div>
          </div>

          {selectedElement.text && (
            <div>
              <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>Nội dung văn bản:</span>
              <div style={{ padding: "var(--cs-space-2)", backgroundColor: "var(--cs-bg-raised)", borderRadius: "var(--cs-radius-sm)", fontSize: "var(--cs-text-sm)" }}>
                {selectedElement.text}
              </div>
            </div>
          )}

          <div style={{ borderTop: "1px solid var(--cs-border-subtle)", paddingTop: "var(--cs-space-2)" }}>
            <span style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600 }}>Ràng buộc Brand Kit:</span>
            <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-2)", marginTop: "var(--cs-space-1)" }}>
              <span style={{ width: "16px", height: "16px", borderRadius: "50%", backgroundColor: brandTokens.primaryColor, border: "1px solid var(--cs-border-strong)" }} />
              <span style={{ fontSize: "var(--cs-text-xs)" }}>Primary Token</span>
            </div>
          </div>
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)", color: "var(--cs-fg-secondary)", fontSize: "var(--cs-text-sm)" }}>
          <p>Nhấp chuột vào một phần tử trên Artboard để chỉnh sửa kích thước, nội dung và phong cách.</p>
          <div style={{ borderTop: "1px solid var(--cs-border-subtle)", paddingTop: "var(--cs-space-2)" }}>
            <span style={{ fontSize: "var(--cs-text-xs)", fontWeight: 700, textTransform: "uppercase" }}>Bộ nhận diện áp dụng:</span>
            <div style={{ marginTop: "var(--cs-space-2)", display: "flex", flexDirection: "column", gap: "var(--cs-space-1)" }}>
              <div>Phông tiêu đề: <strong>{brandTokens.headingFont}</strong></div>
              <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-2)" }}>
                <span>Màu chủ đạo:</span>
                <span style={{ width: "16px", height: "16px", borderRadius: "4px", backgroundColor: brandTokens.primaryColor }} />
              </div>
            </div>
          </div>
        </div>
      )}
    </aside>
  );
};
