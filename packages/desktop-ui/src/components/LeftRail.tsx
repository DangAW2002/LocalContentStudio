import React from "react";
import { Icon } from "./Icon";

export type ShellArea = "home" | "projects" | "brand" | "library" | "settings";

export interface LeftRailProps {
  activeArea: ShellArea;
  onChangeArea: (area: ShellArea) => void;
  runningJobsCount: number;
}

interface NavItem {
  id: ShellArea;
  label: string;
  icon: string;
  shortcut: string;
}

const NAV_ITEMS: NavItem[] = [
  { id: "home", label: "Home", icon: "home", shortcut: "Ctrl+1" },
  { id: "projects", label: "Dự án", icon: "projects", shortcut: "Ctrl+2" },
  { id: "brand", label: "Brand", icon: "brand", shortcut: "Ctrl+3" },
  { id: "library", label: "Thư viện", icon: "library", shortcut: "Ctrl+4" },
  { id: "settings", label: "Tools & Cài đặt", icon: "settings", shortcut: "Ctrl+5" },
];

export const LeftRail: React.FC<LeftRailProps> = ({
  activeArea,
  onChangeArea,
  runningJobsCount,
}) => {
  return (
    <nav
      aria-label="Khu vực làm việc"
      style={{
        width: "var(--cs-rail-w)",
        minWidth: "var(--cs-rail-w)",
        backgroundColor: "var(--cs-bg-panel)",
        borderRight: "1px solid var(--cs-border-subtle)",
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        padding: "8px 0",
        gap: 6,
        userSelect: "none",
        zIndex: 10,
      }}
    >
      {NAV_ITEMS.map((item) => {
        const isActive = activeArea === item.id;
        return (
          <button
            key={item.id}
            type="button"
            aria-current={isActive ? "page" : undefined}
            className="cs-rail-item"
            onClick={() => onChangeArea(item.id)}
            style={{
              position: "relative",
              width: 40,
              height: 40,
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              borderRadius: "var(--cs-radius-control)",
              backgroundColor: isActive ? "var(--cs-selection)" : "transparent",
              color: isActive ? "var(--cs-accent)" : "var(--cs-fg-secondary)",
              border: "none",
              cursor: "pointer",
              transition: "all var(--cs-motion-duration) ease",
            }}
            title={`${item.label} (${item.shortcut})`}
          >
            {/* Active left indicator bar */}
            {isActive && (
              <span
                style={{
                  position: "absolute",
                  left: -6,
                  top: 8,
                  bottom: 8,
                  width: 3,
                  backgroundColor: "var(--cs-accent)",
                  borderRadius: "0 2px 2px 0",
                }}
              />
            )}
            <Icon name={item.icon} size={20} />
            {item.id === "projects" && runningJobsCount > 0 && (
              <span
                style={{
                  position: "absolute",
                  top: 4,
                  right: 4,
                  width: 8,
                  height: 8,
                  borderRadius: "var(--cs-radius-round)",
                  backgroundColor: "var(--cs-accent)",
                  boxShadow: "0 0 0 2px var(--cs-bg-panel)",
                }}
              />
            )}
          </button>
        );
      })}
    </nav>
  );
};
