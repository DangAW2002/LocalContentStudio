import React from "react";
import { Icon } from "./Icon";
import { Tabs, Badge, EmptyState, Button } from "@studio/design-system";
import type { JobSnapshot } from "../api/types";

export interface TaskCenterProps {
  open: boolean;
  onClose: () => void;
  jobs: JobSnapshot[];
}

export const TaskCenter: React.FC<TaskCenterProps> = ({
  open,
  onClose,
  jobs,
}) => {
  const [activeTab, setActiveTab] = React.useState<string>("running");

  if (!open) return null;

  const runningJobs = jobs.filter((j) => j.state === "RUNNING" || j.state === "PENDING" || j.state === "VERIFYING");
  const failedJobs = jobs.filter((j) => j.state === "FAILED");
  const completedJobs = jobs.filter((j) => j.state === "DONE" || j.state === "CANCELLED");

  const tabItems = [
    { id: "running", label: "Đang chạy", count: runningJobs.length },
    { id: "failed", label: "Thất bại", count: failedJobs.length },
    { id: "completed", label: "Hoàn tất", count: completedJobs.length },
  ];

  const currentList =
    activeTab === "running"
      ? runningJobs
      : activeTab === "failed"
      ? failedJobs
      : completedJobs;

  return (
    <div
      role="region"
      aria-label="Task Center"
      style={{
        position: "absolute",
        left: "var(--cs-rail-w)",
        right: 0,
        bottom: "var(--cs-statusbar-h)",
        height: 320,
        backgroundColor: "var(--cs-bg-panel)",
        borderTop: "1px solid var(--cs-border-strong)",
        boxShadow: "var(--cs-shadow-overlay)",
        display: "flex",
        flexDirection: "column",
        zIndex: 30,
      }}
    >
      {/* Drawer Header */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "var(--cs-space-2) var(--cs-space-4)",
          borderBottom: "1px solid var(--cs-border-subtle)",
          backgroundColor: "var(--cs-bg-raised)",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-3)" }}>
          <div style={{ display: "flex", alignItems: "center", gap: 6, fontWeight: 600, fontSize: "var(--cs-text-sm)" }}>
            <Icon name="activity" size={16} color="var(--cs-accent)" />
            <span>Trung tâm Tác vụ (Task Center)</span>
          </div>
          <Tabs
            items={tabItems}
            activeId={activeTab}
            onChange={setActiveTab}
            size="sm"
          />
        </div>

        <button
          type="button"
          onClick={onClose}
          aria-label="Đóng Task Center (Esc)"
          style={{
            background: "none",
            border: "none",
            padding: 4,
            cursor: "pointer",
            color: "var(--cs-fg-secondary)",
            borderRadius: "var(--cs-radius-control)",
            display: "inline-flex",
          }}
          title="Đóng (Esc)"
        >
          <Icon name="x" size={16} />
        </button>
      </div>

      {/* Drawer Body */}
      <div style={{ flex: 1, overflowY: "auto", minHeight: 0 }}>
        {currentList.length === 0 ? (
          <EmptyState
            icon={<Icon name="check-circle" size={24} />}
            title={
              activeTab === "running"
                ? "Không có tác vụ nào đang chạy"
                : activeTab === "failed"
                ? "Không có tác vụ nào bị lỗi"
                : "Chưa có tác vụ nào hoàn tất"
            }
            description={
              activeTab === "running"
                ? "Các tiến trình render, xuất video, AI sinh ảnh sẽ hiển thị tại đây khi bắt đầu."
                : undefined
            }
          />
        ) : (
          <div style={{ display: "flex", flexDirection: "column" }}>
            {currentList.map((job) => (
              <div
                key={job.id}
                style={{
                  display: "grid",
                  gridTemplateColumns: "32px minmax(180px, 1.2fr) minmax(200px, 1.5fr) 100px 110px",
                  alignItems: "center",
                  gap: "var(--cs-space-3)",
                  padding: "8px var(--cs-space-4)",
                  borderBottom: "1px solid var(--cs-border-subtle)",
                  fontSize: "var(--cs-text-sm)",
                }}
              >
                {/* State Icon */}
                <div>
                  {job.state === "RUNNING" && <Icon name="loader" size={16} color="var(--cs-accent)" />}
                  {job.state === "PENDING" && <Icon name="clock" size={16} color="var(--cs-fg-muted)" />}
                  {job.state === "DONE" && <Icon name="check-circle" size={16} color="var(--cs-success)" />}
                  {job.state === "FAILED" && <Icon name="x-circle" size={16} color="var(--cs-danger)" />}
                  {job.state === "CANCELLED" && <Icon name="x" size={16} color="var(--cs-fg-muted)" />}
                </div>

                {/* Job Title & Kind */}
                <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                  <span style={{ fontWeight: 600, color: "var(--cs-fg-primary)" }}>{job.title}</span>
                  <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)", fontFamily: "var(--cs-font-mono)" }}>
                    {job.id} · {job.kind}
                  </span>
                </div>

                {/* Stage Message & Progress Bar */}
                <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
                    {job.stageName || (job.state === "RUNNING" ? "Đang xử lý..." : job.state === "DONE" ? "Hoàn tất" : "Đang chờ")}
                  </span>
                  {typeof job.progress === "number" && (
                    <div style={{ display: "flex", alignItems: "center", gap: 8, width: "100%" }}>
                      <div
                        style={{
                          flex: 1,
                          height: 5,
                          backgroundColor: "var(--cs-progress-track)",
                          borderRadius: "var(--cs-radius-round)",
                          overflow: "hidden",
                        }}
                      >
                        <div
                          style={{
                            height: "100%",
                            width: `${Math.min(100, Math.max(0, job.progress))}%`,
                            backgroundColor: job.state === "FAILED" ? "var(--cs-danger)" : "var(--cs-progress-fill)",
                            transition: "width var(--cs-motion-duration) ease",
                          }}
                        />
                      </div>
                      <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)", minWidth: 32, textAlign: "right", fontFamily: "var(--cs-font-mono)" }}>
                        {Math.round(job.progress)}%
                      </span>
                    </div>
                  )}
                  {job.error && (
                    <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-danger)" }}>
                      {job.error}
                    </span>
                  )}
                </div>

                {/* Peak Memory in MB */}
                <div style={{ textAlign: "right" }}>
                  {job.peakMemoryBytes && job.peakMemoryBytes > 0 ? (
                    <span
                      style={{
                        fontSize: "var(--cs-text-xs)",
                        color: "var(--cs-fg-secondary)",
                        fontFamily: "var(--cs-font-mono)",
                        padding: "2px 6px",
                        backgroundColor: "var(--cs-bg-raised)",
                        borderRadius: "var(--cs-radius-control)",
                        border: "1px solid var(--cs-border-subtle)",
                      }}
                      title="Bộ nhớ đỉnh (Peak Memory)"
                    >
                      {(job.peakMemoryBytes / (1024 * 1024)).toFixed(1)} MB
                    </span>
                  ) : (
                    <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>—</span>
                  )}
                </div>

                {/* Status Badge */}
                <div style={{ textAlign: "right" }}>
                  <Badge
                    variant={
                      job.state === "DONE"
                        ? "success"
                        : job.state === "FAILED"
                        ? "danger"
                        : job.state === "RUNNING"
                        ? "info"
                        : "neutral"
                    }
                  >
                    {job.state === "RUNNING"
                      ? "Đang chạy"
                      : job.state === "DONE"
                      ? "Hoàn tất"
                      : job.state === "FAILED"
                      ? "Thất bại"
                      : job.state === "CANCELLED"
                      ? "Đã hủy"
                      : "Đang chờ"}
                  </Badge>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
