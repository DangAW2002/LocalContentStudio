import React from "react";
import { Icon } from "../../../components/Icon";
import { Button, Badge } from "@studio/design-system";
import type { ArtifactSummary, FileReadResult } from "../../../api/types";

export interface StageToolbarProps {
  selectedFilePath: string | null;
  fileData: FileReadResult | null;
  matchingArtifact: ArtifactSummary | null;
  isDirty: boolean;
  saveStatus: "idle" | "saving" | "saved" | "error";
  saveError: string | null;
  rendering: boolean;
  checking: boolean;
  isLayerHtml: boolean;
  zoom: number;
  setZoom: React.Dispatch<React.SetStateAction<number>>;
  onSave: () => void;
  onRenderArtifact: () => void;
  onCheckArtifact: () => void;
  onOpenExternal: () => void;
  onClearSelection: () => void;
}

export const StageToolbar: React.FC<StageToolbarProps> = ({
  selectedFilePath,
  fileData,
  matchingArtifact,
  isDirty,
  saveStatus,
  saveError,
  rendering,
  checking,
  isLayerHtml,
  zoom,
  setZoom,
  onSave,
  onRenderArtifact,
  onCheckArtifact,
  onOpenExternal,
  onClearSelection,
}) => {
  return (
    <div
      style={{
        height: 38,
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        padding: "0 var(--cs-space-3)",
        backgroundColor: "var(--cs-bg-panel)",
        borderBottom: "1px solid var(--cs-border-subtle)",
        fontSize: "var(--cs-text-xs)",
        userSelect: "none",
        flexShrink: 0,
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 8, minWidth: 0 }}>
        <span
          style={{
            fontWeight: 600,
            color: "var(--cs-fg-primary)",
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}
        >
          {selectedFilePath || "Stage (Khung làm việc)"}
        </span>
        {fileData && (
          <>
            <Badge variant="neutral">{fileData.kind.toUpperCase()}</Badge>
            <span style={{ color: "var(--cs-fg-muted)" }}>
              {(fileData.size / 1024).toFixed(1)} KB
            </span>
          </>
        )}
        {matchingArtifact && (
          <Badge variant="info">
            Artifact: {matchingArtifact.title || matchingArtifact.id}
          </Badge>
        )}
        {isDirty && (
          <Badge variant="warning">Đã sửa · Ctrl+S để lưu</Badge>
        )}
        {!isDirty && saveStatus === "saved" && (
          <span style={{ color: "var(--cs-success)" }}>Đã lưu</span>
        )}
        {saveStatus === "error" && (
          <span style={{ color: "var(--cs-danger)" }} title={saveError ?? undefined}>
            Lỗi lưu: {saveError}
          </span>
        )}
      </div>

      {selectedFilePath && (
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          {/* Artifact Toolbar Actions */}
          {matchingArtifact && (
            <>
              <Button
                size="sm"
                variant="primary"
                busy={rendering}
                busyText="Đang render..."
                onClick={onRenderArtifact}
                leftIcon={<Icon name="play" size={12} />}
              >
                Render
              </Button>
              <Button
                size="sm"
                variant="secondary"
                busy={checking}
                busyText="Đang kiểm tra..."
                onClick={onCheckArtifact}
                leftIcon={<Icon name="check" size={12} />}
              >
                Kiểm tra
              </Button>
            </>
          )}

          {/* Save Button for editable files */}
          {(isLayerHtml ||
            fileData?.kind === "markdown" ||
            fileData?.kind === "text" ||
            fileData?.kind === "code" ||
            fileData?.kind === "json") && (
            <Button
              size="sm"
              variant={isDirty ? "primary" : "quiet"}
              busy={saveStatus === "saving"}
              onClick={onSave}
              leftIcon={<Icon name="check" size={12} />}
            >
              Lưu
            </Button>
          )}

          {fileData?.kind === "image" && (
            <div style={{ display: "flex", alignItems: "center", gap: 2, marginRight: 4 }}>
              <Button size="sm" variant="quiet" onClick={() => setZoom((z) => Math.max(25, z - 25))}>
                -
              </Button>
              <span style={{ fontSize: 11, minWidth: 32, textAlign: "center" }}>{zoom}%</span>
              <Button size="sm" variant="quiet" onClick={() => setZoom((z) => Math.min(300, z + 25))}>
                +
              </Button>
            </div>
          )}

          <Button
            size="sm"
            variant="quiet"
            onClick={onOpenExternal}
            title="Mở bên ngoài"
          >
            <Icon name="external" size={13} />
          </Button>
          <Button
            size="sm"
            variant="quiet"
            onClick={onClearSelection}
            title="Đóng xem trước"
          >
            <Icon name="x" size={13} />
          </Button>
        </div>
      )}
    </div>
  );
};
