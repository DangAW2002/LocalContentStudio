import React from "react";
import { Icon } from "../../../components/Icon";
import { Button, Kbd } from "@studio/design-system";
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
            onChange={onInputChange}
            onKeyDown={onKeyDown}
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
            onClick={onSend}
            leftIcon={<Icon name={runningTurnId || sending ? "clock" : "arrow-right"} size={14} />}
          >
            {runningTurnId || sending ? "Xếp hàng" : "Gửi"}
          </Button>
        </div>
      </div>
    </>
  );
};
