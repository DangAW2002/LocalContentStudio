import React from "react";
import { studioApi } from "../../../api/client";
import { applyTurnEvent, type ThreadMessage } from "../../../runtime/chat/turn-reducer";

export interface UseThreadEventsOptions {
  activeThreadId?: string;
  onTurnDone?: () => void;
}

export function useThreadEvents({ activeThreadId, onTurnDone }: UseThreadEventsOptions) {
  const [messages, setMessages] = React.useState<ThreadMessage[]>([]);
  const [runningTurnId, setRunningTurnId] = React.useState<string | null>(null);
  const [historyReadyFor, setHistoryReadyFor] = React.useState<string | null>(null);
  const [historyReloadTick, setHistoryReloadTick] = React.useState(0);
  const [error, setError] = React.useState<string | null>(null);
  const finishedTurnsRef = React.useRef<Set<string>>(new Set());

  // Load persisted history of the active thread
  React.useEffect(() => {
    let cancelled = false;
    setMessages([]);
    setRunningTurnId(null);
    setHistoryReadyFor(null);
    if (!activeThreadId) return;

    (async () => {
      try {
        let folded: ThreadMessage[] = [];
        let cursor: string | undefined;
        for (let page = 0; page < 50; page++) {
          const res = await studioApi.getThreadEvents({ threadId: activeThreadId, cursor, limit: 500 });
          for (const ev of res.events) folded = applyTurnEvent(folded, ev);
          if (!res.hasMore || !res.nextCursor) break;
          cursor = res.nextCursor;
        }
        // Permission requests are not in the persisted history; re-attach the ones still open
        // (they may have been asked while this thread was not on screen).
        const pending = await studioApi.getPendingPermissions(activeThreadId).catch(() => []);
        for (const p of pending) {
          folded = applyTurnEvent(folded, {
            threadId: p.threadId,
            turnId: p.turnId,
            event: { type: "PermissionRequest", requestId: p.requestId, tool: p.tool, input: p.input },
          });
        }
        if (cancelled) return;
        const live = folded.find((m) => m.role === "agent" && m.isStreaming);
        setRunningTurnId(live?.turnId ?? null);
        setMessages(folded);
        setHistoryReadyFor(activeThreadId);
      } catch (err: any) {
        if (!cancelled) setError(err?.message || "Không tải được lịch sử hội thoại.");
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [activeThreadId, historyReloadTick]);

  // Handle onResync
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

  // Listen to turn-event
  const onTurnDoneRef = React.useRef(onTurnDone);
  onTurnDoneRef.current = onTurnDone;

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
          onTurnDoneRef.current?.();
        }
        if (payload.event.type === "Error") {
          setError(payload.event.message);
        }
      })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => {});

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [activeThreadId]);

  return {
    messages,
    setMessages,
    runningTurnId,
    setRunningTurnId,
    historyReadyFor,
    error,
    setError,
    finishedTurnsRef,
  };
}
