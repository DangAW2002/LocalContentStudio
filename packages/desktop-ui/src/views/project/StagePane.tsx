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
import {
  DesignEditor,
  type DesignEditorHandle,
} from "./DesignEditor";
import { PlainHtmlPreview } from "./PlainHtmlPreview";
import { TimelineEditor } from "./TimelineEditor";
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

/** Simple, clean regex-based markdown renderer without heavy dependencies */
function renderSimpleMarkdown(md: string): React.ReactNode {
  const lines = md.split("\n");
  const elements: React.ReactNode[] = [];
  let inCodeBlock = false;
  let codeBuffer: string[] = [];

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    if (line.trim().startsWith("```")) {
      if (inCodeBlock) {
        elements.push(
          <pre
            key={`code-${i}`}
            style={{
              padding: "var(--cs-space-3)",
              backgroundColor: "var(--cs-bg-raised)",
              border: "1px solid var(--cs-border-subtle)",
              borderRadius: "var(--cs-radius-control)",
              fontFamily: "var(--cs-font-mono)",
              fontSize: "var(--cs-text-xs)",
              overflowX: "auto",
              margin: "8px 0",
            }}
          >
            {codeBuffer.join("\n")}
          </pre>
        );
        codeBuffer = [];
        inCodeBlock = false;
      } else {
        inCodeBlock = true;
      }
      continue;
    }

    if (inCodeBlock) {
      codeBuffer.push(line);
      continue;
    }

    if (line.startsWith("# ")) {
      elements.push(
        <h1 key={i} style={{ fontSize: "var(--cs-text-xl)", margin: "16px 0 8px", color: "var(--cs-fg-primary)" }}>
          {line.slice(2)}
        </h1>
      );
    } else if (line.startsWith("## ")) {
      elements.push(
        <h2 key={i} style={{ fontSize: "var(--cs-text-lg)", margin: "14px 0 6px", color: "var(--cs-fg-primary)" }}>
          {line.slice(3)}
        </h2>
      );
    } else if (line.startsWith("### ")) {
      elements.push(
        <h3 key={i} style={{ fontSize: "var(--cs-text-md)", margin: "12px 0 4px", color: "var(--cs-fg-primary)" }}>
          {line.slice(4)}
        </h3>
      );
    } else if (line.startsWith("- ") || line.startsWith("* ")) {
      elements.push(
        <li key={i} style={{ marginLeft: 20, margin: "2px 0", color: "var(--cs-fg-primary)" }}>
          {line.slice(2)}
        </li>
      );
    } else if (line.trim() === "") {
      elements.push(<div key={i} style={{ height: 8 }} />);
    } else {
      elements.push(
        <p key={i} style={{ margin: "4px 0", lineHeight: 1.6, color: "var(--cs-fg-primary)" }}>
          {line}
        </p>
      );
    }
  }

  if (inCodeBlock && codeBuffer.length > 0) {
    elements.push(
      <pre
        key="code-end"
        style={{
          padding: "var(--cs-space-3)",
          backgroundColor: "var(--cs-bg-raised)",
          border: "1px solid var(--cs-border-subtle)",
          borderRadius: "var(--cs-radius-control)",
          fontFamily: "var(--cs-font-mono)",
          fontSize: "var(--cs-text-xs)",
          overflowX: "auto",
        }}
      >
        {codeBuffer.join("\n")}
      </pre>
    );
  }

  return <div>{elements}</div>;
}

export const StagePane: React.FC<StagePaneProps> = ({
  projectSlug,
  selectedFilePath,
  onClearSelection,
  onAskAi,
  onSelectFile,
  reloadNonce,
}) => {
  const [fileData, setFileData] = React.useState<FileReadResult | null>(null);
  const [loading, setLoading] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const [zoom, setZoom] = React.useState(100);

  // Artifact & Toolbar states
  const [artifacts, setArtifacts] = React.useState<ArtifactSummary[]>([]);
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
    if (!projectSlug) return;
    try {
      const list = await studioApi.listArtifacts(projectSlug).catch(() => []);
      setArtifacts(list);
    } catch {
      // ignore
    }
  }, [projectSlug]);

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
    setMatchingArtifact(match || null);
  }, [selectedFilePath, artifacts]);

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
  }, [selectedFilePath, isDirty, loadFile]);

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
  }, [selectedFilePath, textValue, isLayerHtml, projectSlug, matchingArtifact, loadFile]);

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
        <div
          style={{
            flex: 1,
            display: "flex",
            flexDirection: "column",
            height: "100%",
            backgroundColor: "var(--cs-bg-panel)",
            position: "relative",
            overflow: "hidden",
          }}
        >
          {/* Subheader for Text / Markdown controls */}
          <div
            style={{
              height: 32,
              padding: "0 var(--cs-space-3)",
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              borderBottom: "1px solid var(--cs-border-subtle)",
              backgroundColor: "var(--cs-bg-raised)",
              fontSize: "var(--cs-text-xs)",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              {isDirty && (
                <Badge variant="warning">Chưa lưu (Ctrl+S)</Badge>
              )}
              {saveStatus === "saving" && (
                <span style={{ color: "var(--cs-accent)" }}>Đang lưu...</span>
              )}
              {saveStatus === "saved" && (
                <span style={{ color: "var(--cs-success)" }}>Đã lưu</span>
              )}
              {saveStatus === "error" && (
                <span style={{ color: "var(--cs-danger)" }}>Lỗi: {saveError}</span>
              )}
            </div>

            <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
              {fileData.kind === "markdown" && (
                <Button
                  size="sm"
                  variant={showMdPreview ? "secondary" : "quiet"}
                  onClick={() => setShowMdPreview(!showMdPreview)}
                  leftIcon={<Icon name="eye" size={12} />}
                >
                  {showMdPreview ? "Chỉnh sửa" : "Xem thử Markdown"}
                </Button>
              )}
            </div>
          </div>

          {/* Editor Body */}
          <div style={{ flex: 1, display: "flex", overflow: "hidden", position: "relative" }}>
            {fileData.kind === "markdown" && showMdPreview ? (
              <div
                style={{
                  flex: 1,
                  padding: "var(--cs-space-4) var(--cs-space-6)",
                  overflowY: "auto",
                  overflowX: "hidden",
                  maxWidth: 820,
                  margin: "0 auto",
                  width: "100%",
                  fontSize: "var(--cs-text-sm)",
                  fontFamily: "var(--cs-font-ui)",
                  boxSizing: "border-box",
                }}
              >
                {renderSimpleMarkdown(textValue)}
              </div>
            ) : (
              <textarea
                value={textValue}
                onChange={(e) => {
                  setTextValue(e.target.value);
                  setIsDirty(true);
                }}
                spellCheck={false}
                style={{
                  flex: 1,
                  width: "100%",
                  height: "100%",
                  padding: "var(--cs-space-3)",
                  margin: 0,
                  border: "none",
                  outline: "none",
                  resize: "none",
                  backgroundColor: "var(--cs-bg-panel)",
                  color: "var(--cs-fg-primary)",
                  fontFamily: "var(--cs-font-mono)",
                  fontSize: "var(--cs-text-xs)",
                  lineHeight: 1.6,
                  whiteSpace: "pre",
                  boxSizing: "border-box",
                }}
              />
            )}
          </div>
        </div>
      );
    }

    // 4. Image viewer
    if (fileData.kind === "image") {
      return (
        <div
          style={{
            flex: 1,
            width: "100%",
            height: "100%",
            minWidth: 0,
            minHeight: 0,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            backgroundColor: "var(--cs-canvas-checker)",
            overflow: "auto",
            position: "relative",
            padding: "var(--cs-space-4)",
            boxSizing: "border-box",
          }}
        >
          <img
            src={fileData.assetUrl || (fileData.text ? `data:${fileData.mime};base64,${fileData.text}` : "")}
            alt={selectedFilePath || "Image preview"}
            style={{
              maxWidth: zoom === 100 ? "100%" : `${zoom}%`,
              maxHeight: zoom === 100 ? "100%" : `${zoom}%`,
              objectFit: "contain",
              boxShadow: "var(--cs-shadow-artboard)",
              backgroundColor: "var(--cs-canvas-artboard)",
            }}
          />
        </div>
      );
    }

    // 5. Video viewer
    if (fileData.kind === "video") {
      return (
        <div
          style={{
            flex: 1,
            width: "100%",
            height: "100%",
            minWidth: 0,
            minHeight: 0,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            backgroundColor: "var(--cs-bg-canvas)",
            padding: "var(--cs-space-4)",
            boxSizing: "border-box",
            overflow: "hidden",
          }}
        >
          <video
            controls
            src={fileData.assetUrl}
            style={{
              maxWidth: "100%",
              maxHeight: "100%",
              backgroundColor: "#000",
              boxShadow: "var(--cs-shadow-artboard)",
            }}
          />
        </div>
      );
    }

    // 6. Audio viewer
    if (fileData.kind === "audio") {
      return (
        <div
          style={{
            flex: 1,
            width: "100%",
            height: "100%",
            minWidth: 0,
            minHeight: 0,
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            backgroundColor: "var(--cs-bg-app)",
            gap: "var(--cs-space-4)",
            padding: "var(--cs-space-4)",
            boxSizing: "border-box",
            overflow: "hidden",
          }}
        >
          <Icon name="audio" size={48} color="var(--cs-accent)" />
          <div style={{ fontWeight: 600, fontSize: "var(--cs-text-md)" }}>
            {selectedFilePath}
          </div>
          <audio controls src={fileData.assetUrl} style={{ width: "100%", maxWidth: 460 }} />
        </div>
      );
    }

    // Default binary/unknown
    return (
      <div
        style={{
          flex: 1,
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          padding: "var(--cs-space-6)",
          textAlign: "center",
          gap: "var(--cs-space-3)",
        }}
      >
        <Icon name="file" size={48} color="var(--cs-fg-muted)" />
        <h3 style={{ margin: 0, fontSize: "var(--cs-text-md)", fontWeight: 600 }}>
          Định dạng tệp nhị phân
        </h3>
        <p style={{ margin: 0, fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-secondary)", maxWidth: 360 }}>
          Không thể xem trước trực tiếp định dạng <code>{fileData.mime}</code> ({fileData.size.toLocaleString()} bytes).
        </p>
        <Button
          variant="secondary"
          onClick={handleOpenExternal}
          leftIcon={<Icon name="external" size={14} />}
        >
          Mở bằng ứng dụng hệ thống
        </Button>
      </div>
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
      <div
        style={{
          height: 38,
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "0 var(--cs-space-3)",
          backgroundColor: "var(--cs-bg-panel)",
          borderBottom: "1px solid var(--cs-border-subtle)",
          fontSize: "var(--cs-text-xs)",
          userSelect: "none",
          flexShrink: 0,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 8, minWidth: 0 }}>
          <span
            style={{
              fontWeight: 600,
              color: "var(--cs-fg-primary)",
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {selectedFilePath || "Stage (Khung làm việc)"}
          </span>
          {fileData && (
            <>
              <Badge variant="neutral">{fileData.kind.toUpperCase()}</Badge>
              <span style={{ color: "var(--cs-fg-muted)" }}>
                {(fileData.size / 1024).toFixed(1)} KB
              </span>
            </>
          )}
          {matchingArtifact && (
            <Badge variant="info">
              Artifact: {matchingArtifact.title || matchingArtifact.id}
            </Badge>
          )}
          {isDirty && (
            <Badge variant="warning">Đã sửa</Badge>
          )}
        </div>

        {selectedFilePath && (
          <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
            {/* Artifact Toolbar Actions */}
            {matchingArtifact && (
              <>
                <Button
                  size="sm"
                  variant="primary"
                  busy={rendering}
                  busyText="Đang render..."
                  onClick={handleRenderArtifact}
                  leftIcon={<Icon name="play" size={12} />}
                >
                  Render
                </Button>
                <Button
                  size="sm"
                  variant="secondary"
                  busy={checking}
                  busyText="Đang kiểm tra..."
                  onClick={handleCheckArtifact}
                  leftIcon={<Icon name="check" size={12} />}
                >
                  Kiểm tra
                </Button>
              </>
            )}

            {/* Save Button for editable files */}
            {(isLayerHtml ||
              fileData?.kind === "markdown" ||
              fileData?.kind === "text" ||
              fileData?.kind === "code" ||
              fileData?.kind === "json") && (
              <Button
                size="sm"
                variant={isDirty ? "primary" : "quiet"}
                busy={saveStatus === "saving"}
                onClick={handleSave}
                leftIcon={<Icon name="check" size={12} />}
              >
                Lưu
              </Button>
            )}

            {fileData?.kind === "image" && (
              <div style={{ display: "flex", alignItems: "center", gap: 2, marginRight: 4 }}>
                <Button size="sm" variant="quiet" onClick={() => setZoom((z) => Math.max(25, z - 25))}>
                  -
                </Button>
                <span style={{ fontSize: 11, minWidth: 32, textAlign: "center" }}>{zoom}%</span>
                <Button size="sm" variant="quiet" onClick={() => setZoom((z) => Math.min(300, z + 25))}>
                  +
                </Button>
              </div>
            )}

            <Button
              size="sm"
              variant="quiet"
              onClick={handleOpenExternal}
              title="Mở bên ngoài"
            >
              <Icon name="external" size={13} />
            </Button>
            <Button
              size="sm"
              variant="quiet"
              onClick={onClearSelection}
              title="Đóng xem trước"
            >
              <Icon name="x" size={13} />
            </Button>
          </div>
        )}
      </div>

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
        <div
          style={{
            padding: "6px var(--cs-space-3)",
            backgroundColor: "var(--cs-bg-raised)",
            borderBottom: "1px solid var(--cs-border-subtle)",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            fontSize: "var(--cs-text-xs)",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
            <span style={{ fontWeight: 600, color: "var(--cs-success)" }}>
              ✓ Render xong ({renderResult.cached ? "từ cache" : "mới"}):
            </span>
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
              {renderResult.outputs.map((outPath) => (
                <button
                  key={outPath}
                  type="button"
                  onClick={() => {
                    if (isMediaOutput(outPath) && onSelectFile) {
                      onSelectFile(outPath);
                    }
                  }}
                  style={{
                    padding: "2px 6px",
                    borderRadius: "var(--cs-radius-control)",
                    backgroundColor: "var(--cs-bg-panel)",
                    border: "1px solid var(--cs-border-subtle)",
                    fontSize: 11,
                    fontFamily: "var(--cs-font-mono)",
                    cursor: onSelectFile ? "pointer" : "default",
                    color: "var(--cs-accent)",
                    display: "inline-flex",
                    alignItems: "center",
                    gap: 4,
                  }}
                  title="Bấm để xem output trên Stage"
                >
                  <Icon name="file" size={11} />
                  <span>{outPath}</span>
                </button>
              ))}
            </div>
          </div>
          <Button size="sm" variant="quiet" onClick={() => setRenderResult(null)}>
            Đóng
          </Button>
        </div>
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
      {showCheckDialog && checkResult && (
        <Dialog
          title="Kết quả kiểm tra tiêu chuẩn Brand & Channel"
          open={showCheckDialog}
          onClose={() => setShowCheckDialog(false)}
          actions={
            <div style={{ display: "flex", gap: "var(--cs-space-2)", alignItems: "center" }}>
              {!checkResult.passed && onAskAi && (
                <Button variant="primary" onClick={handleAskAgentToFix}>
                  Nhờ agent sửa
                </Button>
              )}
              <Button variant="secondary" onClick={() => setShowCheckDialog(false)}>
                Đóng
              </Button>
            </div>
          }
        >
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)", maxHeight: 420, overflowY: "auto" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <Badge variant={checkResult.passed ? "success" : "danger"}>
                {checkResult.passed ? "ĐẠT TOÀN BỘ KÊNH" : "CHƯA ĐẠT TIÊU CHUẨN"}
              </Badge>
              <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
                Kiểm tra theo quy chuẩn của từng kênh phân phối.
              </span>
            </div>

            {Object.entries(checkResult.channels).length === 0 ? (
              <div style={{ fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-muted)" }}>
                Không có dữ liệu kênh kiểm tra.
              </div>
            ) : (
              Object.entries(checkResult.channels).map(([chId, chRes]) => (
                <div
                  key={chId}
                  style={{
                    padding: "var(--cs-space-3)",
                    border: "1px solid var(--cs-border-subtle)",
                    borderRadius: "var(--cs-radius-control)",
                    backgroundColor: "var(--cs-bg-raised)",
                    display: "flex",
                    flexDirection: "column",
                    gap: 6,
                  }}
                >
                  <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                    <span style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)" }}>
                      Kênh: {chId}
                    </span>
                    <Badge variant={chRes.passed ? "success" : "danger"}>
                      {chRes.passed ? "Đạt" : "Không đạt"}
                    </Badge>
                  </div>

                  {chRes.findings && chRes.findings.length > 0 ? (
                    <div style={{ display: "flex", flexDirection: "column", gap: 6, marginTop: 4 }}>
                      {chRes.findings.map((f, fIdx) => (
                        <div
                          key={fIdx}
                          style={{
                            padding: "6px 8px",
                            backgroundColor: "var(--cs-bg-panel)",
                            borderRadius: "var(--cs-radius-control)",
                            borderLeft: `3px solid ${
                              f.severity === "error"
                                ? "var(--cs-danger)"
                                : f.severity === "warn"
                                ? "var(--cs-warning)"
                                : "var(--cs-accent)"
                            }`,
                            fontSize: "var(--cs-text-xs)",
                            display: "flex",
                            flexDirection: "column",
                            gap: 2,
                          }}
                        >
                          <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                            <Badge
                              variant={
                                f.severity === "error"
                                  ? "danger"
                                  : f.severity === "warn"
                                  ? "warning"
                                  : "info"
                              }
                            >
                              {f.severity.toUpperCase()}
                            </Badge>
                            <span style={{ fontWeight: 600 }}>{f.message_vi}</span>
                          </div>
                          {f.fix_hint_vi && (
                            <div style={{ color: "var(--cs-fg-secondary)", marginTop: 2 }}>
                              💡 Gợi ý khắc phục: {f.fix_hint_vi}
                            </div>
                          )}
                          {f.location && (
                            <div style={{ color: "var(--cs-fg-muted)", fontSize: 11 }}>
                              Vị trí: <code>{f.location}</code>
                            </div>
                          )}
                        </div>
                      ))}
                    </div>
                  ) : (
                    <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-success)" }}>
                      ✓ Không có cảnh báo hay lỗi vi phạm nào.
                    </div>
                  )}
                </div>
              ))
            )}
          </div>
        </Dialog>
      )}
    </div>
  );
};
