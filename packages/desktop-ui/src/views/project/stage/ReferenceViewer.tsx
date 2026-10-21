import React from "react";
import { studioApi } from "../../../api/client";
import { Icon } from "../../../components/Icon";
import type { ArtifactSummary, ProjectFileEntry } from "../../../api/types";

export interface ReferenceViewerProps {
  projectSlug: string;
  matchingArtifact: ArtifactSummary | null;
  selectedFilePath: string | null;
  textValue: string;
  setTextValue: (val: string) => void;
  setIsDirty: (dirty: boolean) => void;
}

export const ReferenceViewer: React.FC<ReferenceViewerProps> = ({
  projectSlug,
  matchingArtifact,
  selectedFilePath,
  textValue,
  setTextValue,
  setIsDirty,
}) => {
  const [images, setImages] = React.useState<{ path: string; name: string }[]>([]);
  const [loadingImages, setLoadingImages] = React.useState(false);

  // Load image files located in this reference artifact's directory
  React.useEffect(() => {
    if (!matchingArtifact || !projectSlug) return;
    setLoadingImages(true);
    studioApi
      .getProjectFiles(projectSlug)
      .then((files: ProjectFileEntry[]) => {
        const prefix = `artifacts/${matchingArtifact.id}/`;
        const imgs = files
          .filter(
            (f) =>
              f.path.startsWith(prefix) &&
              /\.(png|jpg|jpeg|webp)$/i.test(f.path)
          )
          .map((f) => ({
            path: f.path,
            name: f.name,
          }));
        setImages(imgs);
      })
      .catch(() => setImages([]))
      .finally(() => setLoadingImages(false));
  }, [matchingArtifact, projectSlug]);

  return (
    <div
      style={{
        display: "flex",
        height: "100%",
        width: "100%",
        backgroundColor: "var(--cs-bg-canvas)",
        overflow: "hidden",
      }}
    >
      {/* Markdown / Profile text editor */}
      <div
        style={{
          flex: 1,
          display: "flex",
          flexDirection: "column",
          borderRight: "1px solid var(--cs-border-subtle)",
          minWidth: 320,
        }}
      >
        <div
          style={{
            padding: "8px 16px",
            backgroundColor: "var(--cs-bg-panel)",
            borderBottom: "1px solid var(--cs-border-subtle)",
            fontSize: "var(--cs-text-xs)",
            fontWeight: 600,
            color: "var(--cs-fg-secondary)",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
          }}
        >
          <span>Hồ sơ mô tả (profile.md)</span>
          <span style={{ fontWeight: 400, color: "var(--cs-fg-muted)" }}>
            Ctrl+S để lưu
          </span>
        </div>
        <textarea
          value={textValue}
          onChange={(e) => {
            setTextValue(e.target.value);
            setIsDirty(true);
          }}
          style={{
            flex: 1,
            width: "100%",
            border: "none",
            outline: "none",
            resize: "none",
            padding: "16px",
            fontFamily: "var(--cs-font-code)",
            fontSize: "var(--cs-text-sm)",
            lineHeight: 1.6,
            backgroundColor: "var(--cs-bg-canvas)",
            color: "var(--cs-fg-primary)",
            boxSizing: "border-box",
          }}
          placeholder="# Mô tả hồ sơ tham chiếu..."
        />
      </div>

      {/* Reference Images Grid */}
      <div
        style={{
          width: 340,
          display: "flex",
          flexDirection: "column",
          backgroundColor: "var(--cs-bg-panel)",
          flexShrink: 0,
        }}
      >
        <div
          style={{
            padding: "8px 16px",
            borderBottom: "1px solid var(--cs-border-subtle)",
            fontSize: "var(--cs-text-xs)",
            fontWeight: 600,
            color: "var(--cs-fg-secondary)",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
          }}
        >
          <span>Ảnh tham chiếu ({images.length})</span>
          {loadingImages && (
            <Icon name="loader" size={12} className="cs-spin" color="var(--cs-fg-muted)" />
          )}
        </div>

        <div
          style={{
            flex: 1,
            overflowY: "auto",
            padding: "16px",
            display: "flex",
            flexDirection: "column",
            gap: "12px",
          }}
        >
          {images.length === 0 ? (
            <div
              style={{
                padding: "24px 16px",
                textAlign: "center",
                color: "var(--cs-fg-muted)",
                fontSize: "var(--cs-text-xs)",
                border: "1px dashed var(--cs-border-subtle)",
                borderRadius: "var(--cs-radius-control)",
              }}
            >
              Chưa có ảnh tham chiếu nào trong thư mục hồ sơ. Lưu ảnh (PNG/JPG/WebP) vào thư mục artifact để AI nhận diện khuôn mặt và phong cách.
            </div>
          ) : (
            images.map((img) => (
              <div
                key={img.path}
                style={{
                  border: "1px solid var(--cs-border-subtle)",
                  borderRadius: "var(--cs-radius-control)",
                  overflow: "hidden",
                  backgroundColor: "var(--cs-bg-raised)",
                }}
              >
                <div
                  style={{
                    height: 160,
                    width: "100%",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    backgroundColor: "var(--cs-bg-canvas)",
                    overflow: "hidden",
                  }}
                >
                  <img
                    src={`/api/file?project=${encodeURIComponent(projectSlug)}&path=${encodeURIComponent(img.path)}`}
                    alt={img.name}
                    style={{
                      maxWidth: "100%",
                      maxHeight: "100%",
                      objectFit: "contain",
                    }}
                    onError={(e) => {
                      (e.currentTarget as HTMLElement).style.display = "none";
                    }}
                  />
                </div>
                <div
                  style={{
                    padding: "6px 10px",
                    fontSize: "var(--cs-text-xs)",
                    color: "var(--cs-fg-primary)",
                    whiteSpace: "nowrap",
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                  }}
                  title={img.name}
                >
                  {img.name}
                </div>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
};
