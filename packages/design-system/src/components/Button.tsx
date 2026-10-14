import React from "react";

export type ButtonVariant = "primary" | "secondary" | "destructive" | "quiet";

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  busy?: boolean;
  busyText?: string;
  unavailableReason?: string;
  leftIcon?: React.ReactNode;
  rightIcon?: React.ReactNode;
}

export const Button: React.FC<ButtonProps> = ({
  children,
  variant = "secondary",
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

  const baseStyle: React.CSSProperties = {
    display: "inline-flex",
    alignItems: "center",
    justifyContent: "center",
    gap: "var(--cs-space-2)",
    height: "var(--cs-control-height)",
    minHeight: "var(--cs-control-height)",
    minWidth: "var(--cs-hit-target)",
    padding: "0 var(--cs-space-3)",
    borderRadius: "var(--cs-radius-md)",
    fontFamily: "var(--cs-font-ui)",
    fontSize: "var(--cs-text-sm)",
    fontWeight: 500,
    cursor: isDisabled ? "not-allowed" : "pointer",
    opacity: isDisabled ? 0.6 : 1,
    transition: "background-color var(--cs-motion-duration) ease, opacity var(--cs-motion-duration) ease",
    boxShadow: variant === "primary" ? "0 1px 2px rgba(0,0,0,0.05)" : "none",
    ...getVariantStyles(),
    ...style,
  };

  return (
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
          {leftIcon && <span aria-hidden="true">{leftIcon}</span>}
          <span>{children}</span>
          {rightIcon && <span aria-hidden="true">{rightIcon}</span>}
        </>
      )}
    </button>
  );
};
