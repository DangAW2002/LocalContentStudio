import React from "react";
import { Icon } from "./Icon";
import { cleanDisplayPath } from "../utils/path";
import type { EngineProbeResult, JobSnapshot, DiskUsageReport } from "../api/types";

export interface StatusBarProps {
  workspacePath?: string;
  engines: EngineProbeResult[];
  jobs: JobSnapshot[];
  diskUsage?: DiskUsageReport;
  onOpenTaskCenter: () => void;
  onOpenSettings: () => void;
}

export const StatusBar: React.FC<StatusBarProps> = ({
  workspacePath,
  engines,
  jobs,
  diskUsage,
  onOpenTaskCenter,
  onOpenSettings,
}) => {
  const runningJobs = jobs.filter((j) => j.state === "RUNNING");
  const failedJobs = jobs.filter((j) => j.state === "FAILED");

  const claudeProbe = engines.find((e) => e.id === "claude");
  const agyProbe = engines.find((e) => e.id === "agy");

  return (
    <footer
      style={{
        height: "var(--cs-statusbar-h)",
        minHeight: "var(--cs-statusbar-h)",
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        padding: "0 var(--cs-space-3)",
        backgroundColor: "var(--cs-bg-panel)",
        borderTop: "1px solid var(--cs-border-subtle)",
        fontSize: "var(--cs-text-xs)",
        color: "var(--cs-fg-secondary)",
        userSelect: "none",
        zIndex: 20,
      }}
    >
      {/* Left: Workspace directory */}
      <div style={{ display: "flex", alignItems: "center", gap: 6, minWidth: 0 }}>
        <Icon name="folder" size={13} color="var(--cs-fg-muted)" />
        <span
          style={{
            maxWidth: 340,
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}
          title={cleanDisplayPath(workspacePath) || "Chưa mở workspace"}
        >
          {workspacePath ? `Thư mục: ${cleanDisplayPath(workspacePath)}` : "Chưa chọn workspace"}
        </span>
      </div>

      {/* Center: Engines Probe status */}
      <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
        <button
          type="button"
          onClick={onOpenSettings}
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: 5,
            background: "none",
            border: "none",
            color: "inherit",
            cursor: "pointer",
            fontSize: "inherit",
            padding: "2px 4px",
            borderRadius: "var(--cs-radius-control)",
          }}
          title="Xem chi tiết engine (Claude & agy)"
        >
          <span
            style={{
              width: 7,
              height: 7,
              borderRadius: "50%",
              backgroundColor: claudeProbe?.installed ? "var(--cs-success)" : "var(--cs-warning)",
            }}
          />
          <span>Claude: {claudeProbe?.installed ? (claudeProbe.version || "Sẵn sàng") : "Chưa cài"}</span>
        </button>

        <span style={{ color: "var(--cs-border-subtle)" }}>|</span>

        <button
          type="button"
          onClick={onOpenSettings}
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: 5,
            background: "none",
            border: "none",
            color: "inherit",
            cursor: "pointer",
            fontSize: "inherit",
            padding: "2px 4px",
            borderRadius: "var(--cs-radius-control)",
          }}
          title="Xem chi tiết agy"
        >
          <span
            style={{
              width: 7,
              height: 7,
              borderRadius: "50%",
              backgroundColor: agyProbe?.installed ? "var(--cs-success)" : "var(--cs-warning)",
            }}
          />
          <span>agy: {agyProbe?.installed ? (agyProbe.version || "Sẵn sàng") : "Chưa cài"}</span>
        </button>
      </div>

      {/* Right: Jobs snapshot + Disk Usage */}
      <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
        {diskUsage && (
          <span style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
            <Icon name="hard-drive" size={12} color="var(--cs-fg-muted)" />
            <span>Đĩa: {diskUsage.formatted.total}</span>
          </span>
        )}

        <button
          type="button"
          onClick={onOpenTaskCenter}
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: 6,
            background: "none",
            border: "none",
            color: runningJobs.length > 0 ? "var(--cs-accent)" : failedJobs.length > 0 ? "var(--cs-danger)" : "inherit",
            fontWeight: runningJobs.length > 0 ? 600 : 400,
            cursor: "pointer",
            fontSize: "inherit",
            padding: "2px 6px",
            borderRadius: "var(--cs-radius-control)",
          }}
        >
          {runningJobs.length > 0 ? (
            <>
              <Icon name="activity" size={13} color="var(--cs-accent)" />
              <span>{runningJobs.length} tác vụ đang chạy</span>
            </>
          ) : failedJobs.length > 0 ? (
            <>
              <Icon name="alert-circle" size={13} color="var(--cs-danger)" />
              <span>{failedJobs.length} tác vụ lỗi</span>
            </>
          ) : (
            <>
              <Icon name="check" size={13} color="var(--cs-fg-muted)" />
              <span>0 tác vụ đang chạy</span>
            </>
          )}
        </button>
      </div>
    </footer>
  );
};
