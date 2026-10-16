import React from "react";

export type CalloutVariant = "info" | "warning" | "danger" | "success" | "neutral";

export interface CalloutProps {
  variant?: CalloutVariant;
  title?: string;
  children: React.ReactNode;
  actions?: React.ReactNode;
  icon?: React.ReactNode;
  style?: React.CSSProperties;
}

export const Callout: React.FC<CalloutProps> = ({
  variant = "info",
  title,
  children,
  actions,
  icon,
  style,
}) => {
  const getVariantStyles = () => {
    switch (variant) {
      case "danger":
        return {
          bg: "var(--cs-status-danger-bg)",
          border: "var(--cs-danger)",
          fg: "var(--cs-danger)",
        };
      case "warning":
        return {
          bg: "var(--cs-status-warning-bg)",
          border: "var(--cs-warning)",
          fg: "var(--cs-warning)",
        };
      case "success":
        return {
          bg: "var(--cs-status-success-bg)",
          border: "var(--cs-success)",
          fg: "var(--cs-success)",
        };
      case "neutral":
        return {
          bg: "var(--cs-status-neutral-bg)",
          border: "var(--cs-border-strong)",
          fg: "var(--cs-fg-secondary)",
        };
      case "info":
      default:
        return {
          bg: "var(--cs-status-info-bg)",
          border: "var(--cs-accent)",
          fg: "var(--cs-accent)",
        };
    }
  };

  const colors = getVariantStyles();
  const role = variant === "danger" ? "alert" : "status";

  return (
    <div
      role={role}
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "var(--cs-space-2)",
        padding: "var(--cs-space-3)",
        backgroundColor: colors.bg,
        borderLeft: `3px solid ${colors.border}`,
        borderRadius: "var(--cs-radius-none)",
        fontSize: "var(--cs-text-sm)",
        color: "var(--cs-fg-primary)",
        ...style,
      }}
    >
      <div style={{ display: "flex", alignItems: "flex-start", gap: "var(--cs-space-2)" }}>
        {icon && (
          <span style={{ color: colors.fg, display: "inline-flex", marginTop: 1 }}>
            {icon}
          </span>
        )}
        <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: 4 }}>
          {title && (
            <div style={{ fontWeight: 600, color: colors.fg }}>
              {title}
            </div>
          )}
          <div style={{ lineHeight: 1.4 }}>{children}</div>
        </div>
      </div>
      {actions && (
        <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-2)", marginTop: 4 }}>
          {actions}
        </div>
      )}
    </div>
  );
};
