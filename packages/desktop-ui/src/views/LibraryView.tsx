import React from "react";
import { Icon } from "../components/Icon";
import { Button, EmptyState, ErrorState, Input } from "@studio/design-system";
import { studioApi } from "../api/client";
import type { AssetItem, AssetListFilter } from "../api/types";
import { AssetCard } from "./library/AssetCard";
import { AssetRow } from "./library/AssetRow";
import { AssetDetailPanel } from "./library/AssetDetailPanel";
import { AssetDropZone } from "./library/AssetDropZone";

interface LibraryViewProps {
  onUseInThread?: (path: string, projectSlug?: string) => void;
}

export const LibraryView: React.FC<LibraryViewProps> = ({ onUseInThread }) => {
  const [assets, setAssets] = React.useState<AssetItem[]>([]);
  const [filterKind, setFilterKind] = React.useState<string>("all");
  const [filterSource, setFilterSource] = React.useState<string>("all");
  const [filterProject, setFilterProject] = React.useState<string>("all");
  const [filterBrand, setFilterBrand] = React.useState<string>("all");
  const [search, setSearch] = React.useState("");
  const [viewMode, setViewMode] = React.useState<"grid" | "list">("grid");
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);
  const [selectedAsset, setSelectedAsset] = React.useState<AssetItem | null>(null);
  const [isDragging, setIsDragging] = React.useState(false);
  const [importing, setImporting] = React.useState(false);
  const [undoToast, setUndoToast] = React.useState<{
    message: string;
    checkpointId: string;
  } | null>(null);
  const [feedbackMsg, setFeedbackMsg] = React.useState<string | null>(null);

  const fileInputRef = React.useRef<HTMLInputElement>(null);

  const loadAssets = React.useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const filter: AssetListFilter = {
        kind: filterKind === "all" ? undefined : filterKind,
        source: filterSource === "all" ? undefined : filterSource,
        project: filterProject === "all" ? undefined : filterProject,
        brand: filterBrand === "all" ? undefined : filterBrand,
        search: search.trim() || undefined,
      };
      const list = await studioApi.getAssets(filter);
      setAssets(list);

      // Keep selectedAsset in sync
      if (selectedAsset) {
        const found = list.find((a) => a.id === selectedAsset.id);
        setSelectedAsset(found || null);
      }
    } catch (err: any) {
      setError(err?.message || "Chưa kết nối Studio core");
    } finally {
      setLoading(false);
    }
  }, [filterKind, filterSource, filterProject, filterBrand, search, selectedAsset?.id]);

  React.useEffect(() => {
    loadAssets();
  }, [loadAssets]);

  // Extract unique projects and brands from assets for dropdowns
  const availableProjects = React.useMemo(() => {
    const set = new Set<string>();
    for (const a of assets) {
      if (a.project) set.add(a.project);
    }
    return Array.from(set).sort();
  }, [assets]);

  const availableBrands = React.useMemo(() => {
    const set = new Set<string>();
    for (const a of assets) {
      if (a.brand) set.add(a.brand);
    }
    return Array.from(set).sort();
  }, [assets]);

  // Handle Drag & Drop import
  const handleDropPaths = async (paths: string[]) => {
    if (paths.length === 0) return;
    setImporting(true);
    setFeedbackMsg(null);
    try {
      const res = await studioApi.importAssets(paths);
      if (res.success) {
        setUndoToast({
          message: `Đã nhập ${res.count} tệp vào thư viện.`,
          checkpointId: res.checkpointId,
        });
        await loadAssets();
      }
    } catch (err: any) {
      setFeedbackMsg(err?.message || "Không thể nhập tệp.");
    } finally {
      setImporting(false);
    }
  };

  // Handle input file picker
  const handleFileInputChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(e.target.files || []);
    if (files.length === 0) return;

    const paths: string[] = [];
    for (const f of files) {
      const p = (f as any).path;
      if (typeof p === "string" && p) {
        paths.push(p);
      }
    }

    if (paths.length > 0) {
      await handleDropPaths(paths);
    } else {
      setFeedbackMsg("Trình duyệt không cung cấp đường dẫn cục bộ của tệp.");
    }

    if (fileInputRef.current) {
      fileInputRef.current.value = "";
    }
  };

  // Handle Delete asset
  const handleDeleteAsset = async (asset: AssetItem) => {
    try {
      const res = await studioApi.deleteAsset(asset.path);
      if (res.success) {
        setSelectedAsset(null);
        setUndoToast({
          message: `Đã xoá tài nguyên "${asset.name}".`,
          checkpointId: res.checkpointId,
        });
        await loadAssets();
      }
    } catch (err: any) {
      setFeedbackMsg(err?.message || "Không thể xoá tài nguyên.");
    }
  };

  // Handle Undo
  const handleUndo = async () => {
    if (!undoToast) return;
    try {
      await studioApi.restoreCheckpoint({ id: undoToast.checkpointId });
      setUndoToast(null);
      setFeedbackMsg("Đã hoàn tác thành công.");
      await loadAssets();
      setTimeout(() => setFeedbackMsg(null), 3000);
    } catch (err: any) {
      setFeedbackMsg(err?.message || "Không thể hoàn tác.");
    }
  };

  // Handle Open Folder
  const handleOpenFolder = async (asset: AssetItem) => {
    try {
      await studioApi.openAssetFolder(asset.path);
    } catch (err: any) {
      setFeedbackMsg(err?.message || "Không thể mở thư mục.");
    }
  };

  // Handle "Dùng trong thread"
  const handleUseInThread = (asset: AssetItem) => {
    const insertText = `@${asset.path} `;
    if (onUseInThread) {
      onUseInThread(asset.path, asset.project);
    } else {
      // Dispatch fallback custom event
      window.dispatchEvent(
        new CustomEvent("studio:ask-ai", { detail: { text: insertText } })
      );
    }
  };

  const sourceFilterOptions = [
    { id: "all", label: "Tất cả nguồn" },
    { id: "generated", label: "AI sinh" },
    { id: "imported", label: "Đã nhập" },
    { id: "uploaded", label: "Đã tải lên" },
  ];

  const kindFilterOptions = [
    { id: "all", label: "Tất cả loại" },
    { id: "image", label: "Hình ảnh" },
    { id: "video", label: "Video" },
    { id: "audio", label: "Âm thanh" },
    { id: "document", label: "Tài liệu" },
    { id: "font", label: "Font" },
  ];

  return (
    <AssetDropZone
      onDropPaths={handleDropPaths}
      isDragging={isDragging}
      onDragStateChange={setIsDragging}
    >
      <input
        ref={fileInputRef}
        type="file"
        multiple
        style={{ display: "none" }}
        onChange={handleFileInputChange}
      />

      <div
        style={{
          display: "flex",
          flexDirection: "column",
          height: "100%",
          backgroundColor: "var(--cs-bg-app)",
          overflow: "hidden",
        }}
      >
        {/* Top Header */}
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
            flexShrink: 0,
          }}
        >
          <div>
            <h1
              style={{
                margin: "0 0 2px 0",
                fontSize: "var(--cs-text-base)",
                fontWeight: 700,
                color: "var(--cs-fg-primary)",
              }}
            >
              Thư viện Tài nguyên
            </h1>
            <p
              style={{
                margin: 0,
                fontSize: "var(--cs-text-xs)",
                color: "var(--cs-fg-secondary)",
              }}
            >
              Quản lý nguyên liệu gốc và kết quả AI sinh trong workspace. Kéo-thả để nhập tệp.
            </p>
          </div>

          {/* Header Action Controls */}
          <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-2)" }}>
            <Button
              variant="secondary"
              size="sm"
              onClick={() => fileInputRef.current?.click()}
              disabled={importing}
              title="Nhập tệp từ máy tính vào thư viện"
            >
              <Icon name="upload" size={14} />
              <span style={{ marginLeft: 6 }}>
                {importing ? "Đang sao chép..." : "Nhập tệp"}
              </span>
            </Button>

            {/* View Mode Toggle */}
            <div
              style={{
                display: "flex",
                backgroundColor: "var(--cs-bg-raised)",
                borderRadius: "var(--cs-radius-control)",
                padding: 2,
              }}
            >
              <button
                type="button"
                onClick={() => setViewMode("grid")}
                style={{
                  padding: "4px 8px",
                  border: "none",
                  backgroundColor: viewMode === "grid" ? "var(--cs-bg-panel)" : "transparent",
                  color: viewMode === "grid" ? "var(--cs-fg-primary)" : "var(--cs-fg-muted)",
                  borderRadius: "var(--cs-radius-control)",
                  cursor: "pointer",
                  display: "flex",
                  alignItems: "center",
                }}
                title="Chế độ xem lưới"
              >
                <Icon name="grid" size={14} />
              </button>
              <button
                type="button"
                onClick={() => setViewMode("list")}
                style={{
                  padding: "4px 8px",
                  border: "none",
                  backgroundColor: viewMode === "list" ? "var(--cs-bg-panel)" : "transparent",
                  color: viewMode === "list" ? "var(--cs-fg-primary)" : "var(--cs-fg-muted)",
                  borderRadius: "var(--cs-radius-control)",
                  cursor: "pointer",
                  display: "flex",
                  alignItems: "center",
                }}
                title="Chế độ xem danh sách"
              >
                <Icon name="list" size={14} />
              </button>
            </div>
          </div>
        </div>

        {/* Filter Toolbar */}
        <div
          style={{
            padding: "var(--cs-space-2) var(--cs-space-5)",
            backgroundColor: "var(--cs-bg-raised)",
            borderBottom: "1px solid var(--cs-border-subtle)",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            flexWrap: "wrap",
            gap: "var(--cs-space-3)",
            flexShrink: 0,
          }}
        >
          {/* Left filters: Source & Kind Pills */}
          <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-2)", flexWrap: "wrap" }}>
            {/* Source selector */}
            <div
              style={{
                display: "flex",
                backgroundColor: "var(--cs-bg-panel)",
                borderRadius: "var(--cs-radius-control)",
                border: "1px solid var(--cs-border-subtle)",
                padding: 2,
              }}
            >
              {sourceFilterOptions.map((opt) => (
                <button
                  key={opt.id}
                  type="button"
                  onClick={() => setFilterSource(opt.id)}
                  style={{
                    padding: "3px 8px",
                    border: "none",
                    backgroundColor: filterSource === opt.id ? "var(--cs-accent)" : "transparent",
                    color: filterSource === opt.id ? "#fff" : "var(--cs-fg-secondary)",
                    fontWeight: filterSource === opt.id ? 600 : 400,
                    fontSize: 11,
                    borderRadius: "var(--cs-radius-control)",
                    cursor: "pointer",
                  }}
                >
                  {opt.label}
                </button>
              ))}
            </div>

            {/* Kind selector */}
            <div
              style={{
                display: "flex",
                backgroundColor: "var(--cs-bg-panel)",
                borderRadius: "var(--cs-radius-control)",
                border: "1px solid var(--cs-border-subtle)",
                padding: 2,
              }}
            >
              {kindFilterOptions.map((opt) => (
                <button
                  key={opt.id}
                  type="button"
                  onClick={() => setFilterKind(opt.id)}
                  style={{
                    padding: "3px 8px",
                    border: "none",
                    backgroundColor: filterKind === opt.id ? "var(--cs-bg-raised)" : "transparent",
                    color: filterKind === opt.id ? "var(--cs-fg-primary)" : "var(--cs-fg-secondary)",
                    fontWeight: filterKind === opt.id ? 600 : 400,
                    fontSize: 11,
                    borderRadius: "var(--cs-radius-control)",
                    cursor: "pointer",
                  }}
                >
                  {opt.label}
                </button>
              ))}
            </div>

            {/* Project dropdown (if projects available) */}
            {availableProjects.length > 0 && (
              <select
                value={filterProject}
                onChange={(e) => setFilterProject(e.target.value)}
                style={{
                  height: 26,
                  fontSize: 11,
                  backgroundColor: "var(--cs-bg-panel)",
                  color: "var(--cs-fg-primary)",
                  border: "1px solid var(--cs-border-subtle)",
                  borderRadius: "var(--cs-radius-control)",
                  padding: "0 6px",
                  cursor: "pointer",
                }}
              >
                <option value="all">Tất cả dự án</option>
                {availableProjects.map((p) => (
                  <option key={p} value={p}>
                    Dự án: {p}
                  </option>
                ))}
              </select>
            )}

            {/* Brand dropdown (if brands available) */}
            {availableBrands.length > 0 && (
              <select
                value={filterBrand}
                onChange={(e) => setFilterBrand(e.target.value)}
                style={{
                  height: 26,
                  fontSize: 11,
                  backgroundColor: "var(--cs-bg-panel)",
                  color: "var(--cs-fg-primary)",
                  border: "1px solid var(--cs-border-subtle)",
                  borderRadius: "var(--cs-radius-control)",
                  padding: "0 6px",
                  cursor: "pointer",
                }}
              >
                <option value="all">Tất cả thương hiệu</option>
                {availableBrands.map((b) => (
                  <option key={b} value={b}>
                    Brand: {b}
                  </option>
                ))}
              </select>
            )}
          </div>

          {/* Right: Search box */}
          <div style={{ width: 220 }}>
            <Input
              placeholder="Tìm theo tên, prompt..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              style={{ height: 26, fontSize: "var(--cs-text-xs)" }}
            />
          </div>
        </div>

        {/* Undo Toast / Status Banner */}
        {undoToast && (
          <div
            style={{
              padding: "var(--cs-space-2) var(--cs-space-5)",
              backgroundColor: "var(--cs-bg-raised)",
              borderBottom: "1px solid var(--cs-border-subtle)",
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              fontSize: "var(--cs-text-xs)",
              color: "var(--cs-fg-primary)",
            }}
          >
            <span>{undoToast.message}</span>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <Button variant="secondary" size="sm" onClick={handleUndo}>
                <Icon name="undo" size={12} />
                <span style={{ marginLeft: 4 }}>Hoàn tác</span>
              </Button>
              <button
                type="button"
                onClick={() => setUndoToast(null)}
                style={{ background: "transparent", border: "none", cursor: "pointer", color: "var(--cs-fg-muted)" }}
              >
                <Icon name="x" size={14} />
              </button>
            </div>
          </div>
        )}

        {/* Feedback message (error or success) */}
        {feedbackMsg && (
          <div
            style={{
              padding: "var(--cs-space-2) var(--cs-space-5)",
              backgroundColor: "rgba(239, 68, 68, 0.1)",
              borderBottom: "1px solid var(--cs-danger)",
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              fontSize: "var(--cs-text-xs)",
              color: "var(--cs-danger)",
            }}
          >
            <span>{feedbackMsg}</span>
            <button
              type="button"
              onClick={() => setFeedbackMsg(null)}
              style={{ background: "transparent", border: "none", cursor: "pointer", color: "var(--cs-danger)" }}
            >
              <Icon name="x" size={14} />
            </button>
          </div>
        )}

        {/* Main Body: List/Grid + Detail Panel */}
        <div style={{ flex: 1, display: "flex", minHeight: 0, overflow: "hidden" }}>
          {/* Main Area */}
          <div style={{ flex: 1, overflowY: "auto", padding: "var(--cs-space-4)", minWidth: 0 }}>
            {error ? (
              <div style={{ width: "100%", maxWidth: 800, margin: "0 auto" }}>
                <ErrorState
                  title="Tải thư viện tài nguyên"
                  reason={error}
                  safetyNote="Các tệp tài nguyên gốc nằm trong thư mục library/ và projects/*/assets/ của workspace."
                  onRetry={loadAssets}
                />
              </div>
            ) : loading ? (
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  height: "100%",
                  color: "var(--cs-fg-muted)",
                }}
              >
                Đang quét tệp thư viện...
              </div>
            ) : assets.length === 0 ? (
              <EmptyState
                icon={<Icon name="library" size={36} />}
                title="Không tìm thấy tài nguyên nào"
                description={
                  search || filterKind !== "all" || filterSource !== "all"
                    ? "Không có tệp nào phù hợp với bộ lọc hiện tại. Thử đổi bộ lọc hoặc từ khoá tìm kiếm."
                    : "Kéo-thả hình ảnh, video hoặc tài liệu vào đây hoặc bấm 'Nhập tệp' để bắt đầu."
                }
              />
            ) : viewMode === "grid" ? (
              <div
                style={{
                  display: "grid",
                  gridTemplateColumns: "repeat(auto-fill, minmax(180px, 1fr))",
                  gap: "var(--cs-space-3)",
                }}
              >
                {assets.map((asset) => (
                  <AssetCard
                    key={asset.id}
                    asset={asset}
                    isSelected={selectedAsset?.id === asset.id}
                    onClick={() => setSelectedAsset(asset)}
                  />
                ))}
              </div>
            ) : (
              /* List Mode */
              <div
                style={{
                  backgroundColor: "var(--cs-bg-panel)",
                  border: "1px solid var(--cs-border-subtle)",
                  borderRadius: "var(--cs-radius-control)",
                  overflow: "hidden",
                }}
              >
                {/* Table Header */}
                <div
                  style={{
                    display: "grid",
                    gridTemplateColumns: "40px minmax(200px, 2fr) 90px 100px 90px 110px auto",
                    gap: "var(--cs-space-3)",
                    padding: "8px var(--cs-space-3)",
                    backgroundColor: "var(--cs-bg-raised)",
                    borderBottom: "1px solid var(--cs-border-subtle)",
                    fontSize: 11,
                    fontWeight: 700,
                    color: "var(--cs-fg-muted)",
                    textTransform: "uppercase",
                  }}
                >
                  <span />
                  <span>Tên tệp</span>
                  <span>Nguồn</span>
                  <span>Dự án</span>
                  <span>Dung lượng</span>
                  <span>Ngày tạo</span>
                  <span>Thao tác</span>
                </div>

                {assets.map((asset) => (
                  <AssetRow
                    key={asset.id}
                    asset={asset}
                    isSelected={selectedAsset?.id === asset.id}
                    onClick={() => setSelectedAsset(asset)}
                    onUseInThread={handleUseInThread}
                    onOpenFolder={handleOpenFolder}
                  />
                ))}
              </div>
            )}
          </div>

          {/* Right Detail Panel */}
          {selectedAsset && (
            <AssetDetailPanel
              asset={selectedAsset}
              onClose={() => setSelectedAsset(null)}
              onUseInThread={handleUseInThread}
              onOpenFolder={handleOpenFolder}
              onDeleteAsset={handleDeleteAsset}
            />
          )}
        </div>
      </div>
    </AssetDropZone>
  );
};
