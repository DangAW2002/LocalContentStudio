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
import type { ChannelFormat, ScopeItem, ScopeKind } from "../api/types";

const LAYER_LABELS: Record<string, string> = {
  workspace: "Thư mục Workspace",
  org: "Tổ chức",
  app: "Mặc định ứng dụng",
};

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

  // New tag inputs
  const [newPreferWord, setNewPreferWord] = React.useState("");
  const [newAvoidWord, setNewAvoidWord] = React.useState("");
  const [newRule, setNewRule] = React.useState("");
  const [newExtraColor, setNewExtraColor] = React.useState("#6366f1");

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

          // Attempt loading logo preview
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
          // If file not found, use default brand spec for brand view
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
      // App/Org layers are read-only: saving one creates a workspace-layer override with the same
      // id, which the scope chain then resolves ahead of the inherited copy.
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
            <div
              style={{
                width: 300,
                borderRight: "1px solid var(--cs-border-subtle)",
                backgroundColor: "var(--cs-bg-panel)",
                overflowY: "auto",
                flexShrink: 0,
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
                      transition: "background-color var(--cs-motion-duration) ease",
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
                        {LAYER_LABELS[item.layer] || item.layer}
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
                    /* Full Brand@1 Editor */
                    <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-5)" }}>
                      {/* 1. Basic Info */}
                      <div style={cardSectionStyle}>
                        <div style={sectionTitleStyle}>1. THÔNG TIN THƯƠNG HIỆU</div>
                        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
                          <label style={fieldLabelStyle}>
                            <div style={{ display: "flex", justifyContent: "space-between" }}>
                              <span>Tên thương hiệu</span>
                              {fieldErrors.name && <span style={errorTextStyle}>{fieldErrors.name}</span>}
                            </div>
                            <input
                              type="text"
                              value={brandSpec.name}
                              onChange={(e) => setBrandSpec({ ...brandSpec, name: e.target.value })}
                              style={{ ...inputStyle, borderColor: fieldErrors.name ? "var(--cs-danger)" : "var(--cs-border-subtle)" }}
                            />
                          </label>

                          <label style={fieldLabelStyle}>
                            <span>Khẩu hiệu (Slogan)</span>
                            <input
                              type="text"
                              value={brandSpec.slogan}
                              onChange={(e) => setBrandSpec({ ...brandSpec, slogan: e.target.value })}
                              style={inputStyle}
                            />
                          </label>
                        </div>
                      </div>

                      {/* 2. Color Palette & Swatches */}
                      <div style={cardSectionStyle}>
                        <div style={sectionTitleStyle}>2. BẢNG MÀU NHẬN DIỆN (PALETTE)</div>
                        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: 12 }}>
                          {PALETTE_FIELDS.map(({ key, label, fallback }) => {
                            const value = brandSpec.colors[key];
                            const setValue = (v: string) =>
                              setBrandSpec({ ...brandSpec, colors: { ...brandSpec.colors, [key]: v } });
                            return (
                              <div key={key} style={colorBoxStyle}>
                                <span style={{ fontWeight: 600, fontSize: "var(--cs-text-xs)" }}>{label}</span>
                                <div style={{ display: "flex", alignItems: "center", gap: 8, minWidth: 0 }}>
                                  {/* The native picker doubles as the swatch. */}
                                  <input
                                    type="color"
                                    aria-label={`Chọn ${label}`}
                                    value={isValidHexColor(value) ? value : fallback}
                                    onChange={(e) => setValue(e.target.value.toUpperCase())}
                                    style={swatchInputStyle}
                                  />
                                  <input
                                    type="text"
                                    aria-label={`Mã màu ${label}`}
                                    value={value}
                                    spellCheck={false}
                                    onChange={(e) => setValue(e.target.value)}
                                    style={{
                                      ...inputStyle,
                                      flex: 1,
                                      minWidth: 0,
                                      width: "100%",
                                      boxSizing: "border-box",
                                      fontFamily: "var(--cs-font-mono)",
                                      fontSize: 11,
                                      borderColor: fieldErrors[key] ? "var(--cs-danger)" : undefined,
                                    }}
                                  />
                                </div>
                                {fieldErrors[key] && <span style={errorTextStyle}>{fieldErrors[key]}</span>}
                              </div>
                            );
                          })}
                        </div>

                        {/* Extra Colors */}
                        <div style={{ display: "flex", flexDirection: "column", gap: 6, marginTop: 8 }}>
                          <span style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600, color: "var(--cs-fg-secondary)" }}>
                            Màu mở rộng (Extra Colors)
                          </span>
                          <div style={{ display: "flex", flexWrap: "wrap", gap: 8, alignItems: "center" }}>
                            {brandSpec.colors.extra.map((col, idx) => (
                              <div
                                key={idx}
                                style={{
                                  display: "inline-flex",
                                  alignItems: "center",
                                  gap: 6,
                                  padding: "3px 8px",
                                  backgroundColor: "var(--cs-bg-raised)",
                                  border: "1px solid var(--cs-border-subtle)",
                                  borderRadius: "var(--cs-radius-control)",
                                  fontSize: 11,
                                }}
                              >
                                <span style={{ width: 14, height: 14, borderRadius: "50%", backgroundColor: col }} />
                                <span style={{ fontFamily: "var(--cs-font-mono)" }}>{col}</span>
                                <button
                                  type="button"
                                  onClick={() => {
                                    const next = [...brandSpec.colors.extra];
                                    next.splice(idx, 1);
                                    setBrandSpec({ ...brandSpec, colors: { ...brandSpec.colors, extra: next } });
                                  }}
                                  style={{ background: "none", border: "none", color: "var(--cs-fg-muted)", cursor: "pointer", padding: 0 }}
                                >
                                  ×
                                </button>
                              </div>
                            ))}

                            <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                              <input
                                type="color"
                                value={newExtraColor}
                                onChange={(e) => setNewExtraColor(e.target.value.toUpperCase())}
                                style={{ width: 24, height: 24, border: "none", cursor: "pointer" }}
                              />
                              <Button
                                size="sm"
                                variant="quiet"
                                onClick={() => {
                                  if (!brandSpec.colors.extra.includes(newExtraColor)) {
                                    setBrandSpec({
                                      ...brandSpec,
                                      colors: { ...brandSpec.colors, extra: [...brandSpec.colors.extra, newExtraColor] },
                                    });
                                  }
                                }}
                              >
                                + Thêm màu
                              </Button>
                            </div>
                          </div>
                        </div>
                      </div>

                      {/* 3. Typography & Live Preview */}
                      <div style={cardSectionStyle}>
                        <div style={sectionTitleStyle}>3. PHÔNG CHỮ & XEM TRƯỚC (TYPOGRAPHY)</div>
                        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
                          <label style={fieldLabelStyle}>
                            <div style={{ display: "flex", justifyContent: "space-between" }}>
                              <span>Font Tiêu đề (Heading)</span>
                              {fieldErrors.fontHeading && <span style={errorTextStyle}>{fieldErrors.fontHeading}</span>}
                            </div>
                            <input
                              type="text"
                              value={brandSpec.fonts.heading}
                              onChange={(e) => setBrandSpec({ ...brandSpec, fonts: { ...brandSpec.fonts, heading: e.target.value } })}
                              style={inputStyle}
                            />
                          </label>

                          <label style={fieldLabelStyle}>
                            <div style={{ display: "flex", justifyContent: "space-between" }}>
                              <span>Font Nội dung (Body)</span>
                              {fieldErrors.fontBody && <span style={errorTextStyle}>{fieldErrors.fontBody}</span>}
                            </div>
                            <input
                              type="text"
                              value={brandSpec.fonts.body}
                              onChange={(e) => setBrandSpec({ ...brandSpec, fonts: { ...brandSpec.fonts, body: e.target.value } })}
                              style={inputStyle}
                            />
                          </label>
                        </div>

                        {/* Live Font & Palette Preview Box */}
                        <div
                          style={{
                            marginTop: 10,
                            padding: "var(--cs-space-4)",
                            backgroundColor: brandSpec.colors.bg,
                            color: brandSpec.colors.text,
                            borderRadius: "var(--cs-radius-control)",
                            border: "1px solid rgba(255,255,255,0.1)",
                            display: "flex",
                            flexDirection: "column",
                            gap: 8,
                          }}
                        >
                          <div style={{ fontSize: 11, color: "var(--cs-fg-muted)", textTransform: "uppercase", letterSpacing: 1 }}>
                            Xem trước hiển thị thực tế
                          </div>
                          <div
                            style={{
                              fontFamily: `${brandSpec.fonts.heading}, system-ui, sans-serif`,
                              fontSize: "var(--cs-text-xl)",
                              fontWeight: 800,
                              color: brandSpec.colors.primary,
                            }}
                          >
                            {brandSpec.name} — Tiêu đề thương hiệu
                          </div>
                          <div
                            style={{
                              fontFamily: `${brandSpec.fonts.body}, system-ui, sans-serif`,
                              fontSize: "var(--cs-text-sm)",
                              lineHeight: 1.5,
                              color: brandSpec.colors.text,
                            }}
                          >
                            {brandSpec.slogan ? `${brandSpec.slogan.replace(/[.!?…]*$/, "")}. ` : ""}Nội dung mẫu để kiểm tra màu nền, màu chữ và phông chữ.
                          </div>
                        </div>
                      </div>

                      {/* 4. Logos & Visual Preview */}
                      <div style={cardSectionStyle}>
                        <div style={sectionTitleStyle}>4. LOGO & QUY CHUẨN HIỂN THỊ</div>
                        <div style={{ display: "grid", gridTemplateColumns: "1fr 140px", gap: 16, alignItems: "start" }}>
                          <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
                            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
                              <label style={fieldLabelStyle}>
                                <span>Đường dẫn Logo đầy đủ (Full)</span>
                                <input
                                  type="text"
                                  value={brandSpec.logos.full}
                                  onChange={(e) => setBrandSpec({ ...brandSpec, logos: { ...brandSpec.logos, full: e.target.value } })}
                                  style={inputStyle}
                                />
                              </label>

                              <label style={fieldLabelStyle}>
                                <span>Biểu tượng / Mark logo</span>
                                <input
                                  type="text"
                                  value={brandSpec.logos.mark}
                                  onChange={(e) => setBrandSpec({ ...brandSpec, logos: { ...brandSpec.logos, mark: e.target.value } })}
                                  style={inputStyle}
                                />
                              </label>
                            </div>

                            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
                              <label style={fieldLabelStyle}>
                                <span>Kích thước tối thiểu (px)</span>
                                <input
                                  type="number"
                                  min="16"
                                  value={brandSpec.logos.min_size_px}
                                  onChange={(e) => setBrandSpec({ ...brandSpec, logos: { ...brandSpec.logos, min_size_px: parseInt(e.target.value, 10) || 32 } })}
                                  style={inputStyle}
                                />
                              </label>

                              <label style={fieldLabelStyle}>
                                <span>Tỉ lệ khoảng cách an toàn</span>
                                <input
                                  type="number"
                                  step="0.05"
                                  min="0.1"
                                  max="1"
                                  value={brandSpec.logos.clear_space_ratio}
                                  onChange={(e) => setBrandSpec({ ...brandSpec, logos: { ...brandSpec.logos, clear_space_ratio: parseFloat(e.target.value) || 0.25 } })}
                                  style={inputStyle}
                                />
                              </label>
                            </div>
                          </div>

                          {/* Logo Preview Box */}
                          <div
                            style={{
                              width: 140,
                              height: 120,
                              borderRadius: "var(--cs-radius-control)",
                              border: "1px dashed var(--cs-border-subtle)",
                              backgroundColor: "var(--cs-bg-raised)",
                              display: "flex",
                              flexDirection: "column",
                              alignItems: "center",
                              justifyContent: "center",
                              padding: 8,
                              boxSizing: "border-box",
                              textAlign: "center",
                            }}
                          >
                            {logoPreviewUrl ? (
                              <img
                                src={logoPreviewUrl}
                                alt="Logo preview"
                                style={{ maxWidth: "100%", maxHeight: 80, objectFit: "contain" }}
                              />
                            ) : (
                              <div style={{ color: "var(--cs-fg-muted)", fontSize: 11, display: "flex", flexDirection: "column", alignItems: "center", gap: 4 }}>
                                <Icon name="brand" size={24} color="var(--cs-fg-muted)" />
                                <span>{brandSpec.logos.full || "Chưa có logo"}</span>
                              </div>
                            )}
                          </div>
                        </div>
                      </div>

                      {/* 5. Voice & Words */}
                      <div style={cardSectionStyle}>
                        <div style={sectionTitleStyle}>5. GIỌNG VĂN & TỪ NGỮ (VOICE WORDS)</div>
                        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
                          {/* Prefer words */}
                          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                            <span style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600, color: "var(--cs-success)" }}>
                              ✓ Từ ngữ ưu tiên sử dụng ({brandSpec.voice_words.prefer.length})
                            </span>
                            <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
                              {brandSpec.voice_words.prefer.map((w, idx) => (
                                <span
                                  key={idx}
                                  style={{
                                    padding: "2px 8px",
                                    backgroundColor: "rgba(16, 185, 129, 0.15)",
                                    border: "1px solid rgba(16, 185, 129, 0.3)",
                                    borderRadius: "var(--cs-radius-control)",
                                    fontSize: 11,
                                    color: "var(--cs-fg-primary)",
                                    display: "inline-flex",
                                    alignItems: "center",
                                    gap: 4,
                                  }}
                                >
                                  {w}
                                  <button
                                    type="button"
                                    onClick={() => {
                                      const next = [...brandSpec.voice_words.prefer];
                                      next.splice(idx, 1);
                                      setBrandSpec({ ...brandSpec, voice_words: { ...brandSpec.voice_words, prefer: next } });
                                    }}
                                    style={{ background: "none", border: "none", cursor: "pointer", color: "var(--cs-fg-muted)", padding: 0 }}
                                  >
                                    ×
                                  </button>
                                </span>
                              ))}
                            </div>
                            <div style={{ display: "flex", gap: 4, marginTop: 4 }}>
                              <input
                                type="text"
                                placeholder="Thêm từ ưu tiên..."
                                value={newPreferWord}
                                onChange={(e) => setNewPreferWord(e.target.value)}
                                onKeyDown={(e) => {
                                  if (e.key === "Enter" && newPreferWord.trim()) {
                                    e.preventDefault();
                                    setBrandSpec({
                                      ...brandSpec,
                                      voice_words: { ...brandSpec.voice_words, prefer: [...brandSpec.voice_words.prefer, newPreferWord.trim()] },
                                    });
                                    setNewPreferWord("");
                                  }
                                }}
                                style={{ ...inputStyle, flex: 1 }}
                              />
                              <Button
                                size="sm"
                                variant="secondary"
                                onClick={() => {
                                  if (newPreferWord.trim()) {
                                    setBrandSpec({
                                      ...brandSpec,
                                      voice_words: { ...brandSpec.voice_words, prefer: [...brandSpec.voice_words.prefer, newPreferWord.trim()] },
                                    });
                                    setNewPreferWord("");
                                  }
                                }}
                              >
                                Thêm
                              </Button>
                            </div>
                          </div>

                          {/* Avoid words */}
                          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                            <span style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600, color: "var(--cs-danger)" }}>
                              ⚠ Từ ngữ cần tránh ({brandSpec.voice_words.avoid.length})
                            </span>
                            <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
                              {brandSpec.voice_words.avoid.map((w, idx) => (
                                <span
                                  key={idx}
                                  style={{
                                    padding: "2px 8px",
                                    backgroundColor: "rgba(239, 68, 68, 0.15)",
                                    border: "1px solid rgba(239, 68, 68, 0.3)",
                                    borderRadius: "var(--cs-radius-control)",
                                    fontSize: 11,
                                    color: "var(--cs-fg-primary)",
                                    display: "inline-flex",
                                    alignItems: "center",
                                    gap: 4,
                                  }}
                                >
                                  {w}
                                  <button
                                    type="button"
                                    onClick={() => {
                                      const next = [...brandSpec.voice_words.avoid];
                                      next.splice(idx, 1);
                                      setBrandSpec({ ...brandSpec, voice_words: { ...brandSpec.voice_words, avoid: next } });
                                    }}
                                    style={{ background: "none", border: "none", cursor: "pointer", color: "var(--cs-fg-muted)", padding: 0 }}
                                  >
                                    ×
                                  </button>
                                </span>
                              ))}
                            </div>
                            <div style={{ display: "flex", gap: 4, marginTop: 4 }}>
                              <input
                                type="text"
                                placeholder="Thêm từ cần tránh..."
                                value={newAvoidWord}
                                onChange={(e) => setNewAvoidWord(e.target.value)}
                                onKeyDown={(e) => {
                                  if (e.key === "Enter" && newAvoidWord.trim()) {
                                    e.preventDefault();
                                    setBrandSpec({
                                      ...brandSpec,
                                      voice_words: { ...brandSpec.voice_words, avoid: [...brandSpec.voice_words.avoid, newAvoidWord.trim()] },
                                    });
                                    setNewAvoidWord("");
                                  }
                                }}
                                style={{ ...inputStyle, flex: 1 }}
                              />
                              <Button
                                size="sm"
                                variant="secondary"
                                onClick={() => {
                                  if (newAvoidWord.trim()) {
                                    setBrandSpec({
                                      ...brandSpec,
                                      voice_words: { ...brandSpec.voice_words, avoid: [...brandSpec.voice_words.avoid, newAvoidWord.trim()] },
                                    });
                                    setNewAvoidWord("");
                                  }
                                }}
                              >
                                Thêm
                              </Button>
                            </div>
                          </div>
                        </div>
                      </div>

                      {/* 6. Brand Rules */}
                      <div style={cardSectionStyle}>
                        <div style={sectionTitleStyle}>6. NGUYÊN TẮC THIẾT KẾ & QUY CHUẨN (RULES)</div>
                        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                          {brandSpec.rules.map((rule, idx) => (
                            <div
                              key={idx}
                              style={{
                                display: "flex",
                                alignItems: "center",
                                justifyContent: "space-between",
                                padding: "6px 10px",
                                backgroundColor: "var(--cs-bg-raised)",
                                border: "1px solid var(--cs-border-subtle)",
                                borderRadius: "var(--cs-radius-control)",
                                fontSize: "var(--cs-text-xs)",
                              }}
                            >
                              <span>{rule}</span>
                              <button
                                type="button"
                                onClick={() => {
                                  const next = [...brandSpec.rules];
                                  next.splice(idx, 1);
                                  setBrandSpec({ ...brandSpec, rules: next });
                                }}
                                style={{ background: "none", border: "none", color: "var(--cs-fg-muted)", cursor: "pointer" }}
                              >
                                ×
                              </button>
                            </div>
                          ))}

                          <div style={{ display: "flex", gap: 4, marginTop: 4 }}>
                            <input
                              type="text"
                              placeholder="Thêm nguyên tắc thiết kế mới..."
                              value={newRule}
                              onChange={(e) => setNewRule(e.target.value)}
                              onKeyDown={(e) => {
                                if (e.key === "Enter" && newRule.trim()) {
                                  e.preventDefault();
                                  setBrandSpec({ ...brandSpec, rules: [...brandSpec.rules, newRule.trim()] });
                                  setNewRule("");
                                }
                              }}
                              style={{ ...inputStyle, flex: 1 }}
                            />
                            <Button
                              size="sm"
                              variant="secondary"
                              onClick={() => {
                                if (newRule.trim()) {
                                  setBrandSpec({ ...brandSpec, rules: [...brandSpec.rules, newRule.trim()] });
                                  setNewRule("");
                                }
                              }}
                            >
                              Thêm nguyên tắc
                            </Button>
                          </div>
                        </div>
                      </div>
                    </div>
                  ) : activeTab === "channels" ? (
                    /* Channels detail view with Safe Zone visualization */
                    <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-4)" }}>
                      <div style={cardSectionStyle}>
                        <div style={sectionTitleStyle}>QUY CHUẨN KHUNG HÌNH & KHU VỰC AN TOÀN (SAFE ZONE)</div>
                        <p style={{ margin: "0 0 12px 0", fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
                          Khu vực an toàn đảm bảo phụ đề, logo và nội dung cốt lõi không bị che bởi giao diện người dùng (nút thả tim, thanh điều hướng, mô tả).
                        </p>

                        {selectedItem.formats && selectedItem.formats.length > 0 ? (
                          <div style={{ display: "flex", gap: 24, flexWrap: "wrap", justifyContent: "center", alignItems: "flex-end" }}>
                            {selectedItem.formats.map((f) => (
                              <SafeZonePreview key={f.name} format={f} />
                            ))}
                          </div>
                        ) : (
                          <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
                            Kênh này chưa khai báo định dạng khung hình.
                          </span>
                        )}
                      </div>

                      {/* Raw Channel Definition Text */}
                      <div style={cardSectionStyle}>
                        <div style={sectionTitleStyle}>CẤU HÌNH KÊNH (YAML)</div>
                        <pre
                          style={{
                            margin: 0,
                            padding: "var(--cs-space-3)",
                            backgroundColor: "var(--cs-bg-raised)",
                            border: "1px solid var(--cs-border-subtle)",
                            borderRadius: "var(--cs-radius-control)",
                            fontFamily: "var(--cs-font-mono)",
                            fontSize: 11,
                            lineHeight: 1.5,
                            overflowX: "auto",
                          }}
                        >
                          {rawText || "Đang tải cấu hình..."}
                        </pre>
                      </div>
                    </div>
                  ) : (
                    /* Playbooks View */
                    <div style={cardSectionStyle}>
                      <div style={sectionTitleStyle}>NỘI DUNG QUY TRÌNH (PLAYBOOK MARKDOWN)</div>
                      <pre
                        style={{
                          margin: 0,
                          padding: "var(--cs-space-4)",
                          backgroundColor: "var(--cs-bg-raised)",
                          border: "1px solid var(--cs-border-subtle)",
                          borderRadius: "var(--cs-radius-control)",
                          fontFamily: "var(--cs-font-mono)",
                          fontSize: 12,
                          lineHeight: 1.6,
                          overflowX: "auto",
                          whiteSpace: "pre-wrap",
                        }}
                      >
                        {rawText || "Đang tải quy trình..."}
                      </pre>
                    </div>
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

/** One channel format drawn to scale, with its safe zone inset by the real margins. */
const SafeZonePreview: React.FC<{ format: ChannelFormat }> = ({ format }) => {
  const BOX = 180;
  const ratio = format.width / Math.max(1, format.height);
  const w = ratio >= 1 ? BOX : Math.round(BOX * ratio);
  const h = ratio >= 1 ? Math.round(BOX / ratio) : BOX;
  const z = format.safe_zone;
  const pct = (v: number) => `${Math.round(v * 100)}%`;
  const margins = `Lề an toàn — trên ${pct(z.top)}, dưới ${pct(z.bottom)}, trái ${pct(z.left)}, phải ${pct(z.right)}`;
  return (
    <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 6, maxWidth: BOX }}>
      <div
        title={margins}
        style={{
          width: w,
          height: h,
          backgroundColor: "#1e293b",
          border: "1px solid var(--cs-border-subtle)",
          borderRadius: "var(--cs-radius-control)",
          position: "relative",
          overflow: "hidden",
        }}
      >
        <div
          style={{
            position: "absolute",
            top: pct(z.top),
            bottom: pct(z.bottom),
            left: pct(z.left),
            right: pct(z.right),
            border: "1px dashed #60a5fa",
            backgroundColor: "rgba(59, 130, 246, 0.12)",
          }}
        />
      </div>
      <span style={{ fontSize: 11, fontWeight: 600, color: "var(--cs-fg-secondary)", textAlign: "center" }}>{format.name}</span>
      <span style={{ fontSize: 11, color: "var(--cs-fg-muted)", textAlign: "center" }}>
        {format.aspect} · {format.width}×{format.height}
      </span>
      <span style={{ fontSize: 10, color: "var(--cs-fg-muted)", textAlign: "center", lineHeight: 1.4 }}>
        Lề trên {pct(z.top)} · dưới {pct(z.bottom)}
        <br />
        trái {pct(z.left)} · phải {pct(z.right)}
      </span>
    </div>
  );
};

const cardSectionStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 8,
  padding: "var(--cs-space-4)",
  backgroundColor: "var(--cs-bg-panel)",
  border: "1px solid var(--cs-border-subtle)",
  borderRadius: "var(--cs-radius-control)",
};

const sectionTitleStyle: React.CSSProperties = {
  fontSize: "var(--cs-text-xs)",
  fontWeight: 700,
  color: "var(--cs-fg-secondary)",
  letterSpacing: 0.5,
};

const fieldLabelStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 4,
  fontSize: "var(--cs-text-xs)",
  color: "var(--cs-fg-secondary)",
};

const inputStyle: React.CSSProperties = {
  padding: "6px 10px",
  backgroundColor: "var(--cs-bg-app)",
  border: "1px solid var(--cs-border-subtle)",
  borderRadius: "var(--cs-radius-control)",
  color: "var(--cs-fg-primary)",
  fontSize: "var(--cs-text-xs)",
  outline: "none",
};

const PALETTE_FIELDS = [
  { key: "primary", label: "Màu chính (Primary)", fallback: "#1e88e5" },
  { key: "accent", label: "Màu nhấn (Accent)", fallback: "#ffb300" },
  { key: "bg", label: "Màu nền (Background)", fallback: "#0b0f14" },
  { key: "text", label: "Màu chữ (Text)", fallback: "#ffffff" },
] as const;

const swatchInputStyle: React.CSSProperties = {
  width: 32,
  height: 32,
  flexShrink: 0,
  padding: 2,
  border: "1px solid var(--cs-border-subtle)",
  borderRadius: "var(--cs-radius-control)",
  backgroundColor: "var(--cs-bg-app)",
  cursor: "pointer",
};

const colorBoxStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 6,
  minWidth: 0,
  padding: 8,
  backgroundColor: "var(--cs-bg-raised)",
  border: "1px solid var(--cs-border-subtle)",
  borderRadius: "var(--cs-radius-control)",
};

const errorTextStyle: React.CSSProperties = {
  color: "var(--cs-danger)",
  fontSize: 10,
  fontWeight: 500,
};
