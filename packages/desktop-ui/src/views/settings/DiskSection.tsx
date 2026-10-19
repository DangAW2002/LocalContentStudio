import React from "react";
import { Icon } from "../../components/Icon";
import { Button, EmptyState } from "@studio/design-system";
import { studioApi } from "../../api/client";
import type { DiskUsageReport } from "../../api/types";

export const DiskSection: React.FC = () => {
  const [diskUsage, setDiskUsage] = React.useState<DiskUsageReport | null>(null);

  const loadDiskUsage = React.useCallback(async () => {
    try {
      const disk = await studioApi.getDiskUsage();
      setDiskUsage(disk);
    } catch {
      // ignore
    }
  }, []);

  React.useEffect(() => {
    loadDiskUsage();
  }, [loadDiskUsage]);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-4)" }}>
      {diskUsage ? (
        <div
          style={{
            padding: "var(--cs-space-4)",
            backgroundColor: "var(--cs-bg-panel)",
            border: "1px solid var(--cs-border-subtle)",
            display: "flex",
            flexDirection: "column",
            gap: "var(--cs-space-4)",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
            <div>
              <h2 style={{ margin: "0 0 2px 0", fontSize: "var(--cs-text-md)", fontWeight: 700 }}>
                Dung lượng Sử dụng: {diskUsage.formatted.total}
              </h2>
              <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
                Phân bổ theo khu vực lưu trữ trên đĩa
              </span>
            </div>

            <Button size="sm" variant="secondary" onClick={loadDiskUsage}>
              Dọn bộ nhớ đệm
            </Button>
          </div>

          {/* Areas grid */}
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: "var(--cs-space-3)" }}>
            <div style={{ padding: "var(--cs-space-3)", backgroundColor: "var(--cs-bg-raised)", border: "1px solid var(--cs-border-subtle)" }}>
              <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>CAS Blobs (Checkpoints)</div>
              <div style={{ fontSize: "var(--cs-text-lg)", fontWeight: 700, marginTop: 4 }}>
                {diskUsage.formatted.blobs}
              </div>
            </div>

            <div style={{ padding: "var(--cs-space-3)", backgroundColor: "var(--cs-bg-raised)", border: "1px solid var(--cs-border-subtle)" }}>
              <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>Bộ nhớ đệm (Cache)</div>
              <div style={{ fontSize: "var(--cs-text-lg)", fontWeight: 700, marginTop: 4 }}>
                {diskUsage.formatted.cache}
              </div>
            </div>

            <div style={{ padding: "var(--cs-space-3)", backgroundColor: "var(--cs-bg-raised)", border: "1px solid var(--cs-border-subtle)" }}>
              <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>Dự án (Projects)</div>
              <div style={{ fontSize: "var(--cs-text-lg)", fontWeight: 700, marginTop: 4 }}>
                {diskUsage.formatted.projects}
              </div>
            </div>

            <div style={{ padding: "var(--cs-space-3)", backgroundColor: "var(--cs-bg-raised)", border: "1px solid var(--cs-border-subtle)" }}>
              <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>Thư viện (Library)</div>
              <div style={{ fontSize: "var(--cs-text-lg)", fontWeight: 700, marginTop: 4 }}>
                {diskUsage.formatted.library}
              </div>
            </div>
          </div>
        </div>
      ) : (
        <EmptyState
          icon={<Icon name="hard-drive" size={28} />}
          title="Chưa có dữ liệu dung lượng"
          description="Không thể kết nối đến API disk_usage của backend."
        />
      )}
    </div>
  );
};
