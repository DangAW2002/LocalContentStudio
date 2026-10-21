import React from "react";
import { Badge, Button, Dialog } from "@studio/design-system";
import type { DownstreamPlan } from "../../../api/types";

export interface DownstreamPlanDialogProps {
  open: boolean;
  plan: DownstreamPlan | null;
  onClose: () => void;
  onRun: (runPaid: boolean) => Promise<void>;
}

export const DownstreamPlanDialog: React.FC<DownstreamPlanDialogProps> = ({
  open,
  plan,
  onClose,
  onRun,
}) => {
  const [running, setRunning] = React.useState(false);

  if (!plan) return null;

  const handleRunFree = async () => {
    setRunning(true);
    try {
      await onRun(false);
      onClose();
    } finally {
      setRunning(false);
    }
  };

  const handleRunAll = async () => {
    setRunning(true);
    try {
      await onRun(true);
      onClose();
    } finally {
      setRunning(false);
    }
  };

  return (
    <Dialog
      title={`Cập nhật phía dưới cho: ${plan.targetRef}`}
      open={open}
      onClose={onClose}
      actions={
        <>
          <Button variant="secondary" onClick={onClose} disabled={running}>
            Đóng
          </Button>
          {plan.freeStepsCount > 0 && (
            <Button
              variant="secondary"
              busy={running}
              busyText="Đang render..."
              onClick={handleRunFree}
            >
              Chạy bước miễn phí ({plan.freeStepsCount})
            </Button>
          )}
          {plan.paidStepsCount > 0 && (
            <Button
              variant="primary"
              busy={running}
              busyText="Đang thực thi..."
              onClick={handleRunAll}
            >
              {`Chạy cả bước tốn phí (${plan.paidStepsCount}) (≈ $${plan.totalEstimatedCostUsd.toFixed(2)})`}
            </Button>
          )}
        </>
      }
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
        <p style={{ margin: 0, fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-secondary)" }}>
          Các bước cập nhật theo thứ tự topo phụ thuộc ({plan.steps.length} bước):
        </p>

        <div
          style={{
            maxHeight: 280,
            overflowY: "auto",
            border: "1px solid var(--cs-border-subtle)",
            borderRadius: "var(--cs-radius-control)",
            padding: "4px 8px",
            display: "flex",
            flexDirection: "column",
            gap: 4,
          }}
        >
          {plan.steps.map((step, idx) => (
            <div
              key={step.id}
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                padding: "6px 8px",
                borderBottom: idx < plan.steps.length - 1 ? "1px solid var(--cs-border-subtle)" : "none",
                fontSize: "var(--cs-text-xs)",
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{ fontWeight: 600, color: "var(--cs-fg-muted)", width: 16 }}>
                  {idx + 1}.
                </span>
                <span style={{ fontWeight: 600, color: "var(--cs-fg-primary)" }}>
                  {step.id}
                </span>
                <span style={{ color: "var(--cs-fg-muted)" }}>
                  ({step.kind})
                </span>
              </div>

              <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                {step.pinned ? (
                  <Badge variant="warning">Đã khóa (Bỏ qua)</Badge>
                ) : step.paid ? (
                  <Badge variant="danger">
                    {`Tốn phí ${step.estimatedCostUsd ? `≈ $${step.estimatedCostUsd}` : ""}`}
                  </Badge>
                ) : (
                  <Badge variant="success">Miễn phí</Badge>
                )}
              </div>
            </div>
          ))}
        </div>
      </div>
    </Dialog>
  );
};
