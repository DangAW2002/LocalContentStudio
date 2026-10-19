import React from "react";
import { Button } from "@studio/design-system";
import { isValidHexColor, type BrandSpec } from "../../utils/brandYaml";

export interface BrandColorSectionProps {
  brandSpec: BrandSpec;
  setBrandSpec: (spec: BrandSpec) => void;
  fieldErrors: Record<string, string>;
}

export const PALETTE_FIELDS = [
  { key: "primary", label: "Màu chính (Primary)", fallback: "#1e88e5" },
  { key: "accent", label: "Màu nhấn (Accent)", fallback: "#ffb300" },
  { key: "bg", label: "Màu nền (Background)", fallback: "#0b0f14" },
  { key: "text", label: "Màu chữ (Text)", fallback: "#ffffff" },
] as const;

export const BrandColorSection: React.FC<BrandColorSectionProps> = ({
  brandSpec,
  setBrandSpec,
  fieldErrors,
}) => {
  const [newExtraColor, setNewExtraColor] = React.useState("#6366f1");

  return (
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

const inputStyle: React.CSSProperties = {
  padding: "6px 10px",
  backgroundColor: "var(--cs-bg-app)",
  border: "1px solid var(--cs-border-subtle)",
  borderRadius: "var(--cs-radius-control)",
  color: "var(--cs-fg-primary)",
  fontSize: "var(--cs-text-xs)",
  outline: "none",
};

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
