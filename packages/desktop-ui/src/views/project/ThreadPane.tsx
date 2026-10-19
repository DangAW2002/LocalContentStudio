import React from "react";
import { useStickToBottom } from "../../runtime/chat/useStickToBottom";
import { renderMarkdown } from "../../runtime/markdown";
import {
  loadQueue,
  saveQueue,
  newQueuedSend,
  removeQueued,
  prioritizeQueued,
  moveQueued,
  type QueuedSend,
} from "../../runtime/chat/send-queue";
import { QueuedSendStack } from "../../components/QueuedSendStack";
import { Icon } from "../../components/Icon";
import { Button, Badge, Callout, Kbd, Dialog } from "@studio/design-system";
import { studioApi } from "../../api/client";
import type {
  EngineTurnEvent,
  ProjectFileEntry,
  PublishRequest,
  ScopeItem,
  ThreadSummary,
} from "../../api/types";
import { PublishDialog } from "../../components/PublishDialog";

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
}

export interface ThreadPaneProps {
  projectSlug: string;
  activeThread?: ThreadSummary;
  projectFiles: ProjectFileEntry[];
  playbooks: ScopeItem[];
  onSelectFile: (path: string) => void;
  pendingInsert?: string | null;
  onInsertConsumed?: () => void;
  onNavigateSettings?: (tab?: string) => void;
  onUndoTurnSuccess?: () => void;
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
    case "Text":
      next = { ...msg, text: event.text };
      break;
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

export const ThreadPane: React.FC<ThreadPaneProps> = ({
  projectSlug,
  activeThread,
  projectFiles,
  playbooks,
  onSelectFile,
  pendingInsert,
  onInsertConsumed,
  onNavigateSettings,
  onUndoTurnSuccess,
}) => {
  const [messages, setMessages] = React.useState<ThreadMessage[]>([]);
  const [inputPrompt, setInputPrompt] = React.useState("");
  const [runningTurnId, setRunningTurnId] = React.useState<string | null>(null);
  // True between pressing Send and the backend accepting (or rejecting) the turn.
  const [sending, setSending] = React.useState(false);
  // Turns whose Done arrived, possibly before startTurn resolved (very short turns).
  const finishedTurnsRef = React.useRef<Set<string>>(new Set());
  // Prompts typed while the agent is busy (see runtime/chat/send-queue.ts).
  const [queue, setQueue] = React.useState<QueuedSend[]>([]);
  const queueRef = React.useRef<QueuedSend[]>([]);
  const [queuePaused, setQueuePaused] = React.useState(false);
  // The queue only drains once the thread's history is loaded, so it never races the reload.
  const [historyReadyFor, setHistoryReadyFor] = React.useState<string | null>(null);
  const drainingRef = React.useRef<string | null>(null);
  // Bumped when the web event stream lost events and history must be reloaded.
  const [historyReloadTick, setHistoryReloadTick] = React.useState(0);
  const [pickerMode, setPickerMode] = React.useState<"@" | "/" | null>(null);
  const [pickerIndex, setPickerIndex] = React.useState(0);
  // Index of the trigger character ("@" or "/") in the prompt; the query is what follows it.
  const [pickerStart, setPickerStart] = React.useState(0);
  const pickerListRef = React.useRef<HTMLDivElement>(null);
  const [undoTurnTarget, setUndoTurnTarget] = React.useState<string | null>(null);
  const [undoing, setUndoing] = React.useState(false);
  const [undoToast, setUndoToast] = React.useState<string | null>(null);
  const [error, setError] = React.useState<string | null>(null);
  const [pendingPublishRequests, setPendingPublishRequests] = React.useState<PublishRequest[]>([]);
  const [selectedPublishRequest, setSelectedPublishRequest] = React.useState<PublishRequest | null>(null);
  const [publishDialogOpen, setPublishDialogOpen] = React.useState(false);
  const [expandedToolGroups, setExpandedToolGroups] = React.useState<Set<string>>(new Set());

  const toggleToolGroup = (groupKey: string) => {
    setExpandedToolGroups((prev) => {
      const next = new Set(prev);
      if (next.has(groupKey)) {
        next.delete(groupKey);
      } else {
        next.add(groupKey);
      }
      return next;
    });
  };

  const handleGoToEngines = () => {
    if (onNavigateSettings) {
      onNavigateSettings("engines");
    } else {
      window.dispatchEvent(
        new CustomEvent("studio:navigate", {
          detail: { area: "settings", tab: "engines" },
        })
      );
    }
  };

  const loadPublishRequests = React.useCallback(async () => {
    if (!projectSlug) return;
    try {
      const list = await studioApi.publishList(projectSlug);
      const awaiting = list.filter((r) => r.status === "awaiting_confirmation");
      setPendingPublishRequests(awaiting);
    } catch {
      // ignore
    }
  }, [projectSlug]);

  React.useEffect(() => {
    loadPublishRequests();
  }, [loadPublishRequests]);

  const scroll = useStickToBottom(activeThread?.id);
  const textareaRef = React.useRef<HTMLTextAreaElement>(null);

  // Handle pending text insertion from StagePane (e.g. Ask AI about layer)
  React.useEffect(() => {
    if (pendingInsert) {
      setInputPrompt((prev) => {
        const separator = prev.length > 0 && !prev.endsWith(" ") && !prev.endsWith("\n") ? " " : "";
        return prev + separator + pendingInsert;
      });
      onInsertConsumed?.();
      setTimeout(() => {
        if (textareaRef.current) {
          textareaRef.current.focus();
          textareaRef.current.selectionStart = textareaRef.current.value.length;
          textareaRef.current.selectionEnd = textareaRef.current.value.length;
        }
      }, 0);
    }
  }, [pendingInsert, onInsertConsumed]);

  // Load persisted history of the active thread (same reducer as the live stream).
  React.useEffect(() => {
    let cancelled = false;
    setMessages([]);
    setRunningTurnId(null);
    setHistoryReadyFor(null);
    setQueuePaused(false);
    const restored = activeThread ? loadQueue(activeThread.id) : [];
    queueRef.current = restored;
    setQueue(restored);
    if (!activeThread) return;
    (async () => {
      try {
        let folded: ThreadMessage[] = [];
        let cursor: string | undefined;
        for (let page = 0; page < 50; page++) {
          const res = await studioApi.getThreadEvents({ threadId: activeThread.id, cursor, limit: 500 });
          for (const ev of res.events) folded = applyTurnEvent(folded, ev);
          if (!res.hasMore || !res.nextCursor) break;
          cursor = res.nextCursor;
        }
        if (cancelled) return;
        // A turn without Done/Error is still running (e.g. the thread was reopened mid-turn):
        // keep it streaming so live events continue to land in it and Send stays locked.
        const live = folded.find((m) => m.role === "agent" && m.isStreaming);
        setRunningTurnId(live?.turnId ?? null);
        setMessages(folded);
        setHistoryReadyFor(activeThread.id);
      } catch (err: any) {
        if (!cancelled) setError(err?.message || "Không tải được lịch sử hội thoại.");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [activeThread?.id, historyReloadTick]);

  React.useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    studioApi
      .onResync(() => setHistoryReloadTick((n) => n + 1))
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => {});
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  // Listen to turn-event. Keyed on the thread id (not the object) so re-renders don't resubscribe.
  const loadPublishRequestsRef = React.useRef(loadPublishRequests);
  loadPublishRequestsRef.current = loadPublishRequests;
  const activeThreadId = activeThread?.id;
  React.useEffect(() => {
    if (!activeThreadId) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;

    studioApi
      .onTurnEvent((payload) => {
        if (disposed || payload.threadId !== activeThreadId) return;
        setMessages((prev) => applyTurnEvent(prev, payload, Date.now()));
        if (payload.event.type === "Done") {
          finishedTurnsRef.current.add(payload.turnId);
          setRunningTurnId((cur) => (cur === null || cur === payload.turnId ? null : cur));
          loadPublishRequestsRef.current();
        }
        if (payload.event.type === "Error") {
          setError(payload.event.message);
        }
      })
      .then((fn) => {
        // The effect may have been cleaned up while registration was in flight.
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => {
        // Core absent
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [activeThreadId]);

  const commitQueue = (threadId: string, next: QueuedSend[]) => {
    queueRef.current = next;
    setQueue(next);
    saveQueue(threadId, next);
  };

  /** Start one turn. Returns false when the backend rejected it (bubble removed, error shown). */
  const startPrompt = async (userText: string): Promise<boolean> => {
    if (!activeThread) return false;
    setError(null);
    setSending(true);
    scroll.jumpToLatest();

    // Append user message
    const userMsgId = `user-${Date.now()}`;
    setMessages((prev) => [
      ...prev,
      { id: userMsgId, role: "user", text: userText },
    ]);

    try {
      const res = await studioApi.startTurn({
        threadId: activeThread.id,
        prompt: userText,
      });
      if (!finishedTurnsRef.current.has(res.turnId)) setRunningTurnId(res.turnId);
      return true;
    } catch (err: any) {
      // Rejected before the backend recorded a turn (no UserPrompt echoed back): drop the
      // optimistic bubble.
      setMessages((prev) => prev.filter((m) => !(m.id === userMsgId && !m.turnId)));
      setError(err?.message || "Không thể bắt đầu lượt agent.");
      return false;
    } finally {
      setSending(false);
    }
  };

  // Handle send message: run now when idle, otherwise queue it instead of refusing.
  const handleSend = async () => {
    const userText = inputPrompt.trim();
    if (!userText || !activeThread) return;
    setInputPrompt("");
    setPickerMode(null);

    if (runningTurnId || sending) {
      commitQueue(activeThread.id, [...queueRef.current, newQueuedSend(userText)]);
      return;
    }
    const started = await startPrompt(userText);
    if (started) setQueuePaused(false);
    else setInputPrompt((cur) => cur || userText);
  };

  // Drain: when the thread goes idle, start the head of the queue. A failed start keeps the item
  // and pauses the queue instead of retrying in a loop.
  React.useEffect(() => {
    const threadId = activeThread?.id;
    if (!threadId || historyReadyFor !== threadId) return;
    if (runningTurnId || sending || queuePaused || drainingRef.current) return;
    const next = queue[0];
    if (!next) return;
    drainingRef.current = next.id;
    void (async () => {
      const started = await startPrompt(next.prompt);
      drainingRef.current = null;
      if (started) commitQueue(threadId, removeQueued(queueRef.current, next.id));
      else setQueuePaused(true);
    })();
    // startPrompt reads the latest render's state; the deps below are the triggers.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeThread?.id, historyReadyFor, runningTurnId, sending, queuePaused, queue]);

  /** "Gửi ngay": move to the front and stop the running turn; the drain sends it after Done. */
  const handleSendQueuedNow = (id: string) => {
    if (!activeThread) return;
    commitQueue(activeThread.id, prioritizeQueued(queueRef.current, id));
    setQueuePaused(false);
    if (runningTurnId) void handleCancelTurn();
  };

  const handleEditQueued = (id: string) => {
    if (!activeThread) return;
    const item = queueRef.current.find((q) => q.id === id);
    if (!item) return;
    commitQueue(activeThread.id, removeQueued(queueRef.current, id));
    setInputPrompt((cur) => (cur.trim() ? `${cur}\n${item.prompt}` : item.prompt));
    textareaRef.current?.focus();
  };

  // Handle cancel
  const handleCancelTurn = async () => {
    if (!runningTurnId) return;
    try {
      // The turn ends with a cancelled Done event, which clears runningTurnId. Clearing it here
      // would let the queue start the next turn while the backend still holds this one.
      const res = await studioApi.cancelTurn({ turnId: runningTurnId });
      if (!res.success) setRunningTurnId(null);
    } catch (err: any) {
      setError(err?.message || "Không thể hủy lượt agent.");
    }
  };

  // Handle undo turn
  const handleUndoTurn = async (turnId: string) => {
    setUndoing(true);
    try {
      await studioApi.undoTurn({ turnId });
      setUndoTurnTarget(null);
      // Remove undone messages
      setMessages((prev) => prev.filter((m) => m.turnId !== turnId));
      // Show result toast
      setUndoToast("Đã hoàn tác lượt thành công! Các tệp đã được khôi phục về điểm kiểm soát trước đó.");
      setTimeout(() => setUndoToast(null), 4000);
      // Notify parent to reload Stage open file
      onUndoTurnSuccess?.();
    } catch (err: any) {
      setError(err?.message || "Không thể hoàn tác lượt.");
    } finally {
      setUndoing(false);
    }
  };

  // Handle permission response
  const handlePermissionRespond = async (requestId: string, allow: boolean) => {
    try {
      await studioApi.respondPermission({ requestId, allow });
      setMessages((prev) =>
        prev.map((m) => ({
          ...m,
          permissionRequests: m.permissionRequests?.map((pr) =>
            pr.requestId === requestId ? { ...pr, resolved: true } : pr
          ),
        }))
      );
    } catch (err: any) {
      setError(err?.message || "Không thể phản hồi yêu cầu cấp quyền.");
    }
  };

  // Mention picker, modeled on VS Code's suggest widget: opens on "@"/"/" typed at a word start,
  // filters as you type, ArrowUp/Down to move, Enter/Tab to accept, Escape to close.
  const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = e.target.value;
    const caret = e.target.selectionStart ?? val.length;
    setInputPrompt(val);

    const typed = val.slice(caret - 1, caret);
    const before = val.slice(caret - 2, caret - 1);
    if ((typed === "@" || typed === "/") && (caret === 1 || /\s/.test(before))) {
      setPickerMode(typed);
      setPickerStart(caret - 1);
      setPickerIndex(0);
      return;
    }
    if (pickerMode) {
      // Close once the trigger is gone or the query ends (whitespace / caret moved before it).
      const query = val.slice(pickerStart + 1, caret);
      if (val[pickerStart] !== pickerMode || caret <= pickerStart || /\s/.test(query)) {
        setPickerMode(null);
      } else {
        setPickerIndex(0);
      }
    }
  };

  const pickerQuery = pickerMode ? inputPrompt.slice(pickerStart + 1).split(/\s/)[0].toLowerCase() : "";
  const pickerItems = React.useMemo(() => {
    if (!pickerMode) return [];
    const all =
      pickerMode === "@"
        ? projectFiles.map((f) => ({ key: f.path, insert: f.path, label: f.path, meta: "" }))
        : playbooks.map((pb) => ({ key: pb.id, insert: pb.name, label: pb.name, meta: pb.layer }));
    if (!pickerQuery) return all.slice(0, 50);
    const matches = all.filter((it) => it.label.toLowerCase().includes(pickerQuery));
    // Prefix matches first, like VS Code's fuzzy score favouring word starts.
    matches.sort(
      (x, y) =>
        Number(!x.label.toLowerCase().startsWith(pickerQuery)) - Number(!y.label.toLowerCase().startsWith(pickerQuery)),
    );
    return matches.slice(0, 50);
  }, [pickerMode, pickerQuery, projectFiles, playbooks]);

  React.useEffect(() => {
    const el = pickerListRef.current?.querySelector<HTMLElement>(`[data-index="${pickerIndex}"]`);
    el?.scrollIntoView({ block: "nearest" });
  }, [pickerIndex]);

  const handleSelectPickerItem = (itemText: string) => {
    if (!pickerMode) return;
    const queryEnd = pickerStart + 1 + (inputPrompt.slice(pickerStart + 1).match(/^\S*/)?.[0].length ?? 0);
    const head = inputPrompt.slice(0, pickerStart) + pickerMode + itemText + " ";
    const next = head + inputPrompt.slice(queryEnd).replace(/^ /, "");
    setInputPrompt(next);
    setPickerMode(null);
    requestAnimationFrame(() => {
      const ta = textareaRef.current;
      if (!ta) return;
      ta.focus();
      ta.selectionStart = ta.selectionEnd = head.length;
    });
  };

  /** Keyboard handling of the open picker. Returns true when the key was consumed. */
  const handlePickerKey = (e: React.KeyboardEvent<HTMLTextAreaElement>): boolean => {
    if (!pickerMode) return false;
    if (e.key === "Escape") {
      e.preventDefault();
      setPickerMode(null);
      return true;
    }
    if (pickerItems.length === 0) return false;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const delta = e.key === "ArrowDown" ? 1 : -1;
      setPickerIndex((i) => (i + delta + pickerItems.length) % pickerItems.length);
      return true;
    }
    if ((e.key === "Enter" && !e.shiftKey) || e.key === "Tab") {
      e.preventDefault();
      const item = pickerItems[Math.min(pickerIndex, pickerItems.length - 1)];
      if (item) handleSelectPickerItem(item.insert);
      return true;
    }
    return false;
  };

  const toggleToolCallExpanded = (messageIndex: number, toolCallId: string) => {
    setMessages((prev) =>
      prev.map((msg, i) =>
        i === messageIndex
          ? {
              ...msg,
              toolCalls: msg.toolCalls?.map((tc) =>
                tc.id === toolCallId ? { ...tc, expanded: !tc.expanded } : tc
              ),
            }
          : msg
      )
    );
  };

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        backgroundColor: "var(--cs-bg-panel)",
        borderRight: "1px solid var(--cs-border-subtle)",
        position: "relative",
      }}
    >
      {/* Thread Header */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "var(--cs-space-2) var(--cs-space-4)",
          borderBottom: "1px solid var(--cs-border-subtle)",
          backgroundColor: "var(--cs-bg-raised)",
          fontSize: "var(--cs-text-sm)",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <span style={{ fontWeight: 600, color: "var(--cs-fg-primary)" }}>
            {activeThread?.title || "Hội thoại chính"}
          </span>
          <Badge variant={activeThread?.engine === "claude" ? "info" : "neutral"}>
            {activeThread?.engine === "claude" ? "Claude" : "agy"}
          </Badge>
        </div>

        {runningTurnId && (
          <Button
            size="sm"
            variant="destructive"
            onClick={handleCancelTurn}
            leftIcon={<Icon name="x" size={13} />}
          >
            Hủy lượt
          </Button>
        )}
      </div>

      {/* Messages Stream Body */}
      <div
        ref={scroll.ref}
        style={{
          flex: 1,
          overflowY: "auto",
          padding: "var(--cs-space-4)",
          display: "flex",
          flexDirection: "column",
          gap: "var(--cs-space-4)",
        }}
      >
        {error && (
          <Callout variant="danger" title="Lỗi lượt xử lý">
            {error}
          </Callout>
        )}

        {messages.length === 0 ? (
          <div
            style={{
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              padding: "var(--cs-space-8) var(--cs-space-4)",
              color: "var(--cs-fg-muted)",
              textAlign: "center",
              gap: 8,
            }}
          >
            <Icon name="bot" size={32} color="var(--cs-fg-muted)" />
            <span style={{ fontSize: "var(--cs-text-sm)" }}>
              Bắt đầu giao việc cho agent trong dự án này.
            </span>
            <span style={{ fontSize: "var(--cs-text-xs)" }}>
              Dùng @ để dẫn tệp, / để áp dụng quy trình playbook.
            </span>
          </div>
        ) : (
          messages.map((msg, msgIdx) => (
            <div
              key={msg.id}
              style={{
                display: "flex",
                flexDirection: "column",
                gap: 6,
                alignSelf: msg.role === "user" ? "flex-end" : "flex-start",
                maxWidth: msg.role === "user" ? "85%" : "100%",
                width: msg.role === "agent" ? "100%" : "auto",
              }}
            >
              {/* Message Header */}
              <div style={{ display: "flex", alignItems: "center", gap: 6, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
                <span style={{ fontWeight: 600 }}>{msg.role === "user" ? "Bạn" : "Agent"}</span>
              </div>

              {/* Message Box */}
              <div
                style={{
                  padding: "var(--cs-space-3)",
                  backgroundColor: msg.role === "user" ? "var(--cs-selection)" : "var(--cs-bg-raised)",
                  color: "var(--cs-fg-primary)",
                  border: "1px solid var(--cs-border-subtle)",
                  borderRadius: "var(--cs-radius-control)",
                  fontSize: "var(--cs-text-sm)",
                  lineHeight: 1.5,
                  whiteSpace: "pre-wrap",
                  wordBreak: "break-word",
                }}
              >
                {/* Thinking / working indicator */}
                {msg.role === "agent" && <ThinkingIndicator msg={msg} />}

                {/* Main Text */}
                {msg.text &&
                  (msg.role === "agent" ? (
                    // Markdown handles its own line breaks; pre-wrap would double them.
                    <div className="cs-md" style={{ whiteSpace: "normal" }}>
                      {renderMarkdown(msg.text, { syntaxHighlight: !msg.isStreaming })}
                    </div>
                  ) : (
                    <div>{msg.text}</div>
                  ))}

                {/* Danger Callout inside bubble on error */}
                {msg.errorMessage && (
                  <div style={{ marginTop: msg.text ? 8 : 0 }}>
                    <Callout variant="danger">
                      <div style={{ fontWeight: 600 }}>Engine báo lỗi:</div>
                      <div style={{ marginTop: 2 }}>{msg.errorMessage}</div>
                    </Callout>
                    {isMcpLoadFailure(msg.errorMessage) && (
                      <div style={{ marginTop: 8 }}>
                        <Callout
                          variant="danger"
                          title="Lỗi kết nối Studio MCP"
                          actions={
                            <Button
                              size="sm"
                              variant="primary"
                              onClick={handleGoToEngines}
                              leftIcon={<Icon name="settings" size={13} />}
                            >
                              Kết nối agy với Studio
                            </Button>
                          }
                        >
                          agy chưa kết nối được Studio — mở Tools & Cài đặt → Engines → Kết nối agy với Studio
                        </Callout>
                      </div>
                    )}
                  </div>
                )}

                {/* Collapsed Tool Calls */}
                {msg.toolCalls && msg.toolCalls.length > 0 && (() => {
                  const groups = groupConsecutiveToolCalls(msg.toolCalls, msg.id || String(msgIdx));
                  const hasMcpErr = msg.toolCalls.some((tc) => isMcpLoadFailure(tc.output));

                  return (
                    <div style={{ display: "flex", flexDirection: "column", gap: 6, marginTop: 8 }}>
                      {groups.map((group) => {
                        const isMulti = group.calls.length > 1;
                        const isExpanded = expandedToolGroups.has(group.key);

                        if (!isMulti) {
                          const tc = group.calls[0];
                          const mcpFail = isMcpLoadFailure(tc.output);

                          return (
                            <div
                              key={tc.id}
                              style={{
                                border: tc.isError ? "1px solid var(--cs-danger)" : "1px solid var(--cs-border-subtle)",
                                backgroundColor: "var(--cs-bg-panel)",
                                borderRadius: "var(--cs-radius-control)",
                                fontSize: "var(--cs-text-xs)",
                                overflow: "hidden",
                              }}
                            >
                              <div
                                onClick={() => toggleToolCallExpanded(msgIdx, tc.id)}
                                style={{
                                  display: "flex",
                                  alignItems: "center",
                                  justifyContent: "space-between",
                                  padding: "6px 10px",
                                  cursor: "pointer",
                                  backgroundColor: "var(--cs-bg-raised)",
                                  userSelect: "none",
                                }}
                              >
                                <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                                  <Icon
                                    name={tc.isError ? "x-circle" : "terminal"}
                                    size={13}
                                    color={tc.isError ? "var(--cs-danger)" : "var(--cs-accent)"}
                                  />
                                  <span style={{ fontWeight: 600, fontFamily: "var(--cs-font-mono)" }}>
                                    {tc.name}
                                  </span>
                                  {tc.isError && <Badge variant="danger">Lỗi</Badge>}
                                </div>
                                <Icon name={tc.expanded ? "chev-up" : "chev-down"} size={12} color="var(--cs-fg-muted)" />
                              </div>
                              {tc.expanded && (
                                <div style={{ padding: "8px 10px", display: "flex", flexDirection: "column", gap: 6 }}>
                                  <div>
                                    <span style={{ color: "var(--cs-fg-muted)" }}>Đầu vào:</span>
                                    <pre style={{ margin: "2px 0", fontSize: 11, fontFamily: "var(--cs-font-mono)", overflowX: "auto" }}>
                                      {JSON.stringify(tc.input, null, 2)}
                                    </pre>
                                  </div>
                                  {tc.output && (
                                    <div>
                                      <span style={{ color: "var(--cs-fg-muted)" }}>Đầu ra:</span>
                                      <pre style={{ margin: "2px 0", fontSize: 11, fontFamily: "var(--cs-font-mono)", overflowX: "auto", color: tc.isError ? "var(--cs-danger)" : undefined }}>
                                        {tc.output}
                                      </pre>
                                    </div>
                                  )}
                                  {mcpFail && (
                                    <div style={{ marginTop: 4 }}>
                                      <Callout
                                        variant="danger"
                                        title="Lỗi kết nối Studio MCP"
                                        actions={
                                          <Button
                                            size="sm"
                                            variant="primary"
                                            onClick={handleGoToEngines}
                                            leftIcon={<Icon name="settings" size={13} />}
                                          >
                                            Kết nối agy với Studio
                                          </Button>
                                        }
                                      >
                                        agy chưa kết nối được Studio — mở Tools & Cài đặt → Engines → Kết nối agy với Studio
                                      </Callout>
                                    </div>
                                  )}
                                </div>
                              )}
                            </div>
                          );
                        }

                        // Multi-call group (e.g. view_file ×14)
                        return (
                          <div
                            key={group.key}
                            style={{
                              border: group.hasErrors ? "1px solid var(--cs-danger)" : "1px solid var(--cs-border-subtle)",
                              backgroundColor: "var(--cs-bg-panel)",
                              borderRadius: "var(--cs-radius-control)",
                              fontSize: "var(--cs-text-xs)",
                              overflow: "hidden",
                            }}
                          >
                            <div
                              onClick={() => toggleToolGroup(group.key)}
                              style={{
                                display: "flex",
                                alignItems: "center",
                                justifyContent: "space-between",
                                padding: "6px 10px",
                                cursor: "pointer",
                                backgroundColor: "var(--cs-bg-raised)",
                                userSelect: "none",
                              }}
                            >
                              <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                                <Icon
                                  name={group.hasErrors ? "x-circle" : "terminal"}
                                  size={13}
                                  color={group.hasErrors ? "var(--cs-danger)" : "var(--cs-accent)"}
                                />
                                <span style={{ fontWeight: 600, fontFamily: "var(--cs-font-mono)" }}>
                                  {group.name} ×{group.calls.length}
                                </span>
                                {group.hasErrors && (
                                  <Badge variant="danger">
                                    {group.errorCount} lỗi
                                  </Badge>
                                )}
                              </div>
                              <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                                <span style={{ fontSize: 11, color: "var(--cs-fg-muted)" }}>
                                  {isExpanded ? "Thu gọn" : "Xem chi tiết"}
                                </span>
                                <Icon name={isExpanded ? "chev-up" : "chev-down"} size={12} color="var(--cs-fg-muted)" />
                              </div>
                            </div>

                            {isExpanded && (
                              <div style={{ padding: "6px 10px", display: "flex", flexDirection: "column", gap: 6, backgroundColor: "var(--cs-bg-app)", borderTop: "1px solid var(--cs-border-subtle)" }}>
                                {group.calls.map((tc, cIdx) => {
                                  const mcpFail = isMcpLoadFailure(tc.output);
                                  const inputSummary =
                                    typeof tc.input === "object" && tc.input !== null
                                      ? String((tc.input as any).path || (tc.input as any).file || (tc.input as any).filePath || "")
                                      : "";

                                  return (
                                    <div
                                      key={tc.id}
                                      style={{
                                        border: tc.isError ? "1px solid var(--cs-danger)" : "1px solid var(--cs-border-subtle)",
                                        backgroundColor: "var(--cs-bg-panel)",
                                        borderRadius: "var(--cs-radius-control)",
                                        fontSize: "var(--cs-text-xs)",
                                        overflow: "hidden",
                                      }}
                                    >
                                      <div
                                        onClick={() => toggleToolCallExpanded(msgIdx, tc.id)}
                                        style={{
                                          display: "flex",
                                          alignItems: "center",
                                          justifyContent: "space-between",
                                          padding: "5px 8px",
                                          cursor: "pointer",
                                          backgroundColor: "var(--cs-bg-raised)",
                                          userSelect: "none",
                                        }}
                                      >
                                        <div style={{ display: "flex", alignItems: "center", gap: 6, minWidth: 0 }}>
                                          <Icon
                                            name={tc.isError ? "x-circle" : "terminal"}
                                            size={12}
                                            color={tc.isError ? "var(--cs-danger)" : "var(--cs-accent)"}
                                          />
                                          <span style={{ fontWeight: 600, fontFamily: "var(--cs-font-mono)", fontSize: 11 }}>
                                            #{cIdx + 1}
                                          </span>
                                          {inputSummary && (
                                            <span style={{ color: "var(--cs-fg-muted)", fontFamily: "var(--cs-font-mono)", fontSize: 11, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                                              {inputSummary}
                                            </span>
                                          )}
                                          {tc.isError && <Badge variant="danger">Lỗi</Badge>}
                                        </div>
                                        <Icon name={tc.expanded ? "chev-up" : "chev-down"} size={11} color="var(--cs-fg-muted)" />
                                      </div>

                                      {tc.expanded && (
                                        <div style={{ padding: "8px 10px", display: "flex", flexDirection: "column", gap: 6 }}>
                                          <div>
                                            <span style={{ color: "var(--cs-fg-muted)" }}>Đầu vào:</span>
                                            <pre style={{ margin: "2px 0", fontSize: 11, fontFamily: "var(--cs-font-mono)", overflowX: "auto" }}>
                                              {JSON.stringify(tc.input, null, 2)}
                                            </pre>
                                          </div>
                                          {tc.output && (
                                            <div>
                                              <span style={{ color: "var(--cs-fg-muted)" }}>Đầu ra:</span>
                                              <pre style={{ margin: "2px 0", fontSize: 11, fontFamily: "var(--cs-font-mono)", overflowX: "auto", color: tc.isError ? "var(--cs-danger)" : undefined }}>
                                                {tc.output}
                                              </pre>
                                            </div>
                                          )}
                                          {mcpFail && (
                                            <div style={{ marginTop: 4 }}>
                                              <Callout
                                                variant="danger"
                                                title="Lỗi kết nối Studio MCP"
                                                actions={
                                                  <Button
                                                    size="sm"
                                                    variant="primary"
                                                    onClick={handleGoToEngines}
                                                    leftIcon={<Icon name="settings" size={13} />}
                                                  >
                                                    Kết nối agy với Studio
                                                  </Button>
                                                }
                                              >
                                                agy chưa kết nối được Studio — mở Tools & Cài đặt → Engines → Kết nối agy với Studio
                                              </Callout>
                                            </div>
                                          )}
                                        </div>
                                      )}
                                    </div>
                                  );
                                })}
                              </div>
                            )}
                          </div>
                        );
                      })}

                      {/* Clear banner for MCP load failure */}
                      {hasMcpErr && (
                        <div style={{ marginTop: 4 }}>
                          <Callout
                            variant="danger"
                            title="Lỗi kết nối Studio MCP"
                            actions={
                              <Button
                                size="sm"
                                variant="primary"
                                onClick={handleGoToEngines}
                                leftIcon={<Icon name="settings" size={13} />}
                              >
                                Kết nối agy với Studio
                              </Button>
                            }
                          >
                            agy chưa kết nối được Studio — mở Tools & Cài đặt → Engines → Kết nối agy với Studio
                          </Callout>
                        </div>
                      )}
                    </div>
                  );
                })()}

                {/* File Change Chips */}
                {msg.fileChanges && msg.fileChanges.length > 0 && (
                  <div style={{ display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap", marginTop: 8 }}>
                    <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
                      Thay đổi tệp:
                    </span>
                    {msg.fileChanges.map((fc, i) => (
                      <button
                        key={i}
                        type="button"
                        onClick={() => onSelectFile(fc.path)}
                        style={{
                          display: "inline-flex",
                          alignItems: "center",
                          gap: 4,
                          padding: "2px 6px",
                          borderRadius: "var(--cs-radius-control)",
                          backgroundColor: "var(--cs-bg-panel)",
                          border: "1px solid var(--cs-border-subtle)",
                          fontSize: "var(--cs-text-xs)",
                          fontFamily: "var(--cs-font-mono)",
                          cursor: "pointer",
                          color: fc.op === "create" ? "var(--cs-success)" : fc.op === "delete" ? "var(--cs-danger)" : "var(--cs-accent)",
                        }}
                        title="Bấm để xem trên Stage"
                      >
                        <span>{fc.op === "create" ? "+" : fc.op === "delete" ? "-" : "~"}</span>
                        <span>{fc.path}</span>
                      </button>
                    ))}
                  </div>
                )}

                {/* Permission Requests */}
                {msg.permissionRequests?.map((pr) => !pr.resolved && (
                  <div
                    key={pr.requestId}
                    style={{
                      marginTop: 8,
                      padding: "var(--cs-space-3)",
                      border: "1px solid var(--cs-warning)",
                      backgroundColor: "var(--cs-status-warning-bg)",
                      borderRadius: "var(--cs-radius-control)",
                      display: "flex",
                      flexDirection: "column",
                      gap: 6,
                    }}
                  >
                    <div style={{ display: "flex", alignItems: "center", gap: 6, fontWeight: 600, color: "var(--cs-warning)" }}>
                      <Icon name="shield-alert" size={16} />
                      <span>Yêu cầu cấp quyền chạy công cụ</span>
                    </div>
                    <div style={{ fontSize: "var(--cs-text-xs)" }}>
                      Agent muốn thực thi: <code>{pr.tool}</code>
                    </div>
                    <div style={{ display: "flex", gap: "var(--cs-space-2)", marginTop: 4 }}>
                      <Button size="sm" variant="primary" onClick={() => handlePermissionRespond(pr.requestId, true)}>
                        Cho phép
                      </Button>
                      <Button size="sm" variant="secondary" onClick={() => handlePermissionRespond(pr.requestId, false)}>
                        Từ chối
                      </Button>
                    </div>
                  </div>
                ))}

                {/* Denied Actions */}
                {msg.deniedActions?.map((da, i) => (
                  <div
                    key={i}
                    style={{
                      marginTop: 8,
                      padding: "8px",
                      border: "1px solid var(--cs-danger)",
                      backgroundColor: "var(--cs-status-danger-bg)",
                      borderRadius: "var(--cs-radius-control)",
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "space-between",
                    }}
                  >
                    <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-danger)" }}>
                      Thao tác <code>{da.tool}</code> bị từ chối.
                    </span>
                    <Button
                      size="sm"
                      variant="danger-outline"
                      onClick={() => {
                        // Re-run prompt
                        setInputPrompt(`Cho phép và chạy lại ${da.tool}`);
                      }}
                    >
                      Cho phép và chạy lại
                    </Button>
                  </div>
                ))}

                {/* Agent Publish Draft In-Turn Banner */}
                {msg.toolCalls?.some((tc) => tc.name.includes("publish_prepare")) && (
                  <div
                    style={{
                      marginTop: 8,
                      padding: "8px 12px",
                      backgroundColor: "var(--cs-status-info-bg)",
                      border: "1px solid var(--cs-accent)",
                      borderRadius: "var(--cs-radius-control)",
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "space-between",
                      gap: 8,
                    }}
                  >
                    <div style={{ display: "flex", alignItems: "center", gap: 6, fontSize: "var(--cs-text-xs)", color: "var(--cs-accent)" }}>
                      <Icon name="publishing" size={14} color="var(--cs-accent)" />
                      <span style={{ fontWeight: 600 }}>Agent đã chuẩn bị bản đăng — xem lại và xác nhận</span>
                    </div>
                    <Button
                      size="sm"
                      variant="primary"
                      onClick={async () => {
                        try {
                          const list = await studioApi.publishList(projectSlug);
                          const req = list.find((r) => r.status === "awaiting_confirmation") || list[0];
                          if (req) {
                            setSelectedPublishRequest(req);
                            setPublishDialogOpen(true);
                          }
                        } catch {
                          // ignore
                        }
                      }}
                    >
                      Xem lại & xác nhận
                    </Button>
                  </div>
                )}
              </div>

              {/* Per-Turn Footer (Usage / Undo) */}
              {msg.role === "agent" && (
                <div
                  style={{
                    display: "flex",
                    alignItems: "flex-start",
                    justifyContent: "space-between",
                    gap: 8,
                    fontSize: "var(--cs-text-xs)",
                    color: "var(--cs-fg-muted)",
                    padding: "0 4px",
                  }}
                >
                  {/* Usage / Cost */}
                  <div style={{ flex: 1, minWidth: 0 }}>
                    {msg.usage ? <UsageSummary usage={msg.usage} /> : null}
                  </div>

                  {/* Undo Button */}
                  {msg.turnId && !msg.isStreaming && (
                    <button
                      type="button"
                      disabled={Boolean(runningTurnId)}
                      onClick={() => setUndoTurnTarget(msg.turnId!)}
                      style={{
                        background: "none",
                        border: "none",
                        padding: "2px 4px",
                        color: Boolean(runningTurnId) ? "var(--cs-fg-muted)" : "var(--cs-fg-secondary)",
                        cursor: Boolean(runningTurnId) ? "not-allowed" : "pointer",
                        opacity: Boolean(runningTurnId) ? 0.4 : 1,
                        display: "inline-flex",
                        alignItems: "center",
                        gap: 4,
                        fontSize: "var(--cs-text-xs)",
                        borderRadius: "var(--cs-radius-control)",
                        whiteSpace: "nowrap",
                        flexShrink: 0,
                      }}
                      title={
                        Boolean(runningTurnId)
                          ? "Không thể hoàn tác khi agent đang chạy một lượt khác"
                          : "Hoàn tác các file đã sửa trong lượt này"
                      }
                    >
                      <Icon name="undo" size={12} />
                      <span>Hoàn tác</span>
                    </button>
                  )}
                </div>
              )}
            </div>
          ))
        )}
      </div>

      {scroll.showJumpToLatest && (
        <div style={{ position: "relative", height: 0 }}>
          <button
            type="button"
            onClick={scroll.jumpToLatest}
            style={{
              position: "absolute",
              bottom: 12,
              left: "50%",
              transform: "translateX(-50%)",
              zIndex: 5,
              display: "inline-flex",
              alignItems: "center",
              gap: 6,
              padding: "4px 12px",
              fontSize: "var(--cs-text-xs)",
              color: "var(--cs-fg-primary)",
              backgroundColor: "var(--cs-bg-raised)",
              border: "1px solid var(--cs-border-strong)",
              borderRadius: 999,
              boxShadow: "var(--cs-shadow-overlay)",
              cursor: "pointer",
            }}
          >
            ↓ Về tin mới nhất
          </button>
        </div>
      )}

      {/* Mention Pickers Popup */}
      {pickerMode && (
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
                  onClick={() => handleSelectPickerItem(item.insert)}
                  onMouseMove={() => i !== pickerIndex && setPickerIndex(i)}
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
      )}

      {/* Draft Publish Banner */}
      {pendingPublishRequests.length > 0 && (
        <div
          style={{
            margin: "0 var(--cs-space-3) var(--cs-space-2) var(--cs-space-3)",
            padding: "8px 12px",
            backgroundColor: "var(--cs-status-info-bg)",
            border: "1px solid var(--cs-accent)",
            borderRadius: "var(--cs-radius-control)",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            gap: 12,
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 8, minWidth: 0 }}>
            <Icon name="publishing" size={16} color="var(--cs-accent)" />
            <span style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600, color: "var(--cs-accent)" }}>
              Agent đã chuẩn bị bản đăng — xem lại và xác nhận
            </span>
          </div>
          <Button
            size="sm"
            variant="primary"
            onClick={() => {
              setSelectedPublishRequest(pendingPublishRequests[0]);
              setPublishDialogOpen(true);
            }}
          >
            Xem lại & xác nhận
          </Button>
        </div>
      )}

      {/* Queued prompts */}
      {activeThread && (
        <QueuedSendStack
          items={queue}
          paused={queuePaused}
          onSendNow={handleSendQueuedNow}
          onEdit={handleEditQueued}
          onRemove={(id) => commitQueue(activeThread.id, removeQueued(queueRef.current, id))}
          onMove={(id, delta) => commitQueue(activeThread.id, moveQueued(queueRef.current, id, delta))}
          onResume={() => setQueuePaused(false)}
        />
      )}

      {/* Prompt Input Box */}
      <div
        style={{
          padding: "var(--cs-space-3)",
          borderTop: "1px solid var(--cs-border-subtle)",
          backgroundColor: "var(--cs-bg-panel)",
        }}
      >
        <div style={{ position: "relative" }}>
          <textarea
            ref={textareaRef}
            value={inputPrompt}
            onChange={handleInputChange}
            onKeyDown={(e) => {
              if (handlePickerKey(e)) return;
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                handleSend();
              }
            }}
            placeholder="Yêu cầu agent... Gõ @ chèn tệp, / chèn playbook"
            rows={3}
            style={{
              width: "100%",
              padding: "var(--cs-space-2) var(--cs-space-3)",
              fontFamily: "var(--cs-font-ui)",
              fontSize: "var(--cs-text-sm)",
              backgroundColor: "var(--cs-bg-raised)",
              color: "var(--cs-fg-primary)",
              border: "1px solid var(--cs-border-subtle)",
              borderRadius: "var(--cs-radius-control)",
              outline: "none",
              resize: "none",
              boxSizing: "border-box",
            }}
          />
        </div>

        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginTop: 6 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 6, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
            <Kbd>@</Kbd> <span>Tệp</span>
            <Kbd>/</Kbd> <span>Playbook</span>
          </div>
          <Button
            size="sm"
            variant="primary"
            disabled={!inputPrompt.trim()}
            onClick={handleSend}
            leftIcon={<Icon name={runningTurnId || sending ? "clock" : "arrow-right"} size={14} />}
          >
            {runningTurnId || sending ? "Xếp hàng" : "Gửi"}
          </Button>
        </div>
      </div>

      {/* Undo Confirm Dialog */}
      {undoTurnTarget && (() => {
        const targetMsg = messages.find((m) => m.turnId === undoTurnTarget);
        const files = targetMsg?.fileChanges || [];

        return (
          <Dialog
            title="Xác nhận hoàn tác lượt này?"
            open={Boolean(undoTurnTarget)}
            onClose={() => setUndoTurnTarget(null)}
            actions={
              <>
                <Button variant="secondary" onClick={() => setUndoTurnTarget(null)} disabled={undoing}>
                  Hủy
                </Button>
                <Button
                  variant="destructive"
                  busy={undoing}
                  busyText="Đang hoàn tác..."
                  onClick={() => handleUndoTurn(undoTurnTarget)}
                >
                  Hoàn tác lượt
                </Button>
              </>
            }
          >
            <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              <p style={{ margin: 0, fontSize: "var(--cs-text-sm)", lineHeight: 1.5 }}>
                Thao tác này sẽ khôi phục lại toàn bộ tệp bị thay đổi trong lượt agent này về điểm kiểm soát (checkpoint) ngay trước đó.
              </p>

              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                <span style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600, color: "var(--cs-fg-secondary)" }}>
                  Các tệp sẽ được khôi phục ({files.length}):
                </span>
                {files.length > 0 ? (
                  <div
                    style={{
                      maxHeight: 160,
                      overflowY: "auto",
                      backgroundColor: "var(--cs-bg-raised)",
                      border: "1px solid var(--cs-border-subtle)",
                      borderRadius: "var(--cs-radius-control)",
                      padding: "6px 10px",
                      display: "flex",
                      flexDirection: "column",
                      gap: 4,
                    }}
                  >
                    {files.map((fc, i) => (
                      <div key={i} style={{ display: "flex", alignItems: "center", gap: 6, fontSize: "var(--cs-text-xs)" }}>
                        <Badge
                          variant={
                            fc.op === "create"
                              ? "success"
                              : fc.op === "delete"
                              ? "danger"
                              : "info"
                          }
                        >
                          {fc.op === "create" ? "Tạo mới" : fc.op === "delete" ? "Xóa" : "Sửa đổi"}
                        </Badge>
                        <code style={{ fontFamily: "var(--cs-font-mono)", color: "var(--cs-fg-primary)" }}>
                          {fc.path}
                        </code>
                      </div>
                    ))}
                  </div>
                ) : (
                  <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)", fontStyle: "italic" }}>
                    Tất cả các tệp source bị thay đổi bởi agent trong lượt này sẽ được hoàn tác.
                  </div>
                )}
              </div>
            </div>
          </Dialog>
        );
      })()}

      {/* Undo Result Toast */}
      {undoToast && (
        <div
          style={{
            position: "absolute",
            top: 16,
            left: "50%",
            transform: "translateX(-50%)",
            backgroundColor: "var(--cs-bg-panel)",
            border: "1px solid var(--cs-success, #10b981)",
            boxShadow: "var(--cs-shadow-overlay)",
            borderRadius: "var(--cs-radius-control)",
            padding: "8px 16px",
            zIndex: 100,
            display: "flex",
            alignItems: "center",
            gap: 8,
            fontSize: "var(--cs-text-xs)",
            color: "var(--cs-fg-primary)",
          }}
        >
          <span style={{ color: "var(--cs-success, #10b981)", fontWeight: 700 }}>✓</span>
          <span>{undoToast}</span>
          <button
            type="button"
            onClick={() => setUndoToast(null)}
            style={{
              background: "none",
              border: "none",
              color: "var(--cs-fg-muted)",
              cursor: "pointer",
              marginLeft: 4,
              fontSize: 14,
              lineHeight: 1,
            }}
          >
            ×
          </button>
        </div>
      )}

      {/* Publish Dialog */}
      <PublishDialog
        open={publishDialogOpen}
        onClose={() => {
          setPublishDialogOpen(false);
          setSelectedPublishRequest(null);
          loadPublishRequests();
        }}
        projectSlug={projectSlug}
        initialRequest={selectedPublishRequest}
        onPublished={() => {
          loadPublishRequests();
        }}
      />
    </div>
  );
};

type UsageInfo = NonNullable<ThreadMessage["usage"]>;

/** Per-turn token breakdown: total, cost, cache hit ratio and cache write (warm-up). */
const UsageSummary: React.FC<{ usage: UsageInfo }> = ({ usage }) => {
  const fmt = (n: number) => n.toLocaleString();
  const promptTokens = usage.inputTokens + usage.cacheReadTokens + usage.cacheWriteTokens;
  const total = promptTokens + usage.outputTokens;
  const hitPct = promptTokens > 0 ? Math.round((usage.cacheReadTokens / promptTokens) * 100) : 0;

  const head: string[] = [];
  if (usage.costUsd != null) head.push(`$${usage.costUsd.toFixed(4)}`);
  if (usage.estimatedCostUsd) head.push(`~$${usage.estimatedCostUsd.toFixed(2)} (ước tính)`);
  if (total > 0) head.push(`${fmt(total)} tokens`);

  const detail: string[] = [];
  if (total > 0) {
    detail.push(`${fmt(usage.inputTokens)} in`, `${fmt(usage.outputTokens)} out`);
    if (usage.cacheReadTokens > 0) detail.push(`cache đọc ${fmt(usage.cacheReadTokens)} (${hitPct}%)`);
    if (usage.cacheWriteTokens > 0) detail.push(`cache ghi ${fmt(usage.cacheWriteTokens)}`);
    if (usage.reasoningTokens > 0) detail.push(`suy luận ${fmt(usage.reasoningTokens)}`);
  }

  const tooltip = [
    `Input (không cache): ${fmt(usage.inputTokens)}`,
    `Cache đọc (hit): ${fmt(usage.cacheReadTokens)}`,
    `Cache ghi (warm-up): ${fmt(usage.cacheWriteTokens)}`,
    `Output: ${fmt(usage.outputTokens)}`,
    `Suy luận: ${fmt(usage.reasoningTokens)}`,
    `Tỉ lệ cache hit: ${hitPct}%`,
  ].join("\n");

  return (
    <div title={tooltip}>
      {head.length > 0 && <div>{head.join(" · ")}</div>}
      {detail.length > 0 && <div style={{ opacity: 0.8 }}>{detail.join(" · ")}</div>}
    </div>
  );
};

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

/**
 * Thinking slot of an agent message. Neither CLI hands over thinking text headless (Claude Code
 * redacts it, agy does not report it), so absence is the normal case: show that the agent is
 * working and for how long, and the text only when the engine actually sent some.
 */
const ThinkingIndicator: React.FC<{ msg: ThreadMessage }> = ({ msg }) => {
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
