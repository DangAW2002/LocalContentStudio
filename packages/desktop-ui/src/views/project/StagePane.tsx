import React from "react";
import { Icon } from "../../components/Icon";
import { Button, Badge, EmptyState, Callout } from "@studio/design-system";
import { studioApi } from "../../api/client";
import type { FileKind, FileReadResult } from "../../api/types";

export interface StagePaneProps {
  projectSlug: string;
  selectedFilePath: string | null;
  onClearSelection: () => void;
}

export const StagePane: React.FC<StagePaneProps> = ({
  projectSlug,
  selectedFilePath,
  onClearSelection,
}) => {
  const [fileData, setFileData] = React.useState<FileReadResult | null>(null);
  const [loading, setLoading] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const [zoom, setZoom] = React.useState(100);

  React.useEffect(() => {
    if (!selectedFilePath) {
      setFileData(null);
      return;
    }

    let isMounted = true;
    setLoading(true);
    setError(null);

    const loadFile = async () => {
      try {
        const res = await studioApi.readFile({
          project: projectSlug,
          path: selectedFilePath,
        });
        if (isMounted) {
          setFileData(res);
        }
      } catch (err: any) {
        if (isMounted) {
          setError(err?.message || "Không thể đọc tệp từ workspace.");
        }
      } finally {
        if (isMounted) {
          setLoading(false);
        }
      }
    };

    loadFile();
    return () => {
      isMounted = false;
    };
  }, [projectSlug, selectedFilePath]);

  const handleOpenExternal = () => {
    // In Tauri or web, open externally
    if (fileData?.assetUrl) {
      window.open(fileData.assetUrl, "_blank");
    } else {
      alert(`Mở tệp ${selectedFilePath} bằng ứng dụng hệ thống.`);
    }
  };

  const renderViewer = () => {
    if (!fileData) return null;

    switch (fileData.kind) {
      case "markdown":
      case "text":
        return (
          <div
            style={{
              padding: "var(--cs-space-5)",
              backgroundColor: "var(--cs-bg-panel)",
              maxWidth: 820,
              margin: "0 auto",
              width: "100%",
              height: "100%",
              overflowY: "auto",
              fontSize: "var(--cs-text-md)",
              lineHeight: 1.6,
              color: "var(--cs-fg-primary)",
              whiteSpace: "pre-wrap",
              fontFamily: fileData.kind === "markdown" ? "var(--cs-font-ui)" : "var(--cs-font-mono)",
            }}
          >
            {fileData.text || "(Tệp rỗng)"}
          </div>
        );

      case "image":
        return (
          <div
            style={{
              flex: 1,
              width: "100%",
              height: "100%",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              backgroundColor: "var(--cs-canvas-checker)",
              overflow: "auto",
              position: "relative",
              padding: "var(--cs-space-4)",
            }}
          >
            <img
              src={fileData.assetUrl || (fileData.text ? `data:${fileData.mime};base64,${fileData.text}` : "")}
              alt={selectedFilePath || "Image preview"}
              style={{
                maxWidth: `${zoom}%`,
                maxHeight: `${zoom}%`,
                objectFit: "contain",
                boxShadow: "var(--cs-shadow-artboard)",
                backgroundColor: "var(--cs-canvas-artboard)",
              }}
            />
          </div>
        );

      case "video":
        return (
          <div
            style={{
              flex: 1,
              width: "100%",
              height: "100%",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              backgroundColor: "var(--cs-bg-canvas)",
              padding: "var(--cs-space-4)",
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

      case "audio":
        return (
          <div
            style={{
              flex: 1,
              width: "100%",
              height: "100%",
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              backgroundColor: "var(--cs-bg-app)",
              gap: "var(--cs-space-4)",
              padding: "var(--cs-space-4)",
            }}
          >
            <Icon name="audio" size={48} color="var(--cs-accent)" />
            <div style={{ fontWeight: 600, fontSize: "var(--cs-text-md)" }}>
              {selectedFilePath}
            </div>
            <audio controls src={fileData.assetUrl} style={{ width: "100%", maxWidth: 460 }} />
          </div>
        );

      case "html":
        return (
          <div
            style={{
              flex: 1,
              width: "100%",
              height: "100%",
              display: "flex",
              backgroundColor: "var(--cs-bg-canvas)",
              position: "relative",
            }}
          >
            <iframe
              title="HTML Sandbox Preview"
              srcDoc={fileData.text}
              sandbox="allow-scripts"
              style={{
                width: "100%",
                height: "100%",
                border: "none",
                backgroundColor: "#ffffff",
              }}
            />
          </div>
        );

      case "code":
      case "json":
        return (
          <div
            style={{
              flex: 1,
              width: "100%",
              height: "100%",
              overflow: "auto",
              backgroundColor: "var(--cs-bg-panel)",
              padding: "var(--cs-space-3)",
            }}
          >
            <pre
              style={{
                margin: 0,
                fontFamily: "var(--cs-font-mono)",
                fontSize: "var(--cs-text-xs)",
                lineHeight: 1.5,
                color: "var(--cs-fg-primary)",
              }}
            >
              {fileData.text || ""}
            </pre>
          </div>
        );

      case "unknown":
      default:
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
              onClick={handleOpenExternal}
              leftIcon={<Icon name="external" size={14} />}
            >
              Mở bằng ứng dụng hệ thống
            </Button>
          </div>
        );
    }
  };

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        backgroundColor: "var(--cs-bg-canvas)",
        overflow: "hidden",
      }}
    >
      {/* Stage Toolbar Header */}
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
          <span style={{ fontWeight: 600, color: "var(--cs-fg-primary)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
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
        </div>

        {selectedFilePath && (
          <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
            {fileData?.kind === "image" && (
              <div style={{ display: "flex", alignItems: "center", gap: 2, marginRight: 8 }}>
                <Button size="sm" variant="quiet" onClick={() => setZoom((z) => Math.max(25, z - 25))}>
                  -
                </Button>
                <span style={{ fontSize: 11, minWidth: 36, textAlign: "center" }}>{zoom}%</span>
                <Button size="sm" variant="quiet" onClick={() => setZoom((z) => Math.min(300, z + 25))}>
                  +
                </Button>
              </div>
            )}
            <Button
              size="sm"
              variant="quiet"
              onClick={handleOpenExternal}
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

      {/* Stage Body */}
      <div
        style={{
          flex: 1,
          display: "flex",
          flexDirection: "column",
          minHeight: 0,
          position: "relative",
          overflow: "hidden",
        }}
      >
        {error && (
          <div style={{ padding: "var(--cs-space-4)" }}>
            <Callout variant="danger" title="Không thể đọc tệp">
              {error}
            </Callout>
          </div>
        )}

        {loading ? (
          <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", color: "var(--cs-fg-muted)" }}>
            Đang tải nội dung tệp...
          </div>
        ) : !selectedFilePath ? (
          <EmptyState
            icon={<Icon name="eye" size={28} />}
            title="Khung nhìn Stage"
            description="Bấm vào một tệp trong hội thoại hoặc danh sách Outputs để xem trước tại đây (Markdown, HTML, ảnh, video, audio, code)."
          />
        ) : (
          renderViewer()
        )}
      </div>
    </div>
  );
};
