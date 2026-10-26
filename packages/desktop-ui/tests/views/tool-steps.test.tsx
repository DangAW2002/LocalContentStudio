// @vitest-environment jsdom
import React from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { ToolSteps } from "../../src/views/project/thread/ToolSteps";
import type { ToolCallItem } from "../../src/runtime/chat/turn-reducer";

describe("ToolSteps (Task D)", () => {
  afterEach(() => {
    cleanup();
  });

  const dummyCalls: ToolCallItem[] = [
    { id: "c1", name: "read_file", input: { path: "script.md" }, output: "hello" },
    { id: "c2", name: "read_file", input: { path: "notes.txt" }, output: "world" },
    { id: "c3", name: "run_command", input: { command: "npm test" }, output: "err", isError: true },
    { id: "c4", name: "generate_image", input: {}, output: "img.png" },
    { id: "c5", name: "run_command", input: { command: "cargo build" }, output: "fail", isError: true },
  ];

  it("renders null if toolCalls is empty", () => {
    const { container } = render(
      <ToolSteps
        toolCalls={[]}
        msgIdx={0}
        expandedToolGroups={new Set()}
        toggleToolGroup={vi.fn()}
        toggleToolCallExpanded={vi.fn()}
      />
    );
    expect(container.firstChild).toBeNull();
  });

  it("renders collapsed summary row by default showing step count and error count", () => {
    render(
      <ToolSteps
        toolCalls={dummyCalls}
        msgIdx={0}
        expandedToolGroups={new Set()}
        toggleToolGroup={vi.fn()}
        toggleToolCallExpanded={vi.fn()}
      />
    );

    const summary = screen.getByTestId("tool-steps-summary");
    expect(summary.textContent).toContain("▸");
    expect(summary.textContent).toContain("5 bước");
    expect(summary.textContent).toContain("2 lỗi");

    // Expanded details are not shown by default
    expect(screen.queryByTestId("tool-steps-expanded")).toBeNull();
  });

  it("toggles expanded details on click and renders describeStep labels", () => {
    render(
      <ToolSteps
        toolCalls={dummyCalls}
        msgIdx={0}
        expandedToolGroups={new Set(["0-read_file-0"])}
        toggleToolGroup={vi.fn()}
        toggleToolCallExpanded={vi.fn()}
      />
    );

    const summary = screen.getByTestId("tool-steps-summary");
    fireEvent.click(summary);

    // Expanded view is visible
    expect(screen.getByTestId("tool-steps-expanded")).toBeTruthy();
    expect(summary.textContent).toContain("▾");

    // Uses Vietnamese neutral action labels
    expect(screen.getByText(/Đọc tệp ×2/)).toBeTruthy();
    expect(screen.getByText(/Tạo ảnh/)).toBeTruthy();
    expect(screen.getByText(/Chạy lệnh: npm test/)).toBeTruthy();

    // Click again collapses
    fireEvent.click(summary);
    expect(screen.queryByTestId("tool-steps-expanded")).toBeNull();
    expect(summary.textContent).toContain("▸");
  });

  it("stays collapsed while streaming: the live activity line already shows the running step", () => {
    render(
      <ToolSteps
        toolCalls={dummyCalls}
        msgIdx={0}
        isStreaming={true}
        expandedToolGroups={new Set()}
        toggleToolGroup={vi.fn()}
        toggleToolCallExpanded={vi.fn()}
      />
    );
    expect(screen.getByTestId("tool-steps-summary")).toBeTruthy();
    expect(screen.queryByTestId("tool-steps-latest")).toBeNull();
    expect(screen.queryByTestId("tool-steps-expanded")).toBeNull();
  });

  it("auto-expands when the completed turn errored and the last tool failed", () => {
    render(
      <ToolSteps
        toolCalls={dummyCalls} // last tool c5 has isError: true
        msgIdx={0}
        isStreaming={false}
        errorMessage="Process exited with code 1"
        expandedToolGroups={new Set()}
        toggleToolGroup={vi.fn()}
        toggleToolCallExpanded={vi.fn()}
      />
    );

    // Auto-expanded to show the cause
    expect(screen.getByTestId("tool-steps-expanded")).toBeTruthy();
    const summary = screen.getByTestId("tool-steps-summary");
    expect(summary.textContent).toContain("▾");
    expect(screen.getByText("fail")).toBeTruthy();
  });

  it("does not auto-expand if completed turn errored but last tool succeeded", () => {
    const callsWithSuccessLast: ToolCallItem[] = [
      { id: "c1", name: "run_command", input: { command: "test" }, output: "err", isError: true },
      { id: "c2", name: "read_file", input: { path: "script.md" }, output: "ok", isError: false },
    ];

    render(
      <ToolSteps
        toolCalls={callsWithSuccessLast}
        msgIdx={0}
        isStreaming={false}
        errorMessage="Something else failed"
        expandedToolGroups={new Set()}
        toggleToolGroup={vi.fn()}
        toggleToolCallExpanded={vi.fn()}
      />
    );

    // Should remain collapsed
    expect(screen.queryByTestId("tool-steps-expanded")).toBeNull();
  });
});
