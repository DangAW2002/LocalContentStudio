import React from "react";
import { Icon } from "../../components/Icon";

interface AssetDropZoneProps {
  onDropPaths: (paths: string[]) => void;
  isDragging: boolean;
  onDragStateChange: (dragging: boolean) => void;
  children: React.ReactNode;
}

export const AssetDropZone: React.FC<AssetDropZoneProps> = ({
  onDropPaths,
  isDragging,
  onDragStateChange,
  children,
}) => {
  const dragCounterRef = React.useRef(0);

  const handleDragEnter = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounterRef.current += 1;
    if (e.dataTransfer.items && e.dataTransfer.items.length > 0) {
      onDragStateChange(true);
    }
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounterRef.current -= 1;
    if (dragCounterRef.current <= 0) {
      dragCounterRef.current = 0;
      onDragStateChange(false);
    }
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounterRef.current = 0;
    onDragStateChange(false);

    const files = Array.from(e.dataTransfer.files);
    if (files.length === 0) return;

    // In Tauri / WebView2 desktop, dropped File objects expose the absolute local path via .path
    const paths: string[] = [];
    for (const file of files) {
      const p = (file as any).path;
      if (typeof p === "string" && p.trim()) {
        paths.push(p);
      }
    }

    if (paths.length > 0) {
      onDropPaths(paths);
    }
  };

  return (
    <div
      onDragEnter={handleDragEnter}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      style={{
        position: "relative",
        display: "flex",
        flexDirection: "column",
        height: "100%",
        width: "100%",
        overflow: "hidden",
      }}
    >
      {children}

      {/* Drag Over Overlay */}
      {isDragging && (
        <div
          style={{
            position: "absolute",
            inset: 0,
            backgroundColor: "rgba(var(--cs-accent-rgb, 99, 102, 241), 0.12)",
            backdropFilter: "blur(2px)",
            border: "2px dashed var(--cs-accent)",
            zIndex: 100,
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            gap: 12,
            pointerEvents: "none",
          }}
        >
          <div
            style={{
              width: 56,
              height: 56,
              borderRadius: "50%",
              backgroundColor: "var(--cs-bg-panel)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              boxShadow: "0 4px 12px rgba(0, 0, 0, 0.15)",
            }}
          >
            <Icon name="upload" size={28} color="var(--cs-accent)" />
          </div>

          <div style={{ textAlign: "center" }}>
            <span style={{ fontSize: "var(--cs-text-base)", fontWeight: 700, color: "var(--cs-fg-primary)", display: "block" }}>
              Thả tệp vào đây để nhập vào Thư viện
            </span>
            <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", marginTop: 4, display: "block" }}>
              Tối đa 200 MB mỗi tệp · Tự động sao chép và tạo checkpoint hoàn tác
            </span>
          </div>
        </div>
      )}
    </div>
  );
};
