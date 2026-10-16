import React from "react";
import { Button } from "./Button";

export interface ErrorStateProps {
  title?: string;
  reason?: string;
  safetyNote?: string;
  correlationId?: string;
  onRetry?: () => void;
  actionText?: string;
  style?: React.CSSProperties;
}

export const ErrorState: React.FC<ErrorStateProps> = ({
  title = "Không thể tải dữ liệu",
  reason = "Đã xảy ra sự cố trong quá trình giao tiếp.",
  safetyNote = "Dữ liệu tệp và dự án của bạn vẫn an toàn trên máy.",
  correlationId,
  onRetry,
  actionText = "Thử lại",
  style,
}) => {
  return (
    <div
      role="alert"
      style={{
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        textAlign: "center",
        padding: "var(--cs-space-8) var(--cs-space-4)",
        backgroundColor: "var(--cs-bg-panel)",
        border: "1px solid var(--cs-border-subtle)",
        borderLeft: "4px solid var(--cs-danger)",
        margin: "var(--cs-space-4) 0",
        ...style,
      }}
    >
      <div
        style={{
          width: 48,
          height: 48,
          borderRadius: "var(--cs-radius-control)",
          backgroundColor: "var(--cs-status-danger-bg)",
          color: "var(--cs-danger)",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          fontSize: 24,
          marginBottom: "var(--cs-space-3)",
          fontWeight: 700,
        }}
      >
        !
      </div>
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
      <p
        style={{
          margin: "0 0 var(--cs-space-2) 0",
          maxWidth: 480,
          fontSize: "var(--cs-text-sm)",
          lineHeight: 1.5,
          color: "var(--cs-fg-secondary)",
        }}
      >
        {reason}
      </p>
      {safetyNote && (
        <p
          style={{
            margin: "0 0 var(--cs-space-4) 0",
            maxWidth: 480,
            fontSize: "var(--cs-text-xs)",
            color: "var(--cs-fg-muted)",
          }}
        >
          {safetyNote}
        </p>
      )}
      {correlationId && (
        <code
          style={{
            marginBottom: "var(--cs-space-4)",
            fontSize: "var(--cs-text-xs)",
            fontFamily: "var(--cs-font-mono)",
            backgroundColor: "var(--cs-bg-raised)",
            padding: "2px 6px",
            border: "1px solid var(--cs-border-subtle)",
          }}
        >
          Mã kiểm soát: {correlationId}
        </code>
      )}
      {onRetry && (
        <Button variant="secondary" onClick={onRetry}>
          {actionText}
        </Button>
      )}
    </div>
  );
};
