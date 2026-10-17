import React from "react";
import { ThreadPane } from "./project/ThreadPane";
import { StagePane } from "./project/StagePane";
import { OutputsPane } from "./project/OutputsPane";
import { Tabs, Button, Select, ErrorState } from "@studio/design-system";
import { studioApi } from "../api/client";
import type {
  ProjectSummary,
  ProjectFileEntry,
  ThreadSummary,
  ScopeItem,
} from "../api/types";

export interface ProjectViewProps {
  initialSlug?: string;
  initialThreadId?: string;
}

export const ProjectView: React.FC<ProjectViewProps> = ({
  initialSlug,
  initialThreadId,
}) => {
  const [projects, setProjects] = React.useState<ProjectSummary[]>([]);
  const [currentSlug, setCurrentSlug] = React.useState<string>(initialSlug || "");
  const [threads, setThreads] = React.useState<ThreadSummary[]>([]);
  const [activeThreadId, setActiveThreadId] = React.useState<string>(initialThreadId || "");
  const [projectFiles, setProjectFiles] = React.useState<ProjectFileEntry[]>([]);
  const [playbooks, setPlaybooks] = React.useState<ScopeItem[]>([]);
  const [selectedFilePath, setSelectedFilePath] = React.useState<string | null>(null);
  const [pendingInsert, setPendingInsert] = React.useState<string | null>(null);

  // Layout states: resizable & narrow tab mode
  const [threadWidth, setThreadWidth] = React.useState(380);
  const [outputsWidth, setOutputsWidth] = React.useState(320);
  const [narrowTab, setNarrowTab] = React.useState<"thread" | "stage" | "outputs">("thread");
  const [isNarrow, setIsNarrow] = React.useState(false);
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);

  const handleAskAi = React.useCallback((text: string) => {
    setPendingInsert(text);
    if (isNarrow) {
      setNarrowTab("thread");
    }
  }, [isNarrow]);

  // Track window width for responsive tabs
  React.useEffect(() => {
    const handleResize = () => {
      setIsNarrow(window.innerWidth < 960);
    };
    handleResize();
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, []);

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
        const [threadList, fileList, pbList] = await Promise.all([
          studioApi.getThreads(currentSlug).catch(() => []),
          studioApi.getProjectFiles(currentSlug).catch(() => []),
          studioApi.getScopeItems("playbooks").catch(() => []),
        ]);

        if (isMounted) {
          setThreads(threadList);
          setProjectFiles(fileList);
          setPlaybooks(pbList);

          if (!activeThreadId && threadList.length > 0) {
            setActiveThreadId(threadList[0].id);
          }
          if (fileList.length > 0 && !selectedFilePath) {
            // Auto-select first main source if exists (e.g. index.html or script.md)
            const mainFile = fileList.find((f) => f.name.includes("index") || f.name.includes("main")) || fileList[0];
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

  const activeThread = threads.find((t) => t.id === activeThreadId) || threads[0];

  const handleCreateThread = async () => {
    if (!currentSlug) return;
    try {
      const newThread = await studioApi.createThread({
        project: currentSlug,
        engine: "claude",
      });
      setThreads((prev) => [newThread, ...prev]);
      setActiveThreadId(newThread.id);
    } catch (err: any) {
      setError(err?.message || "Không thể tạo hội thoại mới.");
    }
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
          height: 40,
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "0 var(--cs-space-4)",
          backgroundColor: "var(--cs-bg-panel)",
          borderBottom: "1px solid var(--cs-border-subtle)",
          flexShrink: 0,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-3)" }}>
          <div style={{ width: 220 }}>
            <Select
              size="sm"
              value={currentSlug}
              onChange={(e) => {
                setCurrentSlug(e.target.value);
                setActiveThreadId("");
                setSelectedFilePath(null);
              }}
              options={
                projects.length > 0
                  ? projects.map((p) => ({ value: p.slug, label: p.title }))
                  : [{ value: "", label: "Chưa có dự án" }]
              }
            />
          </div>

          <Button size="sm" variant="quiet" onClick={handleCreateThread}>
            + Hội thoại mới
          </Button>
        </div>

        {/* Tab switcher for narrow screens */}
        {isNarrow && (
          <Tabs
            items={[
              { id: "thread", label: "Thread" },
              { id: "stage", label: "Stage" },
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
              />
            )}
            {narrowTab === "stage" && (
              <StagePane
                projectSlug={currentSlug}
                selectedFilePath={selectedFilePath}
                onClearSelection={() => setSelectedFilePath(null)}
                onAskAi={handleAskAi}
                onSelectFile={setSelectedFilePath}
              />
            )}
            {narrowTab === "outputs" && (
              <OutputsPane
                projectSlug={currentSlug}
                projectFiles={projectFiles}
                onSelectFile={(path) => {
                  setSelectedFilePath(path);
                  setNarrowTab("stage");
                }}
              />
            )}
          </div>
        ) : (
          // Three resizable panes side-by-side
          <>
            {/* Thread Pane */}
            <div style={{ width: threadWidth, height: "100%", minWidth: 280, maxWidth: 600, flexShrink: 0 }}>
              <ThreadPane
                projectSlug={currentSlug}
                activeThread={activeThread}
                projectFiles={projectFiles}
                playbooks={playbooks}
                pendingInsert={pendingInsert}
                onInsertConsumed={() => setPendingInsert(null)}
                onSelectFile={setSelectedFilePath}
              />
            </div>

            {/* Stage Center Pane */}
            <div style={{ flex: 1, height: "100%", minWidth: 320, overflow: "hidden" }}>
              <StagePane
                projectSlug={currentSlug}
                selectedFilePath={selectedFilePath}
                onClearSelection={() => setSelectedFilePath(null)}
                onAskAi={handleAskAi}
                onSelectFile={setSelectedFilePath}
              />
            </div>

            {/* Outputs Pane */}
            <div style={{ width: outputsWidth, height: "100%", minWidth: 260, maxWidth: 500, flexShrink: 0 }}>
              <OutputsPane
                projectSlug={currentSlug}
                projectFiles={projectFiles}
                onSelectFile={setSelectedFilePath}
              />
            </div>
          </>
        )}
      </div>
    </div>
  );
};
