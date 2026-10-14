import React from "react";
import { Badge, Button } from "@studio/design-system";

export interface DesignViewProps {
  onSelectElement: (element: any) => void;
  onExport: () => void;
}

export const DesignView: React.FC<DesignViewProps> = ({
  onSelectElement,
  onExport,
}) => {
  const [activeBoardIdx, setActiveBoardIdx] = React.useState(0);
  const [zoom, setZoom] = React.useState(100);

  // 6 Artboards for carousel
  const artboards = Array.from({ length: 6 }).map((_, idx) => ({
    id: `artboard-${idx + 1}`,
    name: `Trang ${idx + 1} / 6`,
    elements: [
      {
        id: `el-head-${idx + 1}`,
        type: "text",
        semanticRole: "headline",
        text: `Tiêu đề thông điệp số ${idx + 1}`,
        box: { x: 80, y: 100, w: 920, h: 120 },
      },
      {
        id: `el-img-${idx + 1}`,
        type: "image",
        semanticRole: "productImage",
        box: { x: 80, y: 260, w: 920, h: 600 },
      },
      {
        id: `el-cta-${idx + 1}`,
        type: "text",
        semanticRole: "cta",
        text: idx === 5 ? "Bấm vào link bio để đăng ký ngay!" : "Vuốt sang trang kế tiếp →",
        box: { x: 80, y: 920, w: 920, h: 80 },
      },
    ],
  }));

  const currentBoard = artboards[activeBoardIdx];

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", width: "100%", backgroundColor: "var(--cs-bg-canvas)" }}>
      {/* Top Toolbar */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "var(--cs-space-2) var(--cs-space-4)",
          backgroundColor: "var(--cs-bg-panel)",
          borderBottom: "1px solid var(--cs-border-subtle)",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-3)" }}>
          <span style={{ fontWeight: 600, fontSize: "var(--cs-text-md)" }}>Carousel 1080x1350 (6 trang)</span>
          <Badge variant="success">Bản đã lưu: rev-4</Badge>
          <Badge variant="info">Brand Kit: v2</Badge>
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-2)" }}>
          <Button variant="quiet" onClick={() => setZoom(Math.max(50, zoom - 10))}>-</Button>
          <span style={{ fontSize: "var(--cs-text-xs)", fontFamily: "var(--cs-font-mono)", width: "45px", textAlign: "center" }}>
            {zoom}%
          </span>
          <Button variant="quiet" onClick={() => setZoom(Math.min(200, zoom + 10))}>+</Button>
          <Button variant="quiet" onClick={() => setZoom(100)}>100%</Button>
          <div style={{ width: "1px", height: "20px", backgroundColor: "var(--cs-border-subtle)" }} />
          <Button variant="primary" onClick={onExport}>
            🖼️ Xuất PNG / Xem trước
          </Button>
        </div>
      </div>

      {/* Artboard Switcher Tabs */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: "var(--cs-space-1)",
          padding: "var(--cs-space-2) var(--cs-space-4)",
          backgroundColor: "var(--cs-bg-raised)",
          borderBottom: "1px solid var(--cs-border-subtle)",
          overflowX: "auto",
        }}
      >
        {artboards.map((board, idx) => (
          <button
            key={board.id}
            onClick={() => {
              setActiveBoardIdx(idx);
              onSelectElement(null);
            }}
            style={{
              padding: "var(--cs-space-1) var(--cs-space-3)",
              borderRadius: "var(--cs-radius-sm)",
              border: "1px solid",
              borderColor: activeBoardIdx === idx ? "var(--cs-accent)" : "var(--cs-border-subtle)",
              backgroundColor: activeBoardIdx === idx ? "var(--cs-bg-panel)" : "transparent",
              color: activeBoardIdx === idx ? "var(--cs-accent)" : "var(--cs-fg-primary)",
              fontWeight: activeBoardIdx === idx ? 600 : 400,
              cursor: "pointer",
              fontSize: "var(--cs-text-xs)",
              whiteSpace: "nowrap",
            }}
          >
            {board.name}
          </button>
        ))}
      </div>

      {/* Canvas Viewport */}
      <div
        style={{
          flex: 1,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          overflow: "auto",
          padding: "var(--cs-space-6)",
        }}
      >
        <div
          role="region"
          aria-label={`Canvas ${currentBoard.name}`}
          style={{
            width: `${(1080 * zoom) / 250}px`,
            height: `${(1350 * zoom) / 250}px`,
            backgroundColor: "var(--cs-canvas-artboard)",
            boxShadow: "var(--cs-shadow-panel)",
            position: "relative",
            borderRadius: "var(--cs-radius-sm)",
            border: "1px solid var(--cs-border-strong)",
            transition: "width 150ms ease, height 150ms ease",
            overflow: "hidden",
          }}
        >
          {/* Safe zones guides */}
          <div
            style={{
              position: "absolute",
              inset: "8%",
              border: "1px dashed var(--cs-canvas-guide)",
              pointerEvents: "none",
              opacity: 0.5,
            }}
          />

          {/* Render Elements */}
          {currentBoard.elements.map((el) => {
            const scale = zoom / 250;
            return (
              <div
                key={el.id}
                onClick={(e) => {
                  e.stopPropagation();
                  onSelectElement(el);
                }}
                style={{
                  position: "absolute",
                  left: `${el.box.x * scale}px`,
                  top: `${el.box.y * scale}px`,
                  width: `${el.box.w * scale}px`,
                  height: `${el.box.h * scale}px`,
                  border: "1px solid var(--cs-accent)",
                  backgroundColor: el.type === "image" ? "var(--cs-bg-raised)" : "transparent",
                  cursor: "pointer",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  padding: "4px",
                  fontSize: `${14 * (zoom / 100)}px`,
                  fontWeight: el.semanticRole === "headline" ? 700 : 400,
                  color: "var(--cs-fg-primary)",
                  boxSizing: "border-box",
                }}
              >
                {el.type === "image" ? (
                  <span style={{ fontSize: "12px", color: "var(--cs-fg-secondary)" }}>
                    [Hình ảnh sản phẩm - 920x600]
                  </span>
                ) : (
                  el.text
                )}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
};
