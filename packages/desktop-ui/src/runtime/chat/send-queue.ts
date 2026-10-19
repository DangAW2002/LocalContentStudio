/**
 * Per-thread queue of prompts typed while the agent is busy. Semantics follow open-design's
 * chat send queue (https://github.com/nexu-io/open-design, apps/web/src/components/ProjectView.tsx
 * — Apache-2.0): queue instead of rejecting, auto-drain the head when the thread goes idle, keep an
 * item that failed to start, "send now" = move to front + stop the running turn (upstream dropped
 * mid-turn stdin steering because Claude Code ignores user frames written mid-turn).
 *
 * Pure functions + localStorage persistence; no React here.
 */

export interface QueuedSend {
  id: string;
  prompt: string;
  createdAt: number;
}

const STORAGE_PREFIX = "lcs.chat.queue.v1.";

export function loadQueue(threadId: string): QueuedSend[] {
  try {
    const raw = localStorage.getItem(STORAGE_PREFIX + threadId);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(
      (q): q is QueuedSend =>
        q && typeof q.id === "string" && typeof q.prompt === "string" && q.prompt.trim() !== "",
    );
  } catch {
    return [];
  }
}

export function saveQueue(threadId: string, queue: QueuedSend[]): void {
  try {
    if (queue.length === 0) localStorage.removeItem(STORAGE_PREFIX + threadId);
    else localStorage.setItem(STORAGE_PREFIX + threadId, JSON.stringify(queue));
  } catch {
    // Storage full or unavailable: the queue still works for this session.
  }
}

export function newQueuedSend(prompt: string): QueuedSend {
  return { id: `q-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`, prompt, createdAt: Date.now() };
}

export function removeQueued(queue: QueuedSend[], id: string): QueuedSend[] {
  return queue.filter((q) => q.id !== id);
}

export function prioritizeQueued(queue: QueuedSend[], id: string): QueuedSend[] {
  const item = queue.find((q) => q.id === id);
  if (!item) return queue;
  return [item, ...queue.filter((q) => q.id !== id)];
}

export function moveQueued(queue: QueuedSend[], id: string, delta: -1 | 1): QueuedSend[] {
  const from = queue.findIndex((q) => q.id === id);
  const to = from + delta;
  if (from === -1 || to < 0 || to >= queue.length) return queue;
  const next = queue.slice();
  const [item] = next.splice(from, 1);
  next.splice(to, 0, item!);
  return next;
}
