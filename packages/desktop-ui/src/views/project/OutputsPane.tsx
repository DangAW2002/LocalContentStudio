import React from "react";
import { Icon } from "../../components/Icon";
import { Tabs, Button, Badge, Dialog, EmptyState } from "@studio/design-system";
import { studioApi } from "../../api/client";
import { ProjectFolderTree } from "./ProjectFolderTree";
import { PublishDialog } from "../../components/PublishDialog";
import { CreateArtifactDialog } from "./CreateArtifactDialog";
import { ChannelMatrixView } from "./outputs/ChannelMatrixView";
import type {
  ArtifactSummary,
  CheckpointSummary,
  ProjectFileEntry,
  CatalogKit,
  CatalogModule,
} from "../../api/types";

export interface OutputsPaneProps {
  projectSlug: string;
  projectFiles: ProjectFileEntry[];
  onSelectFile: (path: string) => void;
  artifacts?: ArtifactSummary[];
  onRefreshArtifacts?: () => Promise<any> | any;
  onAskAgentToFix?: (promptText: string) => void;
}

export const OutputsPane: React.FC<OutputsPaneProps> = ({
  projectSlug,
  projectFiles,
  onSelectFile,
  artifacts: propsArtifacts,
  onRefreshArtifacts,
  onAskAgentToFix,
}) => {
  const [activeTab, setActiveTab] = React.useState<string>("artifacts");
  const [internalArtifacts, setInternalArtifacts] = React.useState<ArtifactSummary[]>([]);
  const artifacts = propsArtifacts ?? internalArtifacts;
  const [checkpoints, setCheckpoints] = React.useState<CheckpointSummary[]>([]);
  const [kits, setKits] = React.useState<CatalogKit[]>([]);
  const [modules, setModules] = React.useState<CatalogModule[]>([]);
  const [activeModuleIds, setActiveModuleIds] = React.useState<string[]>([]);
  // Budget as resolved by the backend (kit + modules); null until the first save.
  const [serverBudget, setServerBudget] = React.useState<number | null>(null);
  const [modulesError, setModulesError] = React.useState<string | null>(null);
  const [modulesSaving, setModulesSaving] = React.useState(false);
  const [restoreConfirmId, setRestoreConfirmId] = React.useState<string | null>(null);
  const [restoring, setRestoring] = React.useState(false);
  const [loading, setLoading] = React.useState(false);
  const [publishArtifact, setPublishArtifact] = React.useState<ArtifactSummary | null>(null);
  const [publishDialogOpen, setPublishDialogOpen] = React.useState(false);
  const [showCreateArtifactDialog, setShowCreateArtifactDialog] = React.useState(false);
  const [showHiddenFiles, setShowHiddenFiles] = React.useState(false);

  // Load artifacts, checkpoints and catalog info
  const loadArtifactsData = React.useCallback(async () => {
    if (onRefreshArtifacts) {
      await onRefreshArtifacts();
    }
    if (!projectSlug) return;
    try {
      const artList = await studioApi.listArtifacts(projectSlug).catch(() => []);
      setInternalArtifacts(artList);
    } catch {
      // ignore
    }
  }, [projectSlug, onRefreshArtifacts]);

  React.useEffect(() => {
    let isMounted = true;
    setLoading(true);

    const loadData = async () => {
      try {
        const [artList, cpList, kitList, modList, projects] = await Promise.all([
          studioApi.listArtifacts(projectSlug).catch(() => []),
          studioApi.getCheckpoints().catch(() => []),
          studioApi.getCatalogKits().catch(() => []),
          studioApi.getCatalogModules().catch(() => []),
          studioApi.getProjects().catch(() => []),
        ]);
        if (isMounted) {
          setInternalArtifacts(artList);
          setCheckpoints(cpList);
          setKits(kitList);
          setModules(modList);
          // Modules actually enabled in project.yaml.
          setActiveModuleIds(projects.find((p) => p.slug === projectSlug)?.modules ?? []);
          setServerBudget(null);
          setModulesError(null);
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

  // Refresh artifacts and checkpoints on fs-event or turn Done
  React.useEffect(() => {
    let unlistenFs: (() => void) | undefined;
    let unlistenTurn: (() => void) | undefined;

    const setupListeners = async () => {
      try {
        unlistenFs = await studioApi.onFsEvent(() => {
          loadArtifactsData();
          studioApi.getCheckpoints().then(setCheckpoints).catch(() => {});
        });
      } catch {
        // Core absent
      }

      try {
        unlistenTurn = await studioApi.onTurnEvent((payload) => {
          if (payload.event.type === "Done") {
            loadArtifactsData();
            studioApi.getCheckpoints().then(setCheckpoints).catch(() => {});
          }
        });
      } catch {
        // Core absent
      }
    };

    setupListeners();
    return () => {
      if (unlistenFs) unlistenFs();
      if (unlistenTurn) unlistenTurn();
    };
  }, [loadArtifactsData]);

  // Handle restore checkpoint
  const handleRestoreCheckpoint = async () => {
    if (!restoreConfirmId) return;
    setRestoring(true);
    try {
      await studioApi.restoreCheckpoint({ id: restoreConfirmId });
      setRestoreConfirmId(null);
      // Reload checkpoints & artifacts
      const [updatedCp, updatedArt] = await Promise.all([
        studioApi.getCheckpoints().catch(() => []),
        studioApi.listArtifacts(projectSlug).catch(() => []),
      ]);
      setCheckpoints(updatedCp);
      setInternalArtifacts(updatedArt);
      if (onRefreshArtifacts) {
        onRefreshArtifacts();
      }
    } catch (err: any) {
      alert(err?.message || "Không thể khôi phục checkpoint.");
    } finally {
      setRestoring(false);
    }
  };

  const ARTIFACT_KIND_VI: Record<string, string> = {
    design: "Thiết kế",
    motion: "Chuyển động",
    timeline: "Dòng thời gian",
    text: "Văn bản",
    generation: "AI sinh",
    audio: "Âm thanh",
  };

  const tabs = [
    { id: "artifacts", label: "Thành phẩm", count: artifacts.length > 0 ? artifacts.length : undefined },
    { id: "channels", label: "Kênh" },
    { id: "checkpoints", label: "Checkpoints", count: checkpoints.length > 0 ? checkpoints.length : undefined },
    { id: "modules", label: "Modules" },
  ];

  // Token budget calculations (ADR-010 limit: max 2,000 tokens)
  const MAX_TOKEN_BUDGET = 2000;
  const currentTokenUsage =
    serverBudget ??
    modules.filter((m) => activeModuleIds.includes(m.id)).reduce((sum, m) => sum + (m.tokenCount || 150), 300);

  const toggleModule = async (id: string) => {
    const prev = activeModuleIds;
    const next = prev.includes(id) ? prev.filter((m) => m !== id) : [...prev, id];
    setActiveModuleIds(next);
    setModulesSaving(true);
    setModulesError(null);
    try {
      const res = await studioApi.setProjectModules({ project: projectSlug, modules: next });
      setActiveModuleIds(res.modules);
      setServerBudget(res.totalTokenBudget);
    } catch (err: any) {
      setActiveModuleIds(prev);
      setModulesError(err?.message || "Không lưu được danh sách module.");
    } finally {
      setModulesSaving(false);
    }
  };

  const renderModuleRow = (mod: CatalogModule) => {
    const isActive = activeModuleIds.includes(mod.id);
    return (
      <div
        key={mod.id}
        style={{
          padding: "8px 10px",
          border: `1px solid ${isActive ? "var(--cs-accent)" : "var(--cs-border-subtle)"}`,
          backgroundColor: isActive ? "var(--cs-accent-subtle)" : "var(--cs-bg-raised)",
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          gap: 8,
          borderRadius: "var(--cs-radius-control)",
          fontSize: "var(--cs-text-xs)",
        }}
      >
        <div style={{ display: "flex", flexDirection: "column", gap: 2, minWidth: 0 }}>
          <span style={{ fontWeight: 600, color: "var(--cs-fg-primary)", overflow: "hidden", textOverflow: "ellipsis" }}>
            {mod.name}
          </span>
          <span style={{ color: "var(--cs-fg-muted)", fontSize: 11 }}>
            {mod.skillCount} skills · ~{mod.tokenCount} tokens
          </span>
        </div>
        <Button size="sm" variant="secondary" disabled={modulesSaving} onClick={() => toggleModule(mod.id)}>
          {isActive ? "Tắt" : "Bật"}
        </Button>
      </div>
    );
  };

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
            {/* Artifacts section */}
            <div
              style={{
                padding: "6px var(--cs-space-3)",
                fontSize: "var(--cs-text-xs)",
                fontWeight: 700,
                color: "var(--cs-fg-muted)",
                backgroundColor: "var(--cs-bg-panel)",
                borderBottom: "1px solid var(--cs-border-subtle)",
                textTransform: "uppercase",
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
              }}
            >
              <span>Artifacts ({artifacts.length})</span>
              <Button
                size="sm"
                variant="secondary"
                onClick={() => setShowCreateArtifactDialog(true)}
                leftIcon={<Icon name="plus" size={11} />}
              >
                Tạo artifact
              </Button>
            </div>

            {artifacts.length === 0 ? (
              <div
                style={{
                  padding: "var(--cs-space-3)",
                  fontSize: "var(--cs-text-xs)",
                  color: "var(--cs-fg-muted)",
                  borderBottom: "1px solid var(--cs-border-subtle)",
                  display: "flex",
                  flexDirection: "column",
                  gap: 8,
                  alignItems: "flex-start",
                }}
              >
                <span>Chưa có artifact nào được định nghĩa trong dự án.</span>
                <Button
                  size="sm"
                  variant="primary"
                  onClick={() => setShowCreateArtifactDialog(true)}
                  leftIcon={<Icon name="plus" size={11} />}
                >
                  Tạo artifact đầu tiên
                </Button>
              </div>
            ) : (
              artifacts.map((art) => (
                <div
                  key={art.id}
                  onClick={() => onSelectFile(art.source)}
                  style={{
                    display: "flex",
                    flexDirection: "column",
                    gap: 4,
                    padding: "8px var(--cs-space-3)",
                    borderBottom: "1px solid var(--cs-border-subtle)",
                    cursor: "pointer",
                    fontSize: "var(--cs-text-xs)",
                    transition: "background-color var(--cs-motion-duration) ease",
                  }}
                  onMouseEnter={(e) => (e.currentTarget.style.backgroundColor = "var(--cs-bg-hover)")}
                  onMouseLeave={(e) => (e.currentTarget.style.backgroundColor = "transparent")}
                >
                  <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 6 }}>
                    <div style={{ display: "flex", alignItems: "center", gap: 6, minWidth: 0 }}>
                      <Icon
                        name={
                          art.kind === "design"
                            ? "design"
                            : art.kind === "motion"
                            ? "motion"
                            : art.kind === "timeline"
                            ? "video"
                            : art.kind === "audio"
                            ? "audio"
                            : "package"
                        }
                        size={14}
                        color="var(--cs-accent)"
                      />
                      <span
                        style={{
                          fontWeight: 600,
                          color: "var(--cs-fg-primary)",
                          overflow: "hidden",
                          textOverflow: "ellipsis",
                          whiteSpace: "nowrap",
                        }}
                      >
                        {art.title || art.id}
                      </span>
                    </div>
                    <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                      <Badge variant="info">
                        {ARTIFACT_KIND_VI[art.kind] || art.kind}
                      </Badge>
                      <Button
                        size="sm"
                        variant="secondary"
                        onClick={(e) => {
                          e.stopPropagation();
                          setPublishArtifact(art);
                          setPublishDialogOpen(true);
                        }}
                      >
                        Đăng…
                      </Button>
                    </div>
                  </div>

                  <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", color: "var(--cs-fg-muted)", fontSize: 11 }}>
                    <span style={{ fontFamily: "var(--cs-font-mono)" }}>{art.source}</span>
                    {art.channels && art.channels.length > 0 && (
                      <div style={{ display: "flex", gap: 4 }}>
                        {art.channels.map((ch) => (
                          <span
                            key={ch}
                            style={{
                              padding: "1px 4px",
                              backgroundColor: "var(--cs-bg-raised)",
                              borderRadius: "var(--cs-radius-control)",
                              border: "1px solid var(--cs-border-subtle)",
                            }}
                          >
                            {ch}
                          </span>
                        ))}
                      </div>
                    )}
                  </div>
                </div>
              ))
            )}

            {/* Project files tree section */}
            {(() => {
              const isDotPath = (path: string) =>
                path.split("/").some((part) => part.startsWith("."));
              const visibleFiles = showHiddenFiles
                ? projectFiles
                : projectFiles.filter((f) => !isDotPath(f.path));

              return (
                <>
                  <div
                    style={{
                      padding: "6px var(--cs-space-3)",
                      fontSize: "var(--cs-text-xs)",
                      fontWeight: 700,
                      color: "var(--cs-fg-muted)",
                      backgroundColor: "var(--cs-bg-panel)",
                      borderBottom: "1px solid var(--cs-border-subtle)",
                      borderTop: "1px solid var(--cs-border-subtle)",
                      textTransform: "uppercase",
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "space-between",
                    }}
                  >
                    <span>Cây tệp dự án ({visibleFiles.length})</span>
                    <label
                      style={{
                        display: "inline-flex",
                        alignItems: "center",
                        gap: 4,
                        fontSize: 11,
                        fontWeight: 500,
                        textTransform: "none",
                        cursor: "pointer",
                        color: "var(--cs-fg-secondary)",
                        userSelect: "none",
                      }}
                    >
                      <input
                        type="checkbox"
                        checked={showHiddenFiles}
                        onChange={(e) => setShowHiddenFiles(e.target.checked)}
                        style={{ cursor: "pointer", margin: 0 }}
                      />
                      Hiện tệp ẩn
                    </label>
                  </div>

                  <ProjectFolderTree
                    files={visibleFiles}
                    onSelectFile={onSelectFile}
                  />
                </>
              );
            })()}
          </div>
        )}

        {activeTab === "channels" && (
          <ChannelMatrixView
            projectSlug={projectSlug}
            artifacts={artifacts}
            onSelectFile={onSelectFile}
            onAskAgentToFix={onAskAgentToFix}
            onRefreshArtifacts={onRefreshArtifacts}
          />
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
                  <div style={{ display: "flex", alignItems: "center", gap: 8, color: "var(--cs-fg-muted)" }} title={cp.id}>
                    <span>{formatCheckpointTime(cp.timestamp)}</span>
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
              <div style={{ display: "flex", alignItems: "baseline", justifyContent: "space-between", flexWrap: "wrap", gap: "2px 8px", fontSize: "var(--cs-text-xs)" }}>
                <span style={{ fontWeight: 600, whiteSpace: "nowrap" }}>Ngân sách token mỗi lượt</span>
                <span style={{ fontFamily: "var(--cs-font-mono)", fontWeight: 600, whiteSpace: "nowrap", color: budgetPercent > 80 ? "var(--cs-danger)" : "var(--cs-accent)" }}>
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

            {/* Modules: enabled first, then the rest of the catalog. Toggling saves to project.yaml. */}
            {modulesError && (
              <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-danger)" }}>{modulesError}</div>
            )}
            {modules.length === 0 ? (
              <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
                Không có module nào trong catalog.
              </div>
            ) : (
              <>
                <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                  <span style={moduleGroupTitleStyle}>
                    Đang bật ({modules.filter((m) => activeModuleIds.includes(m.id)).length})
                  </span>
                  {activeModuleIds.length === 0 ? (
                    <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
                      Chưa bật module nào — agent chỉ dùng kit mặc định của dự án.
                    </div>
                  ) : (
                    modules.filter((m) => activeModuleIds.includes(m.id)).map(renderModuleRow)
                  )}
                </div>
                <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                  <span style={moduleGroupTitleStyle}>Có sẵn</span>
                  {modules.filter((m) => !activeModuleIds.includes(m.id)).map(renderModuleRow)}
                </div>
              </>
            )}
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

      {/* Publish Dialog */}
      <PublishDialog
        open={publishDialogOpen}
        onClose={() => {
          setPublishDialogOpen(false);
          setPublishArtifact(null);
        }}
        projectSlug={projectSlug}
        artifact={publishArtifact}
      />

      {/* Create Artifact Dialog */}
      <CreateArtifactDialog
        open={showCreateArtifactDialog}
        onClose={() => setShowCreateArtifactDialog(false)}
        projectSlug={projectSlug}
        existingIds={artifacts.map((a) => a.id)}
        onCreated={async (sourcePath) => {
          if (onRefreshArtifacts) {
            await onRefreshArtifacts();
          }
          await loadArtifactsData();
          onSelectFile(sourcePath);
        }}
      />
    </div>
  );
};

const moduleGroupTitleStyle: React.CSSProperties = {
  fontSize: "var(--cs-text-xs)",
  fontWeight: 700,
  textTransform: "uppercase",
  color: "var(--cs-fg-muted)",
};

/** "18/10 14:27" — day/month then time, as Vietnamese readers expect. */
export function formatCheckpointTime(ts: string): string {
  const d = new Date(ts);
  if (Number.isNaN(d.getTime())) return ts;
  const two = (n: number) => String(n).padStart(2, "0");
  return `${two(d.getDate())}/${two(d.getMonth() + 1)} ${two(d.getHours())}:${two(d.getMinutes())}`;
}
