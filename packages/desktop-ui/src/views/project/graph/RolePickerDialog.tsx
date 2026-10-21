import React from "react";
import { Button, Dialog } from "@studio/design-system";

export interface RolePickerDialogProps {
  open: boolean;
  sourceId: string;
  targetId: string;
  onClose: () => void;
  onSelectRole: (role: "uses" | "reference" | "variant_of") => Promise<void>;
}

export const RolePickerDialog: React.FC<RolePickerDialogProps> = ({
  open,
  sourceId,
  targetId,
  onClose,
  onSelectRole,
}) => {
  const [submitting, setSubmitting] = React.useState(false);

  const handleChoose = async (role: "uses" | "reference" | "variant_of") => {
    setSubmitting(true);
    try {
      await onSelectRole(role);
      onClose();
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Dialog
      title="Chọn vai trò liên kết"
      open={open}
      onClose={onClose}
      actions={
        <Button variant="secondary" onClick={onClose} disabled={submitting}>
          Hủy
        </Button>
      }
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
        <p style={{ margin: 0, fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-secondary)" }}>
          Liên kết từ <strong>{sourceId}</strong> tới <strong>{targetId}</strong>:
        </p>

        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          <button
            type="button"
            className="cs-menu-item"
            style={{
              padding: "10px 12px",
              display: "flex",
              flexDirection: "column",
              alignItems: "flex-start",
              border: "1px solid var(--cs-border-subtle)",
              borderRadius: "var(--cs-radius-control)",
              background: "var(--cs-bg-panel)",
              cursor: "pointer",
              textAlign: "left",
            }}
            onClick={() => handleChoose("uses")}
            disabled={submitting}
          >
            <span style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-primary)" }}>
              uses (Phụ thuộc trực tiếp)
            </span>
            <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
              Đầu ra phụ thuộc vào đầu vào này. Khi nguồn đổi, node này sẽ bị đánh dấu lỗi thời.
            </span>
          </button>

          <button
            type="button"
            className="cs-menu-item"
            style={{
              padding: "10px 12px",
              display: "flex",
              flexDirection: "column",
              alignItems: "flex-start",
              border: "1px solid var(--cs-border-subtle)",
              borderRadius: "var(--cs-radius-control)",
              background: "var(--cs-bg-panel)",
              cursor: "pointer",
              textAlign: "left",
            }}
            onClick={() => handleChoose("reference")}
            disabled={submitting}
          >
            <span style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-primary)" }}>
              reference (Hồ sơ tham chiếu)
            </span>
            <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
              Dùng để đối chiếu tính nhất quán (nhân vật, phong cách, giọng đọc). Không tự động lan truyền lỗi thời.
            </span>
          </button>

          <button
            type="button"
            className="cs-menu-item"
            style={{
              padding: "10px 12px",
              display: "flex",
              flexDirection: "column",
              alignItems: "flex-start",
              border: "1px solid var(--cs-border-subtle)",
              borderRadius: "var(--cs-radius-control)",
              background: "var(--cs-bg-panel)",
              cursor: "pointer",
              textAlign: "left",
            }}
            onClick={() => handleChoose("variant_of")}
            disabled={submitting}
          >
            <span style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-primary)" }}>
              variant_of (Biến thể)
            </span>
            <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
              Node này là biến thể (kích thước khác, ngôn ngữ khác, cắt ngắn) của node gốc.
            </span>
          </button>
        </div>
      </div>
    </Dialog>
  );
};
