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

  // Zoom: pixels per second on the timeline lanes
  const pxPerSec = 50;
  const timelineWidth = Math.max(600, totalDuration * pxPerSec);

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
    const x = clientX - containerRect.left + scrollLeft;
    const time = Math.max(0, Math.min(totalDuration, x / pxPerSec));
    setPlayhead(time);
    if (videoRef.current) {
      videoRef.current.currentTime = time;
    }
  };

  // Mutate clip in doc and notify parent
  const updateSelectedClip = (updater: (clip: TimelineClip) => void) => {
    if (!doc || !selectedClipKey) return;
    const { trackIdx, clipIdx } = selectedClipKey;
    const newDoc = JSON.parse(JSON.stringify(doc)) as TimelineDoc;
    const targetClip = newDoc.tracks[trackIdx]?.clips[clipIdx];
    if (!targetClip) return;

    updater(targetClip);

    const valid = validateTimelineDoc(newDoc);
    setDoc(newDoc);
    onChange(JSON.stringify(newDoc, null, 2), valid);
  };

  const selectedClip = React.useMemo(() => {
    if (!doc || !selectedClipKey) return null;
    const { trackIdx, clipIdx } = selectedClipKey;
    return doc.tracks[trackIdx]?.clips[clipIdx] || null;
  }, [doc, selectedClipKey]);

  const selectedTrack = React.useMemo(() => {
    if (!doc || !selectedClipKey) return null;
    return doc.tracks[selectedClipKey.trackIdx] || null;
  }, [doc, selectedClipKey]);

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
                gap: 8,
                color: "var(--cs-fg-muted)",
                fontSize: "var(--cs-text-xs)",
              }}
            >
              <Icon name="video" size={36} color="var(--cs-fg-muted)" />
              <span>Chưa có bản render xem trước (bấm Render ở trên để xuất MP4)</span>
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

      {/* Middle Controls Bar */}
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

        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
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
          {/* Timeline Inner Canvas with Width Based on Duration */}
          <div style={{ width: timelineWidth, position: "relative", minHeight: "100%" }}>
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
              {Array.from({ length: totalDuration + 1 }).map((_, sec) => (
                <div
                  key={sec}
                  style={{
                    position: "absolute",
                    left: sec * pxPerSec,
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
                    {/* Track Header Label */}
                    <div
                      style={{
                        position: "sticky",
                        left: 0,
                        zIndex: 4,
                        display: "inline-flex",
                        alignItems: "center",
                        gap: 4,
                        padding: "2px 8px",
                        backgroundColor: "var(--cs-bg-panel)",
                        borderRight: "1px solid var(--cs-border-subtle)",
                        borderRadius: "0 var(--cs-radius-control) var(--cs-radius-control) 0",
                        fontSize: 11,
                        fontWeight: 600,
                        color: colors.border,
                      }}
                    >
                      <span>{TRACK_KIND_LABELS[track.kind] || track.kind}</span>
                      <span style={{ color: "var(--cs-fg-muted)", fontSize: 10 }}>({track.id})</span>
                    </div>

                    {/* Clips inside this Track */}
                    {track.clips.map((clip, clipIdx) => {
                      const isSelected =
                        selectedClipKey?.trackIdx === trackIdx &&
                        selectedClipKey?.clipIdx === clipIdx;
                      const left = clip.start * pxPerSec;
                      const width = Math.max(20, clip.duration * pxPerSec);

                      return (
                        <div
                          key={clip.id}
                          onClick={(e) => {
                            e.stopPropagation();
                            setSelectedClipKey({ trackIdx, clipIdx });
                          }}
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
                left: playhead * pxPerSec,
                top: 0,
                bottom: 0,
                width: 2,
                backgroundColor: "#ff3b30",
                zIndex: 10,
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

              <label style={labelStyle}>
                Mã định danh (ID)
                <input
                  type="text"
                  value={selectedClip.id}
                  onChange={(e) => updateSelectedClip((c) => (c.id = e.target.value))}
                  style={inputStyle}
                />
              </label>

              {/* Caption text or source asset path */}
              <label style={labelStyle}>
                {selectedTrack.kind === "captions" || selectedTrack.kind === "caption"
                  ? "Nội dung phụ đề (Text)"
                  : "Nguồn media (Source path)"}
                {selectedTrack.kind === "captions" || selectedTrack.kind === "caption" ? (
                  <textarea
                    rows={3}
                    value={selectedClip.source}
                    onChange={(e) => updateSelectedClip((c) => (c.source = e.target.value))}
                    style={{ ...inputStyle, resize: "vertical" }}
                  />
                ) : (
                  <input
                    type="text"
                    value={selectedClip.source}
                    onChange={(e) => updateSelectedClip((c) => (c.source = e.target.value))}
                    style={inputStyle}
                  />
                )}
              </label>

              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
                <label style={labelStyle}>
                  Bắt đầu (s)
                  <input
                    type="number"
                    step="0.1"
                    min="0"
                    value={selectedClip.start}
                    onChange={(e) => updateSelectedClip((c) => (c.start = Math.max(0, parseFloat(e.target.value) || 0)))}
                    style={inputStyle}
                  />
                </label>

                <label style={labelStyle}>
                  Thời lượng (s)
                  <input
                    type="number"
                    step="0.1"
                    min="0.1"
                    value={selectedClip.duration}
                    onChange={(e) => updateSelectedClip((c) => (c.duration = Math.max(0.1, parseFloat(e.target.value) || 0.1)))}
                    style={inputStyle}
                  />
                </label>

                <label style={labelStyle}>
                  Điểm vào (In s)
                  <input
                    type="number"
                    step="0.1"
                    min="0"
                    value={selectedClip.in_point ?? 0}
                    onChange={(e) => updateSelectedClip((c) => (c.in_point = Math.max(0, parseFloat(e.target.value) || 0)))}
                    style={inputStyle}
                  />
                </label>

                <label style={labelStyle}>
                  Điểm ra (Out s)
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
                    style={inputStyle}
                  />
                </label>
              </div>

              <div style={{ color: "var(--cs-fg-muted)", fontSize: "var(--cs-text-xs)", lineHeight: 1.5 }}>
                Chỉnh sửa thông số sẽ tự động cập nhật timeline và lưu khi bấm nút Lưu.
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
