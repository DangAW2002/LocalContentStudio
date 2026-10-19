import React from "react";
import { Badge } from "@studio/design-system";
import type { ScopeItem } from "../../api/types";

export const LAYER_LABELS: Record<string, string> = {
  workspace: "Thư mục Workspace",
  org: "Tổ chức",
  app: "Mặc định ứng dụng",
};

export interface ScopeListProps {
  items: ScopeItem[];
  selectedId: string | null;
  onSelectId: (id: string) => void;
}

export const ScopeList: React.FC<ScopeListProps> = ({
  items,
  selectedId,
  onSelectId,
}) => {
  return (
    <div
      style={{
        width: 300,
        borderRight: "1px solid var(--cs-border-subtle)",
        backgroundColor: "var(--cs-bg-panel)",
        overflowY: "auto",
        flexShrink: 0,
      }}
    >
      {items.map((item) => {
        const isSelected = item.id === selectedId;
        return (
          <div
            key={item.id}
            onClick={() => onSelectId(item.id)}
            style={{
              padding: "var(--cs-space-3) var(--cs-space-4)",
              borderBottom: "1px solid var(--cs-border-subtle)",
              cursor: "pointer",
              backgroundColor: isSelected ? "var(--cs-selection)" : "transparent",
              borderLeft: isSelected ? "2px solid var(--cs-accent)" : "2px solid transparent",
              transition: "background-color var(--cs-motion-duration) ease",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 4 }}>
              <span style={{ fontWeight: 600, color: "var(--cs-fg-primary)", fontSize: "var(--cs-text-sm)" }}>
                {item.name}
              </span>
              <Badge
                variant={
                  item.layer === "workspace"
                    ? "info"
                    : item.layer === "org"
                    ? "neutral"
                    : "warning"
                }
              >
                {LAYER_LABELS[item.layer] || item.layer}
              </Badge>
            </div>
            {item.summary && (
              <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", lineHeight: 1.4 }}>
                {item.summary}
              </span>
            )}
          </div>
        );
      })}
    </div>
  );
};
