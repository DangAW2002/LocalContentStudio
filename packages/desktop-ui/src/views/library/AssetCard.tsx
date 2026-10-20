import React from "react";
import { Icon } from "../../components/Icon";
import { Badge } from "@studio/design-system";
import type { AssetItem } from "../../api/types";

interface AssetCardProps {
  asset: AssetItem;
  isSelected?: boolean;
  onClick: () => void;
}

export const AssetCard: React.FC<AssetCardProps> = ({
  asset,
  isSelected,
  onClick,
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
        backgroundColor: "var(--cs-bg-panel)",
        border: `1px solid ${isSelected ? "var(--cs-accent)" : "var(--cs-border-subtle)"}`,
        display: "flex",
        flexDirection: "column",
        cursor: "pointer",
        borderRadius: "var(--cs-radius-control)",
        overflow: "hidden",
        transition: "border-color var(--cs-motion-duration) ease, transform 0.1s ease",
        outline: isSelected ? "2px solid var(--cs-focus)" : "none",
        position: "relative",
      }}
      onMouseEnter={(e) => {
        if (!isSelected) e.currentTarget.style.borderColor = "var(--cs-accent)";
      }}
      onMouseLeave={(e) => {
        if (!isSelected) e.currentTarget.style.borderColor = "var(--cs-border-subtle)";
      }}
    >
      {/* 4:3 Thumbnail Canvas */}
      <div
        style={{
          aspectRatio: "4 / 3",
          backgroundColor: "var(--cs-canvas-checker)",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          position: "relative",
          overflow: "hidden",
        }}
      >
        {/* Source Badge */}
        <span
          style={{
            position: "absolute",
            top: 6,
            left: 6,
            zIndex: 2,
          }}
        >
          <Badge variant={sourceVariant as any}>{sourceLabel}</Badge>
        </span>

        {/* AI Prompt indicator */}
        {asset.meta?.prompt && (
          <span
            style={{
              position: "absolute",
              top: 6,
              right: 6,
              backgroundColor: "rgba(0, 0, 0, 0.65)",
              color: "#f59e0b",
              borderRadius: "50%",
              width: 22,
              height: 22,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              zIndex: 2,
            }}
            title="Có bản ghi prompt AI"
          >
            <Icon name="sparkles" size={12} color="#f59e0b" />
          </span>
        )}

        {thumbSrc ? (
          <img
            src={thumbSrc}
            alt={asset.name}
            loading="lazy"
            style={{
              width: "100%",
              height: "100%",
              objectFit: "cover",
              display: "block",
            }}
          />
        ) : (
          <div
            style={{
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              gap: 4,
              color: "var(--cs-fg-muted)",
            }}
          >
            <Icon name={kindIcon} size={32} color="var(--cs-fg-muted)" />
            <span style={{ fontSize: 10, textTransform: "uppercase", fontWeight: 600 }}>
              {asset.kind}
            </span>
          </div>
        )}
      </div>

      {/* Info Footer */}
      <div
        style={{
          padding: "var(--cs-space-2) var(--cs-space-3)",
          display: "flex",
          flexDirection: "column",
          gap: 3,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 4 }}>
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
        </div>

        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            fontSize: 11,
            color: "var(--cs-fg-muted)",
          }}
        >
          <span>{formatSize(asset.size)}</span>
          {asset.project ? (
            <span
              style={{
                maxWidth: 90,
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
                color: "var(--cs-accent)",
              }}
              title={`Dự án: ${asset.project}`}
            >
              {asset.project}
            </span>
          ) : (
            <span>{new Date(asset.createdAt).toLocaleDateString("vi-VN")}</span>
          )}
        </div>
      </div>
    </div>
  );
};
