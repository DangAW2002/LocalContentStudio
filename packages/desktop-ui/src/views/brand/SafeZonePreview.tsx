import React from "react";
import type { ChannelFormat } from "../../api/types";

export interface SafeZonePreviewProps {
  format: ChannelFormat;
}

/** One channel format drawn to scale, with its safe zone inset by the real margins. */
export const SafeZonePreview: React.FC<SafeZonePreviewProps> = ({ format }) => {
  const BOX = 180;
  const ratio = format.width / Math.max(1, format.height);
  const w = ratio >= 1 ? BOX : Math.round(BOX * ratio);
  const h = ratio >= 1 ? Math.round(BOX / ratio) : BOX;
  const z = format.safe_zone;
  const pct = (v: number) => `${Math.round(v * 100)}%`;
  const margins = `Lề an toàn — trên ${pct(z.top)}, dưới ${pct(z.bottom)}, trái ${pct(z.left)}, phải ${pct(z.right)}`;

  return (
    <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 6, maxWidth: BOX }}>
      <div
        title={margins}
        data-testid="safe-zone-box"
        style={{
          width: w,
          height: h,
          backgroundColor: "#1e293b",
          border: "1px solid var(--cs-border-subtle)",
          borderRadius: "var(--cs-radius-control)",
          position: "relative",
          overflow: "hidden",
        }}
      >
        <div
          data-testid="safe-zone-overlay"
          style={{
            position: "absolute",
            top: pct(z.top),
            bottom: pct(z.bottom),
            left: pct(z.left),
            right: pct(z.right),
            border: "1px dashed #60a5fa",
            backgroundColor: "rgba(59, 130, 246, 0.12)",
          }}
        />
      </div>
      <span style={{ fontSize: 11, fontWeight: 600, color: "var(--cs-fg-secondary)", textAlign: "center" }}>{format.name}</span>
      <span style={{ fontSize: 11, color: "var(--cs-fg-muted)", textAlign: "center" }}>
        {format.aspect} · {format.width}×{format.height}
      </span>
      <span style={{ fontSize: 10, color: "var(--cs-fg-muted)", textAlign: "center", lineHeight: 1.4 }}>
        Lề trên {pct(z.top)} · dưới {pct(z.bottom)}
        <br />
        trái {pct(z.left)} · phải {pct(z.right)}
      </span>
    </div>
  );
};
