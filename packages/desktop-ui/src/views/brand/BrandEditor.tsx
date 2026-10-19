import React from "react";
import type { BrandSpec } from "../../utils/brandYaml";
import { BrandColorSection } from "./BrandColorSection";
import { BrandTypographySection } from "./BrandTypographySection";
import { BrandLogosSection } from "./BrandLogosSection";
import { BrandVoiceSection } from "./BrandVoiceSection";
import { BrandTemplatePreview } from "./BrandTemplatePreview";

export interface BrandEditorProps {
  brandSpec: BrandSpec;
  setBrandSpec: (spec: BrandSpec) => void;
  fieldErrors: Record<string, string>;
  logoPreviewUrl: string | null;
}

export const BrandEditor: React.FC<BrandEditorProps> = ({
  brandSpec,
  setBrandSpec,
  fieldErrors,
  logoPreviewUrl,
}) => {
  return (
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
      <BrandColorSection
        brandSpec={brandSpec}
        setBrandSpec={setBrandSpec}
        fieldErrors={fieldErrors}
      />

      {/* 3. Typography & Live Preview */}
      <BrandTypographySection
        brandSpec={brandSpec}
        setBrandSpec={setBrandSpec}
        fieldErrors={fieldErrors}
      />

      {/* 4. Logos & Visual Preview */}
      <BrandLogosSection
        brandSpec={brandSpec}
        setBrandSpec={setBrandSpec}
        logoPreviewUrl={logoPreviewUrl}
      />

      {/* 5 & 6. Voice Words & Rules */}
      <BrandVoiceSection
        brandSpec={brandSpec}
        setBrandSpec={setBrandSpec}
      />

      {/* 7. Template Preview */}
      <BrandTemplatePreview
        brandSpec={brandSpec}
        logoPreviewUrl={logoPreviewUrl}
      />
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

const errorTextStyle: React.CSSProperties = {
  color: "var(--cs-danger)",
  fontSize: 10,
  fontWeight: 500,
};
