// @vitest-environment jsdom
import { describe, expect, it, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { useThreadEvents } from "../../src/views/project/thread/useThreadEvents";
import { studioApi } from "../../src/api/client";

vi.mock("../../src/api/client", () => ({
  studioApi: {
    getThreadEvents: vi.fn(),
    getPendingPermissions: vi.fn(),
    onResync: vi.fn().mockResolvedValue(() => {}),
    onTurnEvent: vi.fn().mockResolvedValue(() => {}),
  },
}));

describe("useThreadEvents (Task D3 & A2)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("sets startedAt = Date.now() on the last agent message if isStreaming and missing startedAt", async () => {
    const mockEvents = [
      {
        threadId: "th-1",
        turnId: "turn-1",
        event: { type: "UserPrompt", text: "Xin chào" },
      },
      {
        threadId: "th-1",
        turnId: "turn-1",
        event: { type: "TextDelta", delta: "Đang trả lời..." },
      },
    ];

    (studioApi.getThreadEvents as any).mockResolvedValueOnce({
      events: mockEvents,
      hasMore: false,
    });
    (studioApi.getPendingPermissions as any).mockResolvedValueOnce([]);

    const before = Date.now();
    const { result } = renderHook(() => useThreadEvents({ activeThreadId: "th-1" }));

    await waitFor(() => {
      expect(result.current.historyReadyFor).toBe("th-1");
    });

    const agentMsg = result.current.messages.find((m) => m.role === "agent");
    expect(agentMsg).toBeDefined();
    expect(agentMsg?.isStreaming).toBe(true);
    expect(agentMsg?.startedAt).toBeDefined();
    expect(agentMsg!.startedAt!).toBeGreaterThanOrEqual(before);
    expect(agentMsg!.startedAt!).toBeLessThanOrEqual(Date.now());
    expect(result.current.runningTurnId).toBe("turn-1");
  });

  it("marks older streaming agent messages as isStreaming: false when loading history", async () => {
    const mockEvents = [
      // Turn 1: abandoned or missing Done
      {
        threadId: "th-1",
        turnId: "turn-1",
        event: { type: "UserPrompt", text: "Câu 1" },
      },
      {
        threadId: "th-1",
        turnId: "turn-1",
        event: { type: "TextDelta", delta: "Trả lời 1" },
      },
      // Turn 2: active turn
      {
        threadId: "th-1",
        turnId: "turn-2",
        event: { type: "UserPrompt", text: "Câu 2" },
      },
      {
        threadId: "th-1",
        turnId: "turn-2",
        event: { type: "TextDelta", delta: "Trả lời 2" },
      },
    ];

    (studioApi.getThreadEvents as any).mockResolvedValueOnce({
      events: mockEvents,
      hasMore: false,
    });
    (studioApi.getPendingPermissions as any).mockResolvedValueOnce([]);

    const { result } = renderHook(() => useThreadEvents({ activeThreadId: "th-1" }));

    await waitFor(() => {
      expect(result.current.historyReadyFor).toBe("th-1");
    });

    const agentMsgs = result.current.messages.filter((m) => m.role === "agent");
    expect(agentMsgs).toHaveLength(2);
    // Turn 1 is marked false
    expect(agentMsgs[0].isStreaming).toBe(false);
    // Turn 2 remains true
    expect(agentMsgs[1].isStreaming).toBe(true);
    expect(result.current.runningTurnId).toBe("turn-2");
  });
});
