import React from "react";
import { Icon } from "../components/Icon";
import { Tabs, Badge, EmptyState, ErrorState, Button } from "@studio/design-system";
import { PaneBoundary } from "../components/PaneBoundary";
import { studioApi } from "../api/client";
import {
  type BrandSpec,
  DEFAULT_BRAND_SPEC,
  parseBrandYaml,
  stringifyBrandYaml,
  isValidHexColor,
} from "../utils/brandYaml";
import type { ScopeItem, ScopeKind } from "../api/types";
import { ScopeList, LAYER_LABELS } from "./brand/ScopeList";
import { BrandEditor } from "./brand/BrandEditor";
import { ChannelDetail } from "./brand/ChannelDetail";
import { PlaybookDetail } from "./brand/PlaybookDetail";

export const BrandView: React.FC = () => {
  return (
    <PaneBoundary name="Thương hiệu & Quy chuẩn">
      <BrandViewInner />
    </PaneBoundary>
  );
};

const BrandViewInner: React.FC = () => {
  const [activeTab, setActiveTab] = React.useState<"brands" | "channels" | "playbooks">("brands");
  const [items, setItems] = React.useState<ScopeItem[]>([]);
  const [selectedId, setSelectedId] = React.useState<string | null>(null);
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);

  // Active Brand editing state
  const [brandSpec, setBrandSpec] = React.useState<BrandSpec | null>(null);
  const [brandLoading, setBrandLoading] = React.useState(false);
  const [brandSaving, setBrandSaving] = React.useState(false);
  const [saveSuccess, setSaveSuccess] = React.useState(false);
  const [brandError, setBrandError] = React.useState<string | null>(null);
  const [fieldErrors, setFieldErrors] = React.useState<Record<string, string>>({});
  const [logoPreviewUrl, setLogoPreviewUrl] = React.useState<string | null>(null);

  // Raw file content for channel / playbook
  const [rawText, setRawText] = React.useState<string | null>(null);

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

  // Load selected brand YAML or raw content
  React.useEffect(() => {
    if (!selectedItem) {
      setBrandSpec(null);
      setRawText(null);
      return;
    }

    let isMounted = true;
    setBrandLoading(true);
    setBrandError(null);
    setSaveSuccess(false);
    setFieldErrors({});

    const loadContent = async () => {
      try {
        const itemPath = selectedItem.path || "";
        const targetYamlPath = itemPath.endsWith(".yaml") || itemPath.endsWith(".md")
          ? itemPath
          : activeTab === "brands"
          ? `${itemPath}/brand.yaml`
          : itemPath;

        const res = await studioApi.readFile({ path: targetYamlPath });
        if (!isMounted) return;

        if (activeTab === "brands") {
          const text = res.text || "";
          const parsed = text ? parseBrandYaml(text) : DEFAULT_BRAND_SPEC;
          setBrandSpec(parsed);
          setRawText(text);

          if (parsed.logos?.full) {
            const logoPath = itemPath ? `${itemPath}/${parsed.logos.full}` : parsed.logos.full;
            studioApi
              .readFile({ path: logoPath })
              .then((logoRes) => {
                if (isMounted) {
                  setLogoPreviewUrl(logoRes.assetUrl || (logoRes.text ? `data:image/svg+xml;utf8,${encodeURIComponent(logoRes.text)}` : null));
                }
              })
              .catch(() => {
                if (isMounted) setLogoPreviewUrl(null);
              });
          } else {
            setLogoPreviewUrl(null);
          }
        } else {
          setRawText(res.text || "");
          setBrandSpec(null);
        }
      } catch (err: any) {
        if (isMounted) {
          if (activeTab === "brands") {
            setBrandSpec(DEFAULT_BRAND_SPEC);
          } else {
            setBrandError(err?.message || "Không thể đọc nội dung cấu hình.");
          }
        }
      } finally {
        if (isMounted) setBrandLoading(false);
      }
    };

    loadContent();
    return () => {
      isMounted = false;
    };
  }, [selectedItem, activeTab]);

  // Validation function
  const validateForm = (): boolean => {
    if (!brandSpec) return false;
    const errors: Record<string, string> = {};

    if (!brandSpec.name.trim()) {
      errors.name = "Tên thương hiệu không được để trống";
    }

    if (!isValidHexColor(brandSpec.colors.primary)) {
      errors.primary = "Màu primary không đúng định dạng hex (vd: #1E88E5)";
    }
    if (!isValidHexColor(brandSpec.colors.accent)) {
      errors.accent = "Màu accent không đúng định dạng hex (vd: #FFB300)";
    }
    if (!isValidHexColor(brandSpec.colors.bg)) {
      errors.bg = "Màu nền bg không đúng định dạng hex (vd: #0B0F14)";
    }
    if (!isValidHexColor(brandSpec.colors.text)) {
      errors.text = "Màu chữ text không đúng định dạng hex (vd: #FFFFFF)";
    }

    if (!brandSpec.fonts.heading.trim()) {
      errors.fontHeading = "Font tiêu đề không được để trống";
    }
    if (!brandSpec.fonts.body.trim()) {
      errors.fontBody = "Font nội dung không được để trống";
    }

    setFieldErrors(errors);
    return Object.keys(errors).length === 0;
  };

  // Save edited Brand
  const handleSaveBrand = async () => {
    if (!brandSpec || !selectedItem) return;
    if (!validateForm()) return;

    setBrandSaving(true);
    setBrandError(null);
    setSaveSuccess(false);

    try {
      const inherited = selectedItem.layer !== "workspace";
      const itemPath = !inherited && selectedItem.path ? selectedItem.path : `brands/${selectedItem.id}`;
      const targetYamlPath = itemPath.endsWith(".yaml") ? itemPath : `${itemPath}/brand.yaml`;
      const yamlContent = stringifyBrandYaml(brandSpec);

      await studioApi.writeFile({
        path: targetYamlPath,
        text: yamlContent,
      });

      if (inherited) {
        const list = await studioApi.getScopeItems("brands");
        setItems(list);
      }
      setSaveSuccess(true);
      setTimeout(() => setSaveSuccess(false), 3000);
    } catch (err: any) {
      setBrandError(err?.message || "Lỗi khi lưu tệp brand.yaml.");
    } finally {
      setBrandSaving(false);
    }
  };

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
            Kế thừa theo chuỗi App → Tổ chức → Workspace. Quản lý nhận diện thương hiệu chuẩn brand@1.
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
              title={`Chưa có ${tabItems.find((t) => t.id === activeTab)?.label || activeTab} nào`}
              description={`Chưa phát hiện tệp cấu hình trong chuỗi kế thừa App / Tổ chức / Workspace.`}
            />
          </div>
        ) : (
          <>
            {/* Left Items List */}
            <ScopeList
              items={items}
              selectedId={selectedId}
              onSelectId={setSelectedId}
            />

            {/* Right Interactive Editor & Detail Pane */}
            <div
              style={{
                flex: 1,
                padding: "var(--cs-space-5)",
                overflowY: "auto",
                backgroundColor: "var(--cs-bg-app)",
              }}
            >
              {selectedItem ? (
                <div
                  style={{
                    backgroundColor: "var(--cs-bg-panel)",
                    border: "1px solid var(--cs-border-subtle)",
                    borderRadius: "var(--cs-radius-control)",
                    padding: "var(--cs-space-5)",
                    display: "flex",
                    flexDirection: "column",
                    gap: "var(--cs-space-5)",
                    maxWidth: 840,
                    margin: "0 auto",
                  }}
                >
                  {/* Item Header */}
                  <div
                    style={{
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "space-between",
                      borderBottom: "1px solid var(--cs-border-subtle)",
                      paddingBottom: "var(--cs-space-3)",
                    }}
                  >
                    <div>
                      <h2 style={{ margin: 0, fontSize: "var(--cs-text-lg)", fontWeight: 700 }}>
                        {selectedItem.name}
                      </h2>
                      <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)", fontFamily: "var(--cs-font-mono)" }}>
                        ID: {selectedItem.id} · Cấp độ: {LAYER_LABELS[selectedItem.layer] || selectedItem.layer}
                      </span>
                    </div>

                    {activeTab === "brands" && (
                      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                        {saveSuccess && (
                          <Badge variant="success">✓ Đã lưu thành công</Badge>
                        )}
                        <Button
                          variant="primary"
                          busy={brandSaving}
                          busyText="Đang lưu..."
                          onClick={handleSaveBrand}
                          leftIcon={<Icon name="check" size={14} />}
                          title={
                            selectedItem.layer !== "workspace"
                              ? "Brand kế thừa chỉ đọc — lưu sẽ tạo bản ghi đè trong workspace này"
                              : undefined
                          }
                        >
                          {selectedItem.layer !== "workspace" ? "Lưu bản ghi đè" : "Lưu thương hiệu"}
                        </Button>
                      </div>
                    )}
                  </div>

                  {brandError && (
                    <div
                      style={{
                        padding: "8px 12px",
                        backgroundColor: "rgba(239, 68, 68, 0.1)",
                        border: "1px solid var(--cs-danger)",
                        borderRadius: "var(--cs-radius-control)",
                        color: "var(--cs-danger)",
                        fontSize: "var(--cs-text-xs)",
                      }}
                    >
                      {brandError}
                    </div>
                  )}

                  {brandLoading ? (
                    <div style={{ padding: "var(--cs-space-6)", textAlign: "center", color: "var(--cs-fg-muted)" }}>
                      Đang tải dữ liệu chi tiết...
                    </div>
                  ) : activeTab === "brands" && brandSpec ? (
                    <BrandEditor
                      brandSpec={brandSpec}
                      setBrandSpec={setBrandSpec}
                      fieldErrors={fieldErrors}
                      logoPreviewUrl={logoPreviewUrl}
                    />
                  ) : activeTab === "channels" ? (
                    <ChannelDetail
                      selectedItem={selectedItem}
                      rawText={rawText}
                    />
                  ) : (
                    <PlaybookDetail rawText={rawText} />
                  )}
                </div>
              ) : (
                <div style={{ color: "var(--cs-fg-muted)", fontSize: "var(--cs-text-sm)", textAlign: "center", marginTop: 40 }}>
                  Chọn một mục từ danh sách bên trái để xem và chỉnh sửa.
                </div>
              )}
            </div>
          </>
        )}
      </div>
    </div>
  );
};
