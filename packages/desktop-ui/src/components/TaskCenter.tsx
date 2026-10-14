import React from "react";
import { Badge, Button } from "@studio/design-system";

export interface TaskJob {
  id: string;
  type: string;
  state: "PENDING" | "RUNNING" | "VERIFYING" | "READY" | "FAILED" | "CANCELLED";
  progress: number;
  stageName: string;
  errorMessage?: string;
}

export interface TaskCenterProps {
  jobs: TaskJob[];
  onCancelJob: (jobId: string) => void;
}

export const TaskCenter: React.FC<TaskCenterProps> = ({ jobs, onCancelJob }) => {
  const [expanded, setExpanded] = React.useState(false);

  const getBadgeVariant = (state: TaskJob["state"]) => {
    switch (state) {
      case "READY":
        return "success";
      case "RUNNING":
      case "VERIFYING":
        return "info";
      case "FAILED":
        return "danger";
      case "PENDING":
      case "CANCELLED":
      default:
        return "warning";
    }
  };

  return (
    <div
      aria-label="Trung tâm tiến trình Task Center"
      style={{
        borderTop: "1px solid var(--cs-border-subtle)",
        backgroundColor: "var(--cs-bg-raised)",
        display: "flex",
        flexDirection: "column",
        transition: "height var(--cs-motion-duration) ease",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "var(--cs-space-1) var(--cs-space-4)",
          height: "32px",
          userSelect: "none",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-3)" }}>
          <button
            onClick={() => setExpanded(!expanded)}
            style={{
              background: "none",
              border: "none",
              cursor: "pointer",
              fontSize: "var(--cs-text-xs)",
              color: "var(--cs-fg-secondary)",
              fontWeight: 600,
            }}
          >
            {expanded ? "▼ Ẩn Task Center" : "▲ Task Center"} ({jobs.length} tiến trình)
          </button>

          {jobs.length > 0 && (
            <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-2)" }}>
              <span style={{ fontSize: "var(--cs-text-xs)" }}>{jobs[0].stageName}</span>
              <div
                style={{
                  width: "120px",
                  height: "6px",
                  backgroundColor: "var(--cs-progress-track)",
                  borderRadius: "3px",
                  overflow: "hidden",
                }}
              >
                <div
                  style={{
                    width: `${jobs[0].progress}%`,
                    height: "100%",
                    backgroundColor: "var(--cs-progress-fill)",
                    transition: "width 200ms ease",
                  }}
                />
              </div>
              <span style={{ fontSize: "var(--cs-text-xs)", fontFamily: "var(--cs-font-mono)" }}>
                {jobs[0].progress}%
              </span>
            </div>
          )}
        </div>

        <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
          Worker supervisor: Online
        </div>
      </div>

      {expanded && (
        <div
          style={{
            padding: "var(--cs-space-3) var(--cs-space-4)",
            maxHeight: "180px",
            overflowY: "auto",
            display: "flex",
            flexDirection: "column",
            gap: "var(--cs-space-2)",
            backgroundColor: "var(--cs-bg-panel)",
            borderTop: "1px solid var(--cs-border-subtle)",
          }}
        >
          {jobs.length === 0 ? (
            <span style={{ fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-secondary)" }}>
              Không có tiến trình nền nào đang chạy.
            </span>
          ) : (
            jobs.map((job) => (
              <div
                key={job.id}
                style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  padding: "var(--cs-space-2)",
                  borderRadius: "var(--cs-radius-sm)",
                  backgroundColor: "var(--cs-bg-raised)",
                  fontSize: "var(--cs-text-sm)",
                }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-2)" }}>
                  <Badge variant={getBadgeVariant(job.state)}>{job.state}</Badge>
                  <span style={{ fontWeight: 600 }}>{job.type}</span>
                  <span style={{ color: "var(--cs-fg-secondary)", fontSize: "var(--cs-text-xs)" }}>{job.stageName}</span>
                  {job.errorMessage && (
                    <span style={{ color: "var(--cs-danger)", fontSize: "var(--cs-text-xs)" }}>
                      Lỗi: {job.errorMessage}
                    </span>
                  )}
                </div>

                <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-3)" }}>
                  <span style={{ fontFamily: "var(--cs-font-mono)", fontSize: "var(--cs-text-xs)" }}>
                    {job.progress}%
                  </span>
                  {(job.state === "RUNNING" || job.state === "PENDING") && (
                    <Button variant="quiet" style={{ height: "24px", padding: "0 6px" }} onClick={() => onCancelJob(job.id)}>
                      Hủy
                    </Button>
                  )}
                </div>
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
};
