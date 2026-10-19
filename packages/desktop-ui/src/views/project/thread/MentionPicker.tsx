import React from "react";
import { Icon } from "../../../components/Icon";

export interface MentionPickerItem {
  key: string;
  insert: string;
  label: string;
  meta?: string;
}

export interface MentionPickerProps {
  pickerMode: "@" | "/" | null;
  pickerIndex: number;
  pickerItems: MentionPickerItem[];
  pickerQuery: string;
  pickerListRef: React.RefObject<HTMLDivElement>;
  onSelect: (itemText: string) => void;
  onHoverIndex: (index: number) => void;
}

export const MentionPicker: React.FC<MentionPickerProps> = ({
  pickerMode,
  pickerIndex,
  pickerItems,
  pickerQuery,
  pickerListRef,
  onSelect,
  onHoverIndex,
}) => {
  if (!pickerMode) return null;

  return (
    <div
      style={{
        position: "absolute",
        bottom: 120,
        left: "var(--cs-space-4)",
        right: "var(--cs-space-4)",
        backgroundColor: "var(--cs-bg-panel)",
        border: "1px solid var(--cs-border-strong)",
        boxShadow: "var(--cs-shadow-overlay)",
        borderRadius: "var(--cs-radius-control)",
        maxHeight: 180,
        overflowY: "auto",
        zIndex: 15,
        padding: "4px 0",
      }}
    >
      <div style={{ padding: "4px 10px", fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)", borderBottom: "1px solid var(--cs-border-subtle)" }}>
        {pickerMode === "@" ? "Chọn tệp trong dự án:" : "Chọn quy trình playbook:"}
      </div>
      <div ref={pickerListRef} role="listbox" aria-label="Gợi ý">
        {pickerItems.length === 0 ? (
          <div style={{ padding: "8px 10px", fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
            {pickerQuery
              ? `Không có kết quả cho "${pickerQuery}"`
              : pickerMode === "@"
                ? "Chưa có tệp nào trong dự án"
                : "Chưa có playbook nào trong scope chain"}
          </div>
        ) : (
          pickerItems.map((item, i) => (
            <div
              key={item.key}
              role="option"
              data-index={i}
              aria-selected={i === pickerIndex}
              // Keep focus in the textarea so typing continues to filter.
              onMouseDown={(e) => e.preventDefault()}
              onClick={() => onSelect(item.insert)}
              onMouseMove={() => i !== pickerIndex && onHoverIndex(i)}
              style={{
                padding: "6px 10px",
                cursor: "pointer",
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                gap: 6,
                fontSize: "var(--cs-text-xs)",
                fontFamily: pickerMode === "@" ? "var(--cs-font-mono)" : undefined,
                backgroundColor: i === pickerIndex ? "var(--cs-selection)" : "transparent",
              }}
            >
              <span style={{ display: "flex", alignItems: "center", gap: 6, minWidth: 0 }}>
                <Icon name={pickerMode === "@" ? "file" : "sparkles"} size={13} color="var(--cs-accent)" />
                <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", fontWeight: pickerMode === "/" ? 600 : undefined }}>
                  {item.label}
                </span>
              </span>
              {item.meta && <span style={{ color: "var(--cs-fg-muted)", fontSize: 11 }}>{item.meta}</span>}
            </div>
          ))
        )}
      </div>
      <div style={{ padding: "4px 10px", fontSize: 11, color: "var(--cs-fg-muted)", borderTop: "1px solid var(--cs-border-subtle)" }}>
        ↑↓ chọn · Enter/Tab chèn · Esc đóng
      </div>
    </div>
  );
};
