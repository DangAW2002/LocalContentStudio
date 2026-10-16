import React from "react";

export interface ChipProps {
  label: string;
  onRemove?: () => void;
  onClick?: () => void;
  selected?: boolean;
  style?: React.CSSProperties;
}

export const Chip: React.FC<ChipProps> = ({
  label,
  onRemove,
  onClick,
  selected = false,
  style,
}) => {
  return (
    <span
      onClick={onClick}
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 6,
        padding: "2px 8px",
        borderRadius: "var(--cs-radius-control)",
        fontSize: "var(--cs-text-xs)",
        fontWeight: 500,
        backgroundColor: selected ? "var(--cs-selection)" : "var(--cs-bg-raised)",
        color: selected ? "var(--cs-accent)" : "var(--cs-fg-primary)",
        border: selected ? "1px solid var(--cs-accent)" : "1px solid var(--cs-border-subtle)",
        cursor: onClick ? "pointer" : "default",
        userSelect: "none",
        ...style,
      }}
    >
      <span>{label}</span>
      {onRemove && (
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onRemove();
          }}
          aria-label={`Gỡ ${label}`}
          style={{
            background: "none",
            border: "none",
            padding: 0,
            margin: 0,
            cursor: "pointer",
            color: "var(--cs-fg-muted)",
            fontSize: 12,
            lineHeight: 1,
            display: "inline-flex",
            alignItems: "center",
            justifyContent: "center",
          }}
        >
          ×
        </button>
      )}
    </span>
  );
};
