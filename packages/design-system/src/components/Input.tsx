import React from "react";

export interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
  hint?: string;
}

export const Input: React.FC<InputProps> = ({
  id,
  label,
  error,
  hint,
  style,
  ...props
}) => {
  const generatedId = React.useId();
  const inputId = id || generatedId;
  const errorId = `${inputId}-error`;
  const hintId = `${inputId}-hint`;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-1)", width: "100%" }}>
      {label && (
        <label
          htmlFor={inputId}
          style={{
            fontSize: "var(--cs-text-sm)",
            fontWeight: 500,
            color: "var(--cs-fg-primary)",
          }}
        >
          {label}
        </label>
      )}
      <input
        id={inputId}
        aria-describedby={error ? errorId : hint ? hintId : undefined}
        aria-invalid={Boolean(error)}
        style={{
          height: "var(--cs-control-height)",
          padding: "0 var(--cs-space-3)",
          borderRadius: "var(--cs-radius-md)",
          border: error ? "1px solid var(--cs-danger)" : "1px solid var(--cs-border-subtle)",
          backgroundColor: "var(--cs-bg-panel)",
          color: "var(--cs-fg-primary)",
          fontFamily: "var(--cs-font-ui)",
          fontSize: "var(--cs-text-md)",
          outline: "none",
          ...style,
        }}
        {...props}
      />
      {error && (
        <span
          id={errorId}
          role="alert"
          style={{
            fontSize: "var(--cs-text-xs)",
            color: "var(--cs-danger)",
            fontWeight: 500,
          }}
        >
          {error}
        </span>
      )}
      {!error && hint && (
        <span
          id={hintId}
          style={{
            fontSize: "var(--cs-text-xs)",
            color: "var(--cs-fg-secondary)",
          }}
        >
          {hint}
        </span>
      )}
    </div>
  );
};
