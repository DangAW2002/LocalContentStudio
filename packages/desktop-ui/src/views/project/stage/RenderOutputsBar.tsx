import React from "react";
import { Icon } from "../../../components/Icon";
import { Button } from "@studio/design-system";

export interface RenderOutputsBarProps {
  outputs: string[];
  cached?: boolean;
  onSelectFile?: (path: string) => void;
  onClose: () => void;
}

/** Check whether a file is an image, video or audio based on path extension */
function isMediaOutput(path: string): boolean {
  const p = path.toLowerCase();
  return (
    p.endsWith(".png") ||
    p.endsWith(".jpg") ||
    p.endsWith(".jpeg") ||
    p.endsWith(".webp") ||
    p.endsWith(".mp4") ||
    p.endsWith(".webm") ||
    p.endsWith(".mov") ||
    p.endsWith(".pdf") ||
    p.endsWith(".mp3") ||
    p.endsWith(".wav")
  );
}

export const RenderOutputsBar: React.FC<RenderOutputsBarProps> = ({
  outputs,
  cached,
  onSelectFile,
  onClose,
}) => {
  return (
    <div
      style={{
        padding: "6px var(--cs-space-3)",
        backgroundColor: "var(--cs-bg-raised)",
        borderBottom: "1px solid var(--cs-border-subtle)",
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        fontSize: "var(--cs-text-xs)",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <span style={{ fontWeight: 600, color: "var(--cs-success)" }}>
          ✓ Render xong ({cached ? "từ cache" : "mới"}):
        </span>
        <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
          {outputs.map((outPath) => (
            <button
              key={outPath}
              type="button"
              onClick={() => {
                if (isMediaOutput(outPath) && onSelectFile) {
                  onSelectFile(outPath);
                }
              }}
              style={{
                padding: "2px 6px",
                borderRadius: "var(--cs-radius-control)",
                backgroundColor: "var(--cs-bg-panel)",
                border: "1px solid var(--cs-border-subtle)",
                fontSize: 11,
                fontFamily: "var(--cs-font-mono)",
                cursor: onSelectFile ? "pointer" : "default",
                color: "var(--cs-accent)",
                display: "inline-flex",
                alignItems: "center",
                gap: 4,
              }}
              title="Bấm để xem output trên Stage"
            >
              <Icon name="file" size={11} />
              <span>{outPath}</span>
            </button>
          ))}
        </div>
      </div>
      <Button size="sm" variant="quiet" onClick={onClose}>
        Đóng
      </Button>
    </div>
  );
};
