import React from "react";
import type { ThreadMessage } from "../../../runtime/chat/turn-reducer";

/** Seconds since `since`, ticking every second while mounted. */
function useElapsedSeconds(since: number | undefined, active: boolean): number {
  const [now, setNow] = React.useState(() => Date.now());
  React.useEffect(() => {
    if (!active) return;
    const id = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(id);
  }, [active]);
  return since === undefined ? 0 : Math.max(0, Math.round((now - since) / 1000));
}

export interface ThinkingIndicatorProps {
  msg: ThreadMessage;
}

/**
 * Thinking slot of an agent message. Neither CLI hands over thinking text headless (Claude Code
 * redacts it, agy does not report it), so absence is the normal case: show that the agent is
 * working and for how long, and the text only when the engine actually sent some.
 */
export const ThinkingIndicator: React.FC<ThinkingIndicatorProps> = ({ msg }) => {
  const working =
    Boolean(msg.isStreaming) && (Boolean(msg.thinkingActive) || (!msg.text && !(msg.toolCalls?.length)));
  const since = msg.thinkingActive ? msg.thinkingStartedAt ?? msg.startedAt : msg.startedAt;
  const elapsed = useElapsedSeconds(since, working);

  if (working) {
    return (
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          marginBottom: msg.text || msg.toolCalls?.length ? 8 : 0,
          fontSize: "var(--cs-text-xs)",
          color: "var(--cs-fg-secondary)",
        }}
      >
        <span className="cs-thinking-dot" aria-hidden />
        <span>{msg.thinkingActive || msg.thoughtSeen ? "Đang suy nghĩ" : "Đang xử lý"}…</span>
        {since !== undefined && elapsed > 0 && <span style={{ color: "var(--cs-fg-muted)" }}>{elapsed}s</span>}
        {msg.thinking && (
          <span
            style={{
              flex: 1,
              minWidth: 0,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
              color: "var(--cs-fg-muted)",
              fontStyle: "italic",
            }}
          >
            {msg.thinking.trim().split("\n").pop()}
          </span>
        )}
      </div>
    );
  }

  if (!msg.thoughtSeen && !msg.thinking) return null;
  const seconds = msg.thinkingMs ? Math.max(1, Math.round(msg.thinkingMs / 1000)) : 0;
  const tokens = msg.usage?.reasoningTokens ?? 0;
  const label = [
    seconds ? `Đã suy nghĩ ${seconds}s` : "Đã suy nghĩ",
    tokens ? `${tokens.toLocaleString()} token` : "",
  ]
    .filter(Boolean)
    .join(" · ");

  const summaryStyle: React.CSSProperties = {
    fontSize: "var(--cs-text-xs)",
    color: "var(--cs-fg-muted)",
    marginBottom: 8,
  };
  if (!msg.thinking) {
    return (
      <div style={summaryStyle} title="Engine không gửi nội dung suy nghĩ ở chế độ chạy nền (Claude Code che nội dung này).">
        {label}
      </div>
    );
  }
  return (
    <details style={{ ...summaryStyle, color: "var(--cs-fg-secondary)" }}>
      <summary style={{ cursor: "pointer", color: "var(--cs-fg-muted)" }}>{label}</summary>
      <div
        style={{
          marginTop: 4,
          padding: "6px 8px",
          borderLeft: "2px solid var(--cs-border-strong)",
          whiteSpace: "pre-wrap",
          fontStyle: "italic",
        }}
      >
        {msg.thinking}
      </div>
    </details>
  );
};
