import React from "react";

export interface TabItem {
  id: string;
  label: string;
  count?: number;
}

export interface TabsProps {
  items: TabItem[];
  activeId: string;
  onChange: (id: string) => void;
  size?: "md" | "sm";
  style?: React.CSSProperties;
}

export const Tabs: React.FC<TabsProps> = ({
  items,
  activeId,
  onChange,
  size = "md",
  style,
}) => {
  const height = size === "sm" ? 32 : 40;
  const fontSize = size === "sm" ? "var(--cs-text-xs)" : "var(--cs-text-sm)";

  return (
    <div
      role="tablist"
      style={{
        display: "flex",
        alignItems: "center",
        gap: "var(--cs-space-4)",
        borderBottom: "1px solid var(--cs-border-subtle)",
        height,
        ...style,
      }}
    >
      {items.map((item) => {
        const isActive = item.id === activeId;
        return (
          <button
            key={item.id}
            role="tab"
            aria-selected={isActive}
            onClick={() => onChange(item.id)}
            style={{
              height: "100%",
              display: "inline-flex",
              alignItems: "center",
              gap: 6,
              background: "none",
              border: "none",
              borderBottom: isActive ? "2px solid var(--cs-accent)" : "2px solid transparent",
              color: isActive ? "var(--cs-accent)" : "var(--cs-fg-secondary)",
              fontWeight: isActive ? 600 : 500,
              fontSize,
              cursor: "pointer",
              padding: "0 4px",
              marginBottom: -1,
              outline: "none",
              transition: "color var(--cs-motion-duration) ease",
            }}
          >
            <span>{item.label}</span>
            {item.count !== undefined && (
              <span
                style={{
                  fontSize: "var(--cs-text-xs)",
                  padding: "1px 5px",
                  borderRadius: "var(--cs-radius-control)",
                  backgroundColor: isActive ? "var(--cs-accent-subtle)" : "var(--cs-bg-raised)",
                  color: isActive ? "var(--cs-accent)" : "var(--cs-fg-muted)",
                }}
              >
                {item.count}
              </span>
            )}
          </button>
        );
      })}
    </div>
  );
};
