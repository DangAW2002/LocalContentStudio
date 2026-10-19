import React from "react";
import { Icon } from "../../components/Icon";
import type { BrandSpec } from "../../utils/brandYaml";

export interface BrandLogosSectionProps {
  brandSpec: BrandSpec;
  setBrandSpec: (spec: BrandSpec) => void;
  logoPreviewUrl: string | null;
}

export const BrandLogosSection: React.FC<BrandLogosSectionProps> = ({
  brandSpec,
  setBrandSpec,
  logoPreviewUrl,
}) => {
  return (
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
