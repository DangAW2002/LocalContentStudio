import React from "react";
import { applyTheme } from "@studio/design-system";
import { TitleBar } from "./components/TitleBar";
import { LeftRail, ShellArea } from "./components/LeftRail";
import { StatusBar } from "./components/StatusBar";
import { TaskCenter } from "./components/TaskCenter";
import { CommandPalette } from "./components/CommandPalette";
import { WorkspaceSwitcherModal } from "./components/WorkspaceSwitcherModal";
import { HomeView } from "./views/HomeView";
import { ProjectView } from "./views/ProjectView";
import { BrandView } from "./views/BrandView";
import { LibraryView } from "./views/LibraryView";
import { SettingsView } from "./views/SettingsView";
import { WebAuthModal } from "./components/WebAuthModal";
import { studioApi } from "./api/client";
import {
  globalCommandRegistry,
  type CommandDefinition,
  type CommandContext,
} from "./runtime/commands";
import type {
  WorkspaceSummary,
  EngineProbeResult,
  JobSnapshot,
  JobState,
  DiskUsageReport,
  ProjectFileEntry,
  ThreadSummary,
  ProjectSummary,
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

  // Context for files and threads
  const [projectList, setProjectList] = React.useState<ProjectSummary[]>([]);
  const [projectFiles, setProjectFiles] = React.useState<ProjectFileEntry[]>([]);
  const [threads, setThreads] = React.useState<ThreadSummary[]>([]);
  const [hasSelectedArtifact, setHasSelectedArtifact] = React.useState(false);

  // Settings tab state
  const [settingsTab, setSettingsTab] = React.useState<
    "engines" | "tools" | "voices" | "env" | "catalog" | "jobs" | "disk" | "api-keys" | "accounts" | "web"
  >("engines");

  // Theme toggle
  const toggleTheme = React.useCallback(() => {
    setTheme((prev) => {
      const next = prev === "light" ? "dark" : "light";
      applyTheme(next);
      return next;
    });
  }, []);

  const handleNavigateSettings = React.useCallback((tab?: string) => {
    if (tab) {
      setSettingsTab(tab as any);
    }
    setActiveArea("settings");
  }, []);

  // Initial data loading
  const loadWorkspaceAndCore = React.useCallback(async () => {
    try {
      const [currWs, recents, engs, jobList, disk, projs] = await Promise.all([
        studioApi.getCurrentWorkspace().catch(() => null),
        studioApi.getRecentWorkspaces().catch(() => []),
        studioApi.probeEngines().catch(() => []),
        studioApi.getJobsSnapshot().catch(() => []),
        studioApi.getDiskUsage().catch(() => undefined),
        studioApi.getProjects().catch(() => []),
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
      setProjectList(projs);
      if (!activeProjectSlug && projs.length > 0) {
        setActiveProjectSlug(projs[0].slug);
      }
    } catch {
      // Disconnected state
    }
  }, [activeProjectSlug]);

  React.useEffect(() => {
    loadWorkspaceAndCore();
  }, [loadWorkspaceAndCore]);

  // Load project files and threads when activeProjectSlug changes
  React.useEffect(() => {
    if (!activeProjectSlug) return;
    let cancelled = false;
    Promise.all([
      studioApi.getProjectFiles(activeProjectSlug).catch(() => []),
      studioApi.getThreads(activeProjectSlug).catch(() => []),
      studioApi.listArtifacts(activeProjectSlug).catch(() => []),
    ]).then(([files, threadList, arts]) => {
      if (cancelled) return;
      setProjectFiles(files);
      setThreads(threadList);
      setHasSelectedArtifact(arts.length > 0);
    });
    return () => {
      cancelled = true;
    };
  }, [activeProjectSlug]);

  const [hasUndoableTurn, setHasUndoableTurn] = React.useState(false);

  React.useEffect(() => {
    const handleThreadState = (e: Event) => {
      const custom = e as CustomEvent<{ hasUndoableTurn?: boolean }>;
      if (typeof custom.detail?.hasUndoableTurn === "boolean") {
        setHasUndoableTurn(custom.detail.hasUndoableTurn);
      }
    };
    window.addEventListener("studio:thread-state", handleThreadState);
    return () => window.removeEventListener("studio:thread-state", handleThreadState);
  }, []);

  React.useEffect(() => {
    const handleOpenSwitcher = () => setWorkspaceSwitcherOpen(true);
    window.addEventListener("studio:open-workspace-switcher", handleOpenSwitcher);
    return () => window.removeEventListener("studio:open-workspace-switcher", handleOpenSwitcher);
  }, []);

  // Listen to custom navigation events
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

  // Command Context
  const commandContext: CommandContext = React.useMemo(() => {
    return {
      activeArea,
      currentSlug: activeProjectSlug,
      activeThreadId,
      hasSelectedArtifact,
      hasRunningTurn: jobs.some((j) => j.state === "RUNNING"),
      hasUndoableTurn,
    };
  }, [activeArea, activeProjectSlug, activeThreadId, hasSelectedArtifact, jobs, hasUndoableTurn]);

  // Register all system commands into globalCommandRegistry
  React.useEffect(() => {
    const definitions: CommandDefinition[] = [
      // Navigation
      {
        id: "nav.home",
        title: "Về trang chủ (Home)",
        category: "Điều hướng",
        icon: "home",
        keybinding: "Ctrl+1",
        run: () => setActiveArea("home"),
      },
      {
        id: "nav.projects",
        title: "Mở Dự án (Projects)",
        category: "Điều hướng",
        icon: "projects",
        keybinding: "Ctrl+2",
        run: () => setActiveArea("projects"),
      },
      {
        id: "nav.brand",
        title: "Mở Brand, Channel & Playbook",
        category: "Điều hướng",
        icon: "brand",
        keybinding: "Ctrl+3",
        run: () => setActiveArea("brand"),
      },
      {
        id: "nav.library",
        title: "Mở Thư viện tài nguyên (Library)",
        category: "Điều hướng",
        icon: "library",
        keybinding: "Ctrl+4",
        run: () => setActiveArea("library"),
      },
      {
        id: "nav.settings",
        title: "Mở Tools & Cài đặt hệ thống",
        category: "Điều hướng",
        icon: "settings",
        keybinding: "Ctrl+5",
        run: () => setActiveArea("settings"),
      },
      // Settings Tabs
      {
        id: "settings.engines",
        title: "Cài đặt: Engines (Claude, agy)",
        category: "Cài đặt",
        icon: "settings",
        run: () => handleNavigateSettings("engines"),
      },
      {
        id: "settings.tools",
        title: "Cài đặt: Công cụ ngoài (Tools)",
        category: "Cài đặt",
        icon: "settings",
        run: () => handleNavigateSettings("tools"),
      },
      {
        id: "settings.voices",
        title: "Cài đặt: Giọng đọc (TTS)",
        category: "Cài đặt",
        icon: "mic",
        run: () => handleNavigateSettings("voices"),
      },
      {
        id: "settings.accounts",
        title: "Cài đặt: Tài khoản đăng bài",
        category: "Cài đặt",
        icon: "share-2",
        run: () => handleNavigateSettings("accounts"),
      },
      {
        id: "settings.env",
        title: "Cài đặt: Kiểm tra môi trường",
        category: "Cài đặt",
        icon: "check-circle",
        run: () => handleNavigateSettings("env"),
      },
      {
        id: "settings.api-keys",
        title: "Cài đặt: Khoá API & Endpoints",
        category: "Cài đặt",
        icon: "key",
        run: () => handleNavigateSettings("api-keys"),
      },
      {
        id: "settings.catalog",
        title: "Cài đặt: Catalog Modules & Kits",
        category: "Cài đặt",
        icon: "package",
        run: () => handleNavigateSettings("catalog"),
      },
      {
        id: "settings.jobs",
        title: "Cài đặt: Cấu hình tác vụ",
        category: "Cài đặt",
        icon: "activity",
        run: () => handleNavigateSettings("jobs"),
      },
      {
        id: "settings.disk",
        title: "Cài đặt: Dung lượng đĩa",
        category: "Cài đặt",
        icon: "hard-drive",
        run: () => handleNavigateSettings("disk"),
      },
      {
        id: "settings.web",
        title: "Cài đặt: Truy cập Web cá nhân",
        category: "Cài đặt",
        icon: "globe",
        run: () => handleNavigateSettings("web"),
      },
      // New conversation threads
      {
        id: "thread.new.claude",
        title: "Hội thoại mới với Claude Code CLI",
        category: "Hội thoại",
        icon: "message-square",
        run: async () => {
          if (!activeProjectSlug) setActiveArea("projects");
          window.dispatchEvent(new CustomEvent("studio:create-thread", { detail: { engine: "claude" } }));
        },
      },
      {
        id: "thread.new.agy",
        title: "Hội thoại mới với Antigravity CLI (agy)",
        category: "Hội thoại",
        icon: "message-square",
        run: async () => {
          if (!activeProjectSlug) setActiveArea("projects");
          window.dispatchEvent(new CustomEvent("studio:create-thread", { detail: { engine: "agy" } }));
        },
      },
      // Layout & Panes
      {
        id: "pane.thread.toggle",
        title: "Ẩn / Hiện khung hội thoại",
        category: "Giao diện",
        icon: "panel-left",
        keybinding: "Ctrl+B",
        run: () => {
          window.dispatchEvent(new CustomEvent("studio:toggle-pane", { detail: { pane: "thread" } }));
        },
      },
      {
        id: "pane.outputs.toggle",
        title: "Ẩn / Hiện khung Outputs & Checkpoints",
        category: "Giao diện",
        icon: "panel-right",
        keybinding: "Ctrl+Alt+B",
        run: () => {
          window.dispatchEvent(new CustomEvent("studio:toggle-pane", { detail: { pane: "outputs" } }));
        },
      },
      // Theme & Interface
      {
        id: "theme.toggle",
        title: `Chuyển sang giao diện ${theme === "light" ? "Tối (Dark)" : "Sáng (Light)"}`,
        category: "Giao diện",
        icon: theme === "light" ? "moon" : "sun",
        run: toggleTheme,
      },
      // Task Center
      {
        id: "taskcenter.toggle",
        title: "Mở / Đóng Trung tâm Tác vụ (Task Center)",
        category: "Thao tác",
        icon: "activity",
        keybinding: "Ctrl+J",
        run: () => setTaskCenterOpen((prev) => !prev),
      },
      // Workspace Actions
      {
        id: "workspace.switch",
        title: "Chuyển hoặc tạo Workspace mới...",
        category: "Workspace",
        icon: "folder",
        run: () => setWorkspaceSwitcherOpen(true),
      },
      {
        id: "workspace.open_folder",
        title: "Mở thư mục Workspace",
        category: "Workspace",
        icon: "folder",
        run: () => {
          setWorkspaceSwitcherOpen(true);
        },
      },
      // Stage & Artifact Operations
      {
        id: "stage.save",
        title: "Lưu thay đổi tệp trên Stage",
        category: "Biên tập",
        icon: "save",
        keybinding: "Ctrl+S",
        run: () => {
          window.dispatchEvent(new CustomEvent("studio:stage-save"));
        },
      },
      {
        id: "artifact.render",
        title: "Render Artifact đang chọn",
        category: "Artifact",
        icon: "play",
        when: (ctx) => Boolean(ctx.hasSelectedArtifact),
        run: () => {
          window.dispatchEvent(new CustomEvent("studio:render-artifact"));
        },
      },
      {
        id: "artifact.check",
        title: "Kiểm tra Tiêu chuẩn Brand cho Artifact",
        category: "Artifact",
        icon: "check-circle",
        when: (ctx) => Boolean(ctx.hasSelectedArtifact),
        run: () => {
          window.dispatchEvent(new CustomEvent("studio:check-artifact"));
        },
      },
      {
        id: "thread.undo_turn",
        title: "Hoàn tác lượt agent gần nhất...",
        category: "Hội thoại",
        icon: "rotate-ccw",
        when: (ctx) => Boolean(ctx.hasUndoableTurn),
        run: () => {
          window.dispatchEvent(new CustomEvent("studio:undo-last-turn"));
        },
      },
      // Shortcuts listing
      {
        id: "help.shortcuts",
        title: "Danh sách phím tắt hệ thống",
        category: "Trợ giúp",
        icon: "command",
        run: () => {
          setCmdOpen(true);
        },
      },
    ];

    return globalCommandRegistry.registerMany(definitions);
  }, [activeArea, activeProjectSlug, theme, toggleTheme, handleNavigateSettings]);

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

  const handleSelectFileFromPalette = (filePath: string) => {
    setActiveArea("projects");
    window.dispatchEvent(new CustomEvent("studio:select-file", { detail: { path: filePath } }));
  };

  const handleSelectThreadFromPalette = (threadId: string) => {
    setActiveArea("projects");
    setActiveThreadId(threadId);
    window.dispatchEvent(new CustomEvent("studio:select-thread", { detail: { threadId } }));
  };

  const handleUseInThread = React.useCallback(
    (assetPath: string, projectSlug?: string) => {
      const target = projectSlug || activeProjectSlug || projectList[0]?.slug;
      if (target) {
        setActiveProjectSlug(target);
      }
      setActiveArea("projects");
      setTimeout(() => {
        window.dispatchEvent(
          new CustomEvent("studio:ask-ai", { detail: { text: `@${assetPath} ` } })
        );
      }, 50);
    },
    [activeProjectSlug, projectList]
  );

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
        return <LibraryView onUseInThread={handleUseInThread} />;
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
        onOpenSettings={handleNavigateSettings}
        onOpenWorkspaceFolder={() => setWorkspaceSwitcherOpen(true)}
      />

      {/* Command Palette Modal (Ctrl+K) */}
      <CommandPalette
        open={cmdOpen}
        onClose={() => setCmdOpen(false)}
        context={commandContext}
        projectFiles={projectFiles}
        threads={threads}
        onSelectFile={handleSelectFileFromPalette}
        onSelectThread={handleSelectThreadFromPalette}
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
