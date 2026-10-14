import React from "react";
import { Badge, Button } from "@studio/design-system";

export interface DashboardViewProps {
  onOpenProject: (projectId: string) => void;
  onCreateNew: () => void;
}

export const DashboardView: React.FC<DashboardViewProps> = ({
  onOpenProject,
  onCreateNew,
}) => {
  return (
    <div style={{ padding: "var(--cs-space-4)", display: "flex", flexDirection: "column", gap: "var(--cs-space-4)", height: "100%", overflowY: "auto" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <div>
          <h1 style={{ margin: 0, fontSize: "var(--cs-text-lg)", fontWeight: 700 }}>
            Creative Production Workspace
          </h1>
          <span style={{ fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-secondary)" }}>
            Hệ thống sản xuất nội dung đa kênh độc lập, local-first
          </span>
        </div>
        <Button variant="primary" onClick={onCreateNew}>
          + Tạo dự án mới
        </Button>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(200px, 1fr))", gap: "var(--cs-space-3)" }}>
        <div style={{ padding: "var(--cs-space-3)", backgroundColor: "var(--cs-bg-panel)", borderRadius: "var(--cs-radius-md)", border: "1px solid var(--cs-border-subtle)" }}>
          <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", textTransform: "uppercase" }}>Dự án đang làm</span>
          <div style={{ fontSize: "28px", fontWeight: 700, margin: "var(--cs-space-1) 0" }}>3</div>
          <Badge variant="info">2 Carousel, 1 Poster</Badge>
        </div>

        <div style={{ padding: "var(--cs-space-3)", backgroundColor: "var(--cs-bg-panel)", borderRadius: "var(--cs-radius-md)", border: "1px solid var(--cs-border-subtle)" }}>
          <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", textTransform: "uppercase" }}>Tài nguyên đã tải</span>
          <div style={{ fontSize: "28px", fontWeight: 700, margin: "var(--cs-space-1) 0" }}>14</div>
          <Badge variant="warning">2 chờ xác nhận bản quyền</Badge>
        </div>

        <div style={{ padding: "var(--cs-space-3)", backgroundColor: "var(--cs-bg-panel)", borderRadius: "var(--cs-radius-md)", border: "1px solid var(--cs-border-subtle)" }}>
          <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", textTransform: "uppercase" }}>Bộ nhận diện Brand</span>
          <div style={{ fontSize: "28px", fontWeight: 700, margin: "var(--cs-space-1) 0" }}>v2</div>
          <Badge variant="success">Đã xuất bản</Badge>
        </div>

        <div style={{ padding: "var(--cs-space-3)", backgroundColor: "var(--cs-bg-panel)", borderRadius: "var(--cs-radius-md)", border: "1px solid var(--cs-border-subtle)" }}>
          <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", textTransform: "uppercase" }}>Trạng thái Local DB</span>
          <div style={{ fontSize: "28px", fontWeight: 700, margin: "var(--cs-space-1) 0" }}>WAL</div>
          <Badge variant="success">Bản sao lưu: Khớp 100%</Badge>
        </div>
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-2)" }}>
        <h2 style={{ margin: 0, fontSize: "var(--cs-text-md)", fontWeight: 600 }}>Dự án gần đây</h2>
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-2)" }}>
          <div
            style={{
              padding: "var(--cs-space-3)",
              backgroundColor: "var(--cs-bg-panel)",
              borderRadius: "var(--cs-radius-md)",
              border: "1px solid var(--cs-border-subtle)",
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
            }}
          >
            <div>
              <div style={{ fontWeight: 600 }}>Chiến dịch Ra Mắt Sản Phẩm Q4</div>
              <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
                Carousel 6 Artboards · Bản sửa: rev-4 (SHA: a1b2c3d4) · Khả năng xuất: PNG/HTML
              </span>
            </div>
            <Button variant="secondary" onClick={() => onOpenProject("proj-1")}>
              Mở chỉnh sửa
            </Button>
          </div>

          <div
            style={{
              padding: "var(--cs-space-3)",
              backgroundColor: "var(--cs-bg-panel)",
              borderRadius: "var(--cs-radius-md)",
              border: "1px solid var(--cs-border-subtle)",
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
            }}
          >
            <div>
              <div style={{ fontWeight: 600 }}>Poster Khuyến Mãi Cuối Tuần</div>
              <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
                Poster đơn 1080x1350 · Bản sửa: rev-2 (SHA: e5f6g7h8) · Đã khóa Brand v2
              </span>
            </div>
            <Button variant="secondary" onClick={() => onOpenProject("proj-2")}>
              Mở chỉnh sửa
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
};
