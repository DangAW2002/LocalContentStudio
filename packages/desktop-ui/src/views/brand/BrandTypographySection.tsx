import React from "react";
import type { BrandSpec } from "../../utils/brandYaml";

export interface BrandTypographySectionProps {
  brandSpec: BrandSpec;
  setBrandSpec: (spec: BrandSpec) => void;
  fieldErrors: Record<string, string>;
}

export const BrandTypographySection: React.FC<BrandTypographySectionProps> = ({
  brandSpec,
  setBrandSpec,
  fieldErrors,
}) => {
  return (
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
