// @vitest-environment jsdom
import React from "react";
import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { Composer } from "../../src/views/project/thread/Composer";
import { ThreadPane } from "../../src/views/project/ThreadPane";

describe("Chat Declutter (Task B)", () => {
  it("Composer renders compact 2-row textarea and does not render separate Kbd hint row", () => {
    const textareaRef = React.createRef<HTMLTextAreaElement>();
    render(
      <Composer
        inputPrompt=""
        textareaRef={textareaRef}
        runningTurnId={null}
        sending={false}
        queue={[]}
        queuePaused={false}
        hasActiveThread={true}
        pendingPublishRequests={[]}
        onInputChange={vi.fn()}
        onKeyDown={vi.fn()}
        onSend={vi.fn()}
        onOpenPublishRequest={vi.fn()}
        onSendQueuedNow={vi.fn()}
        onEditQueued={vi.fn()}
        onRemoveQueued={vi.fn()}
        onMoveQueued={vi.fn()}
        onResumeQueue={vi.fn()}
      />
    );

    const textarea = screen.getByPlaceholderText(
      "Yêu cầu agent… (@ tệp, / playbook)"
    ) as HTMLTextAreaElement;
    expect(textarea).toBeTruthy();
    expect(textarea.rows).toBe(2);

    // Ensure the old redundant row with separate Kbd "@" and "/" was removed
    expect(screen.queryByText("Playbook")).toBeNull();

    // Send button exists
    expect(screen.getByRole("button", { name: "Gửi" })).toBeTruthy();
  });

  it("ThreadPane does not render 'Bạn' label and header only shows model select and cancel button", () => {
    const thread = {
      id: "thread-1",
      project: "proj-1",
      engine: "agy",
      title: "Hội thoại thử nghiệm",
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };

    // Mock studioApi so useThreadEvents doesn't crash
    render(
      <ThreadPane
        projectSlug="proj-1"
        activeThread={thread}
        projectFiles={[]}
        playbooks={[]}
        onSelectFile={vi.fn()}
      />
    );

    // Redundant thread title in ThreadPane header must be removed (already in ProjectView bar)
    expect(screen.queryByText("Hội thoại thử nghiệm")).toBeNull();

    // Redundant "Bạn" label must be removed
    expect(screen.queryByText("Bạn")).toBeNull();

    // Select with engine in title exists
    const select = screen.getByTitle("agy · Đổi model cho các lượt tiếp theo");
    expect(select).toBeTruthy();
  });
});
