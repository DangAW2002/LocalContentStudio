import React from "react";
import { Icon } from "../../components/Icon";
import { Badge, Button } from "@studio/design-system";
import type { AssetItem } from "../../api/types";

interface AssetRowProps {
  asset: AssetItem;
  isSelected?: boolean;
  onClick: () => void;
  onUseInThread?: (asset: AssetItem) => void;
  onOpenFolder?: (asset: AssetItem) => void;
}

export const AssetRow: React.FC<AssetRowProps> = ({
  asset,
  isSelected,
  onClick,
  onUseInThread,
  onOpenFolder,
}) => {
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

  const kindIcon =
    asset.kind === "video"
      ? "video"
      : asset.kind === "audio"
      ? "audio"
      : asset.kind === "document"
      ? "file-text"
      : asset.kind === "font"
      ? "file"
      : "image";

  const formatSize = (bytes: number) => {
    if (bytes >= 1024 * 1024) {
      return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    }
    return `${(bytes / 1024).toFixed(1)} KB`;
  };

  const thumbSrc = asset.thumbnailUrl || asset.previewUrl;

  return (
    <div
      onClick={onClick}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onClick();
        }
      }}
      style={{
        display: "grid",
        gridTemplateColumns: "40px minmax(200px, 2fr) 90px 100px 90px 110px auto",
        alignItems: "center",
        gap: "var(--cs-space-3)",
        padding: "var(--cs-space-2) var(--cs-space-3)",
        backgroundColor: isSelected ? "var(--cs-bg-raised)" : "var(--cs-bg-panel)",
        borderBottom: "1px solid var(--cs-border-subtle)",
        cursor: "pointer",
        transition: "background-color var(--cs-motion-duration) ease",
      }}
      onMouseEnter={(e) => {
        if (!isSelected) e.currentTarget.style.backgroundColor = "var(--cs-bg-raised)";
      }}
      onMouseLeave={(e) => {
        if (!isSelected) e.currentTarget.style.backgroundColor = "var(--cs-bg-panel)";
      }}
    >
      {/* 1. Thumbnail / Icon */}
      <div
        style={{
          width: 36,
          height: 36,
          backgroundColor: "var(--cs-canvas-checker)",
          borderRadius: "var(--cs-radius-control)",
          overflow: "hidden",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          flexShrink: 0,
        }}
      >
        {thumbSrc ? (
          <img
            src={thumbSrc}
            alt={asset.name}
            loading="lazy"
            style={{ width: "100%", height: "100%", objectFit: "cover" }}
          />
        ) : (
          <Icon name={kindIcon} size={18} color="var(--cs-fg-muted)" />
        )}
      </div>

      {/* 2. Name & Path */}
      <div style={{ minWidth: 0, display: "flex", flexDirection: "column", gap: 2 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <span
            style={{
              fontWeight: 600,
              fontSize: "var(--cs-text-xs)",
              color: "var(--cs-fg-primary)",
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
            title={asset.name}
          >
            {asset.name}
          </span>
          {asset.meta?.prompt && (
            <span title="Có bản ghi prompt AI">
              <Icon name="sparkles" size={12} color="#f59e0b" />
            </span>
          )}
        </div>
        <span
          style={{
            fontSize: 11,
            color: "var(--cs-fg-muted)",
            fontFamily: "var(--cs-font-mono)",
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}
          title={asset.path}
        >
          {asset.path}
        </span>
      </div>

      {/* 3. Source Badge */}
      <div>
        <Badge variant={sourceVariant as any}>{sourceLabel}</Badge>
      </div>

      {/* 4. Project Tag */}
      <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
        {asset.project ? (
          <span
            style={{
              padding: "2px 6px",
              backgroundColor: "var(--cs-bg-raised)",
              borderRadius: "var(--cs-radius-control)",
              fontSize: 11,
            }}
          >
            {asset.project}
          </span>
        ) : (
          <span style={{ color: "var(--cs-fg-muted)" }}>—</span>
        )}
      </div>

      {/* 5. Size */}
      <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
        {formatSize(asset.size)}
      </div>

      {/* 6. Created Date */}
      <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
        {new Date(asset.createdAt).toLocaleDateString("vi-VN")}
      </div>

      {/* 7. Quick Actions */}
      <div
        style={{ display: "flex", alignItems: "center", gap: 6 }}
        onClick={(e) => e.stopPropagation()}
      >
        {onUseInThread && (
          <Button
            variant="quiet"
            size="sm"
            onClick={() => onUseInThread(asset)}
            title="Dùng trong thread (@đường-dẫn)"
          >
            <Icon name="sparkles" size={12} />
            <span style={{ marginLeft: 4 }}>Dùng</span>
          </Button>
        )}
        {onOpenFolder && (
          <Button
            variant="quiet"
            size="sm"
            onClick={() => onOpenFolder(asset)}
            title="Mở thư mục chứa tệp"
          >
            <Icon name="folder" size={12} />
          </Button>
        )}
      </div>
    </div>
  );
};
