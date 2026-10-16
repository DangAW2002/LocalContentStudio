import React from "react";
import { Icon } from "../../components/Icon";
import { Tabs, Button, Badge, Dialog, EmptyState } from "@studio/design-system";
import { studioApi } from "../../api/client";
import type {
  CheckpointSummary,
  ProjectFileEntry,
  CatalogKit,
  CatalogModule,
} from "../../api/types";

export interface OutputsPaneProps {
  projectSlug: string;
  projectFiles: ProjectFileEntry[];
  onSelectFile: (path: string) => void;
}

export const OutputsPane: React.FC<OutputsPaneProps> = ({
  projectSlug,
  projectFiles,
  onSelectFile,
}) => {
  const [activeTab, setActiveTab] = React.useState<string>("artifacts");
  const [checkpoints, setCheckpoints] = React.useState<CheckpointSummary[]>([]);
  const [kits, setKits] = React.useState<CatalogKit[]>([]);
  const [modules, setModules] = React.useState<CatalogModule[]>([]);
  const [activeModuleIds, setActiveModuleIds] = React.useState<string[]>([]);
  const [restoreConfirmId, setRestoreConfirmId] = React.useState<string | null>(null);
  const [restoring, setRestoring] = React.useState(false);
  const [loading, setLoading] = React.useState(false);

  // Load checkpoints and catalog info
  React.useEffect(() => {
    let isMounted = true;
    setLoading(true);

    const loadData = async () => {
      try {
        const [cpList, kitList, modList] = await Promise.all([
          studioApi.getCheckpoints().catch(() => []),
          studioApi.getCatalogKits().catch(() => []),
          studioApi.getCatalogModules().catch(() => []),
        ]);
        if (isMounted) {
          setCheckpoints(cpList);
          setKits(kitList);
          setModules(modList);
          // Default initial active modules
          setActiveModuleIds(modList.slice(0, 3).map((m) => m.id));
        }
      } finally {
        if (isMounted) setLoading(false);
      }
    };

    loadData();
    return () => {
      isMounted = false;
    };
  }, [projectSlug]);

  // Handle restore checkpoint
  const handleRestoreCheckpoint = async () => {
    if (!restoreConfirmId) return;
    setRestoring(true);
    try {
      await studioApi.restoreCheckpoint({ id: restoreConfirmId });
      setRestoreConfirmId(null);
      // Reload checkpoints
      const updated = await studioApi.getCheckpoints();
      setCheckpoints(updated);
    } catch (err: any) {
      alert(err?.message || "Không thể khôi phục checkpoint.");
    } finally {
      setRestoring(false);
    }
  };

  const tabs = [
    { id: "artifacts", label: "Thành phẩm", count: projectFiles.length },
    { id: "checkpoints", label: "Checkpoints", count: checkpoints.length },
    { id: "modules", label: "Modules & Ngân sách" },
  ];

  // Token budget calculations (ADR-010 limit: max 2,000 tokens)
  const MAX_TOKEN_BUDGET = 2000;
  const currentTokenUsage = modules
    .filter((m) => activeModuleIds.includes(m.id))
    .reduce((sum, m) => sum + (m.tokenCount || 150), 300);

  const budgetPercent = Math.min(100, Math.round((currentTokenUsage / MAX_TOKEN_BUDGET) * 100));

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        backgroundColor: "var(--cs-bg-panel)",
        borderLeft: "1px solid var(--cs-border-subtle)",
        overflow: "hidden",
      }}
    >
      {/* Outputs Header & Tabs */}
      <div
        style={{
          padding: "var(--cs-space-2) var(--cs-space-3)",
          borderBottom: "1px solid var(--cs-border-subtle)",
          backgroundColor: "var(--cs-bg-raised)",
        }}
      >
        <Tabs
          items={tabs}
          activeId={activeTab}
          onChange={setActiveTab}
          size="sm"
        />
      </div>

      {/* Tab Content Body */}
      <div style={{ flex: 1, overflowY: "auto", minHeight: 0 }}>
        {activeTab === "artifacts" && (
          <div style={{ display: "flex", flexDirection: "column" }}>
            {projectFiles.length === 0 ? (
              <EmptyState
                icon={<Icon name="package" size={24} />}
                title="Chưa có thành phẩm"
                description="Khi agent tạo hoặc xuất bản artifact (ảnh, video, văn bản), tệp sẽ xuất hiện tại đây."
              />
            ) : (
              <div style={{ display: "flex", flexDirection: "column" }}>
                <div
                  style={{
                    padding: "6px var(--cs-space-3)",
                    fontSize: "var(--cs-text-xs)",
                    fontWeight: 700,
                    color: "var(--cs-fg-muted)",
                    backgroundColor: "var(--cs-bg-panel)",
                    borderBottom: "1px solid var(--cs-border-subtle)",
                    textTransform: "uppercase",
                  }}
                >
                  Cây tệp dự án ({projectFiles.length})
                </div>
                {projectFiles.map((file) => (
                  <div
                    key={file.path}
                    onClick={() => onSelectFile(file.path)}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "space-between",
                      padding: "8px var(--cs-space-3)",
                      borderBottom: "1px solid var(--cs-border-subtle)",
                      cursor: "pointer",
                      fontSize: "var(--cs-text-xs)",
                      transition: "background-color var(--cs-motion-duration) ease",
                    }}
                    onMouseEnter={(e) => (e.currentTarget.style.backgroundColor = "var(--cs-bg-hover)")}
                    onMouseLeave={(e) => (e.currentTarget.style.backgroundColor = "transparent")}
                  >
                    <div style={{ display: "flex", alignItems: "center", gap: 6, minWidth: 0 }}>
                      <Icon
                        name={
                          file.kind === "image"
                            ? "image"
                            : file.kind === "video"
                            ? "video"
                            : file.kind === "audio"
                            ? "audio"
                            : file.kind === "markdown" || file.kind === "text"
                            ? "file-text"
                            : "file"
                        }
                        size={14}
                        color="var(--cs-accent)"
                      />
                      <span
                        style={{
                          fontWeight: 500,
                          fontFamily: "var(--cs-font-mono)",
                          overflow: "hidden",
                          textOverflow: "ellipsis",
                          whiteSpace: "nowrap",
                        }}
                      >
                        {file.path}
                      </span>
                    </div>

                    <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                      <Badge variant="neutral">{file.kind}</Badge>
                      <span style={{ color: "var(--cs-fg-muted)" }}>
                        {(file.size / 1024).toFixed(1)}k
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {activeTab === "checkpoints" && (
          <div style={{ display: "flex", flexDirection: "column" }}>
            {checkpoints.length === 0 ? (
              <EmptyState
                icon={<Icon name="history" size={24} />}
                title="Chưa có checkpoint nào"
                description="Checkpoint được tự động tạo trước mỗi lượt agent hoặc khi lưu thay đổi."
              />
            ) : (
              checkpoints.map((cp) => (
                <div
                  key={cp.id}
                  style={{
                    padding: "var(--cs-space-3)",
                    borderBottom: "1px solid var(--cs-border-subtle)",
                    display: "flex",
                    flexDirection: "column",
                    gap: 4,
                    fontSize: "var(--cs-text-xs)",
                  }}
                >
                  <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                    <span style={{ fontWeight: 600, color: "var(--cs-fg-primary)" }}>
                      {cp.message}
                    </span>
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={() => setRestoreConfirmId(cp.id)}
                    >
                      Phục hồi
                    </Button>
                  </div>
                  <div style={{ display: "flex", alignItems: "center", gap: 8, color: "var(--cs-fg-muted)" }}>
                    <code style={{ fontFamily: "var(--cs-font-mono)", fontSize: 11 }}>
                      {cp.id.slice(0, 8)}...
                    </code>
                    <span>·</span>
                    <span>{new Date(cp.timestamp).toLocaleTimeString("vi-VN")}</span>
                    <span>·</span>
                    <span>{cp.filesChanged} tệp thay đổi</span>
                  </div>
                </div>
              ))
            )}
          </div>
        )}

        {activeTab === "modules" && (
          <div style={{ padding: "var(--cs-space-4)", display: "flex", flexDirection: "column", gap: "var(--cs-space-4)" }}>
            {/* Token Budget Card */}
            <div
              style={{
                padding: "var(--cs-space-3)",
                backgroundColor: "var(--cs-bg-raised)",
                border: "1px solid var(--cs-border-subtle)",
                borderRadius: "var(--cs-radius-control)",
                display: "flex",
                flexDirection: "column",
                gap: 6,
              }}
            >
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", fontSize: "var(--cs-text-xs)" }}>
                <span style={{ fontWeight: 600 }}>Ngân sách Token mỗi lượt</span>
                <span style={{ fontFamily: "var(--cs-font-mono)", fontWeight: 600, color: budgetPercent > 80 ? "var(--cs-danger)" : "var(--cs-accent)" }}>
                  {currentTokenUsage.toLocaleString()} / {MAX_TOKEN_BUDGET.toLocaleString()} tokens ({budgetPercent}%)
                </span>
              </div>
              <div
                style={{
                  height: 6,
                  width: "100%",
                  backgroundColor: "var(--cs-progress-track)",
                  borderRadius: "var(--cs-radius-round)",
                  overflow: "hidden",
                }}
              >
                <div
                  style={{
                    height: "100%",
                    width: `${budgetPercent}%`,
                    backgroundColor: budgetPercent > 80 ? "var(--cs-danger)" : "var(--cs-progress-fill)",
                  }}
                />
              </div>
              <span style={{ fontSize: 11, color: "var(--cs-fg-muted)" }}>
                Giới hạn ≤ 2.000 tokens theo chuẩn ADR-010 để tránh ngợp ngữ cảnh agent.
              </span>
            </div>

            {/* Modules List */}
            <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
              <span style={{ fontSize: "var(--cs-text-xs)", fontWeight: 700, textTransform: "uppercase", color: "var(--cs-fg-muted)" }}>
                Modules đang kích hoạt
              </span>
              {modules.length === 0 ? (
                <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
                  Không có module nào trong catalog.
                </div>
              ) : (
                modules.map((mod) => {
                  const isActive = activeModuleIds.includes(mod.id);
                  return (
                    <div
                      key={mod.id}
                      style={{
                        padding: "8px 10px",
                        border: "1px solid var(--cs-border-subtle)",
                        backgroundColor: isActive ? "var(--cs-bg-panel)" : "var(--cs-bg-raised)",
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "space-between",
                        borderRadius: "var(--cs-radius-control)",
                        fontSize: "var(--cs-text-xs)",
                      }}
                    >
                      <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                        <span style={{ fontWeight: 600, color: "var(--cs-fg-primary)" }}>{mod.name}</span>
                        <span style={{ color: "var(--cs-fg-muted)", fontSize: 11 }}>
                          {mod.skillCount} skills · ~{mod.tokenCount} tokens
                        </span>
                      </div>
                      <Button
                        size="sm"
                        variant={isActive ? "secondary" : "quiet"}
                        onClick={() => {
                          if (isActive) {
                            setActiveModuleIds((prev) => prev.filter((id) => id !== mod.id));
                          } else {
                            setActiveModuleIds((prev) => [...prev, mod.id]);
                          }
                        }}
                      >
                        {isActive ? "Bỏ chọn" : "Bật"}
                      </Button>
                    </div>
                  );
                })
              )}
            </div>
          </div>
        )}
      </div>

      {/* Restore Confirm Dialog */}
      {restoreConfirmId && (
        <Dialog
          title="Khôi phục trạng thái tệp?"
          open={Boolean(restoreConfirmId)}
          onClose={() => setRestoreConfirmId(null)}
          actions={
            <>
              <Button variant="secondary" onClick={() => setRestoreConfirmId(null)}>
                Hủy
              </Button>
              <Button
                variant="destructive"
                busy={restoring}
                onClick={handleRestoreCheckpoint}
              >
                Khôi phục tệp
              </Button>
            </>
          }
        >
          <p style={{ margin: 0, fontSize: "var(--cs-text-sm)", lineHeight: 1.5 }}>
            Toàn bộ tệp source của dự án sẽ được đưa về checkpoint <code>{restoreConfirmId.slice(0, 10)}</code>. Mọi chỉnh sửa chưa lưu hoặc chưa commit sẽ bị ghi đè.
          </p>
        </Dialog>
      )}
    </div>
  );
};
