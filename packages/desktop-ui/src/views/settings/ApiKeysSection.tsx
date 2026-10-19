import React from "react";
import { Icon } from "../../components/Icon";
import { Badge, Button, Callout, Input } from "@studio/design-system";
import { studioApi } from "../../api/client";
import type { SecretStatusItem, ProviderPricing } from "../../api/types";

export const ApiKeysSection: React.FC = () => {
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

  const pricingProviders = [
    { id: "imagen", name: "Google Imagen (Tạo ảnh)", type: "image", defaultCost: "0.04" },
    { id: "openai", name: "OpenAI DALL-E (Tạo ảnh)", type: "image", defaultCost: "0.04" },
    { id: "veo", name: "Google Veo (Tạo video)", type: "video", defaultCost: "0.20" },
    { id: "openai_videos", name: "OpenAI Sora / Videos (Tạo video)", type: "video", defaultCost: "0.20" },
  ];

  const loadData = React.useCallback(async () => {
    try {
      const [secretList, endpointMap, pricingMap] = await Promise.all([
        studioApi.getSecretStatus().catch(() => []),
        studioApi.getProviderEndpoints().catch(() => ({})),
        studioApi.getProviderPricing().catch(() => ({})),
      ]);
      setSecrets(secretList);
      setEndpoints(endpointMap);
      setEndpointInputs(endpointMap);
      setPricing(pricingMap);
    } catch {
      // ignore
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

  return (
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
                      onClick={() => handleSaveEndpoint(p.id)}
                    >
                      Lưu
                    </Button>
                    {endpoints[p.id] && (
                      <Button
                        variant="quiet"
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
  );
};
