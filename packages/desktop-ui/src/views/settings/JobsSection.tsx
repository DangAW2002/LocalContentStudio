import React from "react";
import { Button, Input, Callout } from "@studio/design-system";
import { studioApi } from "../../api/client";
import type { BudgetStatus } from "../../api/types";

export const JobsSection: React.FC = () => {
  const [budget, setBudget] = React.useState<BudgetStatus | null>(null);
  const [dailyInput, setDailyInput] = React.useState<string>("");
  const [monthlyInput, setMonthlyInput] = React.useState<string>("");
  const [saving, setSaving] = React.useState(false);
  const [msg, setMsg] = React.useState<string | null>(null);

  React.useEffect(() => {
    studioApi.getBudget().then((b) => {
      setBudget(b);
      setDailyInput(b.dailyBudgetUsd != null ? String(b.dailyBudgetUsd) : "");
      setMonthlyInput(b.monthlyBudgetUsd != null ? String(b.monthlyBudgetUsd) : "");
    }).catch(() => {});
  }, []);

  const handleSaveBudget = async () => {
    setSaving(true);
    setMsg(null);
    try {
      const daily = dailyInput.trim() ? parseFloat(dailyInput) : null;
      const monthly = monthlyInput.trim() ? parseFloat(monthlyInput) : null;
      const res = await studioApi.setBudget(
        daily != null && !isNaN(daily) ? daily : null,
        monthly != null && !isNaN(monthly) ? monthly : null
      );
      setBudget(res);
      setMsg("Đã lưu giới hạn ngân sách thành công!");
    } catch (e: any) {
      setMsg(`Lỗi khi lưu: ${e?.message || e}`);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-4)" }}>
      {/* Ngân sách sử dụng */}
      <div
        style={{
          padding: "var(--cs-space-4)",
          backgroundColor: "var(--cs-bg-panel)",
          border: "1px solid var(--cs-border-subtle)",
          display: "flex",
          flexDirection: "column",
          gap: "var(--cs-space-3)",
        }}
      >
        <h2 style={{ margin: 0, fontSize: "var(--cs-text-md)", fontWeight: 700 }}>
          Ngân sách Chi phí AI (studio.yaml)
        </h2>
        <p style={{ margin: 0, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
          Đặt giới hạn chi phí hàng ngày và hàng tháng cho workspace này. Khi vượt quá ngân sách, các công cụ trả phí (paid tools) sẽ yêu cầu bạn phê duyệt trước khi tiếp tục.
        </p>

        {msg && (
          <Callout variant={msg.includes("Lỗi") ? "danger" : "success"}>
            {msg}
          </Callout>
        )}

        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--cs-space-4)" }}>
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            <label style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600 }}>
              Ngân sách ngày (USD):
            </label>
            <Input
              type="number"
              step="0.5"
              min="0"
              placeholder="VD: 5.0 (để trống nếu không giới hạn)"
              value={dailyInput}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setDailyInput(e.target.value)}
            />
            {budget && (
              <span style={{ fontSize: 11, color: budget.isDailyOver ? "var(--cs-danger)" : "var(--cs-fg-muted)" }}>
                Hôm nay đã chi: ${budget.todaySpendUsd.toFixed(2)}
                {budget.isDailyOver && " (Đã vượt ngân sách ngày!)"}
              </span>
            )}
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            <label style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600 }}>
              Ngân sách tháng (USD):
            </label>
            <Input
              type="number"
              step="1"
              min="0"
              placeholder="VD: 50.0 (để trống nếu không giới hạn)"
              value={monthlyInput}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setMonthlyInput(e.target.value)}
            />
            {budget && (
              <span style={{ fontSize: 11, color: budget.isMonthlyOver ? "var(--cs-danger)" : "var(--cs-fg-muted)" }}>
                Tháng này đã chi: ${budget.monthSpendUsd.toFixed(2)}
                {budget.isMonthlyOver && " (Đã vượt ngân sách tháng!)"}
              </span>
            )}
          </div>
        </div>

        <div style={{ display: "flex", justifyContent: "flex-end" }}>
          <Button
            size="sm"
            variant="primary"
            busy={saving}
            busyText="Đang lưu..."
            onClick={handleSaveBudget}
          >
            Lưu cài đặt ngân sách
          </Button>
        </div>
      </div>

      <div
        style={{
          padding: "var(--cs-space-4)",
          backgroundColor: "var(--cs-bg-panel)",
          border: "1px solid var(--cs-border-subtle)",
          display: "flex",
          flexDirection: "column",
          gap: "var(--cs-space-3)",
        }}
      >
        <h2 style={{ margin: 0, fontSize: "var(--cs-text-md)", fontWeight: 700 }}>
          Pool Tài nguyên & Giới hạn Tiến trình (ADR-009)
        </h2>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--cs-space-4)" }}>
          <div style={{ padding: "var(--cs-space-3)", backgroundColor: "var(--cs-bg-raised)", border: "1px solid var(--cs-border-subtle)" }}>
            <div style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)" }}>Engine Pool (Claude / agy)</div>
            <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", marginTop: 4 }}>
              Tối đa 1 tiến trình CLI đồng thời để bảo vệ bộ nhớ và token budget.
            </div>
            <div style={{ marginTop: 8, fontSize: "var(--cs-text-xs)", color: "var(--cs-accent)", fontWeight: 600 }}>
              Giới hạn: 1 active worker
            </div>
          </div>

          <div style={{ padding: "var(--cs-space-3)", backgroundColor: "var(--cs-bg-raised)", border: "1px solid var(--cs-border-subtle)" }}>
            <div style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)" }}>Render & FFmpeg Heavy Pool</div>
            <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", marginTop: 4 }}>
              Tác vụ dựng video, xuất PNG nặng được giám sát qua Windows Job Object.
            </div>
            <div style={{ marginTop: 8, fontSize: "var(--cs-text-xs)", color: "var(--cs-accent)", fontWeight: 600 }}>
              Giới hạn: 2 concurrent tasks · Timeout: 300s
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

