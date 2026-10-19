import React from "react";
import { Icon } from "../../../components/Icon";
import { Button } from "@studio/design-system";
import type { FileReadResult } from "../../../api/types";

export interface MediaViewerProps {
  fileData: FileReadResult;
  selectedFilePath: string | null;
  zoom: number;
  onOpenExternal: () => void;
}

export const MediaViewer: React.FC<MediaViewerProps> = ({
  fileData,
  selectedFilePath,
  zoom,
  onOpenExternal,
}) => {
  // Image viewer
  if (fileData.kind === "image") {
    return (
      <div
        style={{
          flex: 1,
          width: "100%",
          height: "100%",
          minWidth: 0,
          minHeight: 0,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          backgroundColor: "var(--cs-canvas-checker)",
          overflow: "auto",
          position: "relative",
          padding: "var(--cs-space-4)",
          boxSizing: "border-box",
        }}
      >
        <img
          src={fileData.assetUrl || (fileData.text ? `data:${fileData.mime};base64,${fileData.text}` : "")}
          alt={selectedFilePath || "Image preview"}
          style={{
            maxWidth: zoom === 100 ? "100%" : `${zoom}%`,
            maxHeight: zoom === 100 ? "100%" : `${zoom}%`,
            objectFit: "contain",
            boxShadow: "var(--cs-shadow-artboard)",
            backgroundColor: "var(--cs-canvas-artboard)",
          }}
        />
      </div>
    );
  }

  // Video viewer
  if (fileData.kind === "video") {
    return (
      <div
        style={{
          flex: 1,
          width: "100%",
          height: "100%",
          minWidth: 0,
          minHeight: 0,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          backgroundColor: "var(--cs-bg-canvas)",
          padding: "var(--cs-space-4)",
          boxSizing: "border-box",
          overflow: "hidden",
        }}
      >
        <video
          controls
          src={fileData.assetUrl}
          style={{
            maxWidth: "100%",
            maxHeight: "100%",
            backgroundColor: "#000",
            boxShadow: "var(--cs-shadow-artboard)",
          }}
        />
      </div>
    );
  }

  // Audio viewer
  if (fileData.kind === "audio") {
    return (
      <div
        style={{
          flex: 1,
          width: "100%",
          height: "100%",
          minWidth: 0,
          minHeight: 0,
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          backgroundColor: "var(--cs-bg-app)",
          gap: "var(--cs-space-4)",
          padding: "var(--cs-space-4)",
          boxSizing: "border-box",
          overflow: "hidden",
        }}
      >
        <Icon name="audio" size={48} color="var(--cs-accent)" />
        <div style={{ fontWeight: 600, fontSize: "var(--cs-text-md)" }}>
          {selectedFilePath}
        </div>
        <audio controls src={fileData.assetUrl} style={{ width: "100%", maxWidth: 460 }} />
      </div>
    );
  }

  // Default binary/unknown
  return (
    <div
      style={{
        flex: 1,
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        padding: "var(--cs-space-6)",
        textAlign: "center",
        gap: "var(--cs-space-3)",
      }}
    >
      <Icon name="file" size={48} color="var(--cs-fg-muted)" />
      <h3 style={{ margin: 0, fontSize: "var(--cs-text-md)", fontWeight: 600 }}>
        Định dạng tệp nhị phân
      </h3>
      <p style={{ margin: 0, fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-secondary)", maxWidth: 360 }}>
        Không thể xem trước trực tiếp định dạng <code>{fileData.mime}</code> ({fileData.size.toLocaleString()} bytes).
      </p>
      <Button
        variant="secondary"
        onClick={onOpenExternal}
        leftIcon={<Icon name="external" size={14} />}
      >
        Mở bằng ứng dụng hệ thống
      </Button>
    </div>
  );
};
