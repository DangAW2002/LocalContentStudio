import React from "react";
import { Button, Badge, Dialog } from "@studio/design-system";
import type { ArtifactCheckResult } from "../../../api/types";
import { getCheckErrorLabel } from "../../../runtime/check-error-labels";

export interface CheckResultDialogProps {
  open: boolean;
  onClose: () => void;
  checkResult: ArtifactCheckResult | null;
  onAskAgentToFix?: () => void;
}

export const CheckResultDialog: React.FC<CheckResultDialogProps> = ({
  open,
  onClose,
  checkResult,
  onAskAgentToFix,
}) => {
  if (!open || !checkResult) return null;

  return (
    <Dialog
      title="Kết quả kiểm tra tiêu chuẩn Brand & Channel"
      open={open}
      onClose={onClose}
      actions={
        <div style={{ display: "flex", gap: "var(--cs-space-2)", alignItems: "center" }}>
          {!checkResult.passed && onAskAgentToFix && (
            <Button variant="primary" onClick={onAskAgentToFix}>
              Nhờ agent sửa
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
          <Badge variant={checkResult.passed ? "success" : "danger"}>
            {checkResult.passed ? "ĐẠT TOÀN BỘ KÊNH" : "CHƯA ĐẠT TIÊU CHUẨN"}
          </Badge>
          <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
            Kiểm tra theo quy chuẩn của từng kênh phân phối.
          </span>
        </div>

        {Object.entries(checkResult.channels).length === 0 ? (
          <div style={{ fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-muted)" }}>
            Không có dữ liệu kênh kiểm tra.
          </div>
        ) : (
          Object.entries(checkResult.channels).map(([chId, chRes]) => (
            <div
              key={chId}
              style={{
                padding: "var(--cs-space-3)",
                border: "1px solid var(--cs-border-subtle)",
                borderRadius: "var(--cs-radius-control)",
                backgroundColor: "var(--cs-bg-raised)",
                display: "flex",
                flexDirection: "column",
                gap: 6,
              }}
            >
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                <span style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)" }}>
                  Kênh: {chId}
                </span>
                <Badge variant={chRes.passed ? "success" : "danger"}>
                  {chRes.passed ? "Đạt" : "Không đạt"}
                </Badge>
              </div>

              {chRes.findings && chRes.findings.length > 0 ? (
                <div style={{ display: "flex", flexDirection: "column", gap: 6, marginTop: 4 }}>
                  {chRes.findings.map((f, fIdx) => (
                    <div
                      key={fIdx}
                      style={{
                        padding: "6px 8px",
                        backgroundColor: "var(--cs-bg-panel)",
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
                        gap: 2,
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
                        <span style={{ fontWeight: 600 }}>{f.message_vi}</span>
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
              ) : (
                <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-success)" }}>
                  ✓ Không có cảnh báo hay lỗi vi phạm nào.
                </div>
              )}
            </div>
          ))
        )}
      </div>
    </Dialog>
  );
};
