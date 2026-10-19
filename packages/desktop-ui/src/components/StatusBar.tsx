import React from "react";
import { Icon } from "./Icon";
import { cleanDisplayPath } from "../utils/path";
import type { EngineProbeResult, JobSnapshot, DiskUsageReport, PendingPermission } from "../api/types";

export interface StatusBarProps {
  workspacePath?: string;
  engines: EngineProbeResult[];
  jobs: JobSnapshot[];
  diskUsage?: DiskUsageReport;
  todayCostUsd?: number;
  todayBudgetUsd?: number | null;
  onOpenTaskCenter: () => void;
  onOpenSettings: (tab?: string) => void;
  onOpenWorkspaceFolder?: () => void;
  /** Permission requests waiting for an answer, in any thread. */
  pendingPermissions?: PendingPermission[];
  onOpenPendingPermission?: (p: PendingPermission) => void;
}

export const StatusBar: React.FC<StatusBarProps> = ({
  workspacePath,
  engines,
  jobs,
  diskUsage,
  todayCostUsd,
  todayBudgetUsd,
  onOpenTaskCenter,
  onOpenSettings,
  onOpenWorkspaceFolder,
  pendingPermissions,
  onOpenPendingPermission,
}) => {
  const [windowWidth, setWindowWidth] = React.useState<number>(
    typeof window !== "undefined" ? window.innerWidth : 1024
  );

  React.useEffect(() => {
    if (typeof window === "undefined") return;
    const handleResize = () => setWindowWidth(window.innerWidth);
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, []);

  const isUnder600 = windowWidth < 600;

  const runningJobs = jobs.filter((j) => j.state === "RUNNING");
  const failedJobs = jobs.filter((j) => j.state === "FAILED");

  const claudeProbe = engines.find((e) => e.id === "claude");
  const agyProbe = engines.find((e) => e.id === "agy");

  return (
    <footer
      style={{
        height: isUnder600 ? "auto" : "var(--cs-statusbar-h)",
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
        flexWrap: "wrap",
        gap: "var(--cs-space-2)",
      }}
    >
      {/* Left: Workspace directory (clickable to open folder/switcher) */}
      <div style={{ display: "flex", alignItems: "center", gap: 6, minWidth: 0 }}>
        <button
          type="button"
          onClick={onOpenWorkspaceFolder}
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: 6,
            background: "none",
            border: "none",
            color: "inherit",
            cursor: "pointer",
            fontSize: "inherit",
            padding: "2px 4px",
            borderRadius: "var(--cs-radius-control)",
            minWidth: 0,
            maxWidth: 340,
          }}
          title={workspacePath ? `Thư mục: ${workspacePath} (Bấm để mở Explorer / Đổi thư mục)` : "Chưa chọn workspace"}
        >
          <Icon name="folder" size={13} color="var(--cs-fg-muted)" />
          <span
            style={{
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {workspacePath ? `Thư mục: ${cleanDisplayPath(workspacePath)}` : "Chưa chọn workspace"}
          </span>
        </button>
      </div>

      {/* Center: Engines Probe status - hide under 600px */}
      {!isUnder600 && (
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <button
            type="button"
            onClick={() => onOpenSettings("engines")}
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
            onClick={() => onOpenSettings("engines")}
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
      )}

      {/* Right: Cost (if M07 available) + Disk Usage + Jobs snapshot */}
      <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
        {!isUnder600 && typeof todayCostUsd === "number" && (
          <button
            type="button"
            onClick={() => onOpenSettings("jobs")}
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: 4,
              background: "none",
              border: "none",
              cursor: "pointer",
              fontSize: "inherit",
              padding: "2px 4px",
              borderRadius: "var(--cs-radius-control)",
              color: todayBudgetUsd && todayCostUsd >= todayBudgetUsd ? "var(--cs-danger)" : "var(--cs-fg-secondary)",
              fontWeight: todayBudgetUsd && todayCostUsd >= todayBudgetUsd ? 600 : 400,
            }}
            title={
              todayBudgetUsd
                ? `Hôm nay: $${todayCostUsd.toFixed(2)} / $${todayBudgetUsd.toFixed(2)} (Bấm để đổi ngân sách)`
                : `Hôm nay: $${todayCostUsd.toFixed(2)} (Chưa đặt giới hạn ngày)`
            }
          >
            <Icon name="dollar-sign" size={12} color={todayBudgetUsd && todayCostUsd >= todayBudgetUsd ? "var(--cs-danger)" : "var(--cs-fg-muted)"} />
            <span>
              {todayBudgetUsd != null
                ? `Hôm nay $${todayCostUsd.toFixed(2)} / $${todayBudgetUsd.toFixed(2)}`
                : `$${todayCostUsd.toFixed(2)} hôm nay`}
            </span>
          </button>
        )}

        {!isUnder600 && diskUsage && (
          <button
            type="button"
            onClick={() => onOpenSettings("disk")}
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: 4,
              background: "none",
              border: "none",
              color: "inherit",
              cursor: "pointer",
              fontSize: "inherit",
              padding: "2px 4px",
              borderRadius: "var(--cs-radius-control)",
            }}
            title="Xem chi tiết dung lượng đĩa"
          >
            <Icon name="hard-drive" size={12} color="var(--cs-fg-muted)" />
            <span>Đĩa: {diskUsage.formatted.total}</span>
          </button>
        )}

        {pendingPermissions && pendingPermissions.length > 0 && (
          <button
            type="button"
            onClick={() => onOpenPendingPermission?.(pendingPermissions[0])}
            title="Agent đang chờ bạn cấp quyền — bấm để mở hội thoại"
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: 6,
              background: "none",
              border: "none",
              color: "var(--cs-warning)",
              fontWeight: 600,
              cursor: "pointer",
              fontSize: "inherit",
              padding: "2px 6px",
              borderRadius: "var(--cs-radius-control)",
            }}
          >
            <Icon name="shield" size={13} color="var(--cs-warning)" />
            <span>{pendingPermissions.length} chờ cấp quyền</span>
          </button>
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
