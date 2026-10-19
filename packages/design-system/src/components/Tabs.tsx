import React from "react";

export interface TabItem {
  id: string;
  label: string;
  count?: number;
}

export interface TabsProps {
  items: TabItem[];
  activeId: string;
  onChange: (id: string) => void;
  size?: "md" | "sm";
  style?: React.CSSProperties;
}

export const Tabs: React.FC<TabsProps> = ({
  items,
  activeId,
  onChange,
  size = "md",
  style,
}) => {
  const height = size === "sm" ? 32 : 40;
  const fontSize = size === "sm" ? "var(--cs-text-xs)" : "var(--cs-text-sm)";
  const listRef = React.useRef<HTMLDivElement>(null);

  // Keep the active tab visible when the strip overflows.
  React.useEffect(() => {
    const el = listRef.current?.querySelector<HTMLElement>('[aria-selected="true"]');
    el?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, [activeId]);

  // The scrollbar is hidden, so map vertical wheel to horizontal scroll (as VS Code's tab bar does).
  React.useEffect(() => {
    const el = listRef.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      if (el.scrollWidth <= el.clientWidth || Math.abs(e.deltaX) > Math.abs(e.deltaY)) return;
      e.preventDefault();
      el.scrollLeft += e.deltaY;
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, []);

  return (
    <div
      ref={listRef}
      role="tablist"
      style={{
        display: "flex",
        alignItems: "center",
        gap: "var(--cs-space-4)",
        borderBottom: "1px solid var(--cs-border-subtle)",
        height,
        // Like VS Code tabs: labels never wrap; overflow scrolls horizontally instead.
        overflowX: "auto",
        overflowY: "hidden",
        scrollbarWidth: "none",
        ...style,
      }}
    >
      {items.map((item) => {
        const isActive = item.id === activeId;
        return (
          <button
            key={item.id}
            role="tab"
            aria-selected={isActive}
            onClick={() => onChange(item.id)}
            style={{
              height: "100%",
              display: "inline-flex",
              alignItems: "center",
              gap: 6,
              background: "none",
              border: "none",
              borderBottom: isActive ? "2px solid var(--cs-accent)" : "2px solid transparent",
              color: isActive ? "var(--cs-accent)" : "var(--cs-fg-secondary)",
              fontWeight: isActive ? 600 : 500,
              fontSize,
              cursor: "pointer",
              padding: "0 4px",
              marginBottom: -1,
              whiteSpace: "nowrap",
              flexShrink: 0,
              transition: "color var(--cs-motion-duration) ease",
            }}
          >
            {/* A hidden bold copy reserves the active width, so selecting a tab never shifts its neighbours. */}
            <span style={{ display: "inline-grid" }}>
              <span style={{ gridArea: "1 / 1" }}>{item.label}</span>
              <span aria-hidden style={{ gridArea: "1 / 1", fontWeight: 600, visibility: "hidden" }}>
                {item.label}
              </span>
            </span>
            {item.count !== undefined && (
              <span
                style={{
                  fontSize: "var(--cs-text-xs)",
                  padding: "1px 5px",
                  borderRadius: "var(--cs-radius-control)",
                  backgroundColor: isActive ? "var(--cs-accent-subtle)" : "var(--cs-bg-raised)",
                  color: isActive ? "var(--cs-accent)" : "var(--cs-fg-muted)",
                }}
              >
                {item.count}
              </span>
            )}
          </button>
        );
      })}
    </div>
  );
};
