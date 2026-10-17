import React from "react";
import { Icon } from "../components/Icon";
import { Badge, Button, EmptyState, ErrorState, Input, Dialog } from "@studio/design-system";
import { studioApi } from "../api/client";
import type { AssetItem, AssetListFilter } from "../api/types";

export const LibraryView: React.FC = () => {
  const [assets, setAssets] = React.useState<AssetItem[]>([]);
  const [filterKind, setFilterKind] = React.useState<string>("all");
  const [search, setSearch] = React.useState("");
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);
  const [selectedAsset, setSelectedAsset] = React.useState<AssetItem | null>(null);

  const loadAssets = React.useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const filter: AssetListFilter = {
        kind: filterKind === "all" ? undefined : filterKind,
        search: search.trim() || undefined,
      };
      const list = await studioApi.getAssets(filter);
      setAssets(list);
    } catch (err: any) {
      setError(err?.message || "Chưa kết nối Studio core");
    } finally {
      setLoading(false);
    }
  }, [filterKind, search]);

  React.useEffect(() => {
    loadAssets();
  }, [loadAssets]);

  const filterOptions = [
    { id: "all", label: "Tất cả" },
    { id: "image", label: "Hình ảnh" },
    { id: "video", label: "Video" },
    { id: "audio", label: "Âm thanh" },
    { id: "document", label: "Tài liệu" },
  ];

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        backgroundColor: "var(--cs-bg-app)",
      }}
    >
      {/* Header */}
      <div
        style={{
          padding: "var(--cs-space-3) var(--cs-space-5)",
          backgroundColor: "var(--cs-bg-panel)",
          borderBottom: "1px solid var(--cs-border-subtle)",
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          flexWrap: "wrap",
          gap: "var(--cs-space-3)",
        }}
      >
        <div>
          <h1 style={{ margin: "0 0 4px 0", fontSize: "var(--cs-text-lg)", fontWeight: 700 }}>
            Thư viện Tài nguyên (Library)
          </h1>
          <p style={{ margin: 0, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
            Nguyên liệu gốc và kết quả AI sinh ảnh/video trong workspace.
          </p>
        </div>

        {/* Filter buttons & Search */}
        <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-2)" }}>
          <div style={{ display: "flex", backgroundColor: "var(--cs-bg-raised)", borderRadius: "var(--cs-radius-control)", padding: 2 }}>
            {filterOptions.map((opt) => (
              <button
                key={opt.id}
                type="button"
                onClick={() => setFilterKind(opt.id)}
                style={{
                  padding: "4px 10px",
                  border: "none",
                  backgroundColor: filterKind === opt.id ? "var(--cs-bg-panel)" : "transparent",
                  color: filterKind === opt.id ? "var(--cs-fg-primary)" : "var(--cs-fg-secondary)",
                  fontWeight: filterKind === opt.id ? 600 : 400,
                  fontSize: "var(--cs-text-xs)",
                  borderRadius: "var(--cs-radius-control)",
                  cursor: "pointer",
                  boxShadow: filterKind === opt.id ? "0 1px 2px rgb(0 0 0 / 6%)" : "none",
                }}
              >
                {opt.label}
              </button>
            ))}
          </div>

          <div style={{ width: 200 }}>
            <Input
              placeholder="Tìm theo tên..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              style={{ height: 28, fontSize: "var(--cs-text-xs)" }}
            />
          </div>
        </div>
      </div>

      {/* Grid Content */}
      <div style={{ flex: 1, overflowY: "auto", padding: "var(--cs-space-5)" }}>
        {error ? (
          <div style={{ width: "100%", maxWidth: 800, margin: "0 auto" }}>
            <ErrorState
              title="Tải thư viện tài nguyên"
              reason={error}
              safetyNote="Các tệp tài nguyên gốc nằm trong thư mục library/ của workspace."
              onRetry={loadAssets}
            />
          </div>
        ) : loading ? (
          <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", color: "var(--cs-fg-muted)" }}>
            Đang quét tệp thư viện...
          </div>
        ) : assets.length === 0 ? (
          <EmptyState
            icon={<Icon name="library" size={32} />}
            title="Thư viện trống"
            description="Chưa có hình ảnh, video hoặc tài liệu nào trong thư mục library/."
          />
        ) : (
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))",
              gap: "var(--cs-space-4)",
            }}
          >
            {assets.map((asset) => (
              <div
                key={asset.id}
                onClick={() => setSelectedAsset(asset)}
                style={{
                  backgroundColor: "var(--cs-bg-panel)",
                  border: "1px solid var(--cs-border-subtle)",
                  display: "flex",
                  flexDirection: "column",
                  cursor: "pointer",
                  borderRadius: "var(--cs-radius-none)",
                  overflow: "hidden",
                  transition: "border-color var(--cs-motion-duration) ease",
                }}
                onMouseEnter={(e) => (e.currentTarget.style.borderColor = "var(--cs-accent)")}
                onMouseLeave={(e) => (e.currentTarget.style.borderColor = "var(--cs-border-subtle)")}
              >
                {/* 4:3 Checkerboard Thumbnail */}
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
                  <span
                    style={{
                      position: "absolute",
                      top: 6,
                      left: 6,
                      backgroundColor: "var(--cs-bg-panel)",
                      border: "1px solid var(--cs-border-subtle)",
                      padding: "1px 5px",
                      borderRadius: "var(--cs-radius-control)",
                      fontSize: 10,
                      fontWeight: 700,
                      fontFamily: "var(--cs-font-mono)",
                    }}
                  >
                    {asset.kind.toUpperCase()}
                  </span>

                  {asset.previewUrl ? (
                    <img
                      src={asset.previewUrl}
                      alt={asset.name}
                      style={{ width: "100%", height: "100%", objectFit: "cover" }}
                    />
                  ) : (
                    <Icon
                      name={
                        asset.kind === "video"
                          ? "video"
                          : asset.kind === "audio"
                          ? "audio"
                          : asset.kind === "document"
                          ? "file-text"
                          : "image"
                      }
                      size={28}
                      color="var(--cs-fg-muted)"
                    />
                  )}
                </div>

                {/* Info Footer */}
                <div style={{ padding: "var(--cs-space-2) var(--cs-space-3)", display: "flex", flexDirection: "column", gap: 2 }}>
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
                  <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", fontSize: 11, color: "var(--cs-fg-muted)" }}>
                    <span>{(asset.size / 1024).toFixed(1)} KB</span>
                    <span>{new Date(asset.createdAt).toLocaleDateString("vi-VN")}</span>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Asset Preview Modal */}
      {selectedAsset && (
        <Dialog
          title={selectedAsset.name}
          open={Boolean(selectedAsset)}
          onClose={() => setSelectedAsset(null)}
          actions={
            <Button variant="secondary" onClick={() => setSelectedAsset(null)}>
              Đóng
            </Button>
          }
        >
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
            <div
              style={{
                width: "100%",
                height: 240,
                backgroundColor: "var(--cs-canvas-checker)",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
              }}
            >
              {selectedAsset.previewUrl ? (
                selectedAsset.kind === "video" ? (
                  <video
                    src={selectedAsset.previewUrl}
                    controls
                    style={{ maxHeight: "100%", maxWidth: "100%", backgroundColor: "#000" }}
                  />
                ) : selectedAsset.kind === "audio" ? (
                  <audio
                    src={selectedAsset.previewUrl}
                    controls
                    style={{ width: "80%" }}
                  />
                ) : (
                  <img
                    src={selectedAsset.previewUrl}
                    alt={selectedAsset.name}
                    style={{ maxHeight: "100%", maxWidth: "100%", objectFit: "contain" }}
                  />
                )
              ) : (
                <Icon name="file" size={48} color="var(--cs-fg-muted)" />
              )}
            </div>

            <div style={{ display: "grid", gridTemplateColumns: "100px 1fr", gap: 8, fontSize: "var(--cs-text-xs)" }}>
              <span style={{ color: "var(--cs-fg-muted)" }}>Đường dẫn:</span>
              <code style={{ fontFamily: "var(--cs-font-mono)" }}>{selectedAsset.path}</code>

              <span style={{ color: "var(--cs-fg-muted)" }}>Loại MIME:</span>
              <span>{selectedAsset.mime}</span>

              <span style={{ color: "var(--cs-fg-muted)" }}>Dung lượng:</span>
              <span>{(selectedAsset.size / 1024).toFixed(1)} KB</span>
            </div>
          </div>
        </Dialog>
      )}
    </div>
  );
};
