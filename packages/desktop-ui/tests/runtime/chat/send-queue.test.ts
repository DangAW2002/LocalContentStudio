// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from "vitest";
import {
  loadQueue,
  moveQueued,
  newQueuedSend,
  prioritizeQueued,
  removeQueued,
  saveQueue,
  type QueuedSend,
} from "../../../src/runtime/chat/send-queue";

const q = (id: string): QueuedSend => ({ id, prompt: `p-${id}`, createdAt: 0 });

describe("send queue", () => {
  beforeEach(() => localStorage.clear());

  it("persists per thread and clears the key when empty", () => {
    saveQueue("t1", [q("a"), q("b")]);
    saveQueue("t2", [q("c")]);
    expect(loadQueue("t1").map((x) => x.id)).toEqual(["a", "b"]);
    expect(loadQueue("t2").map((x) => x.id)).toEqual(["c"]);

    saveQueue("t1", []);
    expect(localStorage.getItem("lcs.chat.queue.v1.t1")).toBeNull();
    expect(loadQueue("t1")).toEqual([]);
  });

  it("ignores corrupt or malformed storage", () => {
    localStorage.setItem("lcs.chat.queue.v1.t1", "{not json");
    expect(loadQueue("t1")).toEqual([]);
    localStorage.setItem("lcs.chat.queue.v1.t1", JSON.stringify([{ id: 1 }, { id: "x", prompt: "  " }, q("ok")]));
    expect(loadQueue("t1").map((x) => x.id)).toEqual(["ok"]);
  });

  it("removes, prioritizes and moves items", () => {
    const list = [q("a"), q("b"), q("c")];
    expect(removeQueued(list, "b").map((x) => x.id)).toEqual(["a", "c"]);
    expect(prioritizeQueued(list, "c").map((x) => x.id)).toEqual(["c", "a", "b"]);
    expect(prioritizeQueued(list, "missing")).toBe(list);
    expect(moveQueued(list, "b", -1).map((x) => x.id)).toEqual(["b", "a", "c"]);
    expect(moveQueued(list, "b", 1).map((x) => x.id)).toEqual(["a", "c", "b"]);
    // Out of bounds is a no-op.
    expect(moveQueued(list, "a", -1)).toBe(list);
    expect(moveQueued(list, "c", 1)).toBe(list);
  });

  it("creates unique ids", () => {
    const ids = new Set(Array.from({ length: 50 }, () => newQueuedSend("x").id));
    expect(ids.size).toBe(50);
  });
});
