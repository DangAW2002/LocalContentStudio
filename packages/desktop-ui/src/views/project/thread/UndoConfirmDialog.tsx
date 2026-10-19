import React from "react";
import { Badge, Button, Dialog } from "@studio/design-system";
import type { ThreadMessage } from "../../../runtime/chat/turn-reducer";

export interface UndoConfirmDialogProps {
  targetTurnId: string | null;
  messages: ThreadMessage[];
  undoing: boolean;
  onClose: () => void;
  onConfirm: (turnId: string) => void;
}

export const UndoConfirmDialog: React.FC<UndoConfirmDialogProps> = ({
  targetTurnId,
  messages,
  undoing,
  onClose,
  onConfirm,
}) => {
  if (!targetTurnId) return null;
  const targetMsg = messages.find((m) => m.turnId === targetTurnId);
  const files = targetMsg?.fileChanges || [];

  return (
    <Dialog
      title="Xác nhận hoàn tác lượt này?"
      open={Boolean(targetTurnId)}
      onClose={onClose}
      actions={
        <>
          <Button variant="secondary" onClick={onClose} disabled={undoing}>
            Hủy
          </Button>
          <Button
            variant="destructive"
            busy={undoing}
            busyText="Đang hoàn tác..."
            onClick={() => onConfirm(targetTurnId)}
          >
            Hoàn tác lượt
          </Button>
        </>
      }
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
        <p style={{ margin: 0, fontSize: "var(--cs-text-sm)", lineHeight: 1.5 }}>
          Thao tác này sẽ khôi phục lại toàn bộ tệp bị thay đổi trong lượt agent này về điểm kiểm soát (checkpoint) ngay trước đó.
        </p>

        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          <span style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600, color: "var(--cs-fg-secondary)" }}>
            Các tệp sẽ được khôi phục ({files.length}):
          </span>
          {files.length > 0 ? (
            <div
              style={{
                maxHeight: 160,
                overflowY: "auto",
                backgroundColor: "var(--cs-bg-raised)",
                border: "1px solid var(--cs-border-subtle)",
                borderRadius: "var(--cs-radius-control)",
                padding: "6px 10px",
                display: "flex",
                flexDirection: "column",
                gap: 4,
              }}
            >
              {files.map((fc, i) => (
                <div key={i} style={{ display: "flex", alignItems: "center", gap: 6, fontSize: "var(--cs-text-xs)" }}>
                  <Badge
                    variant={
                      fc.op === "create"
                        ? "success"
                        : fc.op === "delete"
                        ? "danger"
                        : "info"
                    }
                  >
                    {fc.op === "create" ? "Tạo mới" : fc.op === "delete" ? "Xóa" : "Sửa đổi"}
                  </Badge>
                  <code style={{ fontFamily: "var(--cs-font-mono)", color: "var(--cs-fg-primary)" }}>
                    {fc.path}
                  </code>
                </div>
              ))}
            </div>
          ) : (
            <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)", fontStyle: "italic" }}>
              Tất cả các tệp source bị thay đổi bởi agent trong lượt này sẽ được hoàn tác.
            </div>
          )}
        </div>
      </div>
    </Dialog>
  );
};
