import React from "react";
import {
  loadQueue,
  saveQueue,
  removeQueued,
  prioritizeQueued,
  moveQueued,
  type QueuedSend,
} from "../../../runtime/chat/send-queue";

export interface UseSendQueueOptions {
  activeThreadId?: string;
  historyReadyFor: string | null;
  runningTurnId: string | null;
  sending: boolean;
  startPrompt: (userText: string) => Promise<boolean>;
  onCancelTurn?: () => void;
  onEditPrompt?: (text: string) => void;
}

export function useSendQueue({
  activeThreadId,
  historyReadyFor,
  runningTurnId,
  sending,
  startPrompt,
  onCancelTurn,
  onEditPrompt,
}: UseSendQueueOptions) {
  const [queue, setQueue] = React.useState<QueuedSend[]>([]);
  const queueRef = React.useRef<QueuedSend[]>([]);
  const [queuePaused, setQueuePaused] = React.useState(false);
  const drainingRef = React.useRef<string | null>(null);

  // Sync with active thread on switch
  React.useEffect(() => {
    setQueuePaused(false);
    const restored = activeThreadId ? loadQueue(activeThreadId) : [];
    queueRef.current = restored;
    setQueue(restored);
  }, [activeThreadId]);

  const commitQueue = React.useCallback(
    (threadId: string, next: QueuedSend[]) => {
      queueRef.current = next;
      setQueue(next);
      saveQueue(threadId, next);
    },
    []
  );

  const handleSendQueuedNow = React.useCallback(
    (id: string) => {
      if (!activeThreadId) return;
      commitQueue(activeThreadId, prioritizeQueued(queueRef.current, id));
      setQueuePaused(false);
      if (runningTurnId) onCancelTurn?.();
    },
    [activeThreadId, commitQueue, runningTurnId, onCancelTurn]
  );

  const handleEditQueued = React.useCallback(
    (id: string) => {
      if (!activeThreadId) return;
      const item = queueRef.current.find((q) => q.id === id);
      if (!item) return;
      commitQueue(activeThreadId, removeQueued(queueRef.current, id));
      onEditPrompt?.(item.prompt);
    },
    [activeThreadId, commitQueue, onEditPrompt]
  );

  const handleRemoveQueued = React.useCallback(
    (id: string) => {
      if (!activeThreadId) return;
      commitQueue(activeThreadId, removeQueued(queueRef.current, id));
    },
    [activeThreadId, commitQueue]
  );

  const handleMoveQueued = React.useCallback(
    (id: string, delta: 1 | -1) => {
      if (!activeThreadId) return;
      commitQueue(activeThreadId, moveQueued(queueRef.current, id, delta));
    },
    [activeThreadId, commitQueue]
  );

  // Drain: when the thread goes idle, start the head of the queue.
  React.useEffect(() => {
    if (!activeThreadId || historyReadyFor !== activeThreadId) return;
    if (runningTurnId || sending || queuePaused || drainingRef.current) return;
    const next = queue[0];
    if (!next) return;
    drainingRef.current = next.id;
    void (async () => {
      const started = await startPrompt(next.prompt);
      drainingRef.current = null;
      if (started) commitQueue(activeThreadId, removeQueued(queueRef.current, next.id));
      else setQueuePaused(true);
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeThreadId, historyReadyFor, runningTurnId, sending, queuePaused, queue]);

  return {
    queue,
    queueRef,
    queuePaused,
    setQueuePaused,
    commitQueue,
    handleSendQueuedNow,
    handleEditQueued,
    handleRemoveQueued,
    handleMoveQueued,
  };
}
