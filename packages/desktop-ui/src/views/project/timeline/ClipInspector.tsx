import React from "react";
import { Button, Badge } from "@studio/design-system";
import { Icon } from "../../../components/Icon";
import type { TimelineClip, TimelineTrack } from "../TimelineEditor";

export interface ClipInspectorProps {
  selectedClip: TimelineClip | null;
  selectedTrack: TimelineTrack | null;
  clipErrors: Record<string, string>;
  updateSelectedClip: (updater: (clip: TimelineClip) => void) => void;
  onDeleteSelectedClip: () => void;
}

const TRACK_KIND_LABELS: Record<string, string> = {
  video: "Video",
  audio: "Âm thanh",
  captions: "Phụ đề",
  caption: "Phụ đề",
};

export const ClipInspector: React.FC<ClipInspectorProps> = ({
  selectedClip,
  selectedTrack,
  clipErrors,
  updateSelectedClip,
  onDeleteSelectedClip,
}) => {
  return (
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
              onClick={onDeleteSelectedClip}
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
