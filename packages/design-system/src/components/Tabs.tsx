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
  const [canScrollLeft, setCanScrollLeft] = React.useState(false);
  const [canScrollRight, setCanScrollRight] = React.useState(false);

  const updateScrollState = React.useCallback(() => {
    const el = listRef.current;
    if (!el) return;
    const { scrollLeft, scrollWidth, clientWidth } = el;
    setCanScrollLeft(scrollLeft > 2);
    setCanScrollRight(scrollLeft + clientWidth < scrollWidth - 2);
  }, []);

  // Keep the active tab visible when the strip overflows.
  React.useEffect(() => {
    const el = listRef.current?.querySelector<HTMLElement>('[aria-selected="true"]');
    el?.scrollIntoView({ block: "nearest", inline: "nearest" });
    updateScrollState();
  }, [activeId, updateScrollState]);

  // Track resize & scroll to update arrow/mask states
  React.useEffect(() => {
    const el = listRef.current;
    if (!el) return;
    updateScrollState();

    const onScroll = () => updateScrollState();
    el.addEventListener("scroll", onScroll, { passive: true });

    let ro: ResizeObserver | null = null;
    if (typeof ResizeObserver !== "undefined") {
      ro = new ResizeObserver(() => updateScrollState());
      ro.observe(el);
    }

    return () => {
      el.removeEventListener("scroll", onScroll);
      ro?.disconnect();
    };
  }, [updateScrollState]);

  // The scrollbar is hidden, so map vertical wheel to horizontal scroll (as VS Code's tab bar does).
  React.useEffect(() => {
    const el = listRef.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      if (el.scrollWidth <= el.clientWidth || Math.abs(e.deltaX) > Math.abs(e.deltaY)) return;
      e.preventDefault();
      el.scrollLeft += e.deltaY;
      updateScrollState();
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, [updateScrollState]);

  const scrollByAmount = (delta: number) => {
    const el = listRef.current;
    if (!el) return;
    el.scrollBy({ left: delta, behavior: "smooth" });
  };

  return (
    <div
      style={{
        position: "relative",
        display: "flex",
        alignItems: "center",
        width: "100%",
        height,
      }}
    >
      {/* Scroll Left Button */}
      {canScrollLeft && (
        <button
          type="button"
          aria-label="Cuộn sang trái"
          title="Cuộn sang trái"
          onClick={() => scrollByAmount(-150)}
          style={{
            position: "absolute",
            left: 0,
            top: 0,
            bottom: 0,
            zIndex: 3,
            display: "inline-flex",
            alignItems: "center",
            justifyContent: "center",
            width: 24,
            border: "none",
            backgroundColor: "var(--cs-bg-panel)",
            color: "var(--cs-fg-secondary)",
            cursor: "pointer",
            boxShadow: "2px 0 6px rgb(0 0 0 / 10%)",
            opacity: 0.9,
          }}
        >
          ‹
        </button>
      )}

      {/* Main Tablist */}
      <div
        ref={listRef}
        role="tablist"
        style={{
          display: "flex",
          alignItems: "center",
          gap: "var(--cs-space-4)",
          borderBottom: "1px solid var(--cs-border-subtle)",
          height: "100%",
          width: "100%",
          // Like VS Code tabs: labels never wrap; overflow scrolls horizontally instead.
          overflowX: "auto",
          overflowY: "hidden",
          scrollbarWidth: "none",
          paddingLeft: canScrollLeft ? 26 : 0,
          paddingRight: canScrollRight ? 26 : 0,
          transition: "padding 0.15s ease",
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

      {/* Scroll Right Button */}
      {canScrollRight && (
        <button
          type="button"
          aria-label="Cuộn sang phải"
          title="Cuộn sang phải"
          onClick={() => scrollByAmount(150)}
          style={{
            position: "absolute",
            right: 0,
            top: 0,
            bottom: 0,
            zIndex: 3,
            display: "inline-flex",
            alignItems: "center",
            justifyContent: "center",
            width: 24,
            border: "none",
            backgroundColor: "var(--cs-bg-panel)",
            color: "var(--cs-fg-secondary)",
            cursor: "pointer",
            boxShadow: "-2px 0 6px rgb(0 0 0 / 10%)",
            opacity: 0.9,
          }}
        >
          ›
        </button>
      )}
    </div>
  );
};
