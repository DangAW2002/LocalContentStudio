import React from "react";
import type { ThreadMessage } from "../../../runtime/chat/turn-reducer";
import { describeActivity } from "../../../runtime/chat/activity-label";

/** Seconds since `since`, ticking every second while mounted and active. */
function useElapsedSeconds(since: number | undefined, active: boolean): number {
  const [now, setNow] = React.useState(() => Date.now());
  React.useEffect(() => {
    if (!active || since === undefined) return;
    const id = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(id);
  }, [active, since]);
  return since === undefined ? 0 : Math.max(0, Math.round((now - since) / 1000));
}

export interface ThinkingIndicatorProps {
  msg: ThreadMessage;
}

/**
 * Live activity indicator and thinking summary of an agent message.
 * While streaming: shows a single line with blinking dot, activity label, elapsed time, and step count.
 * When done: displays the "Đã suy nghĩ..." summary if thinking was recorded.
 */
export const ThinkingIndicator: React.FC<ThinkingIndicatorProps> = ({ msg }) => {
  const isStreaming = Boolean(msg.isStreaming);
  const elapsed = useElapsedSeconds(msg.startedAt, isStreaming);

  if (isStreaming) {
    const toolCalls = msg.toolCalls || [];
    const lastTool = toolCalls.length > 0 ? toolCalls[toolCalls.length - 1] : undefined;
    const activeTool = lastTool && lastTool.output === undefined && !lastTool.isError ? lastTool : undefined;

    let label: string;
    if (activeTool) {
      label = describeActivity(activeTool.name, activeTool.input);
    } else if (msg.thinkingActive) {
      label = "Đang suy nghĩ";
    } else if (msg.text) {
      label = "Đang viết trả lời";
    } else {
      const isPlaceholder =
        !msg.text &&
        toolCalls.length === 0 &&
        !msg.usage &&
        !msg.thoughtSeen &&
        !(msg.fileChanges && msg.fileChanges.length > 0);
      label = isPlaceholder ? "Đang khởi động agent" : "Đang xử lý";
    }

    const elapsedText = msg.startedAt !== undefined ? `${elapsed}s` : undefined;
    const stepCount = toolCalls.length;
    const stepsText = stepCount > 0 ? `${stepCount} bước` : undefined;

    const hasSubsequentContent = Boolean(
      msg.text ||
      toolCalls.length > 0 ||
      msg.errorMessage ||
      (msg.fileChanges && msg.fileChanges.length > 0)
    );

    return (
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          marginBottom: hasSubsequentContent ? 8 : 0,
          fontSize: "var(--cs-text-xs)",
          color: "var(--cs-fg-secondary)",
          whiteSpace: "nowrap",
          overflow: "hidden",
          textOverflow: "ellipsis",
        }}
      >
        <span className="cs-thinking-dot" aria-hidden />
        <span style={{ fontWeight: 500 }}>{label}</span>
        {elapsedText && <span style={{ color: "var(--cs-fg-muted)" }}>· {elapsedText}</span>}
        {stepsText && <span style={{ color: "var(--cs-fg-muted)" }}>· {stepsText}</span>}
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
