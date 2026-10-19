import React from "react";
import type { BrandSpec } from "../../utils/brandYaml";

export interface BrandTemplatePreviewProps {
  brandSpec: BrandSpec;
  logoPreviewUrl: string | null;
}

export const BrandTemplatePreview: React.FC<BrandTemplatePreviewProps> = ({
  brandSpec,
  logoPreviewUrl,
}) => {
  const [templateType, setTemplateType] = React.useState<"youtube" | "banner">("youtube");

  const primary = brandSpec.colors.primary || "#1E88E5";
  const accent = brandSpec.colors.accent || "#FFB300";
  const bg = brandSpec.colors.bg || "#0B0F14";
  const text = brandSpec.colors.text || "#FFFFFF";
  const headingFont = brandSpec.fonts.heading || "system-ui, sans-serif";
  const bodyFont = brandSpec.fonts.body || "system-ui, sans-serif";
  const brandName = brandSpec.name || "Tên thương hiệu";
  const slogan = brandSpec.slogan || "Khẩu hiệu nhận diện thương hiệu chuẩn brand@1";

  const logoMarkup = logoPreviewUrl
    ? `<img src="${logoPreviewUrl}" alt="logo" class="logo" />`
    : `<div class="logo-fallback">${brandName.slice(0, 2).toUpperCase()}</div>`;

  const htmlContent = React.useMemo(() => {
    if (templateType === "youtube") {
      return `<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8" />
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      background-color: ${bg};
      color: ${text};
      font-family: ${bodyFont}, system-ui, sans-serif;
      width: 100vw;
      height: 100vh;
      display: flex;
      flex-direction: column;
      justify-content: space-between;
      padding: 28px 36px;
      position: relative;
      overflow: hidden;
    }
    .ambient-glow-1 {
      position: absolute;
      top: -20%;
      right: -10%;
      width: 60%;
      height: 80%;
      background: radial-gradient(circle, ${primary}44 0%, transparent 70%);
      pointer-events: none;
    }
    .ambient-glow-2 {
      position: absolute;
      bottom: -30%;
      left: -10%;
      width: 50%;
      height: 70%;
      background: radial-gradient(circle, ${accent}33 0%, transparent 70%);
      pointer-events: none;
    }
    .top-bar {
      display: flex;
      align-items: center;
      justify-content: space-between;
      z-index: 10;
    }
    .brand-identity {
      display: flex;
      align-items: center;
      gap: 12px;
    }
    .logo {
      max-height: 44px;
      max-width: 130px;
      object-fit: contain;
    }
    .logo-fallback {
      width: 44px;
      height: 44px;
      border-radius: 8px;
      background: ${primary};
      color: #fff;
      display: flex;
      align-items: center;
      justify-content: center;
      font-weight: 800;
      font-size: 18px;
    }
    .brand-name {
      font-family: ${headingFont}, system-ui, sans-serif;
      font-size: 18px;
      font-weight: 700;
      letter-spacing: -0.2px;
      color: ${text};
    }
    .tag {
      background: ${accent};
      color: #0b0f14;
      font-size: 12px;
      font-weight: 800;
      padding: 4px 12px;
      border-radius: 20px;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }
    .main-body {
      z-index: 10;
      margin-top: auto;
      margin-bottom: 8px;
    }
    .topic-pill {
      display: inline-block;
      color: ${primary};
      font-size: 14px;
      font-weight: 700;
      margin-bottom: 8px;
      text-transform: uppercase;
      letter-spacing: 1px;
    }
    .headline {
      font-family: ${headingFont}, system-ui, sans-serif;
      font-size: 34px;
      font-weight: 800;
      line-height: 1.15;
      max-width: 85%;
      margin-bottom: 10px;
      text-shadow: 0 2px 10px rgba(0,0,0,0.5);
    }
    .headline span.accent {
      color: ${accent};
    }
    .slogan-line {
      font-size: 14px;
      color: ${text};
      opacity: 0.85;
      font-weight: 500;
    }
  </style>
</head>
<body>
  <div class="ambient-glow-1"></div>
  <div class="ambient-glow-2"></div>
  <div class="top-bar">
    <div class="brand-identity">
      ${logoMarkup}
      <span class="brand-name">${brandName}</span>
    </div>
    <div class="tag">MẪU NỘI DUNG</div>
  </div>
  <div class="main-body">
    <div class="topic-pill">ÁP DỤNG BẢN SẮC THƯƠNG HIỆU</div>
    <h1 class="headline">Tạo Nội Dung Đột Phá Với <span class="accent">${brandName}</span></h1>
    <div class="slogan-line">${slogan}</div>
  </div>
</body>
</html>`;
    } else {
      return `<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8" />
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      background-color: ${bg};
      color: ${text};
      font-family: ${bodyFont}, system-ui, sans-serif;
      width: 100vw;
      height: 100vh;
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 24px 36px;
      position: relative;
      border-left: 8px solid ${primary};
    }
    .info {
      z-index: 10;
      display: flex;
      flex-direction: column;
      gap: 6px;
    }
    .title {
      font-family: ${headingFont}, system-ui, sans-serif;
      font-size: 24px;
      font-weight: 800;
      color: ${text};
    }
    .desc {
      font-size: 14px;
      color: ${accent};
      font-weight: 600;
    }
    .logo {
      max-height: 50px;
      max-width: 140px;
      object-fit: contain;
    }
    .logo-fallback {
      width: 50px;
      height: 50px;
      border-radius: 8px;
      background: ${primary};
      color: #fff;
      display: flex;
      align-items: center;
      justify-content: center;
      font-weight: 800;
      font-size: 20px;
    }
  </style>
</head>
<body>
  <div class="info">
    <div class="title">${brandName}</div>
    <div class="desc">${slogan}</div>
  </div>
  ${logoMarkup}
</body>
</html>`;
    }
  }, [templateType, primary, accent, bg, text, headingFont, bodyFont, brandName, slogan, logoMarkup]);

  return (
    <div style={cardSectionStyle}>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <div>
          <div style={sectionTitleStyle}>7. ÁP BRAND LÊN MẪU (THUMBNAIL PREVIEW)</div>
          <p style={{ margin: "4px 0 0 0", fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
            Xem thử nhận diện thương hiệu tự động áp vào mẫu thumbnail HTML trong iframe sandbox (Plan §6).
          </p>
        </div>
        <div style={{ display: "flex", gap: 6 }}>
          <button
            type="button"
            onClick={() => setTemplateType("youtube")}
            style={{
              ...filterButtonStyle,
              backgroundColor: templateType === "youtube" ? "var(--cs-bg-active)" : "var(--cs-bg-raised)",
              color: templateType === "youtube" ? "var(--cs-fg-primary)" : "var(--cs-fg-secondary)",
            }}
          >
            Video 16:9
          </button>
          <button
            type="button"
            onClick={() => setTemplateType("banner")}
            style={{
              ...filterButtonStyle,
              backgroundColor: templateType === "banner" ? "var(--cs-bg-active)" : "var(--cs-bg-raised)",
              color: templateType === "banner" ? "var(--cs-fg-primary)" : "var(--cs-fg-secondary)",
            }}
          >
            Banner ngang
          </button>
        </div>
      </div>

      <div
        style={{
          width: "100%",
          aspectRatio: templateType === "youtube" ? "16 / 9" : "3 / 1",
          maxHeight: 280,
          borderRadius: "var(--cs-radius-control)",
          overflow: "hidden",
          border: "1px solid var(--cs-border-subtle)",
          boxShadow: "0 4px 16px rgba(0,0,0,0.3)",
          position: "relative",
          backgroundColor: "#000",
        }}
      >
        <iframe
          title="Brand Template Sandbox Preview"
          srcDoc={htmlContent}
          sandbox="allow-same-origin"
          style={{
            width: "100%",
            height: "100%",
            border: "none",
            display: "block",
            pointerEvents: "none",
          }}
        />
      </div>
    </div>
  );
};

const cardSectionStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 12,
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

const filterButtonStyle: React.CSSProperties = {
  padding: "4px 10px",
  border: "1px solid var(--cs-border-subtle)",
  borderRadius: "var(--cs-radius-control)",
  fontSize: 11,
  fontWeight: 600,
  cursor: "pointer",
};
