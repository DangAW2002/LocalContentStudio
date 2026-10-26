import React from "react";
import { PaneBoundary } from "../components/PaneBoundary";
import { ThreadPane } from "./project/ThreadPane";
import { StagePane } from "./project/StagePane";
import { OutputsPane } from "./project/OutputsPane";
import { GraphPane } from "./project/graph/GraphPane";
import { CreateEpisodeDialog } from "./project/CreateEpisodeDialog";
import { Tabs, Button, Select, ErrorState } from "@studio/design-system";
import { studioApi } from "../api/client";
import { Icon } from "../components/Icon";
import { Sash } from "../components/Sash";
import { useDismiss } from "../components/useDismiss";
import {
  DEFAULT_PROJECT_LAYOUT,
  dragPane,
  loadProjectLayout,
  paneMax,
  saveProjectLayout,
  stepPane,
  type PaneState,
  type ProjectLayout,
} from "../runtime/layout/split";
import type {
  EngineType,
  ProjectSummary,
  ProjectFileEntry,
  ThreadSummary,
  ScopeItem,
  ArtifactSummary,
} from "../api/types";

const PANE_LIMITS = { thread: { min: 280, max: 720 }, outputs: { min: 240, max: 560 } } as const;
const CENTER_MIN = 320;
const NEW_THREAD_ENGINES = [
  ["claude", "Claude Code CLI"],
  ["agy", "Antigravity CLI (agy)"],
] as const;

export interface ProjectViewProps {
  initialSlug?: string;
  initialThreadId?: string;
  onNavigateSettings?: (tab?: string) => void;
}

export const ProjectView: React.FC<ProjectViewProps> = ({
  initialSlug,
  initialThreadId,
  onNavigateSettings,
}) => {
  const [projects, setProjects] = React.useState<ProjectSummary[]>([]);
  const [currentSlug, setCurrentSlug] = React.useState<string>(initialSlug || "");
  const [threads, setThreads] = React.useState<ThreadSummary[]>([]);
  const [activeThreadId, setActiveThreadId] = React.useState<string>(initialThreadId || "");
  const [projectFiles, setProjectFiles] = React.useState<ProjectFileEntry[]>([]);
  const [playbooks, setPlaybooks] = React.useState<ScopeItem[]>([]);
  const [artifacts, setArtifacts] = React.useState<ArtifactSummary[]>([]);
  const [selectedFilePath, setSelectedFilePath] = React.useState<string | null>(null);
  const [pendingInsert, setPendingInsert] = React.useState<string | null>(null);
  const [stageReloadNonce, setStageReloadNonce] = React.useState(0);
  const [createEpisodeOpen, setCreateEpisodeOpen] = React.useState(false);

  const projectOptions = React.useMemo(() => {
    if (projects.length === 0) return [{ value: "", label: "Chưa có dự án" }];
    const withSeries = projects.filter((p) => Boolean(p.series));
    const withoutSeries = projects.filter((p) => !p.series);

    const groups = new Map<string, ProjectSummary[]>();
    for (const p of withSeries) {
      const s = p.series!;
      if (!groups.has(s)) groups.set(s, []);
      groups.get(s)!.push(p);
    }

    const opts: { value: string; label: string }[] = [];
    for (const [s, sProjects] of groups.entries()) {
      for (const p of sProjects) {
        opts.push({
          value: p.slug,
          label: `[Series: ${s}] ${p.title}`,
        });
      }
    }

    for (const p of withoutSeries) {
      opts.push({
        value: p.slug,
        label: p.title,
      });
    }

    return opts;
  }, [projects]);

  const refreshArtifacts = React.useCallback(async () => {
    if (!currentSlug) return [];
    try {
      const [artList, fileList] = await Promise.all([
        studioApi.listArtifacts(currentSlug).catch(() => []),
        studioApi.getProjectFiles(currentSlug).catch(() => []),
      ]);
      setArtifacts(artList);
      setProjectFiles(fileList);
      return artList;
    } catch {
      return [];
    }
  }, [currentSlug]);

  // Side panes: VS Code split-view behaviour (drag, snap to hide, double-click reset), persisted.
  const [layout, setLayoutState] = React.useState<ProjectLayout>(loadProjectLayout);
  const setLayout = React.useCallback((update: (prev: ProjectLayout) => ProjectLayout) => {
    setLayoutState((prev) => {
      const next = update(prev);
      saveProjectLayout(next);
      return next;
    });
  }, []);
  const panesRef = React.useRef<HTMLDivElement>(null);
  const dragStartRef = React.useRef<ProjectLayout>(layout);
  const [centerTab, setCenterTab] = React.useState<"stage" | "graph">("stage");
  const [narrowTab, setNarrowTab] = React.useState<"thread" | "stage" | "graph" | "outputs">("thread");
  const [isNarrow, setIsNarrow] = React.useState(false);
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);

  const handleAskAi = React.useCallback((text: string) => {
    setPendingInsert(text);
    if (isNarrow) {
      setNarrowTab("thread");
    }
  }, [isNarrow]);

  // Track window width for responsive auto-collapse and tabs
  React.useEffect(() => {
    let prevWidth = typeof window !== "undefined" ? window.innerWidth : 1200;

    const handleResize = () => {
      const w = window.innerWidth;
      setIsNarrow(w < 768);

      // Under ~1100px: auto-collapse Outputs if crossing down
      if (w < 1100 && prevWidth >= 1100) {
        setLayout((prev) => (prev.outputs.visible ? { ...prev, outputs: { ...prev.outputs, visible: false } } : prev));
      }
      // Under ~900px: auto-collapse Thread if crossing down
      if (w < 900 && prevWidth >= 900) {
        setLayout((prev) => (prev.thread.visible ? { ...prev, thread: { ...prev.thread, visible: false } } : prev));
      }

      prevWidth = w;
    };

    // Run initial auto-collapse checks on mount
    const initW = window.innerWidth;
    if (initW < 1100) {
      setLayout((prev) => (prev.outputs.visible ? { ...prev, outputs: { ...prev.outputs, visible: false } } : prev));
    }
    if (initW < 900) {
      setLayout((prev) => (prev.thread.visible ? { ...prev, thread: { ...prev.thread, visible: false } } : prev));
    }
    setIsNarrow(initW < 768);

    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, [setLayout]);

  // Load project list
  const loadProjects = React.useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const list = await studioApi.getProjects();
      setProjects(list);
      if (!currentSlug && list.length > 0) {
        setCurrentSlug(list[0].slug);
      }
    } catch (err: any) {
      setError(err?.message || "Chưa kết nối Studio core");
    } finally {
      setLoading(false);
    }
  }, [currentSlug]);

  React.useEffect(() => {
    loadProjects();
  }, [loadProjects]);

  // Load active project details (threads, files, playbooks)
  React.useEffect(() => {
    if (!currentSlug) return;

    let isMounted = true;
    const loadProjectData = async () => {
      try {
        const [threadList, fileList, pbList, artList] = await Promise.all([
          studioApi.getThreads(currentSlug).catch(() => []),
          studioApi.getProjectFiles(currentSlug).catch(() => []),
          studioApi.getScopeItems("playbooks").catch(() => []),
          studioApi.listArtifacts(currentSlug).catch(() => []),
        ]);

        if (isMounted) {
          setThreads(threadList);
          setProjectFiles(fileList);
          setPlaybooks(pbList);
          setArtifacts(artList);

          if (!activeThreadId && threadList.length > 0) {
            setActiveThreadId(threadList[0].id);
          }
          // Auto-select a main source (e.g. index.html or script.md); never a directory.
          const files = fileList.filter((f) => !f.isDir);
          if (files.length > 0 && !selectedFilePath) {
            const mainFile = files.find((f) => f.name.includes("index") || f.name.includes("main")) || files[0];
            setSelectedFilePath(mainFile.path);
          }
        }
      } catch (err: any) {
        if (isMounted) setError(err?.message || "Không thể tải dữ liệu dự án.");
      }
    };

    loadProjectData();
    return () => {
      isMounted = false;
    };
  }, [currentSlug]);

  // Auto-titled threads: patch the switcher label without reloading the project.
  React.useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    studioApi
      .onThreadUpdated(({ threadId, title }) =>
        setThreads((prev) => prev.map((t) => (t.id === threadId ? { ...t, title } : t))),
      )
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => {});
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const activeThread = threads.find((t) => t.id === activeThreadId) || threads[0];

  const handleCreateThread = async (engine: EngineType) => {
    if (!currentSlug) return;
    setNewThreadMenuOpen(false);
    try {
      // Same per-engine model choice the Home screen remembers.
      const model = localStorage.getItem(`lcs.model.${engine}`) || undefined;
      const newThread = await studioApi.createThread({
        project: currentSlug,
        engine,
        model,
      });
      setThreads((prev) => [newThread, ...prev]);
      setActiveThreadId(newThread.id);
    } catch (err: any) {
      setError(err?.message || "Không thể tạo hội thoại mới.");
    }
  };

  const [newThreadMenuOpen, setNewThreadMenuOpen] = React.useState(false);
  const newThreadButtonRef = React.useRef<HTMLDivElement>(null);
  const newThreadMenuRef = React.useRef<HTMLDivElement>(null);
  useDismiss(newThreadMenuOpen, () => setNewThreadMenuOpen(false), [newThreadButtonRef, newThreadMenuRef]);

  const togglePane = React.useCallback(
    (pane: keyof ProjectLayout) =>
      setLayout((prev) => ({ ...prev, [pane]: { ...prev[pane], visible: !prev[pane].visible } })),
    [setLayout],
  );

  // VS Code shortcuts: Ctrl+B primary side bar (thread), Ctrl+Alt+B secondary side bar (outputs).
  React.useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey) || e.shiftKey || e.key.toLowerCase() !== "b") return;
      e.preventDefault();
      togglePane(e.altKey ? "outputs" : "thread");
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [togglePane]);

  // Listen to palette commands and navigation events
  React.useEffect(() => {
    const handleTogglePaneEvent = (e: Event) => {
      const custom = e as CustomEvent<{ pane: "thread" | "outputs" }>;
      if (custom.detail?.pane) {
        togglePane(custom.detail.pane);
      }
    };
    const handleCreateThreadEvent = (e: Event) => {
      const custom = e as CustomEvent<{ engine: EngineType }>;
      if (custom.detail?.engine) {
        handleCreateThread(custom.detail.engine);
      }
    };
    const handleSelectFileEvent = (e: Event) => {
      const custom = e as CustomEvent<{ path: string }>;
      if (custom.detail?.path) {
        setSelectedFilePath(custom.detail.path);
        if (isNarrow) setNarrowTab("stage");
      }
    };
    const handleSelectThreadEvent = (e: Event) => {
      const custom = e as CustomEvent<{ threadId: string }>;
      if (custom.detail?.threadId) {
        setActiveThreadId(custom.detail.threadId);
        if (isNarrow) setNarrowTab("thread");
      }
    };
    const handleAskAiEvent = (e: Event) => {
      const custom = e as CustomEvent<{ text: string }>;
      if (custom.detail?.text) {
        handleAskAi(custom.detail.text);
      }
    };

    window.addEventListener("studio:toggle-pane", handleTogglePaneEvent);
    window.addEventListener("studio:create-thread", handleCreateThreadEvent);
    window.addEventListener("studio:select-file", handleSelectFileEvent);
    window.addEventListener("studio:select-thread", handleSelectThreadEvent);
    window.addEventListener("studio:ask-ai", handleAskAiEvent);

    return () => {
      window.removeEventListener("studio:toggle-pane", handleTogglePaneEvent);
      window.removeEventListener("studio:create-thread", handleCreateThreadEvent);
      window.removeEventListener("studio:select-file", handleSelectFileEvent);
      window.removeEventListener("studio:select-thread", handleSelectThreadEvent);
      window.removeEventListener("studio:ask-ai", handleAskAiEvent);
    };
  }, [togglePane, handleCreateThread, isNarrow, handleAskAi]);

  const limitsFor = (pane: keyof ProjectLayout, other: PaneState) => {
    const width = panesRef.current?.clientWidth ?? window.innerWidth;
    const otherSize = other.visible ? other.size : 0;
    return { min: PANE_LIMITS[pane].min, max: paneMax(PANE_LIMITS[pane].max, width, otherSize, CENTER_MIN) };
  };
  const otherOf = (l: ProjectLayout, pane: keyof ProjectLayout) => (pane === "thread" ? l.outputs : l.thread);
  // The thread pane grows to the right of its sash; the outputs pane grows to the left.
  const direction = (pane: keyof ProjectLayout) => (pane === "thread" ? 1 : -1);
  const sashHandlers = (pane: keyof ProjectLayout, label: string) => ({
    label,
    onDragStart: () => {
      dragStartRef.current = layout;
    },
    onDrag: (dx: number) => {
      const start = dragStartRef.current;
      const startPane = start[pane];
      const startSize = startPane.visible ? startPane.size : 0;
      const next = dragPane(startPane, startSize + direction(pane) * dx, limitsFor(pane, otherOf(start, pane)));
      setLayout((prev) => ({ ...prev, [pane]: next }));
    },
    onReset: () => setLayout((prev) => ({ ...prev, [pane]: DEFAULT_PROJECT_LAYOUT[pane] })),
    onStep: (dx: number) =>
      setLayout((prev) => ({
        ...prev,
        [pane]: stepPane(prev[pane], direction(pane) * dx, limitsFor(pane, otherOf(prev, pane))),
      })),
  });

  const threadLabel = (t: ThreadSummary, index: number) => {
    const title = t.title?.trim() || `Hội thoại ${threads.length - index}`;
    return `${title} · ${t.engine === "claude" ? "Claude" : "agy"}`;
  };

  if (error) {
    return (
      <div style={{ padding: "var(--cs-space-6)", width: "100%", maxWidth: 800, margin: "0 auto" }}>
        <ErrorState
          title="Kết nối dự án"
          reason={error}
          safetyNote="Giao diện desktop cần kết nối với Studio Core backend để đọc và đồng bộ dự án."
          onRetry={loadProjects}
        />
      </div>
    );
  }

  if (loading) {
    return (
      <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", color: "var(--cs-fg-muted)" }}>
        Đang tải thông tin dự án...
      </div>
    );
  }

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        width: "100%",
        overflow: "hidden",
      }}
    >
      {/* Project Bar Header */}
      <div
        style={{
          // Narrow screens wrap the bar instead of pushing the tabs off-screen.
          minHeight: 40,
          display: "flex",
          flexWrap: isNarrow ? "wrap" : "nowrap",
          alignItems: "center",
          justifyContent: "space-between",
          gap: isNarrow ? "var(--cs-space-2)" : undefined,
          padding: isNarrow ? "var(--cs-space-2) var(--cs-space-3)" : "0 var(--cs-space-4)",
          backgroundColor: "var(--cs-bg-panel)",
          borderBottom: "1px solid var(--cs-border-subtle)",
          flexShrink: 0,
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: isNarrow ? "var(--cs-space-2)" : "var(--cs-space-3)",
            flexWrap: isNarrow ? "wrap" : "nowrap",
            minWidth: 0,
            width: isNarrow ? "100%" : undefined,
          }}
        >
          <div style={isNarrow ? { flex: "1 1 140px", minWidth: 0 } : { width: 220 }}>
            <Select
              size="sm"
              value={currentSlug}
              onChange={(e) => {
                setCurrentSlug(e.target.value);
                setActiveThreadId("");
                setSelectedFilePath(null);
              }}
              options={projectOptions}
            />
          </div>

          <Button
            size="sm"
            variant="secondary"
            onClick={() => setCreateEpisodeOpen(true)}
            leftIcon={<Icon name="folder" size={12} />}
          >
            + Tập từ series
          </Button>

          {threads.length > 0 && (
            <div style={isNarrow ? { flex: "1 1 140px", minWidth: 0 } : { width: 240 }}>
              <Select
                size="sm"
                aria-label="Chọn hội thoại"
                value={activeThread?.id ?? ""}
                onChange={(e) => setActiveThreadId(e.target.value)}
                options={threads.map((t, i) => ({ value: t.id, label: threadLabel(t, i) }))}
              />
            </div>
          )}

          <div ref={newThreadButtonRef} style={{ position: "relative" }}>
            <Button
              size="sm"
              variant="quiet"
              aria-haspopup="menu"
              aria-expanded={newThreadMenuOpen}
              onClick={() => setNewThreadMenuOpen((o) => !o)}
            >
              + Hội thoại mới ▾
            </Button>
            {newThreadMenuOpen && (
              <div ref={newThreadMenuRef} role="menu" className="cs-menu" style={{ top: "calc(100% + 4px)", left: 0 }}>
                {NEW_THREAD_ENGINES.map(([engine, label]) => (
                  <button
                    key={engine}
                    type="button"
                    role="menuitem"
                    className="cs-menu-item"
                    onClick={() => handleCreateThread(engine)}
                  >
                    {label}
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>

        {!isNarrow && (
          <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-3)" }}>
            <Tabs
              items={[
                { id: "stage", label: "Stage" },
                { id: "graph", label: "Sơ đồ" },
              ]}
              activeId={centerTab}
              onChange={(id) => setCenterTab(id as any)}
              size="sm"
            />
            <div style={{ display: "flex", alignItems: "center", gap: 2 }}>
              <button
                type="button"
                className="cs-icon-btn"
                aria-pressed={layout.thread.visible}
                aria-label={(layout.thread.visible ? "Ẩn" : "Hiện") + " khung hội thoại (Ctrl+B)"}
                title={(layout.thread.visible ? "Ẩn" : "Hiện") + " khung hội thoại (Ctrl+B)"}
                onClick={() => togglePane("thread")}
              >
                <Icon name="panel-left" size={16} />
              </button>
              <button
                type="button"
                className="cs-icon-btn"
                aria-pressed={layout.outputs.visible}
                aria-label={(layout.outputs.visible ? "Ẩn" : "Hiện") + " khung Outputs (Ctrl+Alt+B)"}
                title={(layout.outputs.visible ? "Ẩn" : "Hiện") + " khung Outputs (Ctrl+Alt+B)"}
                onClick={() => togglePane("outputs")}
              >
                <Icon name="panel-right" size={16} />
              </button>
            </div>
          </div>
        )}

        {/* Tab switcher for narrow screens */}
        {isNarrow && (
          <Tabs
            items={[
              { id: "thread", label: "Thread" },
              { id: "stage", label: "Stage" },
              { id: "graph", label: "Sơ đồ" },
              { id: "outputs", label: "Outputs" },
            ]}
            activeId={narrowTab}
            onChange={(id) => setNarrowTab(id as any)}
            size="sm"
          />
        )}
      </div>

      {/* Three Panes Container */}
      <div
        ref={panesRef}
        style={{
          flex: 1,
          display: "flex",
          minHeight: 0,
          overflow: "hidden",
          position: "relative",
        }}
      >
        {isNarrow ? (
          // Single tab visible on narrow screens
          <div style={{ flex: 1, height: "100%", overflow: "hidden" }}>
            {narrowTab === "thread" && (
              <PaneBoundary name="Hội thoại">
                <ThreadPane
                  projectSlug={currentSlug}
                  activeThread={activeThread}
                  projectFiles={projectFiles}
                  playbooks={playbooks}
                  pendingInsert={pendingInsert}
                  onInsertConsumed={() => setPendingInsert(null)}
                  onSelectFile={(path) => {
                    setSelectedFilePath(path);
                    setNarrowTab("stage");
                  }}
                  onNavigateSettings={onNavigateSettings}
                  onUndoTurnSuccess={() => setStageReloadNonce((n) => n + 1)}
                />
              </PaneBoundary>
            )}
            {narrowTab === "stage" && (
              <PaneBoundary name="Stage">
                <StagePane
                  projectSlug={currentSlug}
                  selectedFilePath={selectedFilePath}
                  onClearSelection={() => setSelectedFilePath(null)}
                  onAskAi={handleAskAi}
                  onSelectFile={setSelectedFilePath}
                  reloadNonce={stageReloadNonce}
                  artifacts={artifacts}
                  onRefreshArtifacts={refreshArtifacts}
                />
              </PaneBoundary>
            )}
            {narrowTab === "graph" && (
              <PaneBoundary name="Sơ đồ">
                <GraphPane
                  projectSlug={currentSlug}
                  onSendToAgent={(refs) => {
                    const mentionString = refs.map((r) => `@${r}`).join(" ") + " ";
                    handleAskAi(mentionString);
                    setNarrowTab("thread");
                  }}
                  onOpenArtifact={(artId) => {
                    const art = artifacts.find((a) => a.id === artId);
                    if (art) {
                      setSelectedFilePath(art.source);
                      setNarrowTab("stage");
                    }
                  }}
                />
              </PaneBoundary>
            )}
            {narrowTab === "outputs" && (
              <PaneBoundary name="Outputs">
                <OutputsPane
                  projectSlug={currentSlug}
                  projectFiles={projectFiles}
                  artifacts={artifacts}
                  onRefreshArtifacts={refreshArtifacts}
                  onSelectFile={(path) => {
                    setSelectedFilePath(path);
                    setNarrowTab("stage");
                  }}
                  onAskAgentToFix={handleAskAi}
                />
              </PaneBoundary>
            )}
          </div>
        ) : (
          // Three resizable panes side-by-side
          <>
            {/* Thread Pane */}
            <div
              style={{
                width: layout.thread.size,
                height: "100%",
                flexShrink: 0,
                display: layout.thread.visible ? undefined : "none",
              }}
            >
              <PaneBoundary name="Hội thoại">
                <ThreadPane
                  projectSlug={currentSlug}
                  activeThread={activeThread}
                  projectFiles={projectFiles}
                  playbooks={playbooks}
                  pendingInsert={pendingInsert}
                  onInsertConsumed={() => setPendingInsert(null)}
                  onSelectFile={setSelectedFilePath}
                  onNavigateSettings={onNavigateSettings}
                  onUndoTurnSuccess={() => setStageReloadNonce((n) => n + 1)}
                />
              </PaneBoundary>
            </div>

            <Sash {...sashHandlers("thread", "Đổi độ rộng khung hội thoại")} />

            {/* Stage / Graph Center Pane */}
            <div style={{ flex: 1, height: "100%", minWidth: CENTER_MIN, overflow: "hidden" }}>
              {centerTab === "stage" ? (
                <PaneBoundary name="Stage">
                  <StagePane
                    projectSlug={currentSlug}
                    selectedFilePath={selectedFilePath}
                    onClearSelection={() => setSelectedFilePath(null)}
                    onAskAi={handleAskAi}
                    onSelectFile={setSelectedFilePath}
                    reloadNonce={stageReloadNonce}
                    artifacts={artifacts}
                    onRefreshArtifacts={refreshArtifacts}
                  />
                </PaneBoundary>
              ) : (
                <PaneBoundary name="Sơ đồ">
                  <GraphPane
                    projectSlug={currentSlug}
                    onSendToAgent={(refs) => {
                      const mentionString = refs.map((r) => `@${r}`).join(" ") + " ";
                      handleAskAi(mentionString);
                    }}
                    onOpenArtifact={(artId) => {
                      const art = artifacts.find((a) => a.id === artId);
                      if (art) {
                        setSelectedFilePath(art.source);
                        setCenterTab("stage");
                      }
                    }}
                  />
                </PaneBoundary>
              )}
            </div>

            <Sash {...sashHandlers("outputs", "Đổi độ rộng khung Outputs")} />

            {/* Outputs Pane */}
            <div
              style={{
                width: layout.outputs.size,
                height: "100%",
                flexShrink: 0,
                display: layout.outputs.visible ? undefined : "none",
              }}
            >
              <PaneBoundary name="Outputs">
                <OutputsPane
                  projectSlug={currentSlug}
                  projectFiles={projectFiles}
                  artifacts={artifacts}
                  onRefreshArtifacts={refreshArtifacts}
                  onSelectFile={setSelectedFilePath}
                  onAskAgentToFix={handleAskAi}
                />
              </PaneBoundary>
            </div>
          </>
        )}
      </div>

      <CreateEpisodeDialog
        open={createEpisodeOpen}
        onClose={() => setCreateEpisodeOpen(false)}
        defaultSeriesSlug={projects.find((p) => p.slug === currentSlug)?.series}
        onCreated={async (newProj) => {
          setCreateEpisodeOpen(false);
          await loadProjects();
          setCurrentSlug(newProj.slug);
        }}
      />
    </div>
  );
};
