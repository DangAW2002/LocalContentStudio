import React from "react";
import { Icon } from "./Icon";
import { Kbd } from "@studio/design-system";

export interface TitleBarProps {
  workspaceName?: string;
  projectName?: string;
  activeViewLabel: string;
  runningJobsCount: number;
  theme: "light" | "dark";
  onToggleTheme: () => void;
  onOpenWorkspaceSwitcher: () => void;
  onOpenCommandPalette: () => void;
  onToggleTaskCenter: () => void;
  onNavigateHome: () => void;
  onNavigateProjects: () => void;
}

export const TitleBar: React.FC<TitleBarProps> = ({
  workspaceName = "Chưa chọn Workspace",
  projectName,
  activeViewLabel,
  runningJobsCount,
  theme,
  onToggleTheme,
  onOpenWorkspaceSwitcher,
  onOpenCommandPalette,
  onToggleTaskCenter,
  onNavigateHome,
  onNavigateProjects,
}) => {
  const [windowWidth, setWindowWidth] = React.useState<number>(
    typeof window !== "undefined" ? window.innerWidth : 1024
  );

  React.useEffect(() => {
    if (typeof window === "undefined") return;
    const handleResize = () => setWindowWidth(window.innerWidth);
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, []);

  const isUnder600 = windowWidth < 600;
  const isSmallPhone = windowWidth <= 480;

  return (
    <header
      style={{
        height: "var(--cs-titlebar-h)",
        minHeight: "var(--cs-titlebar-h)",
        // Equal side columns keep the command palette trigger centred regardless of breadcrumb length.
        display: "grid",
        gridTemplateColumns: "minmax(0, 1fr) auto minmax(0, 1fr)",
        alignItems: "center",
        gap: "var(--cs-space-3)",
        padding: "0 var(--cs-space-3)",
        backgroundColor: "var(--cs-bg-panel)",
        borderBottom: "1px solid var(--cs-border-subtle)",
        userSelect: "none",
        zIndex: 20,
      }}
    >
      {/* Left: Brand mark + Workspace switcher + Breadcrumb */}
      <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-2)", minWidth: 0, flexShrink: 1 }}>
        {/* Brand mark */}
        <div
          onClick={onNavigateHome}
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: 8,
            cursor: "pointer",
            fontWeight: 700,
            color: "var(--cs-fg-primary)",
            paddingRight: "var(--cs-space-2)",
            flexShrink: 0,
          }}
          title="Về trang chủ"
        >
          <div
            style={{
              width: 24,
              height: 24,
              backgroundColor: "var(--cs-accent)",
              color: "var(--cs-fg-inverse)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              fontSize: 11,
              fontWeight: 800,
              borderRadius: "var(--cs-radius-control)",
            }}
          >
            LCS
          </div>
          {!isSmallPhone && <span style={{ fontSize: "var(--cs-text-sm)", whiteSpace: "nowrap" }}>Studio</span>}
        </div>

        <div style={{ width: 1, height: 16, backgroundColor: "var(--cs-border-subtle)", flexShrink: 0 }} />

        {/* Workspace Switcher Button */}
        <button
          type="button"
          onClick={onOpenWorkspaceSwitcher}
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: 6,
            height: 30,
            padding: "0 8px",
            border: "1px solid transparent",
            backgroundColor: "transparent",
            color: "var(--cs-fg-primary)",
            fontSize: "var(--cs-text-sm)",
            fontWeight: 600,
            borderRadius: "var(--cs-radius-control)",
            cursor: "pointer",
            maxWidth: isSmallPhone ? 100 : isUnder600 ? 140 : 220,
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
            flexShrink: 1,
          }}
          title="Chuyển hoặc tạo workspace"
        >
          <Icon name="folder" size={14} color="var(--cs-accent)" />
          <span style={{ overflow: "hidden", textOverflow: "ellipsis" }}>{workspaceName}</span>
          <Icon name="chev-down" size={12} color="var(--cs-fg-muted)" />
        </button>

        {/* Breadcrumb - hidden on mobile <= 480px to prevent overflow */}
        {!isSmallPhone && (
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 4,
              fontSize: "var(--cs-text-xs)",
              color: "var(--cs-fg-secondary)",
              marginLeft: 4,
              minWidth: 0,
              overflow: "hidden",
            }}
          >
            <span style={{ color: "var(--cs-border-strong)" }}>/</span>
            {projectName && !isUnder600 && (
              <>
                <button
                  type="button"
                  onClick={onNavigateProjects}
                  style={{
                    background: "none",
                    border: "none",
                    padding: "2px 4px",
                    color: "var(--cs-fg-secondary)",
                    cursor: "pointer",
                    borderRadius: "var(--cs-radius-control)",
                    maxWidth: 90,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}
                >
                  {projectName}
                </button>
                <span style={{ color: "var(--cs-border-strong)" }}>/</span>
              </>
            )}
            <span style={{ color: "var(--cs-fg-primary)", fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {activeViewLabel}
            </span>
          </div>
        )}
      </div>

      {/* Center: Command Palette Trigger */}
      {isUnder600 ? (
        <button
          type="button"
          onClick={onOpenCommandPalette}
          aria-label="Tìm kiếm hoặc chạy lệnh"
          style={{
            display: "inline-flex",
            alignItems: "center",
            justifyContent: "center",
            width: 30,
            height: 30,
            border: "1px solid var(--cs-border-subtle)",
            backgroundColor: "var(--cs-bg-raised)",
            color: "var(--cs-fg-secondary)",
            borderRadius: "var(--cs-radius-control)",
            cursor: "pointer",
            flexShrink: 0,
          }}
          title="Tìm kiếm hoặc chạy lệnh"
        >
          <Icon name="search" size={14} color="var(--cs-fg-muted)" />
        </button>
      ) : (
        <button
          type="button"
          onClick={onOpenCommandPalette}
          style={{
            display: "flex",
            alignItems: "center",
            gap: 8,
            height: 30,
            width: "min(380px, 30vw)",
            padding: "0 8px",
            border: "1px solid var(--cs-border-subtle)",
            backgroundColor: "var(--cs-bg-raised)",
            color: "var(--cs-fg-secondary)",
            borderRadius: "var(--cs-radius-control)",
            cursor: "pointer",
            fontSize: "var(--cs-text-xs)",
            whiteSpace: "nowrap",
            overflow: "hidden",
          }}
          title="Tìm kiếm hoặc chạy lệnh (Ctrl+K)"
        >
          <Icon name="search" size={14} color="var(--cs-fg-muted)" />
          <span style={{ flex: 1, textAlign: "left", overflow: "hidden", textOverflow: "ellipsis" }}>Tìm kiếm hoặc chạy lệnh...</span>
          <Kbd>Ctrl+K</Kbd>
        </button>
      )}

      {/* Right: Task Center Trigger + Theme Toggle */}
      <div style={{ display: "flex", alignItems: "center", justifyContent: "flex-end", gap: "var(--cs-space-2)", flexShrink: 0 }}>
        {/* Task Center Button */}
        <button
          type="button"
          onClick={onToggleTaskCenter}
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: 6,
            height: 30,
            padding: isUnder600 ? "0 6px" : "0 8px",
            border: "1px solid var(--cs-border-subtle)",
            backgroundColor: runningJobsCount > 0 ? "var(--cs-accent-subtle)" : "transparent",
            color: runningJobsCount > 0 ? "var(--cs-accent)" : "var(--cs-fg-secondary)",
            borderRadius: "var(--cs-radius-control)",
            cursor: "pointer",
            fontSize: "var(--cs-text-xs)",
            fontWeight: 500,
            flexShrink: 0,
            whiteSpace: "nowrap",
          }}
          title="Mở Task Center (Ctrl+J)"
        >
          <Icon name="activity" size={14} color={runningJobsCount > 0 ? "var(--cs-accent)" : "var(--cs-fg-muted)"} />
          {!isSmallPhone && <span>Tác vụ</span>}
          {runningJobsCount > 0 && (
            <span
              style={{
                backgroundColor: "var(--cs-accent)",
                color: "var(--cs-fg-inverse)",
                padding: "0 5px",
                borderRadius: "var(--cs-radius-round)",
                fontSize: 10,
                fontWeight: 700,
              }}
            >
              {runningJobsCount}
            </span>
          )}
          {!isUnder600 && <Kbd>Ctrl+J</Kbd>}
        </button>

        {/* Theme Toggle */}
        <button
          type="button"
          onClick={onToggleTheme}
          aria-label={`Chuyển sang giao diện ${theme === "light" ? "Tối" : "Sáng"}`}
          style={{
            display: "inline-flex",
            alignItems: "center",
            justifyContent: "center",
            width: 30,
            height: 30,
            border: "1px solid var(--cs-border-subtle)",
            backgroundColor: "transparent",
            color: "var(--cs-fg-secondary)",
            borderRadius: "var(--cs-radius-control)",
            cursor: "pointer",
            flexShrink: 0,
          }}
          title={`Giao diện: ${theme === "light" ? "Sáng (bấm đổi sang Tối)" : "Tối (bấm đổi sang Sáng)"}`}
        >
          <Icon name={theme === "light" ? "moon" : "sun"} size={14} />
        </button>
      </div>
    </header>
  );
};
