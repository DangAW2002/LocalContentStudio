import React from "react";
import { Icon } from "./Icon";
import type { QueuedSend } from "../runtime/chat/send-queue";

/** How many queued items show before the stack scrolls (upstream: five and a half cards). */
const VISIBLE_ITEMS = 4.5;
const ITEM_HEIGHT_PX = 34;

export interface QueuedSendStackProps {
  items: QueuedSend[];
  paused: boolean;
  onSendNow: (id: string) => void;
  onEdit: (id: string) => void;
  onRemove: (id: string) => void;
  onMove: (id: string, delta: -1 | 1) => void;
  onResume: () => void;
}

/** Queued prompts above the composer. Layout inspired by open-design's QueuedSendStack. */
export const QueuedSendStack: React.FC<QueuedSendStackProps> = ({
  items,
  paused,
  onSendNow,
  onEdit,
  onRemove,
  onMove,
  onResume,
}) => {
  if (items.length === 0) return null;

  const iconButton = (label: string, icon: string, onClick: () => void, disabled = false) => (
    <button
      type="button"
      title={label}
      aria-label={label}
      disabled={disabled}
      onClick={onClick}
      style={{
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        width: 22,
        height: 22,
        padding: 0,
        border: "none",
        background: "none",
        color: disabled ? "var(--cs-fg-muted)" : "var(--cs-fg-secondary)",
        opacity: disabled ? 0.4 : 1,
        cursor: disabled ? "default" : "pointer",
      }}
    >
      <Icon name={icon} size={13} />
    </button>
  );

  return (
    <div
      style={{
        margin: "0 var(--cs-space-4)",
        border: "1px solid var(--cs-border-subtle)",
        borderBottom: "none",
        backgroundColor: "var(--cs-bg-raised)",
        fontSize: "var(--cs-text-xs)",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "4px 8px",
          color: "var(--cs-fg-muted)",
          borderBottom: "1px solid var(--cs-border-subtle)",
        }}
      >
        <span>
          {paused
            ? `Hàng chờ tạm dừng (${items.length})`
            : `Sẽ gửi khi agent xong (${items.length})`}
        </span>
        {paused && (
          <button
            type="button"
            onClick={onResume}
            style={{ border: "none", background: "none", color: "var(--cs-accent)", cursor: "pointer", font: "inherit" }}
          >
            Tiếp tục
          </button>
        )}
      </div>
      <div style={{ maxHeight: VISIBLE_ITEMS * ITEM_HEIGHT_PX, overflowY: "auto" }}>
        {items.map((item, index) => (
          <div
            key={item.id}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 4,
              minHeight: ITEM_HEIGHT_PX,
              padding: "0 4px 0 8px",
              borderTop: index > 0 ? "1px solid var(--cs-border-subtle)" : undefined,
            }}
          >
            <span
              title={item.prompt}
              style={{
                flex: 1,
                minWidth: 0,
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
                color: "var(--cs-fg-primary)",
              }}
            >
              {item.prompt}
            </span>
            <button
              type="button"
              onClick={() => onSendNow(item.id)}
              title="Dừng lượt đang chạy và gửi tin này ngay"
              style={{
                padding: "2px 8px",
                border: "1px solid var(--cs-border-strong)",
                borderRadius: "var(--cs-radius-sm)",
                background: "var(--cs-bg-panel)",
                color: "var(--cs-fg-primary)",
                font: "inherit",
                cursor: "pointer",
                whiteSpace: "nowrap",
              }}
            >
              Gửi ngay
            </button>
            {iconButton("Lên", "chev-up", () => onMove(item.id, -1), index === 0)}
            {iconButton("Xuống", "chev-down", () => onMove(item.id, 1), index === items.length - 1)}
            {iconButton("Sửa", "writing", () => onEdit(item.id))}
            {iconButton("Xoá", "trash", () => onRemove(item.id))}
          </div>
        ))}
      </div>
    </div>
  );
};
