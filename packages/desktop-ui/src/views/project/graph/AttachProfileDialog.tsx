import React from "react";
import { Dialog, Button, Select } from "@studio/design-system";

export interface AttachProfileDialogProps {
  open: boolean;
  targetNodeId: string;
  availableProfiles: { id: string; title: string }[];
  onClose: () => void;
  onAttach: (profileId: string) => void;
}

export const AttachProfileDialog: React.FC<AttachProfileDialogProps> = ({
  open,
  targetNodeId,
  availableProfiles,
  onClose,
  onAttach,
}) => {
  const [selectedId, setSelectedId] = React.useState<string>(
    availableProfiles[0]?.id || ""
  );

  React.useEffect(() => {
    if (availableProfiles.length > 0) {
      setSelectedId(availableProfiles[0].id);
    }
  }, [availableProfiles]);

  if (!open) return null;

  return (
    <Dialog open={open} onClose={onClose} title="Gắn hồ sơ tham chiếu">
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-4)" }}>
        <p style={{ margin: 0, fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-secondary)" }}>
          Gắn hồ sơ tham chiếu nhất quán cho node <strong>{targetNodeId}</strong>.
          Cạnh kiểu <code>reference</code> sẽ được tạo trong đồ thị.
        </p>

        {availableProfiles.length === 0 ? (
          <div
            style={{
              padding: "var(--cs-space-3)",
              backgroundColor: "var(--cs-bg-raised)",
              borderRadius: "var(--cs-radius-control)",
              fontSize: "var(--cs-text-sm)",
              color: "var(--cs-fg-muted)",
              textAlign: "center",
            }}
          >
            Chưa có hồ sơ tham chiếu nào trong dự án hoặc series. Bạn có thể chọn "Đặt làm hồ sơ" từ một artifact hoặc asset trước.
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-1)" }}>
            <label style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600, color: "var(--cs-fg-secondary)" }}>
              Chọn hồ sơ
            </label>
            <Select
              size="md"
              value={selectedId}
              onChange={(e) => setSelectedId(e.target.value)}
              options={availableProfiles.map((p) => ({
                value: p.id,
                label: `${p.title} (${p.id})`,
              }))}
            />
          </div>
        )}

        <div style={{ display: "flex", justifyContent: "flex-end", gap: "var(--cs-space-2)" }}>
          <Button type="button" variant="secondary" onClick={onClose}>
            Hủy
          </Button>
          <Button
            type="button"
            variant="primary"
            disabled={!selectedId || availableProfiles.length === 0}
            onClick={() => {
              if (selectedId) {
                onAttach(selectedId);
                onClose();
              }
            }}
          >
            Gắn hồ sơ
          </Button>
        </div>
      </div>
    </Dialog>
  );
};
