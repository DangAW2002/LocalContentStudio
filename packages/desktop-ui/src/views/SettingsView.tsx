import React from "react";
import { Icon } from "../components/Icon";
import { Tabs, Badge, Button, EmptyState, ErrorState, Callout, Input } from "@studio/design-system";
import { studioApi } from "../api/client";
import { cleanDisplayPath } from "../utils/path";
import type {
  EngineProbeResult,
  CatalogKit,
  CatalogModule,
  DiskUsageReport,
  SecretStatusItem,
  ProviderPricing,
  EnvProbeItem,
} from "../api/types";

export const SettingsView: React.FC = () => {
  const [activeTab, setActiveTab] = React.useState<"engines" | "env" | "catalog" | "jobs" | "disk" | "api-keys">("engines");
  const [engines, setEngines] = React.useState<EngineProbeResult[]>([]);
  const [kits, setKits] = React.useState<CatalogKit[]>([]);
  const [modules, setModules] = React.useState<CatalogModule[]>([]);
  const [diskUsage, setDiskUsage] = React.useState<DiskUsageReport | null>(null);
  const [envItems, setEnvItems] = React.useState<EnvProbeItem[]>([]);

  // API Keys, Endpoints, and Pricing state
  const [secrets, setSecrets] = React.useState<SecretStatusItem[]>([]);
  const [secretInputs, setSecretInputs] = React.useState<Record<string, string>>({});
  const [secretErrors, setSecretErrors] = React.useState<Record<string, string>>({});
  const [secretSuccess, setSecretSuccess] = React.useState<Record<string, string>>({});

  const [endpoints, setEndpoints] = React.useState<Record<string, string>>({});
  const [endpointInputs, setEndpointInputs] = React.useState<Record<string, string>>({});
  const [endpointErrors, setEndpointErrors] = React.useState<Record<string, string>>({});
  const [endpointSuccess, setEndpointSuccess] = React.useState<Record<string, string>>({});

  const [pricing, setPricing] = React.useState<Record<string, ProviderPricing>>({});
  const [pricingInputs, setPricingInputs] = React.useState<Record<string, { image_usd?: string; video_usd_per_s?: string }>>({});
  const [pricingErrors, setPricingErrors] = React.useState<Record<string, string>>({});
  const [pricingSuccess, setPricingSuccess] = React.useState<Record<string, string>>({});

  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);

  const [agyBusy, setAgyBusy] = React.useState(false);
  const [agyStatus, setAgyStatus] = React.useState<{ type: "success" | "danger"; message: string } | null>(null);

  const handleAgyConnect = async () => {
    setAgyBusy(true);
    setAgyStatus(null);
    try {
      const res = await studioApi.agyConnect();
      setAgyStatus({
        type: "success",
        message: `Đã kết nối thành công agy với Studio MCP entry 'studio' (shim: ${cleanDisplayPath(res.shim) || "studio-mcp"}).`,
      });
      loadData();
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
      loadData();
    } catch (err: any) {
      setAgyStatus({
        type: "danger",
        message: err?.message || "Không thể ngắt kết nối agy.",
      });
    } finally {
      setAgyBusy(false);
    }
  };

  const loadData = React.useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [engList, kitList, modList, disk, secretList, endpointMap, pricingMap, envList] = await Promise.all([
        studioApi.probeEngines().catch(() => []),
        studioApi.getCatalogKits().catch(() => []),
        studioApi.getCatalogModules().catch(() => []),
        studioApi.getDiskUsage().catch(() => null),
        studioApi.getSecretStatus().catch(() => []),
        studioApi.getProviderEndpoints().catch(() => ({})),
        studioApi.getProviderPricing().catch(() => ({})),
        studioApi.envProbe().catch(() => []),
      ]);
      setEngines(engList);
      setKits(kitList);
      setModules(modList);
      setDiskUsage(disk);
      setSecrets(secretList);
      setEndpoints(endpointMap);
      setEndpointInputs(endpointMap);
      setPricing(pricingMap);
      setEnvItems(envList);
    } catch (err: any) {
      setError(err?.message || "Chưa kết nối Studio core");
    } finally {
      setLoading(false);
    }
  }, []);

  React.useEffect(() => {
    loadData();
  }, [loadData]);

  const handleSaveSecret = async (name: string) => {
    const val = secretInputs[name] || "";
    if (!val.trim()) {
      setSecretErrors((prev) => ({ ...prev, [name]: "Vui lòng nhập giá trị khoá" }));
      return;
    }
    setSecretErrors((prev) => ({ ...prev, [name]: "" }));
    try {
      await studioApi.setSecret({ name, value: val });
      setSecretSuccess((prev) => ({ ...prev, [name]: "Đã lưu khoá thành công" }));
      setSecretInputs((prev) => ({ ...prev, [name]: "" }));
      // Reload secret status
      const updated = await studioApi.getSecretStatus();
      setSecrets(updated);
      setTimeout(() => {
        setSecretSuccess((prev) => ({ ...prev, [name]: "" }));
      }, 3000);
    } catch (err: any) {
      setSecretErrors((prev) => ({ ...prev, [name]: err?.message || "Lỗi khi lưu khoá" }));
    }
  };

  const handleDeleteSecret = async (name: string) => {
    try {
      await studioApi.deleteSecret({ name });
      setSecretInputs((prev) => ({ ...prev, [name]: "" }));
      setSecretErrors((prev) => ({ ...prev, [name]: "" }));
      setSecretSuccess((prev) => ({ ...prev, [name]: "Đã xoá khoá thành công" }));
      const updated = await studioApi.getSecretStatus();
      setSecrets(updated);
      setTimeout(() => {
        setSecretSuccess((prev) => ({ ...prev, [name]: "" }));
      }, 3000);
    } catch (err: any) {
      setSecretErrors((prev) => ({ ...prev, [name]: err?.message || "Lỗi khi xoá khoá" }));
    }
  };

  const handleSaveEndpoint = async (toolId: string) => {
    const raw = (endpointInputs[toolId] || "").trim();
    setEndpointErrors((prev) => ({ ...prev, [toolId]: "" }));
    try {
      await studioApi.setProviderEndpoint({
        toolId,
        baseUrl: raw.length > 0 ? raw : null,
      });
      setEndpointSuccess((prev) => ({ ...prev, [toolId]: "Đã lưu endpoint thành công" }));
      const updated = await studioApi.getProviderEndpoints();
      setEndpoints(updated);
      setEndpointInputs(updated);
      setTimeout(() => {
        setEndpointSuccess((prev) => ({ ...prev, [toolId]: "" }));
      }, 3000);
    } catch (err: any) {
      setEndpointErrors((prev) => ({ ...prev, [toolId]: err?.message || "Lỗi kiểm tra hoặc lưu endpoint" }));
    }
  };

  const pricingProviders = [
    { id: "imagen", name: "Google Imagen (Tạo ảnh)", type: "image", defaultCost: "0.04" },
    { id: "openai", name: "OpenAI DALL-E (Tạo ảnh)", type: "image", defaultCost: "0.04" },
    { id: "veo", name: "Google Veo (Tạo video)", type: "video", defaultCost: "0.20" },
    { id: "openai_videos", name: "OpenAI Sora / Videos (Tạo video)", type: "video", defaultCost: "0.20" },
  ];

  const handleSavePricing = async (providerId: string) => {
    const input = pricingInputs[providerId] || {};
    let image_usd: number | null = null;
    let video_usd_per_s: number | null = null;

    if (input.image_usd !== undefined && input.image_usd.trim() !== "") {
      const parsed = parseFloat(input.image_usd);
      if (isNaN(parsed) || !isFinite(parsed) || parsed < 0) {
        setPricingErrors((prev) => ({ ...prev, [providerId]: "Giá ảnh phải là số không âm và hữu hạn (>= 0)" }));
        return;
      }
      image_usd = parsed;
    }

    if (input.video_usd_per_s !== undefined && input.video_usd_per_s.trim() !== "") {
      const parsed = parseFloat(input.video_usd_per_s);
      if (isNaN(parsed) || !isFinite(parsed) || parsed < 0) {
        setPricingErrors((prev) => ({ ...prev, [providerId]: "Giá video phải là số không âm và hữu hạn (>= 0)" }));
        return;
      }
      video_usd_per_s = parsed;
    }

    try {
      setPricingErrors((prev) => ({ ...prev, [providerId]: "" }));
      await studioApi.setProviderPricing({
        providerId,
        pricing: {
          image_usd,
          video_usd_per_s,
          models: pricing[providerId]?.models,
        },
      });
      setPricing((prev) => ({
        ...prev,
        [providerId]: {
          image_usd,
          video_usd_per_s,
          models: prev[providerId]?.models,
        },
      }));
      setPricingSuccess((prev) => ({ ...prev, [providerId]: "Đã lưu giá ước tính thành công" }));
      setTimeout(() => {
        setPricingSuccess((prev) => ({ ...prev, [providerId]: "" }));
      }, 3000);
    } catch (err: any) {
      setPricingErrors((prev) => ({ ...prev, [providerId]: err?.message || "Lỗi lưu giá ước tính" }));
    }
  };

  const handleResetPricing = async (providerId: string) => {
    try {
      await studioApi.setProviderPricing({ providerId, pricing: null });
      setPricing((prev) => {
        const next = { ...prev };
        delete next[providerId];
        return next;
      });
      setPricingInputs((prev) => {
        const next = { ...prev };
        delete next[providerId];
        return next;
      });
      setPricingSuccess((prev) => ({ ...prev, [providerId]: "Đã đặt lại về giá mặc định" }));
      setTimeout(() => {
        setPricingSuccess((prev) => ({ ...prev, [providerId]: "" }));
      }, 3000);
    } catch (err: any) {
      setPricingErrors((prev) => ({ ...prev, [providerId]: err?.message || "Lỗi đặt lại giá mặc định" }));
    }
  };

  const tabs = [
    { id: "engines", label: "Engines & Công cụ" },
    { id: "env", label: "Kiểm tra môi trường" },
    { id: "api-keys", label: "Khoá API & Endpoint" },
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
                        onClick={loadData}
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
                                <p style={{ margin: "0 0 10px 0", fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", lineHeight: 1.5 }}>
                                  Đăng ký MCP server <code>studio</code> toàn cục vào agy và thêm quy tắc cho phép <code>mcp(studio)</code> để agy tự động kết nối và điều khiển Studio MCP shim.
                                </p>
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
          )}

          {/* TAB: KIỂM TRA MÔI TRƯỜNG */}
          {activeTab === "env" && (
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-4)" }}>
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                <div>
                  <h2 style={{ margin: 0, fontSize: "var(--cs-text-md)", fontWeight: 700 }}>
                    Kiểm tra môi trường hệ thống
                  </h2>
                  <p style={{ margin: "2px 0 0", fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
                    Quét trực tiếp các công cụ nhị phân trên máy bằng cách thực thi lệnh thực tế (--version). Không đóng gói sẵn các công cụ này vào ứng dụng.
                  </p>
                </div>
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={loadData}
                  leftIcon={<Icon name="refresh" size={13} />}
                >
                  Quét lại môi trường
                </Button>
              </div>

              <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
                {envItems.map((item) => (
                  <div
                    key={item.id}
                    style={{
                      padding: "var(--cs-space-4)",
                      backgroundColor: "var(--cs-bg-panel)",
                      border: "1px solid var(--cs-border-subtle)",
                      borderRadius: "var(--cs-radius-control)",
                      display: "flex",
                      flexDirection: "column",
                      gap: 8,
                    }}
                  >
                    <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                        <Icon
                          name={item.found ? "check-circle" : "alert-circle"}
                          size={18}
                          color={item.found ? "var(--cs-success)" : "var(--cs-danger)"}
                        />
                        <span style={{ fontWeight: 700, fontSize: "var(--cs-text-sm)" }}>
                          {item.name}
                        </span>
                        <Badge variant={item.found ? "success" : "danger"}>
                          {item.found ? "Đã có" : "Chưa có"}
                        </Badge>
                      </div>
                      {item.version && (
                        <span style={{ fontSize: "var(--cs-text-xs)", fontFamily: "var(--cs-font-mono)", color: "var(--cs-fg-primary)" }}>
                          Phiên bản: {item.version}
                        </span>
                      )}
                    </div>

                    {item.path && (
                      <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)", fontFamily: "var(--cs-font-mono)" }}>
                        Đường dẫn: {cleanDisplayPath(item.path)}
                      </div>
                    )}

                    {!item.found && (
                      <div style={{ marginTop: 2, padding: "8px 12px", backgroundColor: "var(--cs-bg-raised)", border: "1px dashed var(--cs-border-subtle)", borderRadius: "var(--cs-radius-control)" }}>
                        <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-warning)", fontWeight: 600 }}>
                          Hướng dẫn cài đặt:
                        </div>
                        <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", marginTop: 2 }}>
                          {item.hint}
                        </div>
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* TAB: KHOÁ API & ENDPOINT */}
          {activeTab === "api-keys" && (
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-5)" }}>
              <Callout variant="neutral" title="Bảo mật khoá API">
                Khoá API được mã hoá và lưu duy nhất trong Windows Credential Manager. Giao diện người dùng không bao giờ hiển thị lại giá trị khoá sau khi lưu.
              </Callout>

              {/* SECTION 1: KHOÁ API */}
              <div
                style={{
                  padding: "var(--cs-space-4)",
                  backgroundColor: "var(--cs-bg-panel)",
                  border: "1px solid var(--cs-border-subtle)",
                  borderRadius: "var(--cs-radius-control)",
                  display: "flex",
                  flexDirection: "column",
                  gap: "var(--cs-space-4)",
                }}
              >
                <div>
                  <h2 style={{ margin: "0 0 4px 0", fontSize: "var(--cs-text-md)", fontWeight: 700 }}>
                    Khoá bí mật (API Keys)
                  </h2>
                  <p style={{ margin: 0, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
                    Quản lý khoá bảo mật được sử dụng bởi các công cụ tạo sinh ảnh/video và tổng hợp giọng nói.
                  </p>
                </div>

                {secrets.length === 0 ? (
                  <div style={{ fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-muted)" }}>
                    Chưa có khoá API nào được yêu cầu bởi các công cụ đã cài đặt.
                  </div>
                ) : (
                  <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
                    {secrets.map((item) => {
                      const inputVal = secretInputs[item.name] || "";
                      const err = secretErrors[item.name];
                      const succ = secretSuccess[item.name];

                      return (
                        <div
                          key={item.name}
                          style={{
                            padding: "var(--cs-space-3) var(--cs-space-4)",
                            backgroundColor: "var(--cs-bg-raised)",
                            border: "1px solid var(--cs-border-subtle)",
                            borderRadius: "var(--cs-radius-control)",
                            display: "flex",
                            flexDirection: "column",
                            gap: "var(--cs-space-2)",
                          }}
                        >
                          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                              <Icon name="shield" size={16} color="var(--cs-accent)" />
                              <span style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)", fontFamily: "var(--cs-font-mono)" }}>
                                {item.name}
                              </span>
                              <Badge variant={item.present ? "success" : "warning"}>
                                {item.present ? "Đã có khoá" : "Chưa cấu hình"}
                              </Badge>
                            </div>
                          </div>

                          <div style={{ display: "flex", gap: "var(--cs-space-2)", alignItems: "flex-start", marginTop: 4 }}>
                            <div style={{ flex: 1 }}>
                              <Input
                                type="password"
                                placeholder={item.present ? "•••••••••••••••• (Nhập khoá mới để cập nhật)" : "Nhập khoá API..."}
                                value={inputVal}
                                onChange={(e) => {
                                  const val = e.target.value;
                                  setSecretInputs((prev) => ({ ...prev, [item.name]: val }));
                                  if (secretErrors[item.name]) {
                                    setSecretErrors((prev) => ({ ...prev, [item.name]: "" }));
                                  }
                                }}
                                error={err}
                              />
                            </div>
                            <Button
                              variant="primary"
                              size="sm"
                              onClick={() => handleSaveSecret(item.name)}
                              disabled={!inputVal.trim()}
                            >
                              Lưu
                            </Button>
                            {item.present && (
                              <Button
                                variant="destructive"
                                size="sm"
                                onClick={() => handleDeleteSecret(item.name)}
                              >
                                Xoá
                              </Button>
                            )}
                          </div>

                          {succ && (
                            <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-success)", fontWeight: 500 }}>
                              ✓ {succ}
                            </div>
                          )}
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>

              {/* SECTION 2: ENDPOINT TUỲ CHỈNH */}
              <div
                style={{
                  padding: "var(--cs-space-4)",
                  backgroundColor: "var(--cs-bg-panel)",
                  border: "1px solid var(--cs-border-subtle)",
                  borderRadius: "var(--cs-radius-control)",
                  display: "flex",
                  flexDirection: "column",
                  gap: "var(--cs-space-4)",
                }}
              >
                <div>
                  <h2 style={{ margin: "0 0 4px 0", fontSize: "var(--cs-text-md)", fontWeight: 700 }}>
                    Endpoint Nhà cung cấp (Provider Endpoints)
                  </h2>
                  <p style={{ margin: 0, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
                    Định tuyến các yêu cầu gọi model sang gateway tương thích cục bộ hoặc máy chủ proxy.
                  </p>
                </div>

                <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-4)" }}>
                  {[
                    {
                      id: "google.imagen",
                      title: "Google Imagen (image.generate@1)",
                      defaultUrl: "https://generativelanguage.googleapis.com/v1beta",
                    },
                    {
                      id: "google.veo",
                      title: "Google Veo (video.generate@1)",
                      defaultUrl: "https://generativelanguage.googleapis.com/v1beta",
                    },
                    {
                      id: "openai.images",
                      title: "OpenAI Images (image.generate@1)",
                      defaultUrl: "https://api.openai.com/v1",
                    },
                    {
                      id: "openai.videos",
                      title: "OpenAI Videos (video.generate@1)",
                      defaultUrl: "https://api.openai.com/v1",
                    },
                  ].map((p) => {
                    const currentVal = endpointInputs[p.id] ?? (endpoints[p.id] || "");
                    const err = endpointErrors[p.id];
                    const succ = endpointSuccess[p.id];

                    return (
                      <div
                        key={p.id}
                        style={{
                          padding: "var(--cs-space-3) var(--cs-space-4)",
                          backgroundColor: "var(--cs-bg-raised)",
                          border: "1px solid var(--cs-border-subtle)",
                          borderRadius: "var(--cs-radius-control)",
                          display: "flex",
                          flexDirection: "column",
                          gap: "var(--cs-space-2)",
                        }}
                      >
                        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                          <span style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)" }}>
                            {p.title}
                          </span>
                          <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)", fontFamily: "var(--cs-font-mono)" }}>
                            ID: {p.id}
                          </span>
                        </div>

                        <div style={{ display: "flex", gap: "var(--cs-space-2)", alignItems: "flex-start", marginTop: 4 }}>
                          <div style={{ flex: 1 }}>
                            <Input
                              label="Endpoint tuỳ chỉnh"
                              placeholder={p.defaultUrl}
                              hint="Để trống để dùng endpoint chính thức. Ví dụ gateway tương thích OpenAI: https://…/v1"
                              value={currentVal}
                              onChange={(e) => {
                                const val = e.target.value;
                                setEndpointInputs((prev) => ({ ...prev, [p.id]: val }));
                                if (endpointErrors[p.id]) {
                                  setEndpointErrors((prev) => ({ ...prev, [p.id]: "" }));
                                }
                              }}
                              error={err}
                            />
                          </div>
                          <div style={{ marginTop: 22, display: "flex", gap: "var(--cs-space-2)" }}>
                            <Button
                              variant="secondary"
                              size="sm"
                              onClick={() => handleSaveEndpoint(p.id)}
                            >
                              Lưu
                            </Button>
                            {endpoints[p.id] && (
                              <Button
                                variant="quiet"
                                size="sm"
                                onClick={() => {
                                  setEndpointInputs((prev) => ({ ...prev, [p.id]: "" }));
                                  studioApi.setProviderEndpoint({ toolId: p.id, baseUrl: null }).then(() => {
                                    setEndpoints((prev) => {
                                      const next = { ...prev };
                                      delete next[p.id];
                                      return next;
                                    });
                                  });
                                }}
                              >
                                Đặt lại mặc định
                              </Button>
                            )}
                          </div>
                        </div>

                        {succ && (
                          <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-success)", fontWeight: 500 }}>
                            ✓ {succ}
                          </div>
                        )}
                      </div>
                    );
                  })}
                </div>
              </div>

              {/* SECTION 3: GIÁ ƯỚC TÍNH */}
              <div
                style={{
                  padding: "var(--cs-space-4)",
                  backgroundColor: "var(--cs-bg-panel)",
                  border: "1px solid var(--cs-border-subtle)",
                  borderRadius: "var(--cs-radius-control)",
                  display: "flex",
                  flexDirection: "column",
                  gap: "var(--cs-space-4)",
                }}
              >
                <div>
                  <h2 style={{ margin: 0, fontSize: "var(--cs-text-md)", fontWeight: 700 }}>
                    Giá ước tính (Estimated Pricing)
                  </h2>
                  <p style={{ margin: "4px 0 0", fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
                    Định giá USD ước tính theo từng provider cho mỗi lần tạo ảnh hoặc mỗi giây video. Giá trị này được lưu tại tầng ứng dụng (không chứa bí mật) và cung cấp cho agent tính toán ngân sách.
                  </p>
                </div>

                <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
                  {pricingProviders.map((p) => {
                    const current = pricing[p.id];
                    const inputVal = pricingInputs[p.id] || {};
                    const err = pricingErrors[p.id];
                    const succ = pricingSuccess[p.id];

                    return (
                      <div
                        key={p.id}
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
                          <div>
                            <span style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)" }}>{p.name}</span>
                            <span style={{ marginLeft: 8, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)", fontFamily: "var(--cs-font-mono)" }}>
                              ({p.id})
                            </span>
                          </div>
                          <Badge variant={current ? "info" : "neutral"}>
                            {current ? "Đã tuỳ chỉnh" : "Mặc định hệ thống"}
                          </Badge>
                        </div>

                        <div style={{ display: "flex", gap: "var(--cs-space-4)", alignItems: "flex-start" }}>
                          {p.type === "image" && (
                            <div style={{ flex: 1 }}>
                              <Input
                                label="Giá mỗi ảnh (USD)"
                                placeholder={`Mặc định: $${p.defaultCost}`}
                                hint="Đơn giá USD cho một lần sinh ảnh (vd: 0.04)"
                                value={inputVal.image_usd !== undefined ? inputVal.image_usd : (current?.image_usd != null ? String(current.image_usd) : "")}
                                onChange={(e) => {
                                  const val = e.target.value;
                                  setPricingInputs((prev) => ({
                                    ...prev,
                                    [p.id]: { ...prev[p.id], image_usd: val },
                                  }));
                                  if (pricingErrors[p.id]) {
                                    setPricingErrors((prev) => ({ ...prev, [p.id]: "" }));
                                  }
                                }}
                                error={err}
                              />
                            </div>
                          )}

                          {p.type === "video" && (
                            <div style={{ flex: 1 }}>
                              <Input
                                label="Giá mỗi giây video (USD/s)"
                                placeholder={`Mặc định: $${p.defaultCost}/s`}
                                hint="Đơn giá USD cho một giây video tạo ra (vd: 0.20)"
                                value={inputVal.video_usd_per_s !== undefined ? inputVal.video_usd_per_s : (current?.video_usd_per_s != null ? String(current.video_usd_per_s) : "")}
                                onChange={(e) => {
                                  const val = e.target.value;
                                  setPricingInputs((prev) => ({
                                    ...prev,
                                    [p.id]: { ...prev[p.id], video_usd_per_s: val },
                                  }));
                                  if (pricingErrors[p.id]) {
                                    setPricingErrors((prev) => ({ ...prev, [p.id]: "" }));
                                  }
                                }}
                                error={err}
                              />
                            </div>
                          )}

                          <div style={{ marginTop: 22, display: "flex", gap: "var(--cs-space-2)" }}>
                            <Button
                              variant="secondary"
                              size="sm"
                              onClick={() => handleSavePricing(p.id)}
                            >
                              Lưu
                            </Button>
                            {current && (
                              <Button
                                variant="quiet"
                                size="sm"
                                onClick={() => handleResetPricing(p.id)}
                              >
                                Đặt lại mặc định
                              </Button>
                            )}
                          </div>
                        </div>

                        {succ && (
                          <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-success)", fontWeight: 500 }}>
                            ✓ {succ}
                          </div>
                        )}
                      </div>
                    );
                  })}
                </div>
              </div>
            </div>
          )}

          {/* TAB 3: CATALOG MODULES & KITS */}
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
