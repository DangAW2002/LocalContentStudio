import React from "react";
import { Button, Dialog, Input } from "@studio/design-system";

export interface NotesDialogProps {
  open: boolean;
  artifactId: string;
  initialNotes: string[];
  onClose: () => void;
  onSave: (notes: string[]) => Promise<void>;
}

export const NotesDialog: React.FC<NotesDialogProps> = ({
  open,
  artifactId,
  initialNotes,
  onClose,
  onSave,
}) => {
  const [text, setText] = React.useState("");
  const [saving, setSaving] = React.useState(false);

  React.useEffect(() => {
    if (open) {
      setText(initialNotes.join("\n"));
    }
  }, [open, initialNotes]);

  const handleConfirm = async () => {
    setSaving(true);
    try {
      const notes = text
        .split("\n")
        .map((s) => s.trim())
        .filter(Boolean);
      await onSave(notes);
      onClose();
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog
      title={`Ghi chú cho artifact: ${artifactId}`}
      open={open}
      onClose={onClose}
      actions={
        <>
          <Button variant="secondary" onClick={onClose} disabled={saving}>
            Hủy
          </Button>
          <Button variant="primary" busy={saving} busyText="Đang lưu..." onClick={handleConfirm}>
            Lưu ghi chú
          </Button>
        </>
      }
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        <p style={{ margin: 0, fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-secondary)" }}>
          Mỗi dòng là một ghi chú. Các ghi chú này sẽ được gửi tóm tắt cho Agent trong các lượt hội thoại:
        </p>
        <textarea
          value={text}
          onChange={(e) => setText(e.target.value)}
          rows={6}
          placeholder="Ví dụ: Giữ nguyên câu mở đầu&#10;Dùng màu nền tương phản cao..."
          style={{
            width: "100%",
            boxSizing: "border-box",
            padding: "8px 12px",
            background: "var(--cs-bg-input)",
            color: "var(--cs-fg-primary)",
            border: "1px solid var(--cs-border-subtle)",
            borderRadius: "var(--cs-radius-control)",
            fontSize: "var(--cs-text-sm)",
            fontFamily: "var(--cs-font-ui)",
            resize: "vertical",
          }}
        />
      </div>
    </Dialog>
  );
};
