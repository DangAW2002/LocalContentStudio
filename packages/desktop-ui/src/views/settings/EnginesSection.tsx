import React from "react";
import { Icon } from "../../components/Icon";
import { Badge, Button, Callout, EmptyState, ErrorState } from "@studio/design-system";
import { studioApi } from "../../api/client";
import { cleanDisplayPath } from "../../utils/path";
import type { EngineProbeResult } from "../../api/types";

let cachedEngines: EngineProbeResult[] | null = null;

export const EnginesSection: React.FC = () => {
  const [engines, setEngines] = React.useState<EngineProbeResult[]>(cachedEngines || []);
  const [loading, setLoading] = React.useState(cachedEngines === null);
  const [error, setError] = React.useState<string | null>(null);
  const [agyBusy, setAgyBusy] = React.useState(false);
  const [agyStatus, setAgyStatus] = React.useState<{ type: "success" | "danger"; message: string } | null>(null);

  const loadEngines = React.useCallback(async (force = false) => {
    if (!force && cachedEngines !== null) {
      setEngines(cachedEngines);
      setLoading(false);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const list = await studioApi.probeEngines();
      cachedEngines = list;
      setEngines(list);
    } catch (err: any) {
      setError(err?.message || "Không thể quét công cụ trên máy.");
    } finally {
      setLoading(false);
    }
  }, []);

  React.useEffect(() => {
    if (cachedEngines === null) {
      loadEngines();
    }
  }, [loadEngines]);

  const handleAgyConnect = async () => {
    setAgyBusy(true);
    setAgyStatus(null);
    try {
      const res = await studioApi.agyConnect();
      setAgyStatus({
        type: "success",
        message: `Đã kết nối thành công agy với Studio MCP entry 'studio' (shim: ${cleanDisplayPath(res.shim) || "studio-mcp"}).`,
      });
      loadEngines(true);
    } catch (err: any) {
      setAgyStatus({
        type: "danger",
        message: err?.message || "Không thể kết nối agy với Studio.",
      });
    } finally {
      setAgyBusy(false);
    }
  };

  const handleAgyDisconnect = async () => {
    setAgyBusy(true);
    setAgyStatus(null);
    try {
      await studioApi.agyDisconnect();
      setAgyStatus({
        type: "success",
        message: "Đã ngắt kết nối agy khỏi Studio MCP thành công.",
      });
      loadEngines(true);
    } catch (err: any) {
      setAgyStatus({
        type: "danger",
        message: err?.message || "Không thể ngắt kết nối agy.",
      });
    } finally {
      setAgyBusy(false);
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-4)" }}>
      {error && (
        <ErrorState
          title="Tải cấu hình hệ thống"
          reason={error}
          safetyNote="Cần kết nối với Studio Core backend qua Tauri để kiểm tra môi trường máy cục bộ."
          onRetry={loadEngines}
        />
      )}

      <Callout variant="neutral" title="Nguyên tắc phụ thuộc ngoài">
        Không đóng gói Node hoặc Chromium trong bản cài. Core probe trực tiếp binary trên máy của bạn và mở khóa công cụ tương ứng khi đã cài đặt.
      </Callout>

      {engines.length === 0 ? (
        <EmptyState
          icon={<Icon name="cpu" size={28} />}
          title={loading ? "Đang quét công cụ trên máy…" : "Chưa có kết quả probe"}
          description={loading ? undefined : "Bấm 'Kiểm tra lại' để quét các công cụ CLI đã cài trên máy."}
          action={
            <Button variant="secondary" onClick={() => loadEngines(true)} busy={loading} busyText="Đang quét...">
              Kiểm tra lại
            </Button>
          }
        />
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-4)" }}>
          {/* Section: Agent Engines */}
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
              <h2 style={{ margin: 0, fontSize: "var(--cs-text-md)", fontWeight: 700 }}>
                Agent Engines (Trí tuệ nhân tạo)
              </h2>
              <Button
                size="sm"
                variant="secondary"
                onClick={() => loadEngines(true)}
                leftIcon={<Icon name="refresh" size={13} />}
              >
                Quét lại
              </Button>
            </div>

            {engines
              .filter((e) => e.id === "claude" || e.id === "agy")
              .map((eng) => {
                const isAgy = eng.id === "agy";
                const isNotLoggedIn = eng.notes?.toLowerCase().includes("chưa đăng nhập");
                const loggedIn = eng.installed && !isNotLoggedIn;

                return (
                  <div
                    key={eng.id}
                    style={{
                      padding: "var(--cs-space-4)",
                      backgroundColor: "var(--cs-bg-panel)",
                      border: "1px solid var(--cs-border-subtle)",
                      borderRadius: "var(--cs-radius-control)",
                      display: "flex",
                      flexDirection: "column",
                      gap: 12,
                    }}
                  >
                    <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                        <Icon name="bot" size={18} color="var(--cs-accent)" />
                        <span style={{ fontWeight: 700, fontSize: "var(--cs-text-md)" }}>
                          {eng.name}
                        </span>
                        <Badge variant={eng.installed ? "success" : "warning"}>
                          {eng.installed ? "Đã cài đặt" : "Chưa cài"}
                        </Badge>
                        {eng.installed && (
                          <Badge variant={loggedIn ? "success" : "warning"}>
                            {loggedIn ? "Đã đăng nhập" : "Chưa đăng nhập"}
                          </Badge>
                        )}
                        {eng.supported && <Badge variant="info">Tương thích</Badge>}
                      </div>
                    </div>

                    <div style={{ display: "flex", alignItems: "center", gap: 12, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
                      <span>Phiên bản: {eng.version || (eng.installed ? "Đã nhận diện" : "Không tìm thấy")}</span>
                      {eng.path && (
                        <span style={{ fontFamily: "var(--cs-font-mono)" }}>
                          Đường dẫn: {cleanDisplayPath(eng.path)}
                        </span>
                      )}
                    </div>

                    {eng.notes && (
                      <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
                        {eng.notes}
                      </div>
                    )}

                    {/* agy MCP Connection Controls */}
                    {isAgy && (
                      <div
                        style={{
                          marginTop: 4,
                          padding: "var(--cs-space-3)",
                          backgroundColor: "var(--cs-bg-raised)",
                          border: "1px solid var(--cs-border-subtle)",
                          borderRadius: "var(--cs-radius-control)",
                        }}
                      >
                        <div style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)", marginBottom: 4 }}>
                          Tích hợp Antigravity (agy) MCP
                        </div>
                        <p style={{ margin: "0 0 8px 0", fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", lineHeight: 1.5 }}>
                          Tự động kết nối Studio MCP với Antigravity (agy) và cấp quyền sử dụng các công cụ trong dự án.
                        </p>
                        <details style={{ margin: "0 0 10px 0", fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
                          <summary style={{ cursor: "pointer", userSelect: "none" }}>Chi tiết cấu hình agy</summary>
                          <div style={{ marginTop: 4, paddingLeft: 8, borderLeft: "2px solid var(--cs-border-subtle)" }}>
                            Đăng ký MCP server <code>studio</code> và thêm quy tắc cho phép <code>mcp(studio/*)</code> vào tệp <code>settings.json</code> của agy.
                          </div>
                        </details>
                        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                          <Button
                            size="sm"
                            variant="primary"
                            onClick={handleAgyConnect}
                            busy={agyBusy}
                            busyText="Đang kết nối..."
                            leftIcon={<Icon name="link" size={13} />}
                          >
                            Kết nối agy với Studio
                          </Button>
                          <Button
                            size="sm"
                            variant="secondary"
                            onClick={handleAgyDisconnect}
                            disabled={agyBusy}
                            leftIcon={<Icon name="unlink" size={13} />}
                          >
                            Ngắt kết nối
                          </Button>
                        </div>
                        {agyStatus && (
                          <div style={{ marginTop: 8 }}>
                            <Callout variant={agyStatus.type}>
                              {agyStatus.message}
                            </Callout>
                          </div>
                        )}
                      </div>
                    )}
                  </div>
                );
              })}
          </div>

          {/* Section: Auxiliary tools */}
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)", marginTop: 8 }}>
            <h2 style={{ margin: 0, fontSize: "var(--cs-text-md)", fontWeight: 700 }}>
              Công cụ Kết xuất & Môi trường Phụ trợ
            </h2>
            {engines
              .filter((e) => e.id !== "claude" && e.id !== "agy")
              .map((eng) => (
                <div
                  key={eng.id}
                  style={{
                    padding: "var(--cs-space-3) var(--cs-space-4)",
                    backgroundColor: "var(--cs-bg-panel)",
                    border: "1px solid var(--cs-border-subtle)",
                    borderRadius: "var(--cs-radius-control)",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                  }}
                >
                  <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                      <span style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)" }}>
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
                          Đường dẫn: {cleanDisplayPath(eng.path)}
                        </span>
                      )}
                    </div>

                    {eng.notes && (
                      <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
                        {eng.notes}
                      </div>
                    )}
                  </div>
                </div>
              ))}
          </div>
        </div>
      )}
    </div>
  );
};
