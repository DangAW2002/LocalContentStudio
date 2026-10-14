import React from "react";
import { Button } from "./Button";

export interface DialogProps {
  open: boolean;
  title: string;
  onClose: () => void;
  children: React.ReactNode;
  footer?: React.ReactNode;
}

export const Dialog: React.FC<DialogProps> = ({
  open,
  title,
  onClose,
  children,
  footer,
}) => {
  React.useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && open) {
        onClose();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      role="presentation"
      style={{
        position: "fixed",
        inset: 0,
        backgroundColor: "rgba(0, 0, 0, 0.45)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 1000,
        backdropFilter: "blur(2px)",
      }}
      onClick={onClose}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="dialog-title"
        style={{
          width: "100%",
          maxWidth: "520px",
          backgroundColor: "var(--cs-bg-panel)",
          borderRadius: "var(--cs-radius-md)",
          boxShadow: "var(--cs-shadow-panel)",
          border: "1px solid var(--cs-border-subtle)",
          overflow: "hidden",
          display: "flex",
          flexDirection: "column",
          maxHeight: "90vh",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <div
          style={{
            padding: "var(--cs-space-4)",
            borderBottom: "1px solid var(--cs-border-subtle)",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
          }}
        >
          <h2
            id="dialog-title"
            style={{
              margin: 0,
              fontSize: "var(--cs-text-lg)",
              fontWeight: 600,
              color: "var(--cs-fg-primary)",
            }}
          >
            {title}
          </h2>
          <Button variant="quiet" onClick={onClose} aria-label="Đóng hộp thoại">
            ✕
          </Button>
        </div>

        <div style={{ padding: "var(--cs-space-4)", overflowY: "auto", flex: 1 }}>
          {children}
        </div>

        {footer && (
          <div
            style={{
              padding: "var(--cs-space-3) var(--cs-space-4)",
              borderTop: "1px solid var(--cs-border-subtle)",
              backgroundColor: "var(--cs-bg-raised)",
              display: "flex",
              justifyContent: "flex-end",
              gap: "var(--cs-space-2)",
            }}
          >
            {footer}
          </div>
        )}
      </div>
    </div>
  );
};
