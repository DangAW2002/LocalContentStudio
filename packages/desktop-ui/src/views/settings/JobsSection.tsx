import React from "react";

export const JobsSection: React.FC = () => {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-4)" }}>
      <div
        style={{
          padding: "var(--cs-space-4)",
          backgroundColor: "var(--cs-bg-panel)",
          border: "1px solid var(--cs-border-subtle)",
          display: "flex",
          flexDirection: "column",
          gap: "var(--cs-space-3)",
        }}
      >
        <h2 style={{ margin: 0, fontSize: "var(--cs-text-md)", fontWeight: 700 }}>
          Pool Tài nguyên & Giới hạn Tiến trình (ADR-009)
        </h2>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--cs-space-4)" }}>
          <div style={{ padding: "var(--cs-space-3)", backgroundColor: "var(--cs-bg-raised)", border: "1px solid var(--cs-border-subtle)" }}>
            <div style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)" }}>Engine Pool (Claude / agy)</div>
            <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", marginTop: 4 }}>
              Tối đa 1 tiến trình CLI đồng thời để bảo vệ bộ nhớ và token budget.
            </div>
            <div style={{ marginTop: 8, fontSize: "var(--cs-text-xs)", color: "var(--cs-accent)", fontWeight: 600 }}>
              Giới hạn: 1 active worker
            </div>
          </div>

          <div style={{ padding: "var(--cs-space-3)", backgroundColor: "var(--cs-bg-raised)", border: "1px solid var(--cs-border-subtle)" }}>
            <div style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)" }}>Render & FFmpeg Heavy Pool</div>
            <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", marginTop: 4 }}>
              Tác vụ dựng video, xuất PNG nặng được giám sát qua Windows Job Object.
            </div>
            <div style={{ marginTop: 8, fontSize: "var(--cs-text-xs)", color: "var(--cs-accent)", fontWeight: 600 }}>
              Giới hạn: 2 concurrent tasks · Timeout: 300s
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
