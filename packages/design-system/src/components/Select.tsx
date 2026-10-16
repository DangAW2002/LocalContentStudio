import React from "react";

export interface SelectOption {
  value: string;
  label: string;
  disabled?: boolean;
}

export interface SelectProps extends Omit<React.SelectHTMLAttributes<HTMLSelectElement>, "size"> {
  label?: string;
  options: SelectOption[];
  hint?: string;
  error?: string;
  size?: "md" | "sm";
}

export const Select: React.FC<SelectProps> = ({
  id,
  label,
  options,
  hint,
  error,
  size = "md",
  style,
  ...props
}) => {
  const generatedId = React.useId();
  const selectId = id || generatedId;
  const height = size === "sm" ? "var(--cs-control-height-sm)" : "var(--cs-control-height)";
  const fontSize = size === "sm" ? "var(--cs-text-xs)" : "var(--cs-text-sm)";

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4, width: "100%" }}>
      {label && (
        <label
          htmlFor={selectId}
          style={{
            fontSize: "var(--cs-text-xs)",
            fontWeight: 600,
            color: "var(--cs-fg-secondary)",
          }}
        >
          {label}
        </label>
      )}
      <select
        id={selectId}
        style={{
          height,
          padding: "0 var(--cs-space-2)",
          borderRadius: "var(--cs-radius-control)",
          border: error ? "1px solid var(--cs-danger)" : "1px solid var(--cs-border-subtle)",
          backgroundColor: "var(--cs-bg-panel)",
          color: "var(--cs-fg-primary)",
          fontFamily: "var(--cs-font-ui)",
          fontSize,
          outline: "none",
          cursor: "pointer",
          ...style,
        }}
        {...props}
      >
        {options.map((opt) => (
          <option key={opt.value} value={opt.value} disabled={opt.disabled}>
            {opt.label}
          </option>
        ))}
      </select>
      {error && (
        <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-danger)" }}>
          {error}
        </span>
      )}
      {!error && hint && (
        <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
          {hint}
        </span>
      )}
    </div>
  );
};
