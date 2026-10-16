import React from "react";
import { Icon } from "../components/Icon";
import { Tabs, Badge, EmptyState, ErrorState } from "@studio/design-system";
import { studioApi } from "../api/client";
import type { ScopeItem, ScopeKind } from "../api/types";

export const BrandView: React.FC = () => {
  const [activeTab, setActiveTab] = React.useState<"brands" | "channels" | "playbooks">("brands");
  const [items, setItems] = React.useState<ScopeItem[]>([]);
  const [selectedId, setSelectedId] = React.useState<string | null>(null);
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);

  const loadScopeItems = React.useCallback(async (kind: ScopeKind) => {
    setLoading(true);
    setError(null);
    try {
      const list = await studioApi.getScopeItems(kind);
      setItems(list);
      if (list.length > 0) {
        setSelectedId(list[0].id);
      } else {
        setSelectedId(null);
      }
    } catch (err: any) {
      setError(err?.message || "Chưa kết nối Studio core");
    } finally {
      setLoading(false);
    }
  }, []);

  React.useEffect(() => {
    loadScopeItems(activeTab);
  }, [activeTab, loadScopeItems]);

  const selectedItem = items.find((i) => i.id === selectedId);

  const tabItems = [
    { id: "brands", label: "Thương hiệu (Brands)" },
    { id: "channels", label: "Kênh (Channels)" },
    { id: "playbooks", label: "Quy trình (Playbooks)" },
  ];

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        backgroundColor: "var(--cs-bg-app)",
      }}
    >
      {/* Top Header & Tabs */}
      <div
        style={{
          padding: "var(--cs-space-3) var(--cs-space-5)",
          backgroundColor: "var(--cs-bg-panel)",
          borderBottom: "1px solid var(--cs-border-subtle)",
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
        }}
      >
        <div>
          <h1 style={{ margin: "0 0 4px 0", fontSize: "var(--cs-text-lg)", fontWeight: 700 }}>
            Tầng Ổn định: Brand, Channel & Playbook
          </h1>
          <p style={{ margin: 0, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
            Kế thừa theo chuỗi App → Tổ chức → Workspace. Xem trước chỉ đọc.
          </p>
        </div>

        <Tabs
          items={tabItems}
          activeId={activeTab}
          onChange={(id) => setActiveTab(id as any)}
          size="sm"
        />
      </div>

      {/* Main Content Area */}
      <div style={{ flex: 1, display: "flex", minHeight: 0 }}>
        {error ? (
          <div style={{ padding: "var(--cs-space-6)", width: "100%", maxWidth: 800, margin: "0 auto" }}>
            <ErrorState
              title="Tải chuỗi kế thừa Scope"
              reason={error}
              safetyNote="Dữ liệu thương hiệu và kênh được lưu an toàn trong thư mục brands/ và channels/ của workspace."
              onRetry={() => loadScopeItems(activeTab)}
            />
          </div>
        ) : loading ? (
          <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center", color: "var(--cs-fg-muted)" }}>
            Đang đọc tệp cấu hình từ workspace...
          </div>
        ) : items.length === 0 ? (
          <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center" }}>
            <EmptyState
              icon={<Icon name="brand" size={32} />}
              title={`Chưa có ${activeTab} nào`}
              description={`Chưa phát hiện tệp cấu hình ${activeTab} trong chuỗi kế thừa App / Tổ chức / Workspace.`}
            />
          </div>
        ) : (
          <>
            {/* Left Items List */}
            <div
              style={{
                width: 320,
                borderRight: "1px solid var(--cs-border-subtle)",
                backgroundColor: "var(--cs-bg-panel)",
                overflowY: "auto",
              }}
            >
              {items.map((item) => {
                const isSelected = item.id === selectedId;
                return (
                  <div
                    key={item.id}
                    onClick={() => setSelectedId(item.id)}
                    style={{
                      padding: "var(--cs-space-3) var(--cs-space-4)",
                      borderBottom: "1px solid var(--cs-border-subtle)",
                      cursor: "pointer",
                      backgroundColor: isSelected ? "var(--cs-selection)" : "transparent",
                      borderLeft: isSelected ? "2px solid var(--cs-accent)" : "2px solid transparent",
                    }}
                  >
                    <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 4 }}>
                      <span style={{ fontWeight: 600, color: "var(--cs-fg-primary)", fontSize: "var(--cs-text-sm)" }}>
                        {item.name}
                      </span>
                      <Badge
                        variant={
                          item.layer === "workspace"
                            ? "info"
                            : item.layer === "org"
                            ? "neutral"
                            : "warning"
                        }
                      >
                        {item.layer === "workspace"
                          ? "Workspace"
                          : item.layer === "org"
                          ? "Tổ chức"
                          : "App"}
                      </Badge>
                    </div>
                    {item.summary && (
                      <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", lineHeight: 1.4 }}>
                        {item.summary}
                      </span>
                    )}
                  </div>
                );
              })}
            </div>

            {/* Right Read-only Preview Pane */}
            <div
              style={{
                flex: 1,
                padding: "var(--cs-space-6)",
                overflowY: "auto",
                backgroundColor: "var(--cs-bg-app)",
              }}
            >
              {selectedItem ? (
                <div
                  style={{
                    backgroundColor: "var(--cs-bg-panel)",
                    border: "1px solid var(--cs-border-subtle)",
                    padding: "var(--cs-space-5)",
                    display: "flex",
                    flexDirection: "column",
                    gap: "var(--cs-space-4)",
                    maxWidth: 720,
                  }}
                >
                  <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", borderBottom: "1px solid var(--cs-border-subtle)", paddingBottom: "var(--cs-space-3)" }}>
                    <div>
                      <h2 style={{ margin: 0, fontSize: "var(--cs-text-lg)", fontWeight: 700 }}>
                        {selectedItem.name}
                      </h2>
                      <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)", fontFamily: "var(--cs-font-mono)" }}>
                        ID: {selectedItem.id}
                      </span>
                    </div>
                    <Badge variant="neutral">
                      Cấp: {selectedItem.layer.toUpperCase()}
                    </Badge>
                  </div>

                  {/* Metadata display */}
                  <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                    <div style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600, color: "var(--cs-fg-secondary)" }}>
                      ĐƯỜNG DẪN TỆP
                    </div>
                    <code style={{ padding: "4px 8px", backgroundColor: "var(--cs-bg-raised)", fontFamily: "var(--cs-font-mono)", fontSize: 12 }}>
                      {selectedItem.path || "Tự động phát hiện"}
                    </code>
                  </div>

                  <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                    <div style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600, color: "var(--cs-fg-secondary)" }}>
                      THÔNG SỐ & NỘI DUNG (CHỈ ĐỌC)
                    </div>
                    <pre
                      style={{
                        padding: "var(--cs-space-3)",
                        backgroundColor: "var(--cs-bg-raised)",
                        border: "1px solid var(--cs-border-subtle)",
                        fontFamily: "var(--cs-font-mono)",
                        fontSize: 12,
                        lineHeight: 1.5,
                        overflowX: "auto",
                        whiteSpace: "pre-wrap",
                      }}
                    >
                      {JSON.stringify(selectedItem.metadata || { ghiChu: "Tệp cấu hình đang áp dụng cho agent" }, null, 2)}
                    </pre>
                  </div>
                </div>
              ) : (
                <div style={{ color: "var(--cs-fg-muted)", fontSize: "var(--cs-text-sm)" }}>
                  Chọn một mục từ danh sách bên trái để xem trước.
                </div>
              )}
            </div>
          </>
        )}
      </div>
    </div>
  );
};
