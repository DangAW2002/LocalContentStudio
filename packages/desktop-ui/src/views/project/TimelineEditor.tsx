import React from "react";
import { Button, Badge } from "@studio/design-system";
import { Icon } from "../../components/Icon";

export interface TimelineClip {
  id: string;
  source: string;
  in_point?: number;
  out_point?: number;
  start: number;
  duration: number;
  transition?: "none" | "fade";
  transition_duration?: number;
}

export interface TimelineTrack {
  id: string;
  kind: "video" | "audio" | "captions" | "caption";
  clips: TimelineClip[];
}

export interface TimelineDoc {
  version: number;
  width: number;
  height: number;
  fps: number;
  tracks: TimelineTrack[];
}

export interface TimelineEditorProps {
  /** Raw JSON string of timeline.json */
  initialJson: string;
  /** Safe asset URL of the latest rendered video preview (if available) */
  previewVideoUrl?: string;
  /** Called when timeline changes with new serialized JSON and validity */
  onChange: (newJson: string, isValid: boolean) => void;
  /** Direct save request */
  onSaveRequest: () => void;
  /** Trigger render */
  onRenderRequest?: () => void;
  /** Trigger check */
  onCheckRequest?: () => void;
  /** Whether a render job is currently running */
  rendering?: boolean;
  /** Whether checking is running */
  checking?: boolean;
}

const TRACK_COLORS: Record<string, { bg: string; border: string; text: string }> = {
  video: {
    bg: "rgba(33, 150, 243, 0.2)",
    border: "var(--cs-accent, #2196F3)",
    text: "var(--cs-fg-primary, #ffffff)",
  },
  audio: {
    bg: "rgba(76, 175, 80, 0.2)",
    border: "#4CAF50",
    text: "var(--cs-fg-primary, #ffffff)",
  },
  captions: {
    bg: "rgba(255, 152, 0, 0.2)",
    border: "#FF9800",
    text: "var(--cs-fg-primary, #ffffff)",
  },
  caption: {
    bg: "rgba(255, 152, 0, 0.2)",
    border: "#FF9800",
    text: "var(--cs-fg-primary, #ffffff)",
  },
};

const TRACK_KIND_LABELS: Record<string, string> = {
  video: "Video",
  audio: "Âm thanh",
  captions: "Phụ đề",
  caption: "Phụ đề",
};

const GUTTER_WIDTH = 140;

/**
 * Validate that doc conforms to TimelineDoc schema:
 * - version >= 1
 * - width > 0, height > 0, fps > 0
 * - tracks array with valid kinds and valid clip durations >= 0
 */
export function validateTimelineDoc(doc: any): boolean {
  if (!doc || typeof doc !== "object") return false;
  if (typeof doc.version !== "number" || doc.version < 1) return false;
  if (typeof doc.width !== "number" || doc.width <= 0) return false;
  if (typeof doc.height !== "number" || doc.height <= 0) return false;
  if (typeof doc.fps !== "number" || doc.fps <= 0) return false;
  if (!Array.isArray(doc.tracks)) return false;

  for (const track of doc.tracks) {
    if (!track || typeof track !== "object") return false;
    if (typeof track.id !== "string") return false;
    if (!["video", "audio", "captions", "caption"].includes(track.kind)) return false;
    if (!Array.isArray(track.clips)) return false;

    for (const clip of track.clips) {
      if (!clip || typeof clip !== "object") return false;
      if (typeof clip.id !== "string") return false;
      if (typeof clip.source !== "string") return false;
      if (typeof clip.start !== "number" || clip.start < 0) return false;
      if (typeof clip.duration !== "number" || clip.duration < 0) return false;
      if (clip.in_point !== undefined && typeof clip.in_point !== "number") return false;
      if (clip.out_point !== undefined && clip.out_point !== null && typeof clip.out_point !== "number") return false;
    }
  }

  return true;
}

export const TimelineEditor: React.FC<TimelineEditorProps> = ({
  initialJson,
  previewVideoUrl,
  onChange,
  onSaveRequest,
  onRenderRequest,
  onCheckRequest,
  rendering,
  checking,
}) => {
  const [doc, setDoc] = React.useState<TimelineDoc | null>(null);
  const [parseError, setParseError] = React.useState<string | null>(null);
  const [selectedClipKey, setSelectedClipKey] = React.useState<{ trackIdx: number; clipIdx: number } | null>(null);
  const [playhead, setPlayhead] = React.useState(0);
  const [isPlaying, setIsPlaying] = React.useState(false);

  const videoRef = React.useRef<HTMLVideoElement>(null);
  const rulerRef = React.useRef<HTMLDivElement>(null);

  // Initialize and parse timeline document
  React.useEffect(() => {
    try {
      const parsed = JSON.parse(initialJson);
      if (!validateTimelineDoc(parsed)) {
        setParseError("Dữ liệu timeline.json không đúng chuẩn TimelineDoc");
        setDoc(null);
      } else {
        setDoc(parsed);
        setParseError(null);
      }
    } catch (err: any) {
      setParseError(`Lỗi cú pháp JSON: ${err?.message || err}`);
      setDoc(null);
    }
  }, [initialJson]);

  // Compute maximum timeline duration
  const totalDuration = React.useMemo(() => {
    if (!doc || !doc.tracks || doc.tracks.length === 0) return 10;
    let maxT = 5;
    for (const track of doc.tracks) {
      for (const clip of track.clips) {
        const end = (clip.start || 0) + (clip.duration || 0);
        if (end > maxT) maxT = end;
      }
    }
    return Math.ceil(maxT) + 2; // Extra padding at end
  }, [doc]);

  // Zoom: pixels per second on the timeline lanes (fit / +/-)
  const [pxPerSec, setPxPerSec] = React.useState<number>(50);
  const timelineWidth = Math.max(600, totalDuration * pxPerSec);

  // Undo history stack for edits & clip deletion
  const [history, setHistory] = React.useState<TimelineDoc[]>([]);

  const handleZoomIn = () => setPxPerSec((prev) => Math.min(200, prev + 15));
  const handleZoomOut = () => setPxPerSec((prev) => Math.max(15, prev - 15));
  const handleZoomFit = () => {
    const availableWidth = Math.max(200, (rulerRef.current?.clientWidth || 800) - GUTTER_WIDTH);
    const target = Math.max(15, Math.floor((availableWidth - 80) / Math.max(1, totalDuration)));
    setPxPerSec(target);
  };

  // Playhead animation when playing without video
  React.useEffect(() => {
    let animId: number;
    if (isPlaying && !previewVideoUrl) {
      let lastTime = performance.now();
      const step = (now: number) => {
        const delta = (now - lastTime) / 1000;
        lastTime = now;
        setPlayhead((prev) => {
          const next = prev + delta;
          if (next >= totalDuration) {
            setIsPlaying(false);
            return 0;
          }
          return next;
        });
        animId = requestAnimationFrame(step);
      };
      animId = requestAnimationFrame(step);
    }
    return () => cancelAnimationFrame(animId);
  }, [isPlaying, previewVideoUrl, totalDuration]);

  // Sync playhead with video element if preview exists
  const handleTimeUpdate = () => {
    if (videoRef.current) {
      setPlayhead(videoRef.current.currentTime);
    }
  };

  const handlePlayPause = () => {
    if (videoRef.current) {
      if (videoRef.current.paused) {
        videoRef.current.play();
        setIsPlaying(true);
      } else {
        videoRef.current.pause();
        setIsPlaying(false);
      }
    } else {
      setIsPlaying(!isPlaying);
    }
  };

  // Click on ruler or lane to seek
  const handleSeek = (clientX: number, containerRect: DOMRect, scrollLeft: number) => {
    const x = clientX - containerRect.left + scrollLeft - GUTTER_WIDTH;
    if (x < 0) return;
    const time = Math.max(0, Math.min(totalDuration, x / pxPerSec));
    setPlayhead(time);
    if (videoRef.current) {
      videoRef.current.currentTime = time;
    }
  };

  // Mutate clip in doc and notify parent, recording history for undo
  const updateSelectedClip = (updater: (clip: TimelineClip) => void) => {
    if (!doc || !selectedClipKey) return;
    const { trackIdx, clipIdx } = selectedClipKey;
    const newDoc = JSON.parse(JSON.stringify(doc)) as TimelineDoc;
    const targetClip = newDoc.tracks[trackIdx]?.clips[clipIdx];
    if (!targetClip) return;

    // Save previous snapshot in history
    setHistory((h) => [...h.slice(-20), JSON.parse(JSON.stringify(doc))]);

    updater(targetClip);

    const valid = validateTimelineDoc(newDoc);
    setDoc(newDoc);
    onChange(JSON.stringify(newDoc, null, 2), valid);
  };

  // Delete currently selected clip and record for undo
  const handleDeleteSelectedClip = React.useCallback(() => {
    if (!doc || !selectedClipKey) return;
    const { trackIdx, clipIdx } = selectedClipKey;
    const newDoc = JSON.parse(JSON.stringify(doc)) as TimelineDoc;
    if (!newDoc.tracks[trackIdx]?.clips[clipIdx]) return;

    // Save previous snapshot in history
    setHistory((h) => [...h.slice(-20), JSON.parse(JSON.stringify(doc))]);

    newDoc.tracks[trackIdx].clips.splice(clipIdx, 1);
    setSelectedClipKey(null);

    const valid = validateTimelineDoc(newDoc);
    setDoc(newDoc);
    onChange(JSON.stringify(newDoc, null, 2), valid);
  }, [doc, selectedClipKey, onChange]);

  // Undo the last edit or deletion
  const handleUndo = React.useCallback(() => {
    if (history.length === 0) return;
    const prevDoc = history[history.length - 1];
    setHistory((h) => h.slice(0, h.length - 1));
    const valid = validateTimelineDoc(prevDoc);
    setDoc(prevDoc);
    onChange(JSON.stringify(prevDoc, null, 2), valid);
  }, [history, onChange]);

  // Keyboard shortcut listener: Delete removes selected clip, Ctrl+Z undoes
  React.useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const activeEl = document.activeElement;
      const isInput =
        activeEl &&
        (activeEl.tagName === "INPUT" ||
          activeEl.tagName === "TEXTAREA" ||
          (activeEl as HTMLElement).isContentEditable);

      if ((e.key === "Delete" || e.key === "Backspace") && !isInput) {
        if (selectedClipKey) {
          e.preventDefault();
          handleDeleteSelectedClip();
        }
      } else if (
        (e.ctrlKey || e.metaKey) &&
        e.key.toLowerCase() === "z" &&
        !e.shiftKey &&
        !isInput
      ) {
        e.preventDefault();
        handleUndo();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [selectedClipKey, handleDeleteSelectedClip, handleUndo]);

  const selectedClip = React.useMemo(() => {
    if (!doc || !selectedClipKey) return null;
    const { trackIdx, clipIdx } = selectedClipKey;
    return doc.tracks[trackIdx]?.clips[clipIdx] || null;
  }, [doc, selectedClipKey]);

  const selectedTrack = React.useMemo(() => {
    if (!doc || !selectedClipKey) return null;
    return doc.tracks[selectedClipKey.trackIdx] || null;
  }, [doc, selectedClipKey]);

  // Clip validation errors in Vietnamese
  const clipErrors = React.useMemo(() => {
    if (!selectedClip || !selectedTrack || !selectedClipKey) return {};
    const errors: Record<string, string> = {};

    if (!selectedClip.id.trim()) {
      errors.id = "Mã clip không được để trống";
    } else {
      const duplicate = selectedTrack.clips.some(
        (c, idx) => idx !== selectedClipKey.clipIdx && c.id === selectedClip.id.trim()
      );
      if (duplicate) {
        errors.id = "Mã clip đã tồn tại trong track này";
      }
    }

    if (!selectedClip.source.trim()) {
      errors.source =
        selectedTrack.kind === "captions" || selectedTrack.kind === "caption"
          ? "Nội dung phụ đề không được để trống"
          : "Đường dẫn nguồn media không được để trống";
    }

    if (selectedClip.start < 0) {
      errors.start = "Thời điểm bắt đầu phải ≥ 0s";
    }

    if (selectedClip.duration <= 0) {
      errors.duration = "Thời lượng phải lớn hơn 0s";
    }

    if (selectedClip.in_point !== undefined && selectedClip.in_point < 0) {
      errors.in_point = "Điểm vào phải ≥ 0s";
    }

    if (
      selectedClip.out_point !== undefined &&
      selectedClip.out_point !== null &&
      selectedClip.out_point <= (selectedClip.in_point ?? 0)
    ) {
      errors.out_point = "Điểm ra phải lớn hơn điểm vào";
    }

    return errors;
  }, [selectedClip, selectedTrack, selectedClipKey]);

  if (parseError) {
    return (
      <div
        style={{
          flex: 1,
          padding: "var(--cs-space-4)",
          backgroundColor: "var(--cs-bg-panel)",
          color: "var(--cs-danger)",
          display: "flex",
          flexDirection: "column",
          gap: "var(--cs-space-3)",
        }}
      >
        <div style={{ fontWeight: 600 }}>Không thể mở Trình chỉnh sửa Timeline</div>
        <div style={{ fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-secondary)" }}>
          {parseError}
        </div>
      </div>
    );
  }

  if (!doc) {
    return (
      <div style={{ padding: "var(--cs-space-4)", color: "var(--cs-fg-muted)" }}>
        Đang tải timeline...
      </div>
    );
  }

  return (
    <div
      style={{
        flex: 1,
        minWidth: 0,
        display: "flex",
        flexDirection: "column",
        height: "100%",
        backgroundColor: "var(--cs-bg-panel)",
        overflow: "hidden",
        position: "relative",
      }}
    >
      {/* Top Preview Section: Video Player & Info */}
      <div
        style={{
          display: "flex",
          height: 260,
          borderBottom: "1px solid var(--cs-border-subtle)",
          backgroundColor: "#0d0f12",
          minWidth: 0,
          width: "100%",
          boxSizing: "border-box",
        }}
      >
        {/* Video Player Display */}
        <div
          style={{
            flex: 1,
            minWidth: 0,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            position: "relative",
            backgroundColor: "#000000",
            overflow: "hidden",
          }}
        >
          {previewVideoUrl ? (
            <video
              ref={videoRef}
              src={previewVideoUrl}
              onTimeUpdate={handleTimeUpdate}
              onEnded={() => setIsPlaying(false)}
              style={{
                maxWidth: "100%",
                maxHeight: "100%",
                objectFit: "contain",
              }}
            />
          ) : (
            <div
              style={{
                display: "flex",
                flexDirection: "column",
                alignItems: "center",
                justifyContent: "center",
                gap: 8,
                color: "var(--cs-fg-muted)",
                fontSize: "var(--cs-text-xs)",
                padding: "var(--cs-space-4)",
                textAlign: "center",
              }}
            >
              <Icon name="video" size={36} color="var(--cs-fg-muted)" />
              <div style={{ fontWeight: 600, color: "var(--cs-fg-secondary)", fontSize: "var(--cs-text-sm)" }}>
                Chưa có bản render — bấm Render
              </div>
              <div style={{ color: "var(--cs-fg-muted)", maxWidth: 320 }}>
                Bản xem trước video MP4 sẽ xuất hiện tại đây sau khi render timeline thành công.
              </div>
              {onRenderRequest && (
                <Button
                  size="sm"
                  variant="primary"
                  busy={rendering}
                  busyText="Đang render..."
                  onClick={onRenderRequest}
                  leftIcon={<Icon name="play" size={12} />}
                >
                  Render ngay
                </Button>
              )}
            </div>
          )}
        </div>

        {/* Video Metadata Card */}
        <div
          style={{
            width: 260,
            borderLeft: "1px solid var(--cs-border-subtle)",
            backgroundColor: "var(--cs-bg-panel)",
            padding: "var(--cs-space-3)",
            display: "flex",
            flexDirection: "column",
            gap: "var(--cs-space-2)",
            fontSize: "var(--cs-text-xs)",
          }}
        >
          <div style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-primary)" }}>
            Thông số Timeline
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 6, color: "var(--cs-fg-secondary)" }}>
            <div>Độ phân giải:</div>
            <div style={{ fontWeight: 600, color: "var(--cs-fg-primary)" }}>{doc.width} × {doc.height}</div>
            <div>Tốc độ khung:</div>
            <div style={{ fontWeight: 600, color: "var(--cs-fg-primary)" }}>{doc.fps} FPS</div>
            <div>Số luồng track:</div>
            <div style={{ fontWeight: 600, color: "var(--cs-fg-primary)" }}>{doc.tracks.length} tracks</div>
            <div>Tổng thời lượng:</div>
            <div style={{ fontWeight: 600, color: "var(--cs-accent)" }}>{totalDuration}s</div>
          </div>

          <div style={{ marginTop: "auto", display: "flex", flexDirection: "column", gap: 6 }}>
            {onRenderRequest && (
              <Button
                size="sm"
                variant="primary"
                busy={rendering}
                busyText="Đang render..."
                onClick={onRenderRequest}
                leftIcon={<Icon name="play" size={12} />}
              >
                Render video
              </Button>
            )}
            {onCheckRequest && (
              <Button
                size="sm"
                variant="secondary"
                busy={checking}
                busyText="Đang kiểm tra..."
                onClick={onCheckRequest}
                leftIcon={<Icon name="check" size={12} />}
              >
                Kiểm tra tiêu chuẩn
              </Button>
            )}
          </div>
        </div>
      </div>

      {/* Middle Controls Bar with Playhead & Zoom Controls */}
      <div
        style={{
          height: 38,
          backgroundColor: "var(--cs-bg-raised)",
          borderBottom: "1px solid var(--cs-border-subtle)",
          padding: "0 var(--cs-space-3)",
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          fontSize: "var(--cs-text-xs)",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <Button
            size="sm"
            variant="quiet"
            onClick={handlePlayPause}
            leftIcon={<Icon name={isPlaying ? "x" : "play"} size={13} />}
          >
            {isPlaying ? "Dừng" : "Phát"}
          </Button>
          <span style={{ fontFamily: "var(--cs-font-mono)", fontWeight: 600, color: "var(--cs-accent)" }}>
            {playhead.toFixed(2)}s / {totalDuration}s
          </span>
        </div>

        {/* Zoom & History Undo Controls */}
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
            <span style={{ color: "var(--cs-fg-muted)", fontSize: 11 }}>Thu phóng:</span>
            <Button
              size="sm"
              variant="quiet"
              onClick={handleZoomOut}
              disabled={pxPerSec <= 15}
              title="Thu nhỏ timeline (-)"
            >
              -
            </Button>
            <span style={{ fontFamily: "var(--cs-font-mono)", fontSize: 11, minWidth: 44, textAlign: "center", color: "var(--cs-fg-primary)" }}>
              {pxPerSec}px/s
            </span>
            <Button
              size="sm"
              variant="quiet"
              onClick={handleZoomIn}
              disabled={pxPerSec >= 200}
              title="Phóng to timeline (+)"
            >
              +
            </Button>
            <Button
              size="sm"
              variant="secondary"
              onClick={handleZoomFit}
              title="Tự động thu phóng vừa khung nhìn"
            >
              Vừa khung
            </Button>
          </div>

          <div style={{ width: 1, height: 16, backgroundColor: "var(--cs-border-subtle)" }} />

          <Button
            size="sm"
            variant="quiet"
            onClick={handleUndo}
            disabled={history.length === 0}
            leftIcon={<Icon name="undo" size={12} />}
            title="Hoàn tác thao tác trước (Ctrl+Z)"
          >
            Hoàn tác
          </Button>

          <Button
            size="sm"
            variant="secondary"
            onClick={onSaveRequest}
            leftIcon={<Icon name="check" size={12} />}
          >
            Lưu timeline.json
          </Button>
        </div>
      </div>

      {/* Main Bottom Section: Multi-track Timeline & Side Inspector */}
      <div style={{ flex: 1, display: "flex", minHeight: 0, minWidth: 0, overflow: "hidden" }}>
        {/* Timeline Tracks Area */}
        <div
          ref={rulerRef}
          style={{
            flex: 1,
            minWidth: 0,
            display: "flex",
            flexDirection: "column",
            overflow: "auto",
            backgroundColor: "var(--cs-bg-canvas)",
            position: "relative",
          }}
          onClick={(e) => {
            if (rulerRef.current) {
              const rect = rulerRef.current.getBoundingClientRect();
              handleSeek(e.clientX, rect, rulerRef.current.scrollLeft);
            }
          }}
        >
          {/* Timeline Inner Canvas with Width Based on Duration + Gutter */}
          <div style={{ width: GUTTER_WIDTH + timelineWidth, position: "relative", minHeight: "100%" }}>
            {/* 1. Time Ruler Header */}
            <div
              style={{
                height: 24,
                backgroundColor: "var(--cs-bg-panel)",
                borderBottom: "1px solid var(--cs-border-subtle)",
                position: "relative",
                display: "flex",
                userSelect: "none",
              }}
            >
              {/* Sticky gutter top header */}
              <div
                style={{
                  position: "sticky",
                  left: 0,
                  width: GUTTER_WIDTH,
                  minWidth: GUTTER_WIDTH,
                  height: "100%",
                  backgroundColor: "var(--cs-bg-panel)",
                  borderRight: "1px solid var(--cs-border-subtle)",
                  zIndex: 7,
                  display: "flex",
                  alignItems: "center",
                  padding: "0 8px",
                  fontSize: 10,
                  fontWeight: 600,
                  color: "var(--cs-fg-muted)",
                  boxSizing: "border-box",
                }}
              >
                Track
              </div>
              {Array.from({ length: totalDuration + 1 }).map((_, sec) => (
                <div
                  key={sec}
                  style={{
                    position: "absolute",
                    left: GUTTER_WIDTH + sec * pxPerSec,
                    top: 0,
                    bottom: 0,
                    borderLeft: "1px solid var(--cs-border-subtle)",
                    paddingLeft: 3,
                    fontSize: 10,
                    fontFamily: "var(--cs-font-mono)",
                    color: "var(--cs-fg-muted)",
                  }}
                >
                  {sec}s
                </div>
              ))}
            </div>

            {/* 2. Tracks Container */}
            <div style={{ display: "flex", flexDirection: "column", gap: 6, padding: "8px 0" }}>
              {doc.tracks.map((track, trackIdx) => {
                const colors = TRACK_COLORS[track.kind] || TRACK_COLORS.video;
                return (
                  <div
                    key={track.id}
                    style={{
                      height: 48,
                      position: "relative",
                      backgroundColor: "rgba(255, 255, 255, 0.02)",
                      borderTop: "1px solid rgba(255, 255, 255, 0.05)",
                      borderBottom: "1px solid rgba(255, 255, 255, 0.05)",
                    }}
                  >
                    {/* Track Header Label in fixed left gutter */}
                    <div
                      style={{
                        position: "sticky",
                        left: 0,
                        zIndex: 6,
                        width: GUTTER_WIDTH,
                        minWidth: GUTTER_WIDTH,
                        height: "100%",
                        display: "flex",
                        alignItems: "center",
                        padding: "0 8px",
                        backgroundColor: "var(--cs-bg-panel)",
                        borderRight: "1px solid var(--cs-border-subtle)",
                        boxSizing: "border-box",
                        overflow: "hidden",
                      }}
                    >
                      <div style={{ display: "flex", flexDirection: "column", gap: 2, minWidth: 0, overflow: "hidden" }}>
                        <span
                          style={{
                            fontWeight: 600,
                            color: colors.border,
                            fontSize: 11,
                            whiteSpace: "nowrap",
                            overflow: "hidden",
                            textOverflow: "ellipsis",
                          }}
                        >
                          {TRACK_KIND_LABELS[track.kind] || track.kind}
                        </span>
                        <span
                          style={{
                            color: "var(--cs-fg-muted)",
                            fontSize: 10,
                            whiteSpace: "nowrap",
                            overflow: "hidden",
                            textOverflow: "ellipsis",
                          }}
                        >
                          ({track.id})
                        </span>
                      </div>
                    </div>

                    {/* Clips inside this Track */}
                    {track.clips.map((clip, clipIdx) => {
                      const isSelected =
                        selectedClipKey?.trackIdx === trackIdx &&
                        selectedClipKey?.clipIdx === clipIdx;
                      const left = GUTTER_WIDTH + clip.start * pxPerSec;
                      const width = Math.max(20, clip.duration * pxPerSec);

                      return (
                        <div
                          key={clip.id}
                          onClick={(e) => {
                            e.stopPropagation();
                            setSelectedClipKey({ trackIdx, clipIdx });
                          }}
                          title={`[${clip.id}] ${clip.source} (${clip.start}s - ${(clip.start + clip.duration).toFixed(1)}s)`}
                          style={{
                            position: "absolute",
                            left,
                            width,
                            top: 4,
                            bottom: 4,
                            backgroundColor: colors.bg,
                            border: `1px solid ${isSelected ? "var(--cs-focus, #3b82f6)" : colors.border}`,
                            borderRadius: "var(--cs-radius-control)",
                            padding: "4px 8px",
                            cursor: "pointer",
                            overflow: "hidden",
                            display: "flex",
                            flexDirection: "column",
                            justifyContent: "center",
                            boxShadow: isSelected ? "0 0 0 2px var(--cs-focus, #3b82f6)" : "none",
                            userSelect: "none",
                            zIndex: isSelected ? 3 : 2,
                          }}
                        >
                          <div
                            title={clip.id}
                            style={{
                              fontSize: 11,
                              fontWeight: 600,
                              color: colors.text,
                              overflow: "hidden",
                              textOverflow: "ellipsis",
                              whiteSpace: "nowrap",
                            }}
                          >
                            {clip.id}
                          </div>
                          <div
                            title={clip.source}
                            style={{
                              fontSize: 10,
                              color: "var(--cs-fg-muted)",
                              overflow: "hidden",
                              textOverflow: "ellipsis",
                              whiteSpace: "nowrap",
                            }}
                          >
                            {clip.source}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                );
              })}
            </div>

            {/* Playhead Red Vertical Line */}
            <div
              style={{
                position: "absolute",
                left: GUTTER_WIDTH + playhead * pxPerSec,
                top: 0,
                bottom: 0,
                width: 2,
                backgroundColor: "#ff3b30",
                zIndex: 5,
                pointerEvents: "none",
              }}
            >
              <div
                style={{
                  position: "absolute",
                  top: 0,
                  left: -5,
                  width: 0,
                  height: 0,
                  borderLeft: "6px solid transparent",
                  borderRight: "6px solid transparent",
                  borderTop: "8px solid #ff3b30",
                }}
              />
            </div>
          </div>
        </div>

        {/* Right Side Inspector Form for Selected Clip */}
        <aside
          style={{
            width: 280,
            flexShrink: 0,
            borderLeft: "1px solid var(--cs-border-subtle)",
            backgroundColor: "var(--cs-bg-panel)",
            padding: "var(--cs-space-3)",
            overflowY: "auto",
            display: "flex",
            flexDirection: "column",
            gap: "var(--cs-space-3)",
            fontSize: "var(--cs-text-sm)",
          }}
        >
          {selectedClip && selectedTrack ? (
            <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                <span style={{ fontWeight: 600, color: "var(--cs-fg-primary)" }}>
                  Chi tiết đoạn clip
                </span>
                <Badge variant="info">
                  {TRACK_KIND_LABELS[selectedTrack.kind] || selectedTrack.kind}
                </Badge>
              </div>

              {/* Clip ID Field with Validation */}
              <label style={labelStyle}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <span>Mã định danh (ID)</span>
                  {clipErrors.id && (
                    <span style={{ color: "var(--cs-danger)", fontSize: 10, fontWeight: 500 }}>
                      {clipErrors.id}
                    </span>
                  )}
                </div>
                <input
                  type="text"
                  value={selectedClip.id}
                  onChange={(e) => updateSelectedClip((c) => (c.id = e.target.value))}
                  style={{
                    ...inputStyle,
                    borderColor: clipErrors.id ? "var(--cs-danger)" : "var(--cs-border-subtle)",
                  }}
                />
              </label>

              {/* Caption text or source asset path with Validation */}
              <label style={labelStyle}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <span>
                    {selectedTrack.kind === "captions" || selectedTrack.kind === "caption"
                      ? "Nội dung phụ đề (Text)"
                      : "Nguồn media (Source path)"}
                  </span>
                  {clipErrors.source && (
                    <span style={{ color: "var(--cs-danger)", fontSize: 10, fontWeight: 500 }}>
                      {clipErrors.source}
                    </span>
                  )}
                </div>
                {selectedTrack.kind === "captions" || selectedTrack.kind === "caption" ? (
                  <textarea
                    rows={3}
                    value={selectedClip.source}
                    onChange={(e) => updateSelectedClip((c) => (c.source = e.target.value))}
                    style={{
                      ...inputStyle,
                      resize: "vertical",
                      borderColor: clipErrors.source ? "var(--cs-danger)" : "var(--cs-border-subtle)",
                    }}
                  />
                ) : (
                  <input
                    type="text"
                    value={selectedClip.source}
                    onChange={(e) => updateSelectedClip((c) => (c.source = e.target.value))}
                    style={{
                      ...inputStyle,
                      borderColor: clipErrors.source ? "var(--cs-danger)" : "var(--cs-border-subtle)",
                    }}
                  />
                )}
              </label>

              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
                <label style={labelStyle}>
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                    <span>Bắt đầu (s)</span>
                    {clipErrors.start && (
                      <span style={{ color: "var(--cs-danger)", fontSize: 9 }}>!</span>
                    )}
                  </div>
                  <input
                    type="number"
                    step="0.1"
                    min="0"
                    value={selectedClip.start}
                    onChange={(e) => updateSelectedClip((c) => (c.start = parseFloat(e.target.value) || 0))}
                    style={{
                      ...inputStyle,
                      borderColor: clipErrors.start ? "var(--cs-danger)" : "var(--cs-border-subtle)",
                    }}
                  />
                  {clipErrors.start && (
                    <span style={{ color: "var(--cs-danger)", fontSize: 10 }}>{clipErrors.start}</span>
                  )}
                </label>

                <label style={labelStyle}>
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                    <span>Thời lượng (s)</span>
                    {clipErrors.duration && (
                      <span style={{ color: "var(--cs-danger)", fontSize: 9 }}>!</span>
                    )}
                  </div>
                  <input
                    type="number"
                    step="0.1"
                    min="0.1"
                    value={selectedClip.duration}
                    onChange={(e) => updateSelectedClip((c) => (c.duration = parseFloat(e.target.value) || 0))}
                    style={{
                      ...inputStyle,
                      borderColor: clipErrors.duration ? "var(--cs-danger)" : "var(--cs-border-subtle)",
                    }}
                  />
                  {clipErrors.duration && (
                    <span style={{ color: "var(--cs-danger)", fontSize: 10 }}>{clipErrors.duration}</span>
                  )}
                </label>

                <label style={labelStyle}>
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                    <span>Điểm vào (In s)</span>
                    {clipErrors.in_point && (
                      <span style={{ color: "var(--cs-danger)", fontSize: 9 }}>!</span>
                    )}
                  </div>
                  <input
                    type="number"
                    step="0.1"
                    min="0"
                    value={selectedClip.in_point ?? 0}
                    onChange={(e) => updateSelectedClip((c) => (c.in_point = parseFloat(e.target.value) || 0))}
                    style={{
                      ...inputStyle,
                      borderColor: clipErrors.in_point ? "var(--cs-danger)" : "var(--cs-border-subtle)",
                    }}
                  />
                  {clipErrors.in_point && (
                    <span style={{ color: "var(--cs-danger)", fontSize: 10 }}>{clipErrors.in_point}</span>
                  )}
                </label>

                <label style={labelStyle}>
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                    <span>Điểm ra (Out s)</span>
                    {clipErrors.out_point && (
                      <span style={{ color: "var(--cs-danger)", fontSize: 9 }}>!</span>
                    )}
                  </div>
                  <input
                    type="number"
                    step="0.1"
                    min="0"
                    placeholder="Toàn bộ"
                    value={selectedClip.out_point ?? ""}
                    onChange={(e) =>
                      updateSelectedClip((c) => {
                        const val = e.target.value.trim();
                        c.out_point = val === "" ? undefined : parseFloat(val) || 0;
                      })
                    }
                    style={{
                      ...inputStyle,
                      borderColor: clipErrors.out_point ? "var(--cs-danger)" : "var(--cs-border-subtle)",
                    }}
                  />
                  {clipErrors.out_point && (
                    <span style={{ color: "var(--cs-danger)", fontSize: 10 }}>{clipErrors.out_point}</span>
                  )}
                </label>
              </div>

              {/* Actions: Delete Clip */}
              <div style={{ display: "flex", flexDirection: "column", gap: 6, marginTop: 4 }}>
                <Button
                  size="sm"
                  variant="destructive"
                  onClick={handleDeleteSelectedClip}
                  leftIcon={<Icon name="x" size={12} />}
                >
                  Xóa clip này (Delete)
                </Button>
                <div style={{ color: "var(--cs-fg-muted)", fontSize: 11, lineHeight: 1.4 }}>
                  Mẹo: Nhấn phím <b>Delete</b> hoặc <b>Backspace</b> để xóa nhanh clip đang chọn. Nhấn <b>Ctrl+Z</b> để hoàn tác.
                </div>
              </div>
            </div>
          ) : (
            <div style={{ color: "var(--cs-fg-muted)", fontSize: "var(--cs-text-xs)" }}>
              Chọn một đoạn clip trên timeline để xem và chỉnh sửa điểm in/out/start và nội dung phụ đề.
            </div>
          )}
        </aside>
      </div>
    </div>
  );
};

const labelStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 3,
  fontSize: "var(--cs-text-xs)",
  color: "var(--cs-fg-secondary)",
};

const inputStyle: React.CSSProperties = {
  padding: "4px 8px",
  border: "1px solid var(--cs-border-subtle)",
  borderRadius: "var(--cs-radius-control)",
  backgroundColor: "var(--cs-bg-app)",
  color: "var(--cs-fg-primary)",
  fontSize: "var(--cs-text-xs)",
  fontFamily: "var(--cs-font-ui)",
  outline: "none",
};
