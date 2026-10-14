import React from "react";
import { Button, Badge } from "@studio/design-system";

export interface ExplorerPanelProps {
  activeView: string;
  onSelectView: (view: string) => void;
  projectsCount: number;
  assetsCount: number;
}

export const ExplorerPanel: React.FC<ExplorerPanelProps> = ({
  activeView,
  onSelectView,
  projectsCount,
  assetsCount,
}) => {
  return (
    <aside
      aria-label="Cây điều hướng Explorer"
      style={{
        width: "240px",
        backgroundColor: "var(--cs-bg-panel)",
        borderRight: "1px solid var(--cs-border-subtle)",
        display: "flex",
        flexDirection: "column",
        height: "100%",
        padding: "var(--cs-space-3)",
        gap: "var(--cs-space-3)",
      }}
    >
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-1)" }}>
        <span style={{ fontSize: "var(--cs-text-xs)", fontWeight: 700, color: "var(--cs-fg-secondary)", textTransform: "uppercase" }}>
          Không gian làm việc
        </span>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <span style={{ fontSize: "var(--cs-text-sm)", fontWeight: 600 }}>Marketing Q4</span>
          <Badge variant="info">Local-first</Badge>
        </div>
      </div>

      <nav style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-1)", flex: 1 }}>
        <button
          onClick={() => onSelectView("dashboard")}
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "var(--cs-space-2) var(--cs-space-3)",
            borderRadius: "var(--cs-radius-md)",
            border: "none",
            backgroundColor: activeView === "dashboard" ? "var(--cs-selection)" : "transparent",
            color: activeView === "dashboard" ? "var(--cs-accent)" : "var(--cs-fg-primary)",
            fontWeight: activeView === "dashboard" ? 600 : 400,
            cursor: "pointer",
            textAlign: "left",
          }}
        >
          <span>📊 Tổng quan (Dashboard)</span>
        </button>

        <button
          onClick={() => onSelectView("design")}
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "var(--cs-space-2) var(--cs-space-3)",
            borderRadius: "var(--cs-radius-md)",
            border: "none",
            backgroundColor: activeView === "design" ? "var(--cs-selection)" : "transparent",
            color: activeView === "design" ? "var(--cs-accent)" : "var(--cs-fg-primary)",
            fontWeight: activeView === "design" ? 600 : 400,
            cursor: "pointer",
            textAlign: "left",
          }}
        >
          <span>🎨 Thiết kế (Poster/Carousel)</span>
          <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>{projectsCount}</span>
        </button>

        <button
          onClick={() => onSelectView("research")}
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "var(--cs-space-2) var(--cs-space-3)",
            borderRadius: "var(--cs-radius-md)",
            border: "none",
            backgroundColor: activeView === "research" ? "var(--cs-selection)" : "transparent",
            color: activeView === "research" ? "var(--cs-accent)" : "var(--cs-fg-primary)",
            fontWeight: activeView === "research" ? 600 : 400,
            cursor: "pointer",
            textAlign: "left",
          }}
        >
          <span>🔍 Nghiên cứu (Research)</span>
        </button>

        <button
          onClick={() => onSelectView("library")}
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "var(--cs-space-2) var(--cs-space-3)",
            borderRadius: "var(--cs-radius-md)",
            border: "none",
            backgroundColor: activeView === "library" ? "var(--cs-selection)" : "transparent",
            color: activeView === "library" ? "var(--cs-accent)" : "var(--cs-fg-primary)",
            fontWeight: activeView === "library" ? 600 : 400,
            cursor: "pointer",
            textAlign: "left",
          }}
        >
          <span>📁 Thư viện tài nguyên</span>
          <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>{assetsCount}</span>
        </button>

        <button
          onClick={() => onSelectView("brand")}
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "var(--cs-space-2) var(--cs-space-3)",
            borderRadius: "var(--cs-radius-md)",
            border: "none",
            backgroundColor: activeView === "brand" ? "var(--cs-selection)" : "transparent",
            color: activeView === "brand" ? "var(--cs-accent)" : "var(--cs-fg-primary)",
            fontWeight: activeView === "brand" ? 600 : 400,
            cursor: "pointer",
            textAlign: "left",
          }}
        >
          <span>🏷️ Brand Kit (Nhận diện)</span>
        </button>

        <button
          onClick={() => onSelectView("writing")}
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "var(--cs-space-2) var(--cs-space-3)",
            borderRadius: "var(--cs-radius-md)",
            border: "none",
            backgroundColor: activeView === "writing" ? "var(--cs-selection)" : "transparent",
            color: activeView === "writing" ? "var(--cs-accent)" : "var(--cs-fg-primary)",
            fontWeight: activeView === "writing" ? 600 : 400,
            cursor: "pointer",
            textAlign: "left",
          }}
        >
          <span>✍️ Soạn thảo (Writing)</span>
        </button>

        <button
          onClick={() => onSelectView("publishing")}
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "var(--cs-space-2) var(--cs-space-3)",
            borderRadius: "var(--cs-radius-md)",
            border: "none",
            backgroundColor: activeView === "publishing" ? "var(--cs-selection)" : "transparent",
            color: activeView === "publishing" ? "var(--cs-accent)" : "var(--cs-fg-primary)",
            fontWeight: activeView === "publishing" ? 600 : 400,
            cursor: "pointer",
            textAlign: "left",
          }}
        >
          <span>🚀 Bàn giao Social (Handoff)</span>
        </button>
      </nav>

      <div style={{ borderTop: "1px solid var(--cs-border-subtle)", paddingTop: "var(--cs-space-2)" }}>
        <Button variant="secondary" style={{ width: "100%" }} onClick={() => onSelectView("design")}>
          + Dự án mới
        </Button>
      </div>
    </aside>
  );
};
