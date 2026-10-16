export type Theme = "light" | "dark" | "system";

export function applyTheme(theme: Theme): void {
  if (typeof document === "undefined") return;
  const root = document.documentElement;

  if (theme === "system") {
    const isDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    root.setAttribute("data-theme", isDark ? "dark" : "light");
  } else {
    root.setAttribute("data-theme", theme);
  }
}

export function getEffectiveTheme(): "light" | "dark" {
  if (typeof document === "undefined") return "light";
  return document.documentElement.getAttribute("data-theme") === "dark" ? "dark" : "light";
}

export const CS_TOKENS = {
  bgApp: "var(--cs-bg-app)",
  bgPanel: "var(--cs-bg-panel)",
  bgCanvas: "var(--cs-bg-canvas)",
  fgPrimary: "var(--cs-fg-primary)",
  fgSecondary: "var(--cs-fg-secondary)",
  fgMuted: "var(--cs-fg-muted)",
  borderSubtle: "var(--cs-border-subtle)",
  borderStrong: "var(--cs-border-strong)",
  accent: "var(--cs-accent)",
  accentHover: "var(--cs-accent-hover)",
  accentSubtle: "var(--cs-accent-subtle)",
  focus: "var(--cs-focus)",
  selection: "var(--cs-selection)",
  success: "var(--cs-success)",
  warning: "var(--cs-warning)",
  danger: "var(--cs-danger)",
} as const;

export * from "./components/Button";
export * from "./components/Badge";
export * from "./components/Input";
export * from "./components/Dialog";
export * from "./components/Panel";
export * from "./components/Callout";
export * from "./components/Chip";
export * from "./components/Kbd";
export * from "./components/EmptyState";
export * from "./components/ErrorState";
export * from "./components/Tabs";
export * from "./components/Select";
export * from "./components/Spinner";
