import React from "react";
import { Badge, Button } from "@studio/design-system";
import { Icon } from "../../components/Icon";
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
  onNewItem?: () => void;
  kindLabel?: string;
}

export const ScopeList: React.FC<ScopeListProps> = ({
  items,
  selectedId,
  onSelectId,
  onNewItem,
  kindLabel,
}) => {
  return (
    <div
      style={{
        width: 300,
        borderRight: "1px solid var(--cs-border-subtle)",
        backgroundColor: "var(--cs-bg-panel)",
        display: "flex",
        flexDirection: "column",
        flexShrink: 0,
      }}
    >
      {/* Top action header */}
      {onNewItem && (
        <div
          style={{
            padding: "var(--cs-space-3) var(--cs-space-4)",
            borderBottom: "1px solid var(--cs-border-subtle)",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
          }}
        >
          <span style={{ fontSize: "var(--cs-text-xs)", fontWeight: 700, color: "var(--cs-fg-secondary)" }}>
            DANH SÁCH {kindLabel ? kindLabel.toUpperCase() : ""}
          </span>
          <Button
            size="sm"
            variant="secondary"
            onClick={onNewItem}
            leftIcon={<Icon name="plus" size={12} />}
          >
            Mới
          </Button>
        </div>
      )}

      {/* Items list */}
      <div style={{ flex: 1, overflowY: "auto" }}>
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
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 4, gap: 6 }}>
                <span style={{ fontWeight: 600, color: "var(--cs-fg-primary)", fontSize: "var(--cs-text-sm)", wordBreak: "break-all" }}>
                  {item.name}
                </span>
                <div style={{ display: "flex", alignItems: "center", gap: 4, flexShrink: 0 }}>
                  {item.error ? (
                    <Badge variant="danger">Lỗi cấu hình</Badge>
                  ) : item.isOverride ? (
                    <Badge variant="warning">Đang ghi đè</Badge>
                  ) : (
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
                  )}
                </div>
              </div>

              {item.error ? (
                <div
                  style={{
                    fontSize: "var(--cs-text-xs)",
                    color: "var(--cs-danger)",
                    lineHeight: 1.4,
                    marginTop: 2,
                    fontFamily: "var(--cs-font-mono)",
                  }}
                >
                  {item.error}
                </div>
              ) : item.summary ? (
                <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", lineHeight: 1.4 }}>
                  {item.summary}
                </span>
              ) : null}
            </div>
          );
        })}
      </div>
    </div>
  );
};
