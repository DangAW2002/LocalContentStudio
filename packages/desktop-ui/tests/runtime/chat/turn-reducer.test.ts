// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import {
  applyTurnEvent,
  groupConsecutiveToolCalls,
  isMcpLoadFailure,
  type ThreadMessage,
  type ToolCallItem,
} from "../../../src/runtime/chat/turn-reducer";
import type { EngineTurnEvent } from "../../../src/api/types";

describe("turn-reducer", () => {
  describe("groupConsecutiveToolCalls", () => {
    it("groups consecutive tool calls with the same name", () => {
      const calls: ToolCallItem[] = [
        { id: "1", name: "read_file", input: { path: "a.txt" } },
        { id: "2", name: "read_file", input: { path: "b.txt" }, isError: true },
        { id: "3", name: "write_file", input: { path: "c.txt" } },
      ];
      const groups = groupConsecutiveToolCalls(calls, "turn1");
      expect(groups).toHaveLength(2);
      expect(groups[0].name).toBe("read_file");
      expect(groups[0].calls).toHaveLength(2);
      expect(groups[0].errorCount).toBe(1);
      expect(groups[0].hasErrors).toBe(true);

      expect(groups[1].name).toBe("write_file");
      expect(groups[1].calls).toHaveLength(1);
      expect(groups[1].hasErrors).toBe(false);
    });

    it("returns empty array for empty tool calls", () => {
      expect(groupConsecutiveToolCalls([], "turn1")).toEqual([]);
    });
  });

  describe("isMcpLoadFailure", () => {
    it("identifies MCP failure strings correctly", () => {
      expect(isMcpLoadFailure(undefined)).toBe(false);
      expect(isMcpLoadFailure("")).toBe(false);
      expect(isMcpLoadFailure("Tool failed to load properly")).toBe(true);
      expect(isMcpLoadFailure("Lỗi: Studio chưa chạy background service")).toBe(true);
      expect(isMcpLoadFailure("Success: File saved successfully")).toBe(false);
    });
  });

  describe("applyTurnEvent", () => {
    it("handles UserPrompt event optimistically and appends agent placeholder", () => {
      const initial: ThreadMessage[] = [
        { id: "opt-1", role: "user", text: "Xin chào" },
      ];
      const payload: EngineTurnEvent = {
        turnId: "t-1",
        event: { type: "UserPrompt", text: "Xin chào" },
      };
      // Optimistic prompt matches: user msg gets turnId, and agent placeholder is added
      const updated = applyTurnEvent(initial, payload, 1000);
      expect(updated).toHaveLength(2);
      expect(updated[0].turnId).toBe("t-1");
      expect(updated[0].id).toBe("opt-1");
      expect(updated[1]).toEqual({
        id: "turn-t-1",
        role: "agent",
        text: "",
        turnId: "t-1",
        isStreaming: true,
        startedAt: 1000,
      });

      // Subsequent new user prompt: adds user message + agent placeholder
      const payload2: EngineTurnEvent = {
        turnId: "t-2",
        event: { type: "UserPrompt", text: "Câu hỏi tiếp theo" },
      };
      const updated2 = applyTurnEvent(updated, payload2);
      expect(updated2).toHaveLength(4);
      expect(updated2[2].id).toBe("user-t-2");
      expect(updated2[2].text).toBe("Câu hỏi tiếp theo");
      expect(updated2[3]).toEqual({
        id: "turn-t-2",
        role: "agent",
        text: "",
        turnId: "t-2",
        isStreaming: true,
        startedAt: undefined,
      });
    });

    it("creates placeholder for live UserPrompt with timestamp and history UserPrompt without timestamp", () => {
      // Live scenario
      const live = applyTurnEvent([], {
        turnId: "t-live",
        event: { type: "UserPrompt", text: "Prompt live" },
      }, 5000);
      expect(live).toHaveLength(2);
      expect(live[1].role === "agent" && live[1].startedAt).toBe(5000);
      expect(live[1].isStreaming).toBe(true);

      // History scenario (now is undefined)
      const history = applyTurnEvent([], {
        turnId: "t-hist",
        event: { type: "UserPrompt", text: "Prompt history" },
      });
      expect(history).toHaveLength(2);
      expect(history[1].role === "agent" && history[1].startedAt).toBeUndefined();
      expect(history[1].isStreaming).toBe(true);
    });

    it("does not duplicate placeholder if agent message already exists", () => {
      const state: ThreadMessage[] = [
        { id: "user-t-1", role: "user", text: "Hello", turnId: "t-1" },
        { id: "turn-t-1", role: "agent", text: "Hi", turnId: "t-1", isStreaming: false },
      ];
      const payload: EngineTurnEvent = {
        turnId: "t-1",
        event: { type: "UserPrompt", text: "Hello" },
      };
      const updated = applyTurnEvent(state, payload);
      expect(updated).toHaveLength(2);
      expect(updated[1].text).toBe("Hi");
    });

    it("ignores Session event", () => {
      const initial: ThreadMessage[] = [];
      const payload: EngineTurnEvent = {
        turnId: "t-1",
        event: { type: "Session", id: "sess-1" } as any,
      };
      expect(applyTurnEvent(initial, payload)).toBe(initial);
    });

    it("accumulates TextDelta and sets Text", () => {
      let state: ThreadMessage[] = [];
      state = applyTurnEvent(state, {
        turnId: "t-1",
        event: { type: "TextDelta", text: "Chào " },
      });
      expect(state[0].text).toBe("Chào ");
      expect(state[0].isStreaming).toBe(true);

      state = applyTurnEvent(state, {
        turnId: "t-1",
        event: { type: "TextDelta", text: "bạn!" },
      });
      expect(state[0].text).toBe("Chào bạn!");

      state = applyTurnEvent(state, {
        turnId: "t-1",
        event: { type: "Text", text: "Nội dung hoàn chỉnh" },
      });
      expect(state[0].text).toBe("Nội dung hoàn chỉnh");
    });

    it("handles Thinking accumulation and closes thinking block on next non-thinking event", () => {
      let state: ThreadMessage[] = [];
      // Start thinking at t = 1000
      state = applyTurnEvent(state, {
        turnId: "t-1",
        event: { type: "Thinking", text: "Đang suy nghĩ..." },
      }, 1000);

      expect(state[0].thinkingActive).toBe(true);
      expect(state[0].thoughtSeen).toBe(true);
      expect(state[0].thinking).toBe("Đang suy nghĩ...");
      expect(state[0].thinkingStartedAt).toBe(1000);

      // More thinking at t = 1500
      state = applyTurnEvent(state, {
        turnId: "t-1",
        event: { type: "Thinking", text: " thêm một chút..." },
      }, 1500);
      expect(state[0].thinking).toBe("Đang suy nghĩ... thêm một chút...");
      expect(state[0].thinkingStartedAt).toBe(1000);

      // Non-thinking event closes thinking block at t = 3000
      state = applyTurnEvent(state, {
        turnId: "t-1",
        event: { type: "TextDelta", text: "Kết quả là" },
      }, 3000);
      expect(state[0].thinkingActive).toBe(false);
      expect(state[0].thinkingStartedAt).toBeUndefined();
      expect(state[0].thinkingMs).toBe(2000); // 3000 - 1000
    });

    it("does not close thinking block on live Usage event with reasoningTokens", () => {
      let state: ThreadMessage[] = [];
      state = applyTurnEvent(state, {
        turnId: "t-1",
        event: { type: "Thinking", text: "Đang suy luận" },
      }, 1000);
      expect(state[0].thinkingActive).toBe(true);

      // Live thinking tokens update (Usage event)
      state = applyTurnEvent(state, {
        turnId: "t-1",
        event: {
          type: "Usage",
          inputTokens: 10,
          outputTokens: 0,
          cacheReadTokens: 0,
          cacheWriteTokens: 0,
          reasoningTokens: 42,
        },
      }, 1200);

      // Thinking is STILL active, but reasoningTokens is populated
      expect(state[0].thinkingActive).toBe(true);
      expect(state[0].usage?.reasoningTokens).toBe(42);
    });

    it("handles ToolCall and ToolResult", () => {
      let state: ThreadMessage[] = [];
      state = applyTurnEvent(state, {
        turnId: "t-1",
        event: { type: "ToolCall", id: "call-1", name: "search", input: { q: "abc" } },
      });
      expect(state[0].toolCalls).toHaveLength(1);
      expect(state[0].toolCalls![0].name).toBe("search");
      expect(state[0].toolCalls![0].output).toBeUndefined();

      state = applyTurnEvent(state, {
        turnId: "t-1",
        event: { type: "ToolResult", id: "call-1", outputPreview: "found 1 item", isError: false },
      });
      expect(state[0].toolCalls![0].output).toBe("found 1 item");
      expect(state[0].toolCalls![0].isError).toBe(false);
    });

    it("handles FileChange and Usage events", () => {
      let state: ThreadMessage[] = [];
      state = applyTurnEvent(state, {
        turnId: "t-1",
        event: { type: "FileChange", path: "src/index.ts", op: "modify" },
      });
      expect(state[0].fileChanges).toHaveLength(1);
      expect(state[0].fileChanges![0]).toEqual({ path: "src/index.ts", op: "modify" });

      state = applyTurnEvent(state, {
        turnId: "t-1",
        event: {
          type: "Usage",
          inputTokens: 100,
          outputTokens: 50,
          cacheReadTokens: 20,
          cacheWriteTokens: 10,
          reasoningTokens: 15,
          costUsd: 0.002,
        },
      });
      expect(state[0].usage).toEqual({
        inputTokens: 100,
        outputTokens: 50,
        cacheReadTokens: 20,
        cacheWriteTokens: 10,
        reasoningTokens: 15,
        costUsd: 0.002,
        estimatedCostUsd: undefined,
      });
    });

    it("attaches FileChange to the correct agent message after Done", () => {
      let state: ThreadMessage[] = [];
      // 1. User prompt creates placeholder
      state = applyTurnEvent(state, {
        turnId: "turn-done-1",
        event: { type: "UserPrompt", text: "Make changes" },
      }, 1000);

      // 2. Text response
      state = applyTurnEvent(state, {
        turnId: "turn-done-1",
        event: { type: "Text", text: "Done making changes" },
      }, 1100);

      // 3. Turn finishes with Done
      state = applyTurnEvent(state, {
        turnId: "turn-done-1",
        event: { type: "Done", status: "completed" },
      }, 1200);

      expect(state[1].isStreaming).toBe(false);
      expect(state[1].fileChanges).toBeUndefined();

      // 4. FileChange arriving after Done (e.g. agy-artifacts sync)
      state = applyTurnEvent(state, {
        turnId: "turn-done-1",
        event: { type: "FileChange", path: "agy-artifacts/output.png", op: "create" },
      }, 1300);

      expect(state[1].isStreaming).toBe(false);
      expect(state[1].fileChanges).toHaveLength(1);
      expect(state[1].fileChanges![0]).toEqual({
        path: "agy-artifacts/output.png",
        op: "create",
      });
    });

    it("handles Done and Error events with fallback messages", () => {
      let state: ThreadMessage[] = [];
      state = applyTurnEvent(state, {
        turnId: "t-1",
        event: { type: "Done", status: "error" },
      });
      expect(state[0].isStreaming).toBe(false);
      expect(state[0].errorMessage).toBe("Lượt xử lý thất bại mà không có nội dung phản hồi từ engine.");

      let state2: ThreadMessage[] = [];
      state2 = applyTurnEvent(state2, {
        turnId: "t-2",
        event: { type: "Error", message: "Quá thời gian kết nối" },
      });
      expect(state2[0].isStreaming).toBe(false);
      expect(state2[0].errorMessage).toBe("Quá thời gian kết nối");
    });
  });
});
