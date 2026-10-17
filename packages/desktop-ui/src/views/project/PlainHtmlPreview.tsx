import React from "react";
import { detectArtboardSize, type ArtboardSize } from "../../utils/artboard";

export interface PlainHtmlPreviewProps {
  html: string;
}

/**
 * Plain HTML artboard preview (HTML without data-layer).
 * Renders at detected artboard dimensions and scales to fit the stage viewport.
 */
export const PlainHtmlPreview: React.FC<PlainHtmlPreviewProps> = ({ html }) => {
  const containerRef = React.useRef<HTMLDivElement>(null);
  const [size, setSize] = React.useState<ArtboardSize>(() => detectArtboardSize(html));
  const [scale, setScale] = React.useState(1);

  React.useEffect(() => {
    setSize(detectArtboardSize(html));
  }, [html]);

  const fit = React.useCallback(() => {
    const box = containerRef.current;
    if (!box) return;
    const padding = 48;
    const availableW = Math.max(10, box.clientWidth - padding);
    const availableH = Math.max(10, box.clientHeight - padding);
    const s = Math.min(availableW / size.w, availableH / size.h, 1);
    setScale(s > 0 ? s : 1);
  }, [size]);

  React.useEffect(() => {
    fit();
    window.addEventListener("resize", fit);
    return () => window.removeEventListener("resize", fit);
  }, [fit]);

  return (
    <div
      ref={containerRef}
      style={{
        flex: 1,
        minWidth: 0,
        minHeight: 0,
        width: "100%",
        height: "100%",
        overflow: "auto",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        backgroundColor: "var(--cs-bg-canvas)",
        position: "relative",
        padding: "var(--cs-space-4)",
        boxSizing: "border-box",
      }}
    >
      <div
        style={{
          width: size.w * scale,
          height: size.h * scale,
          flexShrink: 0,
          boxShadow: "var(--cs-shadow-artboard)",
          backgroundColor: "#ffffff",
        }}
      >
        <iframe
          title="Xem trước Artboard HTML"
          srcDoc={html}
          sandbox="allow-scripts"
          style={{
            width: size.w,
            height: size.h,
            border: "none",
            transform: `scale(${scale})`,
            transformOrigin: "0 0",
            backgroundColor: "#ffffff",
          }}
        />
      </div>
    </div>
  );
};
