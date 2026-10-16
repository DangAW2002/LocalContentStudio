import React from "react";
import { Icon } from "../components/Icon";
import { Tabs, Badge, Button, EmptyState, ErrorState, Callout } from "@studio/design-system";
import { studioApi } from "../api/client";
import type {
  EngineProbeResult,
  CatalogKit,
  CatalogModule,
  DiskUsageReport,
} from "../api/types";

export const SettingsView: React.FC = () => {
  const [activeTab, setActiveTab] = React.useState<"engines" | "catalog" | "jobs" | "disk">("engines");
  const [engines, setEngines] = React.useState<EngineProbeResult[]>([]);
  const [kits, setKits] = React.useState<CatalogKit[]>([]);
  const [modules, setModules] = React.useState<CatalogModule[]>([]);
  const [diskUsage, setDiskUsage] = React.useState<DiskUsageReport | null>(null);

  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);

  const loadData = React.useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [engList, kitList, modList, disk] = await Promise.all([
        studioApi.probeEngines().catch(() => []),
        studioApi.getCatalogKits().catch(() => []),
        studioApi.getCatalogModules().catch(() => []),
        studioApi.getDiskUsage().catch(() => null),
      ]);
      setEngines(engList);
      setKits(kitList);
      setModules(modList);
      setDiskUsage(disk);
    } catch (err: any) {
      setError(err?.message || "Chưa kết nối Studio core");
    } finally {
      setLoading(false);
    }
  }, []);

  React.useEffect(() => {
    loadData();
  }, [loadData]);

  const tabs = [
    { id: "engines", label: "Engines & Công cụ" },
    { id: "catalog", label: "Catalog Modules & Kits" },
    { id: "jobs", label: "Cấu hình Tác vụ" },
    { id: "disk", label: "Dung lượng Đĩa" },
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
      {/* Header */}
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
            Tools & Cài đặt Hệ thống
          </h1>
          <p style={{ margin: 0, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
            Kiểm tra phụ thuộc ngoài, catalog kỹ năng và dung lượng lưu trữ cục bộ.
          </p>
        </div>

        <Tabs
          items={tabs}
          activeId={activeTab}
          onChange={(id) => setActiveTab(id as any)}
          size="sm"
        />
      </div>

      {/* Body */}
      <div style={{ flex: 1, overflowY: "auto", padding: "var(--cs-space-6)" }}>
        <div style={{ maxWidth: 880, margin: "0 auto", display: "flex", flexDirection: "column", gap: "var(--cs-space-5)" }}>
          {error && (
            <ErrorState
              title="Tải cấu hình hệ thống"
              reason={error}
              safetyNote="Cần kết nối với Studio Core backend qua Tauri để kiểm tra môi trường máy cục bộ."
              onRetry={loadData}
            />
          )}

          {/* TAB 1: ENGINES PROBE */}
          {activeTab === "engines" && (
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-4)" }}>
              <Callout variant="neutral" title="Nguyên tắc phụ thuộc ngoài">
                Không đóng gói Node hoặc Chromium trong bản cài. Core probe trực tiếp binary trên máy của bạn và mở khóa công cụ tương ứng khi đã cài đặt.
              </Callout>

              {engines.length === 0 ? (
                <EmptyState
                  icon={<Icon name="cpu" size={28} />}
                  title="Chưa có kết quả probe"
                  description="Bấm 'Kiểm tra lại' để quét các công cụ CLI đã cài trên máy."
                  action={
                    <Button variant="secondary" onClick={loadData}>
                      Kiểm tra lại
                    </Button>
                  }
                />
              ) : (
                <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
                  {engines.map((eng) => (
                    <div
                      key={eng.id}
                      style={{
                        padding: "var(--cs-space-4)",
                        backgroundColor: "var(--cs-bg-panel)",
                        border: "1px solid var(--cs-border-subtle)",
                        borderRadius: "var(--cs-radius-none)",
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "space-between",
                      }}
                    >
                      <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                          <span style={{ fontWeight: 700, fontSize: "var(--cs-text-md)" }}>
                            {eng.name}
                          </span>
                          <Badge variant={eng.installed ? "success" : "warning"}>
                            {eng.installed ? "Đã cài đặt" : "Chưa cài"}
                          </Badge>
                          {eng.supported && <Badge variant="info">Tương thích</Badge>}
                        </div>

                        <div style={{ display: "flex", alignItems: "center", gap: 12, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
                          {eng.version && <span>Phiên bản: {eng.version}</span>}
                          {eng.path && (
                            <span style={{ fontFamily: "var(--cs-font-mono)" }}>
                              Đường dẫn: {eng.path}
                            </span>
                          )}
                        </div>

                        {eng.notes && (
                          <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", marginTop: 2 }}>
                            {eng.notes}
                          </div>
                        )}
                      </div>

                      <Button
                        size="sm"
                        variant="secondary"
                        onClick={loadData}
                        leftIcon={<Icon name="refresh" size={13} />}
                      >
                        Quét lại
                      </Button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {/* TAB 2: CATALOG MODULES & KITS */}
          {activeTab === "catalog" && (
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-5)" }}>
              {/* Kits section */}
              <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
                <h2 style={{ margin: 0, fontSize: "var(--cs-text-md)", fontWeight: 700 }}>
                  Bộ Kit Đặt Sẵn (Kits)
                </h2>
                <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(260px, 1fr))", gap: "var(--cs-space-3)" }}>
                  {kits.map((kit) => (
                    <div
                      key={kit.id}
                      style={{
                        padding: "var(--cs-space-3)",
                        backgroundColor: "var(--cs-bg-panel)",
                        border: "1px solid var(--cs-border-subtle)",
                        display: "flex",
                        flexDirection: "column",
                        gap: 6,
                      }}
                    >
                      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                        <span style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)" }}>{kit.name}</span>
                        <Badge variant="neutral">~{kit.tokenBudget} tokens</Badge>
                      </div>
                      <p style={{ margin: 0, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", lineHeight: 1.4 }}>
                        {kit.description}
                      </p>
                      <div style={{ fontSize: 11, color: "var(--cs-fg-muted)", marginTop: 4 }}>
                        Gồm {kit.modules.length} modules: {kit.modules.join(", ")}
                      </div>
                    </div>
                  ))}
                </div>
              </div>

              {/* Modules section */}
              <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
                <h2 style={{ margin: 0, fontSize: "var(--cs-text-md)", fontWeight: 700 }}>
                  Mô-đun Kỹ Năng Tuyển Chọn (Modules)
                </h2>
                <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                  {modules.map((mod) => (
                    <div
                      key={mod.id}
                      style={{
                        padding: "var(--cs-space-3) var(--cs-space-4)",
                        backgroundColor: "var(--cs-bg-panel)",
                        border: "1px solid var(--cs-border-subtle)",
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "space-between",
                      }}
                    >
                      <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                        <span style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)" }}>{mod.name}</span>
                        <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
                          {mod.description}
                        </span>
                      </div>
                      <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                        <Badge variant="info">{mod.skillCount} skills</Badge>
                        <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)", fontFamily: "var(--cs-font-mono)" }}>
                          ~{mod.tokenCount} tokens
                        </span>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}

          {/* TAB 3: JOBS CONFIG */}
          {activeTab === "jobs" && (
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-4)" }}>
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
          )}

          {/* TAB 4: DISK USAGE */}
          {activeTab === "disk" && (
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

                    <Button size="sm" variant="secondary" onClick={loadData}>
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
          )}
        </div>
      </div>
    </div>
  );
};
