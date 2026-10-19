import React from "react";
import { Icon } from "../../../components/Icon";
import { Button, Badge } from "@studio/design-system";
import { studioApi } from "../../../api/client";
import type {
  ArtifactChannelMatrixRow,
  ArtifactChannelMatrixCell,
  ArtifactSummary,
} from "../../../api/types";
import { ChannelCellPopover } from "./ChannelCellPopover";

export interface ChannelMatrixViewProps {
  projectSlug: string;
  artifacts?: ArtifactSummary[];
  onSelectFile?: (path: string) => void;
  onAskAgentToFix?: (promptText: string) => void;
  onRefreshArtifacts?: () => Promise<any> | any;
}

export const ChannelMatrixView: React.FC<ChannelMatrixViewProps> = ({
  projectSlug,
  onSelectFile,
  onAskAgentToFix,
  onRefreshArtifacts,
}) => {
  const [matrixRows, setMatrixRows] = React.useState<ArtifactChannelMatrixRow[]>([]);
  const [loading, setLoading] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  // Popover state
  const [selectedCell, setSelectedCell] = React.useState<{
    cell: ArtifactChannelMatrixCell;
    artifact: ArtifactChannelMatrixRow;
  } | null>(null);

  // Load matrix
  const loadMatrix = React.useCallback(async () => {
    if (!projectSlug) return;
    setLoading(true);
    setError(null);
    try {
      const rows = await studioApi.getArtifactChannelMatrix(projectSlug);
      setMatrixRows(rows);
    } catch (err: any) {
      setError(err?.message || "Không thể tải ma trận kênh.");
    } finally {
      setLoading(false);
    }
  }, [projectSlug]);

  React.useEffect(() => {
    loadMatrix();
  }, [loadMatrix]);

  // Auto-refresh when job-event (render succeeded) or fs-event (channels changed) occurs
  React.useEffect(() => {
    let unlistenFs: (() => void) | undefined;
    let unlistenJob: (() => void) | undefined;

    const setup = async () => {
      try {
        unlistenFs = await studioApi.onFsEvent((payload) => {
          const channelsChanged = payload.paths.some(
            (p) => p.includes("channels") || p.includes("artifacts") || p.endsWith("artifact.yaml")
          );
          if (channelsChanged) {
            loadMatrix();
          }
        });
      } catch {
        // ignore
      }

      try {
        unlistenJob = await studioApi.onJobEvent((payload) => {
          if (payload.state === "succeeded") {
            loadMatrix();
          }
        });
      } catch {
        // ignore
      }
    };

    setup();
    return () => {
      if (unlistenFs) unlistenFs();
      if (unlistenJob) unlistenJob();
    };
  }, [loadMatrix]);

  // Extract distinct channel columns across all rows
  const distinctChannels = React.useMemo(() => {
    const map = new Map<string, string>();
    for (const row of matrixRows) {
      for (const cell of row.channels) {
        if (!map.has(cell.channel_id)) {
          map.set(cell.channel_id, cell.channel_name);
        }
      }
    }
    return Array.from(map.entries()).map(([id, name]) => ({ id, name }));
  }, [matrixRows]);

  const renderCellBadge = (cell: ArtifactChannelMatrixCell) => {
    switch (cell.status) {
      case "passed":
        return (
          <span
            style={{
              display: "inline-flex",
              alignItems: "center",
              justifyContent: "center",
              width: 22,
              height: 22,
              borderRadius: "50%",
              backgroundColor: "rgba(34, 197, 94, 0.15)",
              color: "var(--cs-success)",
              fontWeight: 700,
              fontSize: 12,
            }}
            title={`Đạt tiêu chuẩn (${cell.channel_name})`}
          >
            ✓
          </span>
        );
      case "warning":
        return (
          <span
            style={{
              display: "inline-flex",
              alignItems: "center",
              justifyContent: "center",
              width: 22,
              height: 22,
              borderRadius: "50%",
              backgroundColor: "rgba(234, 179, 8, 0.15)",
              color: "var(--cs-warning)",
              fontWeight: 700,
              fontSize: 12,
            }}
            title={`${cell.warnings_count} cảnh báo (${cell.channel_name})`}
          >
            ⚠
          </span>
        );
      case "error":
        return (
          <span
            style={{
              display: "inline-flex",
              alignItems: "center",
              justifyContent: "center",
              width: 22,
              height: 22,
              borderRadius: "50%",
              backgroundColor: "rgba(239, 68, 68, 0.15)",
              color: "var(--cs-danger)",
              fontWeight: 700,
              fontSize: 12,
            }}
            title={`${cell.errors_count} lỗi (${cell.channel_name})`}
          >
            ✗
          </span>
        );
      case "unrendered":
      default:
        return (
          <span
            style={{
              display: "inline-flex",
              alignItems: "center",
              justifyContent: "center",
              width: 22,
              height: 22,
              color: "var(--cs-fg-muted)",
              fontWeight: 600,
              fontSize: 13,
            }}
            title="Chưa render"
          >
            —
          </span>
        );
    }
  };

  const getChannelShort = (id: string, name: string) => {
    if (id.includes("youtube") || name.toLowerCase().includes("youtube")) return "YT";
    if (id.includes("tiktok") || name.toLowerCase().includes("tiktok")) return "TT";
    if (id.includes("facebook") || name.toLowerCase().includes("facebook")) return "FB";
    if (id.includes("insta") || name.toLowerCase().includes("insta")) return "IG";
    return id.slice(0, 2).toUpperCase();
  };

  const handleRecheck = async () => {
    if (!selectedCell) return;
    try {
      const art = selectedCell.artifact;
      const res = await studioApi.checkArtifact({
        project: projectSlug,
        id: art.artifact_id,
        channels: [selectedCell.cell.channel_id],
      });
      // Update cell in local state
      const chRes = res.channels[selectedCell.cell.channel_id];
      if (chRes) {
        const hasErr = chRes.findings.some((f) => f.severity === "error");
        const hasWarn = chRes.findings.some((f) => f.severity === "warn");
        const updatedCell: ArtifactChannelMatrixCell = {
          ...selectedCell.cell,
          status: hasErr ? "error" : hasWarn ? "warning" : "passed",
          errors_count: chRes.findings.filter((f) => f.severity === "error").length,
          warnings_count: chRes.findings.filter((f) => f.severity === "warn").length,
          findings: chRes.findings,
        };
        setSelectedCell({
          ...selectedCell,
          cell: updatedCell,
        });
      }
      loadMatrix();
    } catch {
      // ignore
    }
  };

  const handleRender = async () => {
    if (!selectedCell) return;
    try {
      await studioApi.renderArtifact({
        project: projectSlug,
        id: selectedCell.artifact.artifact_id,
      });
      if (onRefreshArtifacts) {
        onRefreshArtifacts();
      }
      loadMatrix();
      setSelectedCell(null);
    } catch (err: any) {
      alert(err?.message || "Lỗi khi render artifact.");
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      {/* Header toolbar */}
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
        <span>Ma trận kênh ({matrixRows.length})</span>
        <Button
          size="sm"
          variant="secondary"
          onClick={loadMatrix}
          busy={loading}
          leftIcon={<Icon name="refresh" size={11} />}
        >
          Làm mới
        </Button>
      </div>

      {/* Body */}
      <div style={{ flex: 1, overflow: "auto", minHeight: 0, padding: "var(--cs-space-2)" }}>
        {error ? (
          <div style={{ color: "var(--cs-danger)", fontSize: "var(--cs-text-xs)", padding: 8 }}>
            {error}
          </div>
        ) : matrixRows.length === 0 ? (
          <div style={{ padding: "var(--cs-space-3)", fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
            Chưa có artifact nào trong dự án để hiển thị ma trận kiểm tra kênh.
          </div>
        ) : (
          <div style={{ width: "100%", overflowX: "auto" }}>
            <table
              style={{
                width: "100%",
                borderCollapse: "collapse",
                fontSize: "var(--cs-text-xs)",
                tableLayout: "fixed",
              }}
            >
              <thead>
                <tr style={{ borderBottom: "1px solid var(--cs-border-subtle)" }}>
                  <th
                    style={{
                      textAlign: "left",
                      padding: "6px 8px",
                      color: "var(--cs-fg-muted)",
                      fontWeight: 600,
                      width: "55%",
                    }}
                  >
                    Artifact
                  </th>
                  {distinctChannels.map((ch) => (
                    <th
                      key={ch.id}
                      style={{
                        textAlign: "center",
                        padding: "6px 4px",
                        color: "var(--cs-fg-muted)",
                        fontWeight: 600,
                        width: `${45 / Math.max(1, distinctChannels.length)}%`,
                      }}
                      title={`${ch.name} (${ch.id})`}
                    >
                      <span
                        style={{
                          cursor: "help",
                          display: "inline-block",
                          padding: "2px 4px",
                          borderRadius: "var(--cs-radius-control)",
                          backgroundColor: "var(--cs-bg-raised)",
                        }}
                      >
                        {getChannelShort(ch.id, ch.name)}
                      </span>
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {matrixRows.map((row) => (
                  <tr
                    key={row.artifact_id}
                    style={{
                      borderBottom: "1px solid var(--cs-border-subtle)",
                      transition: "background-color 0.15s ease",
                    }}
                  >
                    <td
                      style={{
                        padding: "8px 8px",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                        cursor: onSelectFile ? "pointer" : "default",
                      }}
                      onClick={() => onSelectFile && onSelectFile(row.source)}
                      title={`${row.artifact_title} (${row.artifact_id})`}
                    >
                      <div style={{ display: "flex", flexDirection: "column", minWidth: 0 }}>
                        <span style={{ fontWeight: 600, color: "var(--cs-fg-primary)", overflow: "hidden", textOverflow: "ellipsis" }}>
                          {row.artifact_title || row.artifact_id}
                        </span>
                        <span style={{ fontSize: 10, color: "var(--cs-fg-muted)", fontFamily: "var(--cs-font-mono)" }}>
                          {row.artifact_id}
                        </span>
                      </div>
                    </td>

                    {distinctChannels.map((ch) => {
                      const cell = row.channels.find((c) => c.channel_id === ch.id);
                      if (!cell) {
                        return (
                          <td
                            key={ch.id}
                            style={{ textAlign: "center", padding: "6px 4px", color: "var(--cs-fg-muted)" }}
                          >
                            —
                          </td>
                        );
                      }

                      return (
                        <td
                          key={ch.id}
                          style={{
                            textAlign: "center",
                            padding: "6px 4px",
                            cursor: "pointer",
                          }}
                          onClick={() => setSelectedCell({ cell, artifact: row })}
                        >
                          <button
                            type="button"
                            style={{
                              background: "none",
                              border: "none",
                              padding: 0,
                              cursor: "pointer",
                              display: "inline-flex",
                              alignItems: "center",
                              justifyContent: "center",
                            }}
                          >
                            {renderCellBadge(cell)}
                          </button>
                        </td>
                      );
                    })}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      {/* Popover dialog */}
      <ChannelCellPopover
        open={Boolean(selectedCell)}
        onClose={() => setSelectedCell(null)}
        artifactTitle={selectedCell?.artifact.artifact_title || selectedCell?.artifact.artifact_id || ""}
        sourcePath={selectedCell?.artifact.source || "index.html"}
        cell={selectedCell?.cell || null}
        onAskAgentToFix={onAskAgentToFix}
        onRecheck={handleRecheck}
        onRender={handleRender}
      />
    </div>
  );
};
