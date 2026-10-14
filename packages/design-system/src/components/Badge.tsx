import React from "react";

export type BadgeVariant = "info" | "success" | "warning" | "danger";

export interface BadgeProps {
  variant?: BadgeVariant;
  children: React.ReactNode;
  icon?: React.ReactNode;
}

export const Badge: React.FC<BadgeProps> = ({
  variant = "info",
  children,
  icon,
}) => {
  const getColors = () => {
    switch (variant) {
      case "success":
        return {
          bg: "var(--cs-status-success-bg)",
          fg: "var(--cs-success)",
          border: "var(--cs-success)",
        };
      case "warning":
        return {
          bg: "var(--cs-status-warning-bg)",
          fg: "var(--cs-warning)",
          border: "var(--cs-warning)",
        };
      case "danger":
        return {
          bg: "var(--cs-status-danger-bg)",
          fg: "var(--cs-danger)",
          border: "var(--cs-danger)",
        };
      case "info":
      default:
        return {
          bg: "var(--cs-status-info-bg)",
          fg: "var(--cs-accent)",
          border: "var(--cs-accent)",
        };
    }
  };

  const colors = getColors();

  return (
    <span
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: "var(--cs-space-1)",
        padding: "2px var(--cs-space-2)",
        borderRadius: "var(--cs-radius-sm)",
        fontSize: "var(--cs-text-xs)",
        fontWeight: 600,
        backgroundColor: colors.bg,
        color: colors.fg,
        border: `1px solid ${colors.border}`,
        lineHeight: 1.2,
      }}
    >
      {icon && <span aria-hidden="true">{icon}</span>}
      <span>{children}</span>
    </span>
  );
};
