import React from "react";

export interface KbdProps {
  children: React.ReactNode;
  style?: React.CSSProperties;
}

export const Kbd: React.FC<KbdProps> = ({ children, style }) => {
  return (
    <kbd
      style={{
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        padding: "1px 5px",
        minWidth: 18,
        height: 18,
        fontSize: "var(--cs-text-xs)",
        fontFamily: "var(--cs-font-mono)",
        fontWeight: 500,
        backgroundColor: "var(--cs-bg-panel)",
        color: "var(--cs-fg-secondary)",
        border: "1px solid var(--cs-border-strong)",
        borderRadius: "var(--cs-radius-control)",
        boxShadow: "0 1px 0 var(--cs-border-strong)",
        lineHeight: 1,
        userSelect: "none",
        ...style,
      }}
    >
      {children}
    </kbd>
  );
};
