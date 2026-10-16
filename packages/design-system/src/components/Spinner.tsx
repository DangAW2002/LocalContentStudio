import React from "react";

export interface SpinnerProps {
  size?: number;
  color?: string;
  label?: string;
}

export const Spinner: React.FC<SpinnerProps> = ({
  size = 16,
  color = "var(--cs-accent)",
  label = "Đang tải...",
}) => {
  return (
    <span
      role="status"
      aria-label={label}
      style={{
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        width: size,
        height: size,
      }}
    >
      <svg
        width={size}
        height={size}
        viewBox="0 0 24 24"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        style={{
          animation: "cs-spin 0.8s linear infinite",
        }}
      >
        <style>{`
          @keyframes cs-spin {
            from { transform: rotate(0deg); }
            to { transform: rotate(360deg); }
          }
        `}</style>
        <circle
          cx="12"
          cy="12"
          r="10"
          stroke="var(--cs-border-subtle)"
          strokeWidth="3"
        />
        <path
          d="M12 2C6.47715 2 2 6.47715 2 12"
          stroke={color}
          strokeWidth="3"
          strokeLinecap="round"
        />
      </svg>
    </span>
  );
};
