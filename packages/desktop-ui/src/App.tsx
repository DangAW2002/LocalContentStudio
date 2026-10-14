import React from "react";
import { applyTheme, Button, Badge } from "@studio/design-system";
import { ExplorerPanel } from "./components/ExplorerPanel";
import { InspectorPanel } from "./components/InspectorPanel";
import { TaskCenter, TaskJob } from "./components/TaskCenter";
import { CommandPalette, CommandItem } from "./components/CommandPalette";
import { DashboardView } from "./views/DashboardView";
import { DesignView } from "./views/DesignView";
import { ResearchView } from "./views/ResearchView";
import { LibraryView } from "./views/LibraryView";
import { BrandView } from "./views/BrandView";
import { WritingView } from "./views/WritingView";
import { PublishingView } from "./views/PublishingView";

export const App: React.FC = () => {
  const [activeView, setActiveView] = React.useState<string>("design");
  const [theme, setTheme] = React.useState<"light" | "dark">("light");
  const [selectedElement, setSelectedElement] = React.useState<any | null>(null);
  const [cmdOpen, setCmdOpen] = React.useState(false);

  const [jobs, setJobs] = React.useState<TaskJob[]>([
    {
      id: "job-1",
      type: "Render PNG Export",
      state: "READY",
      progress: 100,
      stageName: "Hoàn tất xuất ảnh",
    },
  ]);

  const toggleTheme = () => {
    const next = theme === "light" ? "dark" : "light";
    setTheme(next);
    applyTheme(next);
  };

  React.useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setCmdOpen((prev) => !prev);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  const commands: CommandItem[] = [
    { id: "c1", title: "Mở Tổng quan (Dashboard)", category: "Điều hướng", action: () => setActiveView("dashboard") },
    { id: "c2", title: "Mở Trình thiết kế (Design Canvas)", category: "Điều hướng", action: () => setActiveView("design") },
    { id: "c3", title: "Mở Nghiên cứu (Research)", category: "Điều hướng", action: () => setActiveView("research") },
    { id: "c4", title: "Mở Thư viện tài nguyên (Library)", category: "Điều hướng", action: () => setActiveView("library") },
    { id: "c5", title: "Mở Bộ nhận diện (Brand Kit)", category: "Điều hướng", action: () => setActiveView("brand") },
    { id: "c6", title: "Mở Soạn thảo (Writing)", category: "Điều hướng", action: () => setActiveView("writing") },
    { id: "c7", title: "Mở Bàn giao Social (Publishing)", category: "Điều hướng", action: () => setActiveView("publishing") },
    { id: "c8", title: "Chuyển giao diện Sáng / Tối", category: "Giao diện", action: toggleTheme },
  ];

  const brandTokens = {
    headingFont: "Inter, Segoe UI, sans-serif",
    primaryColor: "var(--cs-accent)",
    surfaceColor: "var(--cs-bg-panel)",
  };

  const renderActiveView = () => {
    switch (activeView) {
      case "dashboard":
        return <DashboardView onOpenProject={() => setActiveView("design")} onCreateNew={() => setActiveView("design")} />;
      case "design":
        return <DesignView onSelectElement={setSelectedElement} onExport={() => setActiveView("publishing")} />;
      case "research":
        return <ResearchView />;
      case "library":
        return <LibraryView />;
      case "brand":
        return <BrandView />;
      case "writing":
        return <WritingView />;
      case "publishing":
        return <PublishingView />;
      default:
        return <DashboardView onOpenProject={() => setActiveView("design")} onCreateNew={() => setActiveView("design")} />;
    }
  };

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100vh",
        width: "100vw",
        backgroundColor: "var(--cs-bg-app)",
        overflow: "hidden",
      }}
    >
      {/* Top Application Bar */}
      <header
        style={{
          height: "44px",
          backgroundColor: "var(--cs-bg-panel)",
          borderBottom: "1px solid var(--cs-border-subtle)",
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "0 var(--cs-space-4)",
          userSelect: "none",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-3)" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-2)" }}>
            <span style={{ fontSize: "18px" }}>🎬</span>
            <span style={{ fontWeight: 700, fontSize: "var(--cs-text-md)" }}>Local Content Studio</span>
          </div>
          <Badge variant="info">Trong workspace: Marketing Q4</Badge>
          <Badge variant="success">Offline / Local DB WAL</Badge>
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-2)" }}>
          <Button variant="secondary" onClick={() => setCmdOpen(true)} style={{ height: "30px", fontSize: "var(--cs-text-xs)" }}>
            🔍 Thao tác nhanh (Ctrl+K)
          </Button>
          <Button variant="quiet" onClick={toggleTheme} style={{ height: "30px", width: "36px" }} aria-label="Đổi giao diện">
            {theme === "light" ? "🌙" : "☀️"}
          </Button>
        </div>
      </header>

      {/* Main Workspace 3-column Area */}
      <div style={{ display: "flex", flex: 1, overflow: "hidden", position: "relative" }}>
        <ExplorerPanel
          activeView={activeView}
          onSelectView={setActiveView}
          projectsCount={3}
          assetsCount={14}
        />

        <main style={{ flex: 1, overflow: "hidden", display: "flex", flexDirection: "column" }}>
          {renderActiveView()}
        </main>

        <InspectorPanel
          selectedElement={selectedElement}
          brandTokens={brandTokens}
        />
      </div>

      {/* Bottom Task Center */}
      <TaskCenter
        jobs={jobs}
        onCancelJob={(id) => {
          setJobs((prev) => prev.filter((j) => j.id !== id));
        }}
      />

      {/* Command Palette */}
      <CommandPalette
        open={cmdOpen}
        onClose={() => setCmdOpen(false)}
        commands={commands}
      />
    </div>
  );
};
