import React from "react";
import { Icon } from "../../../components/Icon";
import { Button } from "@studio/design-system";
import { QueuedSendStack } from "../../../components/QueuedSendStack";
import type { QueuedSend } from "../../../runtime/chat/send-queue";
import type { PublishRequest } from "../../../api/types";

export interface ComposerProps {
  inputPrompt: string;
  textareaRef: React.RefObject<HTMLTextAreaElement>;
  runningTurnId: string | null;
  sending: boolean;
  queue: QueuedSend[];
  queuePaused: boolean;
  hasActiveThread: boolean;
  pendingPublishRequests: PublishRequest[];
  onInputChange: (e: React.ChangeEvent<HTMLTextAreaElement>) => void;
  onKeyDown: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  onSend: () => void;
  onOpenPublishRequest: (req: PublishRequest) => void;
  onSendQueuedNow: (id: string) => void;
  onEditQueued: (id: string) => void;
  onRemoveQueued: (id: string) => void;
  onMoveQueued: (id: string, delta: 1 | -1) => void;
  onResumeQueue: () => void;
}

export const Composer: React.FC<ComposerProps> = ({
  inputPrompt,
  textareaRef,
  runningTurnId,
  sending,
  queue,
  queuePaused,
  hasActiveThread,
  pendingPublishRequests,
  onInputChange,
  onKeyDown,
  onSend,
  onOpenPublishRequest,
  onSendQueuedNow,
  onEditQueued,
  onRemoveQueued,
  onMoveQueued,
  onResumeQueue,
}) => {
  // Auto-expand textarea from 2 rows up to ~8 rows based on content
  React.useEffect(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = "auto";
    const maxHeight = 180; // ~8 lines at ~21px line-height + padding
    const scrollHeight = el.scrollHeight;
    if (scrollHeight > maxHeight) {
      el.style.height = `${maxHeight}px`;
      el.style.overflowY = "auto";
    } else {
      el.style.height = `${Math.max(scrollHeight, 40)}px`;
      el.style.overflowY = "hidden";
    }
  }, [inputPrompt, textareaRef]);

  return (
    <>
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
            onClick={() => onOpenPublishRequest(pendingPublishRequests[0])}
          >
            Xem lại & xác nhận
          </Button>
        </div>
      )}

      {/* Queued prompts */}
      {hasActiveThread && (
        <QueuedSendStack
          items={queue}
          paused={queuePaused}
          onSendNow={onSendQueuedNow}
          onEdit={onEditQueued}
          onRemove={onRemoveQueued}
          onMove={onMoveQueued}
          onResume={onResumeQueue}
        />
      )}

      {/* Prompt Input Box: Single row with auto-expanding textarea and Send button */}
      <div
        style={{
          padding: "var(--cs-space-3)",
          borderTop: "1px solid var(--cs-border-subtle)",
          backgroundColor: "var(--cs-bg-panel)",
          display: "flex",
          alignItems: "flex-end",
          gap: "var(--cs-space-2)",
        }}
      >
        <div style={{ position: "relative", flex: 1, minWidth: 0 }}>
          <textarea
            ref={textareaRef}
            value={inputPrompt}
            onChange={onInputChange}
            onKeyDown={onKeyDown}
            placeholder="Yêu cầu agent… (@ tệp, / playbook)"
            rows={2}
            style={{
              width: "100%",
              padding: "var(--cs-space-2) var(--cs-space-3)",
              fontFamily: "var(--cs-font-ui)",
              fontSize: "var(--cs-text-sm)",
              lineHeight: 1.5,
              backgroundColor: "var(--cs-bg-raised)",
              color: "var(--cs-fg-primary)",
              border: "1px solid var(--cs-border-subtle)",
              borderRadius: "var(--cs-radius-control)",
              outline: "none",
              resize: "none",
              boxSizing: "border-box",
              minHeight: 40,
              maxHeight: 180,
              overflowY: "hidden",
            }}
          />
        </div>

        <Button
          size="sm"
          variant="primary"
          disabled={!inputPrompt.trim()}
          onClick={onSend}
          leftIcon={<Icon name={runningTurnId || sending ? "clock" : "arrow-right"} size={14} />}
          style={{ flexShrink: 0, height: 36, marginBottom: 2 }}
        >
          {runningTurnId || sending ? "Xếp hàng" : "Gửi"}
        </Button>
      </div>
    </>
  );
};
