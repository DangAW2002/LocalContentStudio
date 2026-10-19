import React from "react";
import { Icon } from "../../components/Icon";
import {
  Button,
  Badge,
  EmptyState,
  Callout,
  Dialog,
} from "@studio/design-system";
import { studioApi } from "../../api/client";
import { DesignEditor, type DesignEditorHandle } from "./DesignEditor";
import { PlainHtmlPreview } from "./PlainHtmlPreview";
import { TimelineEditor } from "./TimelineEditor";
import { MediaViewer } from "./stage/MediaViewer";
import { StageToolbar } from "./stage/StageToolbar";
import { CheckResultDialog } from "./stage/CheckResultDialog";
import { TextEditorViewer } from "./stage/TextEditorViewer";
import { RenderOutputsBar } from "./stage/RenderOutputsBar";
import type {
  ArtifactSummary,
  ArtifactRenderResult,
  ArtifactCheckResult,
  FileReadResult,
} from "../../api/types";

export interface StagePaneProps {
  projectSlug: string;
  selectedFilePath: string | null;
  onClearSelection: () => void;
  onAskAi?: (text: string) => void;
  onSelectFile?: (path: string) => void;
  reloadNonce?: number;
  artifacts?: ArtifactSummary[];
  onRefreshArtifacts?: () => Promise<any> | any;
}

/** Check whether a file is an image, video or audio based on path extension */
function isMediaOutput(path: string): boolean {
  const p = path.toLowerCase();
  return (
    p.endsWith(".png") ||
    p.endsWith(".jpg") ||
    p.endsWith(".jpeg") ||
    p.endsWith(".webp") ||
    p.endsWith(".mp4") ||
    p.endsWith(".webm") ||
    p.endsWith(".mov") ||
    p.endsWith(".pdf") ||
    p.endsWith(".mp3") ||
    p.endsWith(".wav")
  );
}

export const StagePane: React.FC<StagePaneProps> = ({
  projectSlug,
  selectedFilePath,
  onClearSelection,
  onAskAi,
  onSelectFile,
  reloadNonce,
  artifacts: propsArtifacts,
  onRefreshArtifacts,
}) => {
  const [fileData, setFileData] = React.useState<FileReadResult | null>(null);
  const [loading, setLoading] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const [zoom, setZoom] = React.useState(100);

  // Artifact & Toolbar states
  const [internalArtifacts, setInternalArtifacts] = React.useState<ArtifactSummary[]>([]);
  const artifacts = propsArtifacts ?? internalArtifacts;
  const [matchingArtifact, setMatchingArtifact] = React.useState<ArtifactSummary | null>(null);
  const [rendering, setRendering] = React.useState(false);
  const [renderResult, setRenderResult] = React.useState<ArtifactRenderResult | null>(null);
  const [checking, setChecking] = React.useState(false);
  const [checkResult, setCheckResult] = React.useState<ArtifactCheckResult | null>(null);
  const [showCheckDialog, setShowCheckDialog] = React.useState(false);

  // Editor states
  const [isDirty, setIsDirty] = React.useState(false);
  const [textValue, setTextValue] = React.useState("");
  const [showMdPreview, setShowMdPreview] = React.useState(false);
  const [saveStatus, setSaveStatus] = React.useState<"idle" | "saving" | "saved" | "error">("idle");
  const [saveError, setSaveError] = React.useState<string | null>(null);
  const [externalChange, setExternalChange] = React.useState(false);

  const designEditorRef = React.useRef<DesignEditorHandle>(null);
  const checkpointerSessionDone = React.useRef<Set<string>>(new Set());

  // Load artifact list for this project
  const loadArtifacts = React.useCallback(async () => {
    if (onRefreshArtifacts) {
      await onRefreshArtifacts();
    }
    if (!projectSlug) return;
    try {
      const list = await studioApi.listArtifacts(projectSlug).catch(() => []);
      setInternalArtifacts(list);
    } catch {
      // ignore
    }
  }, [projectSlug, onRefreshArtifacts]);

  React.useEffect(() => {
    loadArtifacts();
  }, [loadArtifacts]);

  // Determine if selectedFilePath matches an artifact source
  React.useEffect(() => {
    if (!selectedFilePath) {
      setMatchingArtifact(null);
      return;
    }
    const norm = selectedFilePath.replace(/\\/g, "/").replace(/^\.\//, "");
    const match = artifacts.find((a) => {
      const aSrc = a.source.replace(/\\/g, "/").replace(/^\.\//, "");
      return aSrc === norm || norm.endsWith(aSrc) || aSrc.endsWith(norm);
    });
    if (!match && norm.includes("artifacts/")) {
      loadArtifacts();
    }
    setMatchingArtifact(match || null);
  }, [selectedFilePath, artifacts, loadArtifacts]);

  // Timeline preview: asset URL of the newest rendered video (render outputs are workspace-relative).
  const [timelinePreviewUrl, setTimelinePreviewUrl] = React.useState<string | undefined>(undefined);
  const latestVideoOutput = renderResult?.outputs?.find((p) => /\.(mp4|webm|mov)$/i.test(p));
  React.useEffect(() => {
    let cancelled = false;
    setTimelinePreviewUrl(undefined);
    if (!latestVideoOutput) return;
    const norm = latestVideoOutput.replace(/\\/g, "/");
    const prefix = `projects/${projectSlug}/`;
    const path = norm.startsWith(prefix) ? norm.slice(prefix.length) : norm;
    studioApi
      .readFile({ project: projectSlug, path })
      .then((res) => {
        if (!cancelled) setTimelinePreviewUrl(res.assetUrl);
      })
      .catch(() => {
        if (!cancelled) setTimelinePreviewUrl(undefined);
      });
    return () => {
      cancelled = true;
    };
  }, [latestVideoOutput, projectSlug]);

  // Load file content
  const loadFile = React.useCallback(async (keepDirtyState = false) => {
    if (!selectedFilePath) {
      setFileData(null);
      setTextValue("");
      setIsDirty(false);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const res = await studioApi.readFile({
        project: projectSlug,
        path: selectedFilePath,
      });
      setFileData(res);
      if (!keepDirtyState) {
        setTextValue(res.text || "");
        setIsDirty(false);
        setExternalChange(false);
      }
    } catch (err: any) {
      setError(err?.message || "Không thể đọc tệp từ workspace.");
    } finally {
      setLoading(false);
    }
  }, [projectSlug, selectedFilePath]);

  React.useEffect(() => {
    loadFile(false);
    setSaveStatus("idle");
    setSaveError(null);
    setRenderResult(null);
    setCheckResult(null);
  }, [loadFile]);

  // Reload file when reloadNonce changes (e.g., turn undone in ThreadPane)
  React.useEffect(() => {
    if (reloadNonce !== undefined && reloadNonce > 0) {
      loadFile(false);
      loadArtifacts();
    }
  }, [reloadNonce, loadFile, loadArtifacts]);

  // Listen to external change (fs-event touching this file or turn Done)
  React.useEffect(() => {
    if (!selectedFilePath) return;

    let unlistenFs: (() => void) | undefined;
    let unlistenTurn: (() => void) | undefined;

    const normTarget = selectedFilePath.replace(/\\/g, "/").replace(/^\.\//, "");

    const handleExternalModification = () => {
      if (isDirty) {
        setExternalChange(true);
      } else {
        loadFile(false);
      }
    };

    const setup = async () => {
      try {
        unlistenFs = await studioApi.onFsEvent((payload) => {
          const touchedYaml = payload.paths.some((p) => /artifact\.yaml$/i.test(p));
          if (touchedYaml) {
            loadArtifacts();
          }
          const touched = payload.paths.some((p) => {
            const normP = p.replace(/\\/g, "/").replace(/^\.\//, "");
            return normP === normTarget || normP.endsWith("/" + normTarget);
          });
          if (touched) {
            handleExternalModification();
          }
        });
      } catch {
        // Core absent
      }

      try {
        unlistenTurn = await studioApi.onTurnEvent((payload) => {
          if (payload.event.type === "Done") {
            loadArtifacts();
            handleExternalModification();
          }
        });
      } catch {
        // Core absent
      }
    };

    setup();
    return () => {
      if (unlistenFs) unlistenFs();
      if (unlistenTurn) unlistenTurn();
    };
  }, [selectedFilePath, isDirty, loadFile, loadArtifacts]);

  // Determine if HTML has data-layer elements
  const isLayerHtml = React.useMemo(() => {
    if (fileData?.kind !== "html" || !fileData.text) return false;
    return fileData.text.includes("data-layer");
  }, [fileData]);

  // Save handler (Rule 5: first save of session for file -> createCheckpoint)
  const handleSave = React.useCallback(async () => {
    if (!selectedFilePath) return;

    let contentToSave = textValue;
    if (isLayerHtml && designEditorRef.current) {
      contentToSave = designEditorRef.current.serialize();
    }

    setSaveStatus("saving");
    setSaveError(null);

    try {
      const fileKey = `${projectSlug}:${selectedFilePath}`;
      if (!checkpointerSessionDone.current.has(fileKey)) {
        const label = matchingArtifact
          ? `Sửa trực tiếp: ${matchingArtifact.id}`
          : `Sửa trực tiếp: ${selectedFilePath}`;
        // No restore point → do not overwrite the file.
        await studioApi.createCheckpoint({ label });
        checkpointerSessionDone.current.add(fileKey);
      }

      await studioApi.writeFile({
        project: projectSlug,
        path: selectedFilePath,
        text: contentToSave,
      });

      if (/artifact\.yaml$/i.test(selectedFilePath)) {
        await loadArtifacts();
      }

      setIsDirty(false);
      setExternalChange(false);
      setSaveStatus("saved");
      setTextValue(contentToSave);

      // Reload fresh file read data in background
      loadFile(true);

      setTimeout(() => {
        setSaveStatus((s) => (s === "saved" ? "idle" : s));
      }, 2500);
    } catch (err: any) {
      setSaveStatus("error");
      setSaveError(err?.message || "Lỗi khi lưu tệp.");
    }
  }, [selectedFilePath, textValue, isLayerHtml, projectSlug, matchingArtifact, loadFile, loadArtifacts]);

  // Keyboard shortcut Ctrl+S
  React.useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        handleSave();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleSave]);

  // Artifact Render action
  const handleRenderArtifact = async () => {
    if (!matchingArtifact) return;
    setRendering(true);
    setError(null);
    try {
      const res = await studioApi.renderArtifact({
        project: projectSlug,
        id: matchingArtifact.id,
      });
      setRenderResult(res);
    } catch (err: any) {
      setError(err?.message || "Lỗi khi render artifact.");
    } finally {
      setRendering(false);
    }
  };

  // Artifact Check action
  const handleCheckArtifact = async () => {
    if (!matchingArtifact) return;
    setChecking(true);
    setError(null);
    try {
      const res = await studioApi.checkArtifact({
        project: projectSlug,
        id: matchingArtifact.id,
        channels: matchingArtifact.channels,
      });
      setCheckResult(res);
      setShowCheckDialog(true);
    } catch (err: any) {
      setError(err?.message || "Lỗi khi kiểm tra tiêu chuẩn artifact.");
    } finally {
      setChecking(false);
    }
  };

  // Ask AI about layer
  const handleAskAiAboutLayer = (layerId: string, layerText: string) => {
    if (!selectedFilePath || !onAskAi) return;
    const promptText = `@${selectedFilePath}#${layerId} "${layerText}" `;
    onAskAi(promptText);
  };

  const handleOpenExternal = () => {
    if (fileData?.assetUrl) {
      window.open(fileData.assetUrl, "_blank");
    } else {
      alert(`Mở tệp ${selectedFilePath} bằng ứng dụng hệ thống.`);
    }
  };

  // Format prompt for agent to fix failed findings and insert into ThreadPane input
  const handleAskAgentToFix = () => {
    if (!checkResult || !onAskAi) return;
    const srcPath = matchingArtifact ? matchingArtifact.source : selectedFilePath || "index.html";
    const failedLines: string[] = [];

    for (const [chId, chRes] of Object.entries(checkResult.channels)) {
      if (!chRes.passed) {
        for (const f of chRes.findings) {
          if (f.severity === "error" || f.severity === "warn") {
            let line = `- [Kênh ${chId}] [${f.code}] ${f.message_vi}`;
            if (f.fix_hint_vi) {
              line += ` (Gợi ý: ${f.fix_hint_vi})`;
            }
            if (f.location) {
              line += ` tại ${f.location}`;
            }
            failedLines.push(line);
          }
        }
      }
    }

    const promptText = `Hãy sửa artifact @${srcPath} để vượt qua các lỗi kiểm tra sau, sau đó re-render và re-check:\n${failedLines.join("\n")}`;
    onAskAi(promptText);
    setShowCheckDialog(false);
  };

  // Render main viewer/editor based on file kind
  const renderViewer = () => {
    if (!fileData) return null;

    // 1. HTML with data-layer -> DesignEditor
    if (fileData.kind === "html" && isLayerHtml) {
      return (
        <DesignEditor
          ref={designEditorRef}
          html={fileData.text || ""}
          onDirtyChange={setIsDirty}
          onSaveRequest={handleSave}
          onAskAi={handleAskAiAboutLayer}
        />
      );
    }

    // 2. Plain HTML without data-layer (render at artboard size and scale to fit stage)
    if (fileData.kind === "html") {
      return <PlainHtmlPreview html={fileData.text || ""} />;
    }

    // 3. Timeline editor for timeline.json or artifacts of kind 'timeline'
    const isTimelineFile =
      selectedFilePath?.endsWith("timeline.json") ||
      matchingArtifact?.kind === "timeline";
    if (isTimelineFile && (fileData.kind === "json" || fileData.kind === "text")) {
      return (
        <TimelineEditor
          initialJson={textValue || fileData.text || "{}"}
          previewVideoUrl={timelinePreviewUrl}
          onChange={(newJson, _isValid) => {
            setTextValue(newJson);
            setIsDirty(true);
          }}
          onSaveRequest={handleSave}
          onRenderRequest={matchingArtifact ? handleRenderArtifact : undefined}
          onCheckRequest={matchingArtifact ? handleCheckArtifact : undefined}
          rendering={rendering}
          checking={checking}
        />
      );
    }

    // 4. Text / Markdown / Code / JSON editor
    if (
      fileData.kind === "markdown" ||
      fileData.kind === "text" ||
      fileData.kind === "code" ||
      fileData.kind === "json"
    ) {
      return (
        <TextEditorViewer
          kind={fileData.kind}
          textValue={textValue}
          setTextValue={setTextValue}
          setIsDirty={setIsDirty}
        />
      );
    }

    // 4. Media & Binary viewers (image, video, audio, binary)
    return (
      <MediaViewer
        fileData={fileData}
        selectedFilePath={selectedFilePath}
        zoom={zoom}
        onOpenExternal={handleOpenExternal}
      />
    );
  };

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        backgroundColor: "var(--cs-bg-canvas)",
        overflow: "hidden",
      }}
    >
      {/* Stage Toolbar Header */}
      <StageToolbar
        selectedFilePath={selectedFilePath}
        fileData={fileData}
        matchingArtifact={matchingArtifact}
        isDirty={isDirty}
        saveStatus={saveStatus}
        saveError={saveError}
        rendering={rendering}
        checking={checking}
        isLayerHtml={isLayerHtml}
        zoom={zoom}
        setZoom={setZoom}
        onSave={handleSave}
        onRenderArtifact={handleRenderArtifact}
        onCheckArtifact={handleCheckArtifact}
        onOpenExternal={handleOpenExternal}
        onClearSelection={onClearSelection}
      />

      {/* External Change Conflict Callout */}
      {externalChange && (
        <div style={{ padding: "var(--cs-space-2) var(--cs-space-3)" }}>
          <Callout
            variant="warning"
            title="Tệp đã thay đổi bởi agent"
            actions={
              <>
                <Button
                  size="sm"
                  variant="primary"
                  onClick={() => loadFile(false)}
                >
                  Tải lại
                </Button>
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() => setExternalChange(false)}
                >
                  Giữ bản của tôi
                </Button>
              </>
            }
          >
            Tệp nguồn trên đĩa vừa được cập nhật bởi agent hoặc hệ thống ngoài, trong khi bạn có các chỉnh sửa chưa lưu.
          </Callout>
        </div>
      )}

      {/* Render Outputs Bar (when render finished) */}
      {renderResult && renderResult.outputs && renderResult.outputs.length > 0 && (
        <RenderOutputsBar
          outputs={renderResult.outputs}
          cached={renderResult.cached}
          onSelectFile={onSelectFile}
          onClose={() => setRenderResult(null)}
        />
      )}

      {/* Stage Body */}
      <div
        style={{
          flex: 1,
          display: "flex",
          flexDirection: "column",
          minHeight: 0,
          minWidth: 0,
          position: "relative",
          overflow: "hidden",
        }}
      >
        {error && (
          <div style={{ padding: "var(--cs-space-4)" }}>
            <Callout variant="danger" title="Lỗi thao tác">
              {error}
            </Callout>
          </div>
        )}

        {loading ? (
          <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", color: "var(--cs-fg-muted)" }}>
            Đang tải nội dung tệp...
          </div>
        ) : !selectedFilePath ? (
          <EmptyState
            icon={<Icon name="eye" size={28} />}
            title="Khung nhìn Stage"
            description="Bấm vào một tệp trong hội thoại hoặc danh sách Outputs để xem trước hoặc chỉnh sửa tại đây (HTML, Markdown, ảnh, video, code)."
          />
        ) : (
          renderViewer()
        )}
      </div>

      {/* Artifact Check Result Dialog */}
      <CheckResultDialog
        open={showCheckDialog}
        onClose={() => setShowCheckDialog(false)}
        checkResult={checkResult}
        onAskAgentToFix={onAskAi ? handleAskAgentToFix : undefined}
      />
    </div>
  );
};
