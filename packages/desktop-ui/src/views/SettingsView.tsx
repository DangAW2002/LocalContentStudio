import React from "react";
import { Icon } from "../components/Icon";
import { Tabs, Badge, Button, EmptyState, ErrorState, Callout, Input, Spinner } from "@studio/design-system";
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
  InstalledToolInfo,
  ToolInstallPreview,
  ToolInstallScope,
  YouTubeChannelAccount,
  YoutubeClientStatus,
  YouTubeErrorDetail,
} from "../api/types";
import { parseYouTubeError } from "../api/client";
import { YouTubeGuideModal } from "../components/YouTubeGuideModal";

export interface SettingsViewProps {
  initialTab?: "engines" | "tools" | "env" | "catalog" | "jobs" | "disk" | "api-keys" | "accounts";
}

export const SettingsView: React.FC<SettingsViewProps> = ({ initialTab = "engines" }) => {
  const [activeTab, setActiveTab] = React.useState<"engines" | "tools" | "env" | "catalog" | "jobs" | "disk" | "api-keys" | "accounts">(initialTab);

  React.useEffect(() => {
    if (initialTab) {
      setActiveTab(initialTab);
    }
  }, [initialTab]);
  const [engines, setEngines] = React.useState<EngineProbeResult[]>([]);
  const [installedTools, setInstalledTools] = React.useState<InstalledToolInfo[]>([]);
  const [installModalOpen, setInstallModalOpen] = React.useState<"folder" | "url" | null>(null);
  const [installSourceInput, setInstallSourceInput] = React.useState("");
  const [installScopeInput, setInstallScopeInput] = React.useState<ToolInstallScope>("app");
  const [installPreview, setInstallPreview] = React.useState<ToolInstallPreview | null>(null);
  const [installBusy, setInstallBusy] = React.useState(false);
  const [installError, setInstallError] = React.useState<string | null>(null);
  const [actionSuccess, setActionSuccess] = React.useState<string | null>(null);
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

  // Publishing accounts (Tài khoản đăng bài)
  const [ytClientStatus, setYtClientStatus] = React.useState<YoutubeClientStatus | null>(null);
  const [ytChannels, setYtChannels] = React.useState<YouTubeChannelAccount[]>([]);
  const [ytImportMode, setYtImportMode] = React.useState<"file" | "manual">("file");
  const [ytClientIdInput, setYtClientIdInput] = React.useState("");
  const [ytClientSecretInput, setYtClientSecretInput] = React.useState("");
  const [ytImportBusy, setYtImportBusy] = React.useState(false);
  const [ytImportError, setYtImportError] = React.useState<string | null>(null);
  const [ytImportSuccess, setYtImportSuccess] = React.useState<string | null>(null);
  const [ytConnecting, setYtConnecting] = React.useState(false);
  const [ytCountdown, setYtCountdown] = React.useState(300);
  const [ytConnectError, setYtConnectError] = React.useState<YouTubeErrorDetail | null>(null);
  const [ytConnectSuccess, setYtConnectSuccess] = React.useState<string | null>(null);
  const [ytDisconnectingId, setYtDisconnectingId] = React.useState<string | null>(null);
  const [ytRevokeNotice, setYtRevokeNotice] = React.useState(false);
  const [guideModalOpen, setGuideModalOpen] = React.useState(false);
  const ytFileInputRef = React.useRef<HTMLInputElement | null>(null);

  React.useEffect(() => {
    let timer: any = null;
    if (ytConnecting && ytCountdown > 0) {
      timer = setInterval(() => {
        setYtCountdown((prev) => Math.max(0, prev - 1));
      }, 1000);
    }
    return () => {
      if (timer) clearInterval(timer);
    };
  }, [ytConnecting, ytCountdown]);

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
      const [engList, kitList, modList, disk, secretList, endpointMap, pricingMap, envList, toolList, ytClient, ytChanList] = await Promise.all([
        studioApi.probeEngines().catch(() => []),
        studioApi.getCatalogKits().catch(() => []),
        studioApi.getCatalogModules().catch(() => []),
        studioApi.getDiskUsage().catch(() => null),
        studioApi.getSecretStatus().catch(() => []),
        studioApi.getProviderEndpoints().catch(() => ({})),
        studioApi.getProviderPricing().catch(() => ({})),
        studioApi.envProbe().catch(() => []),
        studioApi.toolListInstalled().catch(() => []),
        studioApi.youtubeClientStatus().catch(() => null),
        studioApi.youtubeChannelsList().catch(() => []),
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
      setInstalledTools(toolList);
      setYtClientStatus(ytClient);
      setYtChannels(ytChanList);
    } catch (err: any) {
      setError(err?.message || "Chưa kết nối Studio core");
    } finally {
      setLoading(false);
    }
  }, []);

  const handleOpenInstallModal = (type: "folder" | "url") => {
    setInstallModalOpen(type);
    setInstallSourceInput("");
    setInstallScopeInput("app");
    setInstallPreview(null);
    setInstallError(null);
  };

  const handleCloseInstallModal = () => {
    setInstallModalOpen(null);
    setInstallPreview(null);
    setInstallError(null);
    setInstallBusy(false);
  };

  const handlePreviewInstall = async () => {
    const src = installSourceInput.trim();
    if (!src) {
      setInstallError("Vui lòng nhập đường dẫn thư mục hoặc địa chỉ URL");
      return;
    }
    setInstallBusy(true);
    setInstallError(null);
    try {
      const preview = await studioApi.toolInstallPreview({
        source: src,
        scope: installScopeInput,
      });
      setInstallPreview(preview);
    } catch (err: any) {
      setInstallError(err?.message || "Lỗi khi kiểm tra gói công cụ");
    } finally {
      setInstallBusy(false);
    }
  };

  const handleConfirmInstall = async () => {
    if (!installPreview) return;
    setInstallBusy(true);
    setInstallError(null);
    try {
      const installed = await studioApi.toolInstallConfirm({
        previewId: installPreview.previewId,
      });
      handleCloseInstallModal();
      setActionSuccess(`Đã cài đặt thành công công cụ ${installed.id} (${installed.version})`);
      setTimeout(() => setActionSuccess(null), 4000);
      loadData();
    } catch (err: any) {
      setInstallError(err?.message || "Lỗi khi hoàn tất cài đặt");
    } finally {
      setInstallBusy(false);
    }
  };

  const handleUninstall = async (tool: InstalledToolInfo) => {
    if (!window.confirm(`Bạn có chắc chắn muốn gỡ bỏ công cụ "${tool.id}" (${tool.version}) khỏi hệ thống?`)) {
      return;
    }
    try {
      await studioApi.toolUninstall({
        toolId: tool.id,
        version: tool.version,
      });
      setActionSuccess(`Đã gỡ bỏ công cụ ${tool.id} thành công`);
      setTimeout(() => setActionSuccess(null), 4000);
      loadData();
    } catch (err: any) {
      alert(`Lỗi khi gỡ bỏ công cụ: ${err?.message || err}`);
    }
  };

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

  const handleImportJsonFile = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    setYtImportBusy(true);
    setYtImportError(null);
    setYtImportSuccess(null);
    const reader = new FileReader();
    reader.onload = async (evt) => {
      const content = evt.target?.result as string;
      if (!content) {
        setYtImportError("Tệp tải lên rỗng.");
        setYtImportBusy(false);
        return;
      }
      try {
        const res = await studioApi.youtubeClientImport({ json: content });
        setYtImportSuccess(`Đã nạp OAuth client thành công! (…${res.clientIdLast6})`);
        const status = await studioApi.youtubeClientStatus();
        setYtClientStatus(status);
      } catch (err: any) {
        const parsed = parseYouTubeError(err);
        setYtImportError(parsed ? parsed.messageVi : (err?.message || "Không thể nạp tệp client secret."));
      } finally {
        setYtImportBusy(false);
        if (ytFileInputRef.current) ytFileInputRef.current.value = "";
      }
    };
    reader.onerror = () => {
      setYtImportError("Không đọc được tệp JSON.");
      setYtImportBusy(false);
    };
    reader.readAsText(file);
  };

  const handleImportManual = async () => {
    if (!ytClientIdInput.trim() || !ytClientSecretInput.trim()) {
      setYtImportError("Vui lòng nhập cả Client ID và Client Secret.");
      return;
    }
    setYtImportBusy(true);
    setYtImportError(null);
    setYtImportSuccess(null);
    try {
      const res = await studioApi.youtubeClientImport({
        clientId: ytClientIdInput.trim(),
        clientSecret: ytClientSecretInput.trim(),
      });
      setYtImportSuccess(`Đã nạp OAuth client thành công! (…${res.clientIdLast6})`);
      setYtClientIdInput("");
      setYtClientSecretInput("");
      const status = await studioApi.youtubeClientStatus();
      setYtClientStatus(status);
    } catch (err: any) {
      const parsed = parseYouTubeError(err);
      setYtImportError(parsed ? parsed.messageVi : (err?.message || "Không thể lưu Client ID/Secret."));
    } finally {
      setYtImportBusy(false);
    }
  };

  const handleConnectYouTube = async () => {
    setYtConnecting(true);
    setYtCountdown(300);
    setYtConnectError(null);
    setYtConnectSuccess(null);
    try {
      const account = await studioApi.youtubeConnect();
      setYtConnectSuccess(`Kết nối thành công kênh "${account.title}" (${account.handle ? "@" + account.handle : account.id})!`);
      const updatedList = await studioApi.youtubeChannelsList();
      setYtChannels(updatedList);
    } catch (err: any) {
      const parsed = parseYouTubeError(err);
      if (parsed) {
        setYtConnectError(parsed);
      } else {
        setYtConnectError({
          code: "unknown",
          messageVi: err?.message || "Không thể kết nối YouTube",
          fixVi: "Vui lòng kiểm tra kết nối mạng và thử lại.",
        });
      }
    } finally {
      setYtConnecting(false);
    }
  };

  const handleDisconnectChannel = async (channelId: string) => {
    setYtDisconnectingId(channelId);
    try {
      await studioApi.youtubeChannelDisconnect(channelId);
      setYtRevokeNotice(true);
      const updatedList = await studioApi.youtubeChannelsList();
      setYtChannels(updatedList);
    } catch (err: any) {
      setError(err?.message || "Không thể ngắt kết nối kênh.");
    } finally {
      setYtDisconnectingId(null);
    }
  };

  const tabs = [
    { id: "engines", label: "Engines" },
    { id: "tools", label: "Công cụ" },
    { id: "accounts", label: "Tài khoản đăng bài" },
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
          )}

          {/* TAB 2: CÔNG CỤ (TOOLS) */}
          {activeTab === "tools" && (
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-4)" }}>
              {actionSuccess && (
                <Callout variant="success">
                  {actionSuccess}
                </Callout>
              )}

              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                <div>
                  <h2 style={{ margin: 0, fontSize: "var(--cs-text-md)", fontWeight: 700 }}>
                    Công cụ Hệ thống & Mở rộng
                  </h2>
                  <p style={{ margin: "2px 0 0", fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
                    Quản lý các công cụ có sẵn và công cụ cài đặt thêm từ thư mục cục bộ hoặc URL HTTPS.
                  </p>
                </div>

                <div style={{ display: "flex", gap: "var(--cs-space-2)" }}>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => handleOpenInstallModal("folder")}
                    leftIcon={<Icon name="folder" size={13} />}
                  >
                    Cài từ thư mục…
                  </Button>
                  <Button
                    size="sm"
                    variant="primary"
                    onClick={() => handleOpenInstallModal("url")}
                    leftIcon={<Icon name="link" size={13} />}
                  >
                    Cài từ URL…
                  </Button>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={loadData}
                    leftIcon={<Icon name="refresh" size={13} />}
                  >
                    Làm mới
                  </Button>
                </div>
              </div>

              {/* Install / Preview Modal */}
              {installModalOpen && (
                <div
                  style={{
                    padding: "var(--cs-space-4)",
                    backgroundColor: "var(--cs-bg-panel)",
                    border: "2px solid var(--cs-accent)",
                    borderRadius: "var(--cs-radius-control)",
                    display: "flex",
                    flexDirection: "column",
                    gap: "var(--cs-space-4)",
                  }}
                >
                  <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                      <Icon name={installModalOpen === "folder" ? "folder" : "download"} size={18} color="var(--cs-accent)" />
                      <h3 style={{ margin: 0, fontSize: "var(--cs-text-md)", fontWeight: 700 }}>
                        {installModalOpen === "folder" ? "Cài đặt công cụ từ thư mục cục bộ" : "Cài đặt công cụ từ URL HTTPS"}
                      </h3>
                    </div>
                    <Button size="sm" variant="secondary" onClick={handleCloseInstallModal}>
                      Đóng
                    </Button>
                  </div>

                  {!installPreview ? (
                    <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
                      <p style={{ margin: 0, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
                        {installModalOpen === "folder"
                          ? "Nhập đường dẫn tuyệt đối đến thư mục chứa tệp cấu hình tool.yaml của công cụ."
                          : "Nhập đường dẫn HTTPS đến tệp nén .zip chứa công cụ (tối đa 100 MB). Hệ thống tự động kiểm tra an toàn chống zip-slip."}
                      </p>

                      <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                        <label style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600 }}>
                          {installModalOpen === "folder" ? "Đường dẫn thư mục:" : "Địa chỉ HTTPS (.zip):"}
                        </label>
                        <Input
                          value={installSourceInput}
                          onChange={(e: any) => setInstallSourceInput(e.target.value)}
                          placeholder={
                            installModalOpen === "folder"
                              ? "Ví dụ: C:\\Users\\MyTools\\custom-tool"
                              : "Ví dụ: https://example.com/tools/my-tool-1.0.0.zip"
                          }
                        />
                      </div>

                      <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                        <label style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600 }}>Phạm vi cài đặt:</label>
                        <div style={{ display: "flex", gap: "var(--cs-space-4)", fontSize: "var(--cs-text-xs)" }}>
                          <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer" }}>
                            <input
                              type="radio"
                              name="toolScope"
                              value="app"
                              checked={installScopeInput === "app"}
                              onChange={() => setInstallScopeInput("app")}
                            />
                            Toàn ứng dụng (App - dùng cho mọi workspace)
                          </label>
                          <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer" }}>
                            <input
                              type="radio"
                              name="toolScope"
                              value="workspace"
                              checked={installScopeInput === "workspace"}
                              onChange={() => setInstallScopeInput("workspace")}
                            />
                            Riêng Workspace hiện tại (.studio/tools)
                          </label>
                        </div>
                      </div>

                      {installError && <Callout variant="danger">{installError}</Callout>}

                      <div style={{ display: "flex", justifyContent: "flex-end", gap: 8, marginTop: 4 }}>
                        <Button variant="secondary" onClick={handleCloseInstallModal}>
                          Hủy
                        </Button>
                        <Button
                          variant="primary"
                          onClick={handlePreviewInstall}
                          busy={installBusy}
                          busyText="Đang kiểm tra..."
                        >
                          Kiểm tra & Xem trước
                        </Button>
                      </div>
                    </div>
                  ) : (
                    /* PREVIEW CONFIRMATION DIALOG */
                    <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-4)" }}>
                      <Callout variant="warning" title="Đánh giá rủi ro an toàn công cụ">
                        Công cụ ngoài có thể chạy các tiến trình trên máy của bạn. Hãy xem xét kỹ các quyền hạn và câu lệnh dưới đây trước khi cấp phép cài đặt.
                      </Callout>

                      <div
                        style={{
                          display: "grid",
                          gridTemplateColumns: "140px 1fr",
                          gap: "8px 12px",
                          fontSize: "var(--cs-text-xs)",
                          backgroundColor: "var(--cs-bg-raised)",
                          padding: "var(--cs-space-3)",
                          borderRadius: "var(--cs-radius-control)",
                        }}
                      >
                        <span style={{ color: "var(--cs-fg-muted)" }}>Tên công cụ:</span>
                        <span style={{ fontWeight: 700 }}>{installPreview.toolId} (phiên bản: {installPreview.version})</span>

                        <span style={{ color: "var(--cs-fg-muted)" }}>Phân loại:</span>
                        <span>{installPreview.kind}</span>

                        <span style={{ color: "var(--cs-fg-muted)" }}>Mô tả:</span>
                        <span>{installPreview.description || "Không có mô tả"}</span>

                        <span style={{ color: "var(--cs-fg-muted)" }}>Lệnh thực thi:</span>
                        <span style={{ fontFamily: "var(--cs-font-mono)", color: "var(--cs-accent)" }}>
                          {installPreview.command} {installPreview.args.join(" ")}
                        </span>

                        <span style={{ color: "var(--cs-fg-muted)" }}>Khả năng cung cấp:</span>
                        <span>{installPreview.provides.join(", ") || "Không có"}</span>

                        <span style={{ color: "var(--cs-fg-muted)" }}>Yêu cầu phụ thuộc:</span>
                        <span>{installPreview.needs.join(", ") || "Không yêu cầu"}</span>

                        <span style={{ color: "var(--cs-fg-muted)" }}>Mạng / Internet:</span>
                        <span>{installPreview.network || "Không yêu cầu truy cập mạng"}</span>

                        <span style={{ color: "var(--cs-fg-muted)" }}>Mức độ tác động:</span>
                        <span style={{ color: "var(--cs-warning)", fontWeight: 600 }}>{installPreview.sideEffect}</span>

                        <span style={{ color: "var(--cs-fg-muted)" }}>Thư mục cài đặt:</span>
                        <span style={{ fontFamily: "var(--cs-font-mono)" }}>{installPreview.targetDir}</span>

                        {installPreview.isReinstall && (
                          <>
                            <span style={{ color: "var(--cs-danger)", fontWeight: 700 }}>Lưu ý cài đè:</span>
                            <span style={{ color: "var(--cs-danger)" }}>Phiên bản này đã tồn tại, cài đặt sẽ ghi đè phiên bản cũ.</span>
                          </>
                        )}
                      </div>

                      {installError && <Callout variant="danger">{installError}</Callout>}

                      <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
                        <Button variant="secondary" onClick={() => setInstallPreview(null)}>
                          Quay lại
                        </Button>
                        <Button
                          variant="primary"
                          onClick={handleConfirmInstall}
                          busy={installBusy}
                          busyText="Đang cài đặt..."
                        >
                          Xác nhận Cài đặt Công cụ
                        </Button>
                      </div>
                    </div>
                  )}
                </div>
              )}

              {/* Tool list */}
              {installedTools.length === 0 ? (
                <EmptyState
                  icon={<Icon name="cpu" size={28} />}
                  title="Chưa có công cụ nào"
                  description="Không tìm thấy công cụ trong hệ thống. Bấm Làm mới hoặc cài đặt thêm."
                />
              ) : (
                <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
                  {installedTools.map((tool) => {
                    const isBuiltin =
                      !tool.version ||
                      tool.version === "builtin" ||
                      tool.scope?.toLowerCase().includes("builtin") ||
                      tool.scope?.toLowerCase().includes("có sẵn");

                    const isRealSemver =
                      !!tool.version &&
                      tool.version !== "builtin" &&
                      /^\d+(\.\d+)*(-.+)?$/.test(tool.version.trim());

                    const note = (tool.probe.noteVi || tool.probe.note_vi || "").trim();
                    let probeText = "";
                    if (note.length > 0) {
                      probeText = note;
                    } else if (tool.probe.missing && tool.probe.missing.length > 0) {
                      probeText = `Thiếu: ${tool.probe.missing.join(", ")}`;
                    } else if (tool.probe.available) {
                      probeText = "Sẵn sàng";
                    }

                    return (
                      <div
                        key={`${tool.id}@${tool.version || "builtin"}`}
                        style={{
                          padding: "var(--cs-space-4)",
                          backgroundColor: "var(--cs-bg-panel)",
                          border: "1px solid var(--cs-border-subtle)",
                          borderRadius: "var(--cs-radius-control)",
                          display: "flex",
                          flexDirection: "column",
                          gap: 10,
                        }}
                      >
                        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                            <Icon name="tool" size={16} color="var(--cs-accent)" />
                            <span style={{ fontWeight: 700, fontSize: "var(--cs-text-sm)" }}>
                              {tool.id}
                            </span>
                            {isBuiltin ? (
                              <Badge variant="neutral">Có sẵn</Badge>
                            ) : (
                              <>
                                {tool.version && (
                                  <span style={{ fontSize: "var(--cs-text-xs)", fontFamily: "var(--cs-font-mono)", color: "var(--cs-fg-muted)" }}>
                                    {isRealSemver ? `v${tool.version}` : tool.version}
                                  </span>
                                )}
                                <Badge variant="info">
                                  {tool.scope}
                                </Badge>
                              </>
                            )}
                            <Badge variant={tool.probe.available ? "success" : "warning"}>
                              {tool.probe.available ? "Sẵn sàng" : "Thiếu phụ thuộc"}
                            </Badge>
                          </div>

                          {tool.canUninstall && (
                            <Button
                              size="sm"
                              variant="secondary"
                              onClick={() => handleUninstall(tool)}
                              leftIcon={<Icon name="trash" size={12} />}
                            >
                              Gỡ bỏ
                            </Button>
                          )}
                        </div>

                        <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
                          {tool.manifest.description || "Không có mô tả"}
                        </div>

                        <div style={{ display: "flex", flexWrap: "wrap", gap: 12, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
                          {tool.manifest.run?.command && (
                            <span>Lệnh: <code style={{ color: "var(--cs-fg-primary)" }}>{tool.manifest.run?.command}</code></span>
                          )}
                          {tool.manifest.provides?.length > 0 && (
                            <span>Cung cấp: {tool.manifest.provides.join(", ")}</span>
                          )}
                          {probeText.length > 0 && (
                            <span>
                              Trạng thái probe:{" "}
                              <strong style={{ color: tool.probe.available ? "var(--cs-success)" : "var(--cs-warning)" }}>
                                {probeText}
                              </strong>
                            </span>
                          )}
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          )}

          {/* TAB: TÀI KHOẢN ĐĂNG BÀI */}
          {activeTab === "accounts" && (
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-4)" }}>
              {/* Header */}
              <div style={{ display: "flex", alignItems: "flex-start", justifyContent: "space-between", gap: 16 }}>
                <div>
                  <h2 style={{ margin: "0 0 4px 0", fontSize: "var(--cs-text-md)", fontWeight: 700 }}>
                    Tài khoản Đăng bài (YouTube)
                  </h2>
                  <p style={{ margin: 0, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", lineHeight: 1.5 }}>
                    Quản lý OAuth client và các kênh xuất bản nội dung YouTube. Khóa xác thực được lưu trữ mã hoá an toàn trong Windows Credential Manager trên máy bạn.
                  </p>
                </div>
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() => setGuideModalOpen(true)}
                  leftIcon={<Icon name="help" size={13} />}
                >
                  Xem hướng dẫn thiết lập
                </Button>
              </div>

              {/* SECTION 1: Nạp OAuth client */}
              <div
                style={{
                  padding: "var(--cs-space-4)",
                  backgroundColor: "var(--cs-bg-panel)",
                  border: "1px solid var(--cs-border-subtle)",
                  borderRadius: "var(--cs-radius-control)",
                  display: "flex",
                  flexDirection: "column",
                  gap: "var(--cs-space-3)",
                }}
              >
                <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <span style={{ fontWeight: 700, fontSize: "var(--cs-text-sm)" }}>
                      1. Nạp Google OAuth Client
                    </span>
                    <Badge variant={ytClientStatus?.configured ? "success" : "warning"}>
                      {ytClientStatus?.configured
                        ? `Đã nạp (…${ytClientStatus.clientIdLast6 || "******"})`
                        : "Chưa nạp"}
                    </Badge>
                  </div>
                </div>

                <p style={{ margin: 0, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", lineHeight: 1.5 }}>
                  Studio sử dụng OAuth Client của chính bạn từ Google Cloud Console để tải video lên YouTube. Không bao giờ gửi mã khóa lên máy chủ bên ngoài.
                </p>

                {/* Import method toggle */}
                <div style={{ display: "flex", gap: "var(--cs-space-4)", fontSize: "var(--cs-text-xs)", marginTop: 4 }}>
                  <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer" }}>
                    <input
                      type="radio"
                      name="ytImportMode"
                      value="file"
                      checked={ytImportMode === "file"}
                      onChange={() => setYtImportMode("file")}
                    />
                    Chọn tệp client_secret_*.json (Khuyên dùng)
                  </label>
                  <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer" }}>
                    <input
                      type="radio"
                      name="ytImportMode"
                      value="manual"
                      checked={ytImportMode === "manual"}
                      onChange={() => setYtImportMode("manual")}
                    />
                    Nhập Client ID & Secret thủ công
                  </label>
                </div>

                {ytImportMode === "file" ? (
                  <div style={{ display: "flex", flexDirection: "column", gap: 8, marginTop: 4 }}>
                    <input
                      ref={ytFileInputRef}
                      type="file"
                      accept=".json,application/json"
                      onChange={handleImportJsonFile}
                      style={{ display: "none" }}
                    />
                    <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                      <Button
                        size="sm"
                        variant="primary"
                        onClick={() => ytFileInputRef.current?.click()}
                        busy={ytImportBusy}
                        busyText="Đang nạp tệp..."
                        leftIcon={<Icon name="upload" size={13} />}
                      >
                        Chọn tệp client_secret_*.json
                      </Button>
                      <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
                        Tệp JSON tải về từ mục Credentials của Google Cloud Console (loại Desktop app).
                      </span>
                    </div>
                  </div>
                ) : (
                  <div style={{ display: "flex", flexDirection: "column", gap: 10, marginTop: 4 }}>
                    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                      <label style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600 }}>Client ID:</label>
                      <Input
                        value={ytClientIdInput}
                        onChange={(e: any) => setYtClientIdInput(e.target.value)}
                        placeholder="...apps.googleusercontent.com"
                      />
                    </div>
                    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                      <label style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600 }}>Client Secret:</label>
                      <Input
                        type="password"
                        value={ytClientSecretInput}
                        onChange={(e: any) => setYtClientSecretInput(e.target.value)}
                        placeholder="GOCSPX-..."
                      />
                    </div>
                    <div style={{ display: "flex", justifyContent: "flex-end" }}>
                      <Button
                        size="sm"
                        variant="primary"
                        onClick={handleImportManual}
                        busy={ytImportBusy}
                        busyText="Đang lưu..."
                      >
                        Lưu OAuth Client
                      </Button>
                    </div>
                  </div>
                )}

                {ytImportError && (
                  <Callout variant="danger" style={{ marginTop: 6 }}>
                    {ytImportError}
                  </Callout>
                )}

                {ytImportSuccess && (
                  <Callout variant="success" style={{ marginTop: 6 }}>
                    {ytImportSuccess}
                  </Callout>
                )}
              </div>

              {/* SECTION 2: Danh sách kênh & Kết nối */}
              <div
                style={{
                  padding: "var(--cs-space-4)",
                  backgroundColor: "var(--cs-bg-panel)",
                  border: "1px solid var(--cs-border-subtle)",
                  borderRadius: "var(--cs-radius-control)",
                  display: "flex",
                  flexDirection: "column",
                  gap: "var(--cs-space-3)",
                }}
              >
                <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <span style={{ fontWeight: 700, fontSize: "var(--cs-text-sm)" }}>
                      2. Kênh YouTube đã kết nối ({ytChannels.length})
                    </span>
                  </div>

                  {!ytConnecting && (
                    <Button
                      size="sm"
                      variant="primary"
                      onClick={handleConnectYouTube}
                      disabled={!ytClientStatus?.configured}
                      leftIcon={<Icon name="link" size={13} />}
                    >
                      Kết nối YouTube
                    </Button>
                  )}
                </div>

                {!ytClientStatus?.configured && (
                  <Callout variant="neutral">
                    Vui lòng nạp Google OAuth Client ở bước 1 để kích hoạt chức năng kết nối YouTube.
                  </Callout>
                )}

                {/* Visible 5-minute countdown during connect */}
                {ytConnecting && (
                  <div
                    style={{
                      padding: "var(--cs-space-3) var(--cs-space-4)",
                      backgroundColor: "var(--cs-bg-raised)",
                      border: "1px solid var(--cs-border-subtle)",
                      borderRadius: "var(--cs-radius-control)",
                      display: "flex",
                      flexDirection: "column",
                      gap: 8,
                    }}
                  >
                    <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                        <Spinner size={16} />
                        <span style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)" }}>
                          Đang chờ hoàn tất đăng nhập trong trình duyệt...
                        </span>
                      </div>
                      <span
                        style={{
                          fontFamily: "var(--cs-font-mono)",
                          fontSize: "var(--cs-text-sm)",
                          fontWeight: 700,
                          color: "var(--cs-accent)",
                        }}
                      >
                        {Math.floor(ytCountdown / 60)}:{(ytCountdown % 60).toString().padStart(2, "0")}
                      </span>
                    </div>
                    <Callout variant="warning">
                      Đừng tắt Studio khi đang đăng nhập. Trình duyệt mặc định đang mở trang Google để bạn cấp quyền truy cập.
                    </Callout>
                  </div>
                )}

                {/* Structured error callout */}
                {ytConnectError && (
                  <Callout variant="danger" title={ytConnectError.messageVi}>
                    <p style={{ margin: "0 0 8px 0" }}>{ytConnectError.fixVi}</p>
                    <div style={{ display: "flex", gap: 8, marginTop: 8 }}>
                      <Button size="sm" variant="secondary" onClick={handleConnectYouTube}>
                        Thử lại
                      </Button>
                      <Button size="sm" variant="secondary" onClick={() => setGuideModalOpen(true)}>
                        Mở hướng dẫn
                      </Button>
                    </div>
                  </Callout>
                )}

                {ytConnectSuccess && (
                  <Callout variant="success">
                    {ytConnectSuccess}
                  </Callout>
                )}

                {/* Channels List */}
                {ytChannels.length === 0 ? (
                  <EmptyState
                    icon={<Icon name="video" size={28} />}
                    title="Chưa có kênh YouTube nào được kết nối"
                    description="Sau khi nạp OAuth client, bấm 'Kết nối YouTube' để liên kết kênh của bạn."
                  />
                ) : (
                  <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-2)" }}>
                    {ytChannels.map((chan) => (
                      <div
                        key={chan.id}
                        style={{
                          padding: "var(--cs-space-3) var(--cs-space-4)",
                          backgroundColor: "var(--cs-bg-raised)",
                          border: "1px solid var(--cs-border-subtle)",
                          borderRadius: "var(--cs-radius-control)",
                          display: "flex",
                          alignItems: "center",
                          justifyContent: "space-between",
                        }}
                      >
                        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                          {chan.avatarUrl ? (
                            <img
                              src={chan.avatarUrl}
                              alt={chan.title}
                              style={{ width: 40, height: 40, borderRadius: "50%", objectFit: "cover" }}
                            />
                          ) : (
                            <div
                              style={{
                                width: 40,
                                height: 40,
                                borderRadius: "50%",
                                backgroundColor: "var(--cs-bg-panel)",
                                border: "1px solid var(--cs-border-subtle)",
                                display: "flex",
                                alignItems: "center",
                                justifyContent: "center",
                              }}
                            >
                              <Icon name="video" size={18} color="var(--cs-fg-muted)" />
                            </div>
                          )}
                          <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                              <span style={{ fontWeight: 700, fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-primary)" }}>
                                {chan.title}
                              </span>
                              {chan.handle && (
                                <Badge variant="neutral">
                                  @{chan.handle.replace(/^@/, "")}
                                </Badge>
                              )}
                            </div>
                            <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)", fontFamily: "var(--cs-font-mono)" }}>
                              ID: {chan.id}
                            </span>
                          </div>
                        </div>

                        <Button
                          size="sm"
                          variant="secondary"
                          onClick={() => handleDisconnectChannel(chan.id)}
                          busy={ytDisconnectingId === chan.id}
                          busyText="Đang ngắt..."
                          leftIcon={<Icon name="trash" size={13} />}
                        >
                          Ngắt kết nối
                        </Button>
                      </div>
                    ))}
                  </div>
                )}

                {/* Revoke link info */}
                <div style={{ marginTop: 4 }}>
                  <Callout variant="info">
                    Để thu hồi hoàn toàn quyền truy cập hoặc xóa ứng dụng khỏi Google Account, hãy mở liên kết:{" "}
                    <a
                      href="https://myaccount.google.com/permissions"
                      target="_blank"
                      rel="noreferrer"
                      style={{ color: "var(--cs-accent)", fontWeight: 600, textDecoration: "underline" }}
                    >
                      myaccount.google.com/permissions
                    </a>
                  </Callout>
                </div>
              </div>
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

      <YouTubeGuideModal open={guideModalOpen} onClose={() => setGuideModalOpen(false)} />
    </div>
  );
};
