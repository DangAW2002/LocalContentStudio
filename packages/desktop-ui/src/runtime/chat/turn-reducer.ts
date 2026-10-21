import type { EngineTurnEvent } from "../../api/types";

export interface ThreadMessage {
  id: string;
  role: "user" | "agent";
  text: string;
  thinking?: string;
  /** Live turns only (history has no timing): when the agent message started. */
  startedAt?: number;
  /** A thinking block is open (Claude reports thinking even when its text is redacted). */
  thinkingActive?: boolean;
  thinkingStartedAt?: number;
  /** Accumulated thinking time in ms (live turns only). */
  thinkingMs?: number;
  /** The engine reported thinking at some point in this turn. */
  thoughtSeen?: boolean;
  toolCalls?: Array<{
    id: string;
    name: string;
    input: unknown;
    output?: string;
    isError?: boolean;
    expanded?: boolean;
  }>;
  fileChanges?: Array<{ path: string; op: "create" | "modify" | "delete" }>;
  permissionRequests?: Array<{ requestId: string; tool: string; input: unknown; resolved?: boolean }>;
  deniedActions?: Array<{ tool: string; input: unknown }>;
  usage?: {
    inputTokens: number;
    outputTokens: number;
    cacheReadTokens: number;
    cacheWriteTokens: number;
    reasoningTokens: number;
    costUsd?: number | null;
    estimatedCostUsd?: number | null;
  };
  turnId?: string;
  isStreaming?: boolean;
  errorMessage?: string;
  pinnedWarning?: { artifactId: string; checkpointId: string };
  model?: string;
}

export interface ToolCallItem {
  id: string;
  name: string;
  input: unknown;
  output?: string;
  isError?: boolean;
  expanded?: boolean;
}

export interface ToolCallGroup {
  key: string;
  name: string;
  calls: ToolCallItem[];
  errorCount: number;
  hasErrors: boolean;
}

export function groupConsecutiveToolCalls(calls: ToolCallItem[], prefix: string): ToolCallGroup[] {
  const groups: ToolCallGroup[] = [];
  for (let i = 0; i < calls.length; i++) {
    const call = calls[i];
    const prev = groups[groups.length - 1];
    if (prev && prev.name === call.name) {
      prev.calls.push(call);
      if (call.isError) {
        prev.errorCount++;
        prev.hasErrors = true;
      }
    } else {
      groups.push({
        key: `${prefix}-tcgroup-${groups.length}-${call.name}`,
        name: call.name,
        calls: [call],
        errorCount: call.isError ? 1 : 0,
        hasErrors: !!call.isError,
      });
    }
  }
  return groups;
}

export function isMcpLoadFailure(text?: string): boolean {
  if (!text) return false;
  const lower = text.toLowerCase();
  return (
    lower.includes("failed to load") ||
    lower.includes("studio chưa chạy") ||
    text.includes("Studio chưa chạy")
  );
}

/**
 * Pure reducer: fold one turn event into the message list. Used for the live stream and for history.
 * Every agent-side event lands in the agent message of its turn (created on demand).
 */
export function applyTurnEvent(prev: ThreadMessage[], payload: EngineTurnEvent, now?: number): ThreadMessage[] {
  const event = payload.event;
  if (event.type === "UserPrompt") {
    const last = prev[prev.length - 1];
    // The UI already appended the prompt optimistically: attach the turn id instead of duplicating.
    if (last && last.role === "user" && !last.turnId && last.text === event.text) {
      return [...prev.slice(0, -1), { ...last, turnId: payload.turnId }];
    }
    return [...prev, { id: `user-${payload.turnId}`, role: "user", text: event.text, turnId: payload.turnId }];
  }
  if (event.type === "Session") return prev;

  let list = prev;
  let idx = list.findIndex((m) => m.role === "agent" && m.turnId === payload.turnId);
  if (idx === -1) {
    list = [...list, { id: `turn-${payload.turnId}`, role: "agent", text: "", turnId: payload.turnId, isStreaming: true, startedAt: now }];
    idx = list.length - 1;
  }
  let msg = list[idx];
  // Any non-thinking event closes an open thinking block.
  if (msg.thinkingActive && event.type !== "Thinking" && event.type !== "Usage") {
    const elapsed = now !== undefined && msg.thinkingStartedAt !== undefined ? now - msg.thinkingStartedAt : 0;
    msg = { ...msg, thinkingActive: false, thinkingStartedAt: undefined, thinkingMs: (msg.thinkingMs ?? 0) + elapsed };
  }
  let next: ThreadMessage = msg;
  switch (event.type) {
    case "TextDelta":
      next = { ...msg, text: msg.text + event.text };
      break;
    case "Text": {
      const pinnedWarning = (event as any).pinnedArtifactWarning;
      next = { ...msg, text: event.text, ...(pinnedWarning ? { pinnedWarning } : {}) };
      break;
    }
    case "Thinking":
      next = {
        ...msg,
        thinking: event.text ? (msg.thinking || "") + event.text : msg.thinking,
        thoughtSeen: true,
        thinkingActive: true,
        thinkingStartedAt: msg.thinkingStartedAt ?? now,
      };
      break;
    case "ToolCall":
      next = { ...msg, toolCalls: [...(msg.toolCalls || []), { id: event.id, name: event.name, input: event.input, expanded: false }] };
      break;
    case "ToolResult":
      next = {
        ...msg,
        toolCalls: (msg.toolCalls || []).map((tc) =>
          tc.id === event.id ? { ...tc, output: event.outputPreview, isError: event.isError } : tc
        ),
      };
      break;
    case "FileChange":
      next = { ...msg, fileChanges: [...(msg.fileChanges || []), { path: event.path, op: event.op }] };
      break;
    case "PermissionRequest":
      next = { ...msg, permissionRequests: [...(msg.permissionRequests || []), { requestId: event.requestId, tool: event.tool, input: event.input }] };
      break;
    case "PermissionDenied":
      next = { ...msg, deniedActions: [...(msg.deniedActions || []), { tool: event.tool, input: event.input }] };
      break;
    case "Usage":
      next = {
        ...msg,
        usage: {
          inputTokens: event.inputTokens,
          outputTokens: event.outputTokens,
          cacheReadTokens: event.cacheReadTokens,
          // Older persisted events predate these fields.
          cacheWriteTokens: event.cacheWriteTokens ?? 0,
          reasoningTokens: event.reasoningTokens ?? 0,
          costUsd: event.costUsd,
          estimatedCostUsd: event.estimatedCostUsd,
        },
      };
      break;
    case "Done": {
      const isErrorStatus = event.status === "error" || event.status === "failed";
      let err = msg.errorMessage;
      if (isErrorStatus && !msg.text && !event.text) {
        err = err || "Lượt xử lý thất bại mà không có nội dung phản hồi từ engine.";
      }
      next = {
        ...msg,
        isStreaming: false,
        text: msg.text || event.text || "",
        errorMessage: err,
      };
      break;
    }
    case "Error":
      next = {
        ...msg,
        isStreaming: false,
        errorMessage: event.message,
      };
      break;
  }
  const out = list.slice();
  out[idx] = next;
  return out;
}
