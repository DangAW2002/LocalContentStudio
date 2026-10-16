import React from "react";

export type ButtonVariant = "primary" | "secondary" | "destructive" | "quiet" | "danger-outline";
export type ButtonSize = "md" | "sm";

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  busy?: boolean;
  busyText?: string;
  unavailableReason?: string;
  leftIcon?: React.ReactNode;
  rightIcon?: React.ReactNode;
}

export const Button: React.FC<ButtonProps> = ({
  children,
  variant = "secondary",
  size = "md",
  busy = false,
  busyText = "Đang xử lý...",
  unavailableReason,
  disabled = false,
  leftIcon,
  rightIcon,
  style,
  title,
  ...props
}) => {
  const isDisabled = disabled || busy || Boolean(unavailableReason);
  const effectiveTitle = unavailableReason ? `Không khả dụng: ${unavailableReason}` : title;

  const getVariantStyles = (): React.CSSProperties => {
    switch (variant) {
      case "primary":
        return {
          backgroundColor: "var(--cs-accent)",
          color: "var(--cs-fg-inverse)",
          border: "1px solid var(--cs-accent)",
        };
      case "destructive":
        return {
          backgroundColor: "var(--cs-danger)",
          color: "var(--cs-fg-inverse)",
          border: "1px solid var(--cs-danger)",
        };
      case "danger-outline":
        return {
          backgroundColor: "transparent",
          color: "var(--cs-danger)",
          border: "1px solid var(--cs-danger)",
        };
      case "quiet":
        return {
          backgroundColor: "transparent",
          color: "var(--cs-fg-primary)",
          border: "1px solid transparent",
        };
      case "secondary":
      default:
        return {
          backgroundColor: "var(--cs-bg-panel)",
          color: "var(--cs-fg-primary)",
          border: "1px solid var(--cs-border-subtle)",
        };
    }
  };

  const height = size === "sm" ? "var(--cs-control-height-sm)" : "var(--cs-control-height)";
  const padding = size === "sm" ? "0 var(--cs-space-2)" : "0 var(--cs-space-3)";
  const fontSize = size === "sm" ? "var(--cs-text-xs)" : "var(--cs-text-sm)";

  const baseStyle: React.CSSProperties = {
    display: "inline-flex",
    alignItems: "center",
    justifyContent: "center",
    gap: "var(--cs-space-2)",
    height,
    minHeight: height,
    minWidth: size === "sm" ? "28px" : "var(--cs-hit-target)",
    padding,
    borderRadius: "var(--cs-radius-control)",
    fontFamily: "var(--cs-font-ui)",
    fontSize,
    fontWeight: 500,
    cursor: isDisabled ? "not-allowed" : "pointer",
    opacity: isDisabled ? 0.6 : 1,
    transition: "background-color var(--cs-motion-duration) ease, border-color var(--cs-motion-duration) ease, opacity var(--cs-motion-duration) ease",
    whiteSpace: "nowrap",
    ...getVariantStyles(),
    ...style,
  };

  return (
    <div style={{ display: "inline-flex", flexDirection: "column", gap: 2 }}>
      <button
        disabled={isDisabled}
        aria-busy={busy}
        title={effectiveTitle}
        style={baseStyle}
        {...props}
      >
        {busy ? (
          <span>{busyText}</span>
        ) : (
          <>
            {leftIcon && <span aria-hidden="true" style={{ display: "inline-flex" }}>{leftIcon}</span>}
            {children && <span>{children}</span>}
            {rightIcon && <span aria-hidden="true" style={{ display: "inline-flex" }}>{rightIcon}</span>}
          </>
        )}
      </button>
      {unavailableReason && (
        <span
          style={{
            fontSize: "var(--cs-text-xs)",
            color: "var(--cs-fg-muted)",
          }}
        >
          {unavailableReason}
        </span>
      )}
    </div>
  );
};
