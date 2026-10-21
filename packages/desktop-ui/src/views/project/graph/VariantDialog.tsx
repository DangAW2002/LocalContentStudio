import React from "react";
import { Button, Dialog, Input } from "@studio/design-system";

export interface VariantDialogProps {
  open: boolean;
  sourceId: string;
  onClose: () => void;
  onCreate: (newId: string, title?: string) => Promise<void>;
}

export const VariantDialog: React.FC<VariantDialogProps> = ({
  open,
  sourceId,
  onClose,
  onCreate,
}) => {
  const [newId, setNewId] = React.useState("");
  const [title, setTitle] = React.useState("");
  const [creating, setCreating] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    if (open) {
      setNewId(`${sourceId}-var`);
      setTitle("");
      setError(null);
    }
  }, [open, sourceId]);

  const handleConfirm = async () => {
    if (!newId.trim()) {
      setError("Vui lòng nhập ID cho biến thể.");
      return;
    }
    setCreating(true);
    setError(null);
    try {
      await onCreate(newId.trim(), title.trim() || undefined);
      onClose();
    } catch (err: any) {
      setError(err?.message || "Không thể tạo biến thể.");
    } finally {
      setCreating(false);
    }
  };

  return (
    <Dialog
      title={`Tạo biến thể từ: ${sourceId}`}
      open={open}
      onClose={onClose}
      actions={
        <>
          <Button variant="secondary" onClick={onClose} disabled={creating}>
            Hủy
          </Button>
          <Button variant="primary" busy={creating} busyText="Đang tạo..." onClick={handleConfirm}>
            Tạo biến thể
          </Button>
        </>
      }
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
        <p style={{ margin: 0, fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-secondary)" }}>
          Biến thể mới sẽ tự động liên kết với <code>{sourceId}</code> qua vai trò <code>variant_of</code>:
        </p>
        <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          <label style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600, color: "var(--cs-fg-primary)" }}>
            Mã định danh (ID):
          </label>
          <Input
            value={newId}
            onChange={(e: any) => setNewId(e.target.value)}
            placeholder="vd: thumb-01-9x16"
          />
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          <label style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600, color: "var(--cs-fg-primary)" }}>
            Tiêu đề hiển thị (tùy chọn):
          </label>
          <Input
            value={title}
            onChange={(e: any) => setTitle(e.target.value)}
            placeholder="vd: Thumbnail 9:16 Shorts"
          />
        </div>
        {error && (
          <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-danger)" }}>
            {error}
          </div>
        )}
      </div>
    </Dialog>
  );
};
