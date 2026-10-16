import React from "react";

export interface EmptyStateProps {
  icon?: React.ReactNode;
  title: string;
  description?: string;
  action?: React.ReactNode;
  secondaryAction?: React.ReactNode;
  style?: React.CSSProperties;
}

export const EmptyState: React.FC<EmptyStateProps> = ({
  icon,
  title,
  description,
  action,
  secondaryAction,
  style,
}) => {
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        textAlign: "center",
        padding: "var(--cs-space-8) var(--cs-space-4)",
        color: "var(--cs-fg-secondary)",
        ...style,
      }}
    >
      {icon && (
        <div
          style={{
            width: 56,
            height: 56,
            borderRadius: "var(--cs-radius-control)",
            backgroundColor: "var(--cs-bg-raised)",
            border: "1px solid var(--cs-border-subtle)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            fontSize: 28,
            marginBottom: "var(--cs-space-3)",
            color: "var(--cs-fg-muted)",
          }}
        >
          {icon}
        </div>
      )}
      <h3
        style={{
          margin: "0 0 var(--cs-space-1) 0",
          fontSize: "var(--cs-text-md)",
          fontWeight: 600,
          color: "var(--cs-fg-primary)",
        }}
      >
        {title}
      </h3>
      {description && (
        <p
          style={{
            margin: "0 0 var(--cs-space-4) 0",
            maxWidth: 420,
            fontSize: "var(--cs-text-sm)",
            lineHeight: 1.5,
            color: "var(--cs-fg-secondary)",
          }}
        >
          {description}
        </p>
      )}
      {(action || secondaryAction) && (
        <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-2)" }}>
          {action}
          {secondaryAction}
        </div>
      )}
    </div>
  );
};
