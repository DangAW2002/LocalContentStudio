import React from "react";

export interface PlaybookDetailProps {
  rawText: string | null;
}

export const PlaybookDetail: React.FC<PlaybookDetailProps> = ({ rawText }) => {
  return (
    <div style={cardSectionStyle}>
      <div style={sectionTitleStyle}>NỘI DUNG QUY TRÌNH (PLAYBOOK MARKDOWN)</div>
      <pre
        style={{
          margin: 0,
          padding: "var(--cs-space-4)",
          backgroundColor: "var(--cs-bg-raised)",
          border: "1px solid var(--cs-border-subtle)",
          borderRadius: "var(--cs-radius-control)",
          fontFamily: "var(--cs-font-mono)",
          fontSize: 12,
          lineHeight: 1.6,
          overflowX: "auto",
          whiteSpace: "pre-wrap",
        }}
      >
        {rawText || "Đang tải quy trình..."}
      </pre>
    </div>
  );
};

const cardSectionStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 8,
  padding: "var(--cs-space-4)",
  backgroundColor: "var(--cs-bg-panel)",
  border: "1px solid var(--cs-border-subtle)",
  borderRadius: "var(--cs-radius-control)",
};

const sectionTitleStyle: React.CSSProperties = {
  fontSize: "var(--cs-text-xs)",
  fontWeight: 700,
  color: "var(--cs-fg-secondary)",
  letterSpacing: 0.5,
};
