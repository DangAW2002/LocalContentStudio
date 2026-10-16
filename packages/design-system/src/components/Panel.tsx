import React from "react";

export interface PanelProps {
  title: string;
  children: React.ReactNode;
  actions?: React.ReactNode;
  collapsible?: boolean;
  defaultCollapsed?: boolean;
  style?: React.CSSProperties;
}

export const Panel: React.FC<PanelProps> = ({
  title,
  children,
  actions,
  collapsible = false,
  defaultCollapsed = false,
  style,
}) => {
  const [collapsed, setCollapsed] = React.useState(defaultCollapsed);

  return (
    <section
      style={{
        display: "flex",
        flexDirection: "column",
        backgroundColor: "var(--cs-bg-panel)",
        border: "1px solid var(--cs-border-subtle)",
        borderRadius: "var(--cs-radius-none)",
        overflow: "hidden",
        height: "100%",
        ...style,
      }}
    >
      <header
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "var(--cs-space-2) var(--cs-space-3)",
          backgroundColor: "var(--cs-bg-raised)",
          borderBottom: collapsed ? "none" : "1px solid var(--cs-border-subtle)",
          userSelect: "none",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-2)" }}>
          {collapsible && (
            <button
              onClick={() => setCollapsed(!collapsed)}
              style={{
                background: "none",
                border: "none",
                cursor: "pointer",
                padding: 0,
                color: "var(--cs-fg-secondary)",
                fontSize: "var(--cs-text-xs)",
              }}
              aria-expanded={!collapsed}
              aria-label={collapsed ? `Mở rộng ${title}` : `Thu gọn ${title}`}
            >
              {collapsed ? "▶" : "▼"}
            </button>
          )}
          <h3
            style={{
              margin: 0,
              fontSize: "var(--cs-text-sm)",
              fontWeight: 600,
              color: "var(--cs-fg-primary)",
            }}
          >
            {title}
          </h3>
        </div>

        {actions && !collapsed && (
          <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-1)" }}>
            {actions}
          </div>
        )}
      </header>

      {!collapsed && (
        <div style={{ flex: 1, overflowY: "auto", padding: "var(--cs-space-3)" }}>
          {children}
        </div>
      )}
    </section>
  );
};
