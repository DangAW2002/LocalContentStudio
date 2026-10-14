import React from "react";
import { Badge, Button, Input } from "@studio/design-system";

export const ResearchView: React.FC = () => {
  const [url, setUrl] = React.useState("https://example.com/product-launch-report");
  const [fetching, setFetching] = React.useState(false);

  return (
    <div style={{ padding: "var(--cs-space-4)", display: "flex", flexDirection: "column", gap: "var(--cs-space-4)", height: "100%", overflowY: "auto" }}>
      <div>
        <h1 style={{ margin: 0, fontSize: "var(--cs-text-lg)", fontWeight: 700 }}>
          Thu thập nghiên cứu (Research Ingestion)
        </h1>
        <span style={{ fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-secondary)" }}>
          Trích xuất dữ liệu, cấu trúc bài và bằng chứng số liệu với cơ chế chống SSRF/Private IP
        </span>
      </div>

      <div style={{ display: "flex", gap: "var(--cs-space-2)", alignItems: "flex-end", maxWidth: "700px" }}>
        <Input
          label="Địa chỉ URL nguồn nghiên cứu:"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          hint="Hệ thống tự động chặn localhost, IP dải riêng tư (10.x, 192.168.x), và giới hạn stream bytes."
        />
        <Button
          variant="primary"
          busy={fetching}
          busyText="Đang trích xuất..."
          onClick={() => {
            setFetching(true);
            setTimeout(() => setFetching(false), 800);
          }}
        >
          Trích xuất nguồn
        </Button>
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
        <h2 style={{ margin: 0, fontSize: "var(--cs-text-md)", fontWeight: 600 }}>Gói bằng chứng đã lưu (Evidence Bundle)</h2>
        <div style={{ padding: "var(--cs-space-3)", backgroundColor: "var(--cs-bg-panel)", borderRadius: "var(--cs-radius-md)", border: "1px solid var(--cs-border-subtle)", display: "flex", flexDirection: "column", gap: "var(--cs-space-2)" }}>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
            <span style={{ fontWeight: 600 }}>Báo cáo xu hướng thị trường Q4</span>
            <Badge variant="success">SHA: 8f4a21... · Đã lập chỉ mục FTS5</Badge>
          </div>
          <p style={{ margin: 0, fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-secondary)" }}>
            Nguồn: https://example.com/trends-q4 · 4 trích dẫn số liệu · 2 hình ảnh tham khảo
          </p>
          <div style={{ display: "flex", gap: "var(--cs-space-2)" }}>
            <Button variant="secondary">Gắn vào Content Brief</Button>
            <Button variant="quiet">Xem nguồn gốc</Button>
          </div>
        </div>
      </div>
    </div>
  );
};
