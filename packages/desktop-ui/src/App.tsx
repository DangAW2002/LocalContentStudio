import React from "react";
import { applyTheme } from "@studio/design-system";
import { TitleBar } from "./components/TitleBar";
import { LeftRail, ShellArea } from "./components/LeftRail";
import { StatusBar } from "./components/StatusBar";
import { TaskCenter } from "./components/TaskCenter";
import { CommandPalette, CommandItem } from "./components/CommandPalette";
import { WorkspaceSwitcherModal } from "./components/WorkspaceSwitcherModal";
import { HomeView } from "./views/HomeView";
import { ProjectView } from "./views/ProjectView";
import { BrandView } from "./views/BrandView";
import { LibraryView } from "./views/LibraryView";
import { SettingsView } from "./views/SettingsView";
import { WebAuthModal } from "./components/WebAuthModal";
import { studioApi } from "./api/client";
import type {
  WorkspaceSummary,
  EngineProbeResult,
  JobSnapshot,
  JobState,
  DiskUsageReport,
} from "./api/types";

export const App: React.FC = () => {
  const [activeArea, setActiveArea] = React.useState<ShellArea>("home");
  const [theme, setTheme] = React.useState<"light" | "dark">("light");

  // Navigation context state
  const [activeProjectSlug, setActiveProjectSlug] = React.useState<string | undefined>();
  const [activeThreadId, setActiveThreadId] = React.useState<string | undefined>();

  // Modals & drawers state
  const [cmdOpen, setCmdOpen] = React.useState(false);
  const [taskCenterOpen, setTaskCenterOpen] = React.useState(false);
  const [workspaceSwitcherOpen, setWorkspaceSwitcherOpen] = React.useState(false);

  // Global core data
  const [currentWorkspace, setCurrentWorkspace] = React.useState<WorkspaceSummary | undefined>();
  const [recentWorkspaces, setRecentWorkspaces] = React.useState<WorkspaceSummary[]>([]);
  const [engines, setEngines] = React.useState<EngineProbeResult[]>([]);
  const [jobs, setJobs] = React.useState<JobSnapshot[]>([]);
  const [diskUsage, setDiskUsage] = React.useState<DiskUsageReport | undefined>();

  // Theme toggle
  const toggleTheme = () => {
    const next = theme === "light" ? "dark" : "light";
    setTheme(next);
    applyTheme(next);
  };

  // Initial data loading
  const loadWorkspaceAndCore = React.useCallback(async () => {
    try {
      const [currWs, recents, engs, jobList, disk] = await Promise.all([
        studioApi.getCurrentWorkspace().catch(() => null),
        studioApi.getRecentWorkspaces().catch(() => []),
        studioApi.probeEngines().catch(() => []),
        studioApi.getJobsSnapshot().catch(() => []),
        studioApi.getDiskUsage().catch(() => undefined),
      ]);

      if (currWs?.workspace) {
        setCurrentWorkspace(currWs.workspace);
      } else if (recents.length === 0) {
        setWorkspaceSwitcherOpen(true);
      }
      setRecentWorkspaces(recents);
      setEngines(engs);
      setJobs(jobList);
      setDiskUsage(disk);
    } catch {
      // Disconnected state
    }
  }, []);

  React.useEffect(() => {
    loadWorkspaceAndCore();
  }, [loadWorkspaceAndCore]);

  React.useEffect(() => {
    const handleOpenSwitcher = () => setWorkspaceSwitcherOpen(true);
    window.addEventListener("studio:open-workspace-switcher", handleOpenSwitcher);
    return () => window.removeEventListener("studio:open-workspace-switcher", handleOpenSwitcher);
  }, []);

  // Subscribe to live job events
  React.useEffect(() => {
    let unlisten: (() => void) | undefined;
    const subscribe = async () => {
      try {
        unlisten = await studioApi.onJobEvent((payload) => {
          setJobs((prev) => {
            const exists = prev.some((j) => j.id === payload.jobId);
            const snapshotState: JobState =
              payload.state === "queued" || payload.state === "admitted"
                ? "PENDING"
                : payload.state === "running"
                ? "RUNNING"
                : payload.state === "succeeded"
                ? "DONE"
                : payload.state === "failed"
                ? "FAILED"
                : "CANCELLED";

            const updated: JobSnapshot = {
              id: payload.jobId,
              title: payload.kind,
              kind: payload.kind,
              state: snapshotState,
              progress: payload.progress,
              stageName: payload.message,
              createdAt: payload.event?.createdAt || new Date().toISOString(),
              error: payload.state === "failed" ? payload.message : undefined,
              peakMemoryBytes: payload.peakMemoryBytes,
            };

            if (exists) {
              return prev.map((j) => (j.id === payload.jobId ? { ...j, ...updated } : j));
            } else {
              return [updated, ...prev];
            }
          });
        });
      } catch {
        // Disconnected
      }
    };
    subscribe();
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  // Keyboard shortcuts (No Ctrl+Shift!)
  React.useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Do not handle if Ctrl+Shift is pressed (respecting Unikey/EVKey)
      if (e.ctrlKey && e.shiftKey) return;

      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setCmdOpen((prev) => !prev);
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "j") {
        e.preventDefault();
        setTaskCenterOpen((prev) => !prev);
      } else if (e.key === "Escape") {
        if (cmdOpen) setCmdOpen(false);
        else if (taskCenterOpen) setTaskCenterOpen(false);
        else if (workspaceSwitcherOpen) setWorkspaceSwitcherOpen(false);
      } else if ((e.ctrlKey || e.metaKey) && e.key === "1") {
        e.preventDefault();
        setActiveArea("home");
      } else if ((e.ctrlKey || e.metaKey) && e.key === "2") {
        e.preventDefault();
        setActiveArea("projects");
      } else if ((e.ctrlKey || e.metaKey) && e.key === "3") {
        e.preventDefault();
        setActiveArea("brand");
      } else if ((e.ctrlKey || e.metaKey) && e.key === "4") {
        e.preventDefault();
        setActiveArea("library");
      } else if ((e.ctrlKey || e.metaKey) && e.key === "5") {
        e.preventDefault();
        setActiveArea("settings");
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [cmdOpen, taskCenterOpen, workspaceSwitcherOpen]);

  // Command palette items
  const commands: CommandItem[] = [
    {
      id: "cmd-home",
      title: "Về trang chủ (Home)",
      category: "Điều hướng",
      icon: "home",
      shortcut: "Ctrl+1",
      action: () => setActiveArea("home"),
    },
    {
      id: "cmd-projects",
      title: "Mở Dự án (Projects)",
      category: "Điều hướng",
      icon: "projects",
      shortcut: "Ctrl+2",
      action: () => setActiveArea("projects"),
    },
    {
      id: "cmd-brand",
      title: "Mở Brand, Channel & Playbook",
      category: "Điều hướng",
      icon: "brand",
      shortcut: "Ctrl+3",
      action: () => setActiveArea("brand"),
    },
    {
      id: "cmd-library",
      title: "Mở Thư viện tài nguyên (Library)",
      category: "Điều hướng",
      icon: "library",
      shortcut: "Ctrl+4",
      action: () => setActiveArea("library"),
    },
    {
      id: "cmd-settings",
      title: "Mở Tools & Cài đặt hệ thống",
      category: "Điều hướng",
      icon: "settings",
      shortcut: "Ctrl+5",
      action: () => setActiveArea("settings"),
    },
    {
      id: "cmd-taskcenter",
      title: "Mở / Đóng Trung tâm Tác vụ (Task Center)",
      category: "Thao tác",
      icon: "activity",
      shortcut: "Ctrl+J",
      action: () => setTaskCenterOpen((prev) => !prev),
    },
    {
      id: "cmd-workspace",
      title: "Chuyển hoặc tạo Workspace mới...",
      category: "Workspace",
      icon: "folder",
      action: () => setWorkspaceSwitcherOpen(true),
    },
    {
      id: "cmd-theme",
      title: `Chuyển sang giao diện ${theme === "light" ? "Tối (Dark)" : "Sáng (Light)"}`,
      category: "Giao diện",
      icon: theme === "light" ? "moon" : "sun",
      action: toggleTheme,
    },
  ];

  const runningJobsCount = jobs.filter((j) => j.state === "RUNNING").length;

  const getAreaLabel = (): string => {
    switch (activeArea) {
      case "home":
        return "Home";
      case "projects":
        return "Dự án";
      case "brand":
        return "Brand";
      case "library":
        return "Thư viện";
      case "settings":
        return "Tools & Cài đặt";
    }
  };

  const handleOpenProjectFromHome = (slug: string, threadId?: string) => {
    setActiveProjectSlug(slug);
    setActiveThreadId(threadId);
    setActiveArea("projects");
  };

  const handleOpenWorkspace = async (path: string) => {
    const ws = await studioApi.openWorkspace({ path });
    setCurrentWorkspace(ws);
    await loadWorkspaceAndCore();
  };

  const handleCreateWorkspace = async (path: string, name: string) => {
    const ws = await studioApi.createWorkspace({ path, name });
    setCurrentWorkspace(ws);
    await loadWorkspaceAndCore();
  };

  const [settingsTab, setSettingsTab] = React.useState<"engines" | "tools" | "env" | "catalog" | "jobs" | "disk" | "api-keys">("engines");

  const handleNavigateSettings = React.useCallback((tab?: string) => {
    if (tab) {
      setSettingsTab(tab as any);
    }
    setActiveArea("settings");
  }, []);

  React.useEffect(() => {
    const handleNavEvent = (e: Event) => {
      const customEvent = e as CustomEvent<{ area?: ShellArea; tab?: string }>;
      if (customEvent.detail?.area) {
        if (customEvent.detail.area === "settings" && customEvent.detail.tab) {
          setSettingsTab(customEvent.detail.tab as any);
        }
        setActiveArea(customEvent.detail.area);
      }
    };
    window.addEventListener("studio:navigate", handleNavEvent);
    return () => window.removeEventListener("studio:navigate", handleNavEvent);
  }, []);

  const renderActiveView = () => {
    switch (activeArea) {
      case "home":
        return (
          <HomeView
            onOpenProject={handleOpenProjectFromHome}
            onNavigateSettings={() => handleNavigateSettings("engines")}
            onLoginSuccess={loadWorkspaceAndCore}
          />
        );
      case "projects":
        return (
          <ProjectView
            initialSlug={activeProjectSlug}
            initialThreadId={activeThreadId}
            onNavigateSettings={handleNavigateSettings}
          />
        );
      case "brand":
        return <BrandView />;
      case "library":
        return <LibraryView />;
      case "settings":
        return <SettingsView initialTab={settingsTab} />;
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
      {/* 1. Title bar */}
      <TitleBar
        workspaceName={currentWorkspace?.name}
        projectName={activeArea === "projects" ? activeProjectSlug : undefined}
        activeViewLabel={getAreaLabel()}
        runningJobsCount={runningJobsCount}
        theme={theme}
        onToggleTheme={toggleTheme}
        onOpenWorkspaceSwitcher={() => setWorkspaceSwitcherOpen(true)}
        onOpenCommandPalette={() => setCmdOpen(true)}
        onToggleTaskCenter={() => setTaskCenterOpen((prev) => !prev)}
        onNavigateHome={() => setActiveArea("home")}
        onNavigateProjects={() => setActiveArea("projects")}
      />

      {/* 2. Middle Body: Left Rail + Main Active Area View */}
      <div
        style={{
          flex: 1,
          display: "flex",
          minHeight: 0,
          position: "relative",
          overflow: "hidden",
        }}
      >
        <LeftRail
          activeArea={activeArea}
          onChangeArea={setActiveArea}
          runningJobsCount={runningJobsCount}
        />

        <main
          style={{
            flex: 1,
            display: "flex",
            flexDirection: "column",
            minWidth: 0,
            overflow: "hidden",
            backgroundColor: "var(--cs-bg-app)",
          }}
        >
          {renderActiveView()}
        </main>

        {/* Task Center Drawer (Ctrl+J) */}
        <TaskCenter
          open={taskCenterOpen}
          onClose={() => setTaskCenterOpen(false)}
          jobs={jobs}
        />
      </div>

      {/* 3. Status bar */}
      <StatusBar
        workspacePath={currentWorkspace?.path}
        engines={engines}
        jobs={jobs}
        diskUsage={diskUsage}
        onOpenTaskCenter={() => setTaskCenterOpen(true)}
        onOpenSettings={() => setActiveArea("settings")}
      />

      {/* Command Palette Modal (Ctrl+K) */}
      <CommandPalette
        open={cmdOpen}
        onClose={() => setCmdOpen(false)}
        commands={commands}
      />

      {/* Workspace Switcher Modal */}
      <WorkspaceSwitcherModal
        open={workspaceSwitcherOpen}
        onClose={() => setWorkspaceSwitcherOpen(false)}
        currentWorkspace={currentWorkspace}
        recentWorkspaces={recentWorkspaces}
        onOpenWorkspace={handleOpenWorkspace}
        onCreateWorkspace={handleCreateWorkspace}
      />

      {/* Web Mode Auth & Sudo Dialogs */}
      <WebAuthModal onLoginSuccess={loadWorkspaceAndCore} />
    </div>
  );
};
