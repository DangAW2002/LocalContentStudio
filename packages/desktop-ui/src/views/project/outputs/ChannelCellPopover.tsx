import React from "react";
import { Button, Badge, Dialog } from "@studio/design-system";
import type { ArtifactChannelMatrixCell, CheckFinding } from "../../../api/types";
import { getCheckErrorLabel } from "../../../runtime/check-error-labels";

export interface ChannelCellPopoverProps {
  open: boolean;
  onClose: () => void;
  artifactTitle: string;
  sourcePath: string;
  cell: ArtifactChannelMatrixCell | null;
  onAskAgentToFix?: (promptText: string) => void;
  onRecheck?: () => void;
  onRender?: () => void;
}

export const ChannelCellPopover: React.FC<ChannelCellPopoverProps> = ({
  open,
  onClose,
  artifactTitle,
  sourcePath,
  cell,
  onAskAgentToFix,
  onRecheck,
  onRender,
}) => {
  if (!open || !cell) return null;

  const handleAskFix = () => {
    if (!onAskAgentToFix) return;
    const failedLines: string[] = [];

    for (const f of cell.findings) {
      if (f.severity === "error" || f.severity === "warn") {
        let line = `- [Kênh ${cell.channel_name}] [${getCheckErrorLabel(f.code)}] ${f.message_vi}`;
        if (f.fix_hint_vi) {
          line += ` (Gợi ý: ${f.fix_hint_vi})`;
        }
        if (f.location) {
          line += ` tại ${f.location}`;
        }
        failedLines.push(line);
      }
    }

    const promptText = `Hãy sửa artifact @${sourcePath} để vượt qua các lỗi kiểm tra kênh ${cell.channel_name} sau, sau đó re-render và re-check:\n${failedLines.join("\n")}`;
    onAskAgentToFix(promptText);
    onClose();
  };

  const statusTitle =
    cell.status === "passed"
      ? "Đạt tiêu chuẩn kênh"
      : cell.status === "warning"
      ? "Có cảnh báo tiêu chuẩn"
      : cell.status === "error"
      ? "Chưa đạt tiêu chuẩn kênh"
      : "Chưa có bản render";

  return (
    <Dialog
      title={`${artifactTitle} × ${cell.channel_name}`}
      open={open}
      onClose={onClose}
      actions={
        <div style={{ display: "flex", gap: "var(--cs-space-2)", alignItems: "center" }}>
          {cell.status === "unrendered" && onRender && (
            <Button variant="primary" onClick={onRender}>
              Render ngay
            </Button>
          )}
          {(cell.status === "error" || cell.status === "warning") && onAskAgentToFix && (
            <Button variant="primary" onClick={handleAskFix}>
              Nhờ agent sửa
            </Button>
          )}
          {cell.status !== "unrendered" && onRecheck && (
            <Button variant="secondary" onClick={onRecheck}>
              Kiểm tra lại
            </Button>
          )}
          <Button variant="secondary" onClick={onClose}>
            Đóng
          </Button>
        </div>
      }
    >
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)", maxHeight: 420, overflowY: "auto" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <Badge
            variant={
              cell.status === "passed"
                ? "success"
                : cell.status === "warning"
                ? "warning"
                : cell.status === "error"
                ? "danger"
                : "info"
            }
          >
            {statusTitle.toUpperCase()}
          </Badge>
          {cell.render_hash && (
            <span style={{ fontSize: 11, color: "var(--cs-fg-muted)", fontFamily: "var(--cs-font-mono)" }}>
              Hash: {cell.render_hash.slice(0, 8)}
            </span>
          )}
        </div>

        {cell.status === "unrendered" ? (
          <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)", lineHeight: 1.5 }}>
            Artifact này chưa có bản render nào để kiểm tra tiêu chuẩn. Hãy bấm &quot;Render ngay&quot; hoặc render artifact trên Stage trước khi kiểm tra kênh.
          </div>
        ) : cell.findings.length === 0 ? (
          <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-success)" }}>
            ✓ Mọi tiêu chuẩn định dạng, vùng an toàn và thương hiệu của kênh {cell.channel_name} đều đạt.
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {cell.findings.map((f: CheckFinding, idx: number) => (
              <div
                key={idx}
                style={{
                  padding: "8px 10px",
                  backgroundColor: "var(--cs-bg-raised)",
                  borderRadius: "var(--cs-radius-control)",
                  borderLeft: `3px solid ${
                    f.severity === "error"
                      ? "var(--cs-danger)"
                      : f.severity === "warn"
                      ? "var(--cs-warning)"
                      : "var(--cs-accent)"
                  }`,
                  fontSize: "var(--cs-text-xs)",
                  display: "flex",
                  flexDirection: "column",
                  gap: 4,
                }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap" }}>
                  <Badge
                    variant={
                      f.severity === "error"
                        ? "danger"
                        : f.severity === "warn"
                        ? "warning"
                        : "info"
                    }
                  >
                    {getCheckErrorLabel(f.code)}
                  </Badge>
                  <span style={{ fontWeight: 600, color: "var(--cs-fg-primary)" }}>{f.message_vi}</span>
                </div>
                {f.fix_hint_vi && (
                  <div style={{ color: "var(--cs-fg-secondary)", marginTop: 2 }}>
                    💡 Gợi ý khắc phục: {f.fix_hint_vi}
                  </div>
                )}
                {f.location && (
                  <div style={{ color: "var(--cs-fg-muted)", fontSize: 11 }}>
                    Vị trí: <code>{f.location}</code>
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </Dialog>
  );
};
