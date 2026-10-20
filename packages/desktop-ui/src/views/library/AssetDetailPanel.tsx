import React from "react";
import { Icon } from "../../components/Icon";
import { Badge, Button, Dialog } from "@studio/design-system";
import type { AssetItem } from "../../api/types";

interface AssetDetailPanelProps {
  asset: AssetItem | null;
  onClose: () => void;
  onUseInThread: (asset: AssetItem) => void;
  onOpenFolder: (asset: AssetItem) => void;
  onDeleteAsset: (asset: AssetItem) => void;
}

export const AssetDetailPanel: React.FC<AssetDetailPanelProps> = ({
  asset,
  onClose,
  onUseInThread,
  onOpenFolder,
  onDeleteAsset,
}) => {
  const [copiedPrompt, setCopiedPrompt] = React.useState(false);
  const [copiedPath, setCopiedPath] = React.useState(false);
  const [confirmDeleteOpen, setConfirmDeleteOpen] = React.useState(false);

  if (!asset) return null;

  const handleCopyPrompt = async () => {
    if (!asset.meta?.prompt) return;
    try {
      await navigator.clipboard.writeText(asset.meta.prompt);
      setCopiedPrompt(true);
      setTimeout(() => setCopiedPrompt(false), 2000);
    } catch {
      // Fallback
    }
  };

  const handleCopyPath = async () => {
    try {
      await navigator.clipboard.writeText(asset.path);
      setCopiedPath(true);
      setTimeout(() => setCopiedPath(false), 2000);
    } catch {
      // Fallback
    }
  };

  const sourceLabel =
    asset.source === "generated"
      ? "AI sinh"
      : asset.source === "imported"
      ? "Đã nhập"
      : "Đã tải lên";

  const sourceVariant =
    asset.source === "generated"
      ? "warning"
      : asset.source === "imported"
      ? "info"
      : "neutral";

  const formatSize = (bytes: number) => {
    if (bytes >= 1024 * 1024) {
      return `${(bytes / (1024 * 1024)).toFixed(2)} MB (${bytes.toLocaleString()} bytes)`;
    }
    return `${(bytes / 1024).toFixed(1)} KB (${bytes.toLocaleString()} bytes)`;
  };

  return (
    <>
      <div
        style={{
          width: 360,
          borderLeft: "1px solid var(--cs-border-subtle)",
          backgroundColor: "var(--cs-bg-panel)",
          display: "flex",
          flexDirection: "column",
          height: "100%",
          flexShrink: 0,
          overflowY: "auto",
        }}
      >
        {/* Header */}
        <div
          style={{
            padding: "var(--cs-space-3) var(--cs-space-4)",
            borderBottom: "1px solid var(--cs-border-subtle)",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 8, minWidth: 0 }}>
            <span style={{ fontWeight: 700, fontSize: "var(--cs-text-sm)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {asset.name}
            </span>
          </div>
          <button
            type="button"
            onClick={onClose}
            style={{
              background: "transparent",
              border: "none",
              color: "var(--cs-fg-muted)",
              cursor: "pointer",
              padding: 4,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
            }}
            title="Đóng chi tiết"
          >
            <Icon name="x" size={16} />
          </button>
        </div>

        {/* Media Preview Box */}
        <div
          style={{
            width: "100%",
            height: 220,
            backgroundColor: "var(--cs-canvas-checker)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            position: "relative",
            overflow: "hidden",
            borderBottom: "1px solid var(--cs-border-subtle)",
          }}
        >
          {asset.previewUrl ? (
            asset.kind === "video" ? (
              <video
                src={asset.previewUrl}
                controls
                style={{ maxHeight: "100%", maxWidth: "100%", backgroundColor: "#000" }}
              />
            ) : asset.kind === "audio" ? (
              <div style={{ width: "90%", display: "flex", flexDirection: "column", alignItems: "center", gap: 12 }}>
                <Icon name="audio" size={48} color="var(--cs-accent)" />
                <audio src={asset.previewUrl} controls style={{ width: "100%" }} />
              </div>
            ) : (
              <img
                src={asset.previewUrl}
                alt={asset.name}
                style={{ width: "100%", height: "100%", objectFit: "contain" }}
              />
            )
          ) : (
            <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 8, color: "var(--cs-fg-muted)" }}>
              <Icon name="file" size={48} />
              <span style={{ fontSize: "var(--cs-text-xs)" }}>Không có bản xem trước trực tiếp</span>
            </div>
          )}
        </div>

        {/* Content Body */}
        <div style={{ padding: "var(--cs-space-4)", display: "flex", flexDirection: "column", gap: "var(--cs-space-4)", flex: 1 }}>
          {/* Action Buttons */}
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-2)" }}>
            <Button
              variant="primary"
              onClick={() => onUseInThread(asset)}
              style={{ width: "100%", justifyContent: "center" }}
              title="Chèn @đường-dẫn vào ô nhập của thread đang mở"
            >
              <Icon name="sparkles" size={14} />
              <span style={{ marginLeft: 6 }}>Dùng trong thread</span>
            </Button>

            <div style={{ display: "flex", gap: "var(--cs-space-2)" }}>
              <Button
                variant="secondary"
                onClick={() => onOpenFolder(asset)}
                style={{ flex: 1, justifyContent: "center" }}
                title="Mở thư mục chứa tệp trong Explorer"
              >
                <Icon name="folder" size={14} />
                <span style={{ marginLeft: 6 }}>Mở thư mục</span>
              </Button>

              <Button
                variant="destructive"
                onClick={() => setConfirmDeleteOpen(true)}
                title="Xoá asset khỏi workspace (hoàn tác được)"
              >
                <Icon name="trash" size={14} />
                <span style={{ marginLeft: 4 }}>Xoá</span>
              </Button>
            </div>
          </div>

          {/* AI Generation Prompt Section */}
          {asset.meta?.prompt && (
            <div
              style={{
                backgroundColor: "var(--cs-bg-raised)",
                border: "1px solid var(--cs-border-subtle)",
                borderRadius: "var(--cs-radius-control)",
                padding: "var(--cs-space-3)",
                display: "flex",
                flexDirection: "column",
                gap: 8,
              }}
            >
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                <span style={{ fontSize: "var(--cs-text-xs)", fontWeight: 700, color: "var(--cs-fg-primary)", display: "flex", alignItems: "center", gap: 4 }}>
                  <Icon name="sparkles" size={13} color="#f59e0b" />
                  Bản ghi Prompt AI
                </span>
                <Button
                  variant="quiet"
                  size="sm"
                  onClick={handleCopyPrompt}
                  style={{ height: 24, fontSize: 11, padding: "2px 8px" }}
                >
                  <Icon name={copiedPrompt ? "check" : "copy"} size={12} color={copiedPrompt ? "var(--cs-success)" : undefined} />
                  <span style={{ marginLeft: 4 }}>{copiedPrompt ? "Đã chép" : "Sao chép"}</span>
                </Button>
              </div>

              <p
                style={{
                  margin: 0,
                  fontSize: "var(--cs-text-xs)",
                  color: "var(--cs-fg-secondary)",
                  lineHeight: 1.5,
                  userSelect: "text",
                  backgroundColor: "var(--cs-bg-panel)",
                  padding: "var(--cs-space-2)",
                  borderRadius: "var(--cs-radius-control)",
                  border: "1px solid var(--cs-border-subtle)",
                }}
              >
                {asset.meta.prompt}
              </p>

              {/* Provider & Model & Cost */}
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 6, fontSize: 11, marginTop: 2 }}>
                {asset.meta.provider && (
                  <div>
                    <span style={{ color: "var(--cs-fg-muted)" }}>Nhà cung cấp: </span>
                    <span style={{ fontWeight: 600 }}>{asset.meta.provider}</span>
                  </div>
                )}
                {asset.meta.model && (
                  <div>
                    <span style={{ color: "var(--cs-fg-muted)" }}>Model: </span>
                    <span style={{ fontWeight: 600 }}>{asset.meta.model}</span>
                  </div>
                )}
                {asset.meta.costUsd !== undefined && asset.meta.costUsd !== null && (
                  <div>
                    <span style={{ color: "var(--cs-fg-muted)" }}>Chi phí: </span>
                    <span style={{ fontWeight: 600, color: "var(--cs-accent)" }}>
                      ${asset.meta.costUsd.toFixed(3)} USD
                    </span>
                  </div>
                )}
                {asset.brand && (
                  <div>
                    <span style={{ color: "var(--cs-fg-muted)" }}>Brand: </span>
                    <span style={{ fontWeight: 600 }}>{asset.brand}</span>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Metadata Properties */}
          <div style={{ display: "flex", flexDirection: "column", gap: 8, fontSize: "var(--cs-text-xs)" }}>
            <span style={{ fontWeight: 700, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-primary)" }}>
              Thông tin chi tiết
            </span>

            <div style={{ display: "grid", gridTemplateColumns: "100px minmax(0, 1fr)", gap: "6px 8px", alignItems: "baseline" }}>
              <span style={{ color: "var(--cs-fg-muted)" }}>Nguồn gốc:</span>
              <div>
                <Badge variant={sourceVariant as any}>{sourceLabel}</Badge>
              </div>

              {asset.project && (
                <>
                  <span style={{ color: "var(--cs-fg-muted)" }}>Dự án:</span>
                  <span style={{ fontWeight: 600 }}>{asset.project}</span>
                </>
              )}

              <span style={{ color: "var(--cs-fg-muted)" }}>Đường dẫn:</span>
              <div style={{ display: "flex", alignItems: "center", gap: 4, minWidth: 0 }}>
                <code
                  style={{
                    fontFamily: "var(--cs-font-mono)",
                    fontSize: 11,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                    minWidth: 0,
                    backgroundColor: "var(--cs-bg-raised)",
                    padding: "2px 4px",
                    borderRadius: "var(--cs-radius-control)",
                  }}
                  title={asset.path}
                >
                  {asset.path}
                </code>
                <button
                  type="button"
                  onClick={handleCopyPath}
                  style={{ background: "transparent", border: "none", cursor: "pointer", color: "var(--cs-fg-muted)" }}
                  title="Sao chép đường dẫn"
                >
                  <Icon name={copiedPath ? "check" : "copy"} size={12} color={copiedPath ? "var(--cs-success)" : undefined} />
                </button>
              </div>

              <span style={{ color: "var(--cs-fg-muted)" }}>Định dạng:</span>
              <span>{asset.mime}</span>

              <span style={{ color: "var(--cs-fg-muted)" }}>Dung lượng:</span>
              <span>{formatSize(asset.size)}</span>

              <span style={{ color: "var(--cs-fg-muted)" }}>Ngày tạo:</span>
              <span>{new Date(asset.createdAt).toLocaleString("vi-VN")}</span>
            </div>
          </div>
        </div>
      </div>

      {/* Delete Confirmation Dialog */}
      {confirmDeleteOpen && (
        <Dialog
          title="Xác nhận xoá asset"
          open={confirmDeleteOpen}
          onClose={() => setConfirmDeleteOpen(false)}
          actions={
            <div style={{ display: "flex", gap: "var(--cs-space-2)" }}>
              <Button variant="secondary" onClick={() => setConfirmDeleteOpen(false)}>
                Hủy
              </Button>
              <Button
                variant="destructive"
                onClick={() => {
                  setConfirmDeleteOpen(false);
                  onDeleteAsset(asset);
                }}
              >
                Xác nhận xoá
              </Button>
            </div>
          }
        >
          <div style={{ fontSize: "var(--cs-text-sm)", lineHeight: 1.5, display: "flex", flexDirection: "column", gap: 8 }}>
            <p style={{ margin: 0 }}>
              Bạn có chắc chắn muốn xoá tài nguyên <strong>{asset.name}</strong>?
            </p>
            <p style={{ margin: 0, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
              Thao tác này sẽ xoá tệp trên đĩa và tạo một điểm checkpoint an toàn. Bạn có thể hoàn tác ngay sau khi xoá.
            </p>
          </div>
        </Dialog>
      )}
    </>
  );
};
