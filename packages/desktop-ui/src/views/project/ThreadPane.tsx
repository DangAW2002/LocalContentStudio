import React from "react";
import { useStickToBottom } from "../../runtime/chat/useStickToBottom";
import { newQueuedSend } from "../../runtime/chat/send-queue";
import { Icon } from "../../components/Icon";
import { Button, Badge, Callout, Select, type SelectOption } from "@studio/design-system";
import { studioApi } from "../../api/client";
import type {
  EngineModelOption,
  ProjectFileEntry,
  PublishRequest,
  ScopeItem,
  ThreadSummary,
} from "../../api/types";
import { PublishDialog } from "../../components/PublishDialog";
import {
  type ThreadMessage,
  type ToolCallItem,
  type ToolCallGroup,
  groupConsecutiveToolCalls,
  isMcpLoadFailure,
  applyTurnEvent,
} from "../../runtime/chat/turn-reducer";
import { useThreadEvents } from "./thread/useThreadEvents";
import { useSendQueue } from "./thread/useSendQueue";
import { useMentionPicker } from "./thread/useMentionPicker";
import { MentionPicker } from "./thread/MentionPicker";
import { AgentMessage } from "./thread/AgentMessage";
import { Composer } from "./thread/Composer";
import { UndoConfirmDialog } from "./thread/UndoConfirmDialog";

// Re-export for any external consumers
export type { ThreadMessage, ToolCallItem, ToolCallGroup };
export { groupConsecutiveToolCalls, isMcpLoadFailure, applyTurnEvent };

export interface ThreadPaneProps {
  projectSlug: string;
  activeThread?: ThreadSummary;
  projectFiles: ProjectFileEntry[];
  playbooks: ScopeItem[];
  onSelectFile: (path: string) => void;
  pendingInsert?: string | null;
  onInsertConsumed?: () => void;
  onNavigateSettings?: (tab?: string) => void;
  onUndoTurnSuccess?: () => void;
}

export const ThreadPane: React.FC<ThreadPaneProps> = ({
  projectSlug,
  activeThread,
  projectFiles,
  playbooks,
  onSelectFile,
  pendingInsert,
  onInsertConsumed,
  onNavigateSettings,
  onUndoTurnSuccess,
}) => {
  const [inputPrompt, setInputPrompt] = React.useState("");
  const [sending, setSending] = React.useState(false);
  const [undoTurnTarget, setUndoTurnTarget] = React.useState<string | null>(null);
  const [undoing, setUndoing] = React.useState(false);
  const [undoToast, setUndoToast] = React.useState<string | null>(null);

  // M07: Mid-thread model selection and options
  const [availableModels, setAvailableModels] = React.useState<EngineModelOption[]>([]);
  const [selectedModel, setSelectedModel] = React.useState<string>(activeThread?.model || "");
  const [savingModel, setSavingModel] = React.useState(false);

  // M07: Edit & resend last message target
  const [editResendTargetTurnId, setEditResendTargetTurnId] = React.useState<string | null>(null);
  const [editResendPrompt, setEditResendPrompt] = React.useState<string>("");

  const [pendingPublishRequests, setPendingPublishRequests] = React.useState<PublishRequest[]>([]);
  const [selectedPublishRequest, setSelectedPublishRequest] = React.useState<PublishRequest | null>(null);
  const [publishDialogOpen, setPublishDialogOpen] = React.useState(false);
  const [expandedToolGroups, setExpandedToolGroups] = React.useState<Set<string>>(new Set());

  const scroll = useStickToBottom(activeThread?.id);
  const textareaRef = React.useRef<HTMLTextAreaElement>(null);

  const toggleToolGroup = (groupKey: string) => {
    setExpandedToolGroups((prev) => {
      const next = new Set(prev);
      if (next.has(groupKey)) next.delete(groupKey);
      else next.add(groupKey);
      return next;
    });
  };

  const loadPublishRequests = React.useCallback(async () => {
    if (!projectSlug) return;
    try {
      const list = await studioApi.publishList(projectSlug);
      const awaiting = list.filter((r) => r.status === "awaiting_confirmation");
      setPendingPublishRequests(awaiting);
    } catch {
      // ignore
    }
  }, [projectSlug]);

  React.useEffect(() => {
    loadPublishRequests();
  }, [loadPublishRequests]);

  const {
    messages,
    setMessages,
    runningTurnId,
    setRunningTurnId,
    historyReadyFor,
    error,
    setError,
    finishedTurnsRef,
  } = useThreadEvents({
    activeThreadId: activeThread?.id,
    onTurnDone: loadPublishRequests,
  });

  // Sync selectedModel with activeThread
  React.useEffect(() => {
    setSelectedModel(activeThread?.model || "");
    if (activeThread?.engine) {
      studioApi.getEngineModels(activeThread.engine as any).then((models: EngineModelOption[]) => {
        setAvailableModels(models);
      }).catch(() => {
        setAvailableModels([]);
      });
    }
  }, [activeThread?.id, activeThread?.engine, activeThread?.model]);

  // Handle pending text insertion from StagePane
  React.useEffect(() => {
    if (pendingInsert) {
      setInputPrompt((prev) => {
        const separator = prev.length > 0 && !prev.endsWith(" ") && !prev.endsWith("\n") ? " " : "";
        return prev + separator + pendingInsert;
      });
      onInsertConsumed?.();
      setTimeout(() => {
        if (textareaRef.current) {
          textareaRef.current.focus();
          textareaRef.current.selectionStart = textareaRef.current.value.length;
          textareaRef.current.selectionEnd = textareaRef.current.value.length;
        }
      }, 0);
    }
  }, [pendingInsert, onInsertConsumed]);

  /** Start one turn. Returns false when the backend rejected it */
  const startPrompt = async (userText: string): Promise<boolean> => {
    if (!activeThread) return false;
    setError(null);
    setSending(true);
    scroll.jumpToLatest();

    const userMsgId = `user-${Date.now()}`;
    setMessages((prev) => [
      ...prev,
      { id: userMsgId, role: "user", text: userText },
    ]);

    try {
      const res = await studioApi.startTurn({
        threadId: activeThread.id,
        prompt: userText,
      });
      if (!finishedTurnsRef.current.has(res.turnId)) setRunningTurnId(res.turnId);
      return true;
    } catch (err: any) {
      setMessages((prev) => prev.filter((m) => !(m.id === userMsgId && !m.turnId)));
      setError(err?.message || "Không thể bắt đầu lượt agent.");
      return false;
    } finally {
      setSending(false);
    }
  };

  const handleCancelTurn = async () => {
    if (!runningTurnId) return;
    try {
      const res = await studioApi.cancelTurn({ turnId: runningTurnId });
      if (!res.success) setRunningTurnId(null);
    } catch (err: any) {
      setError(err?.message || "Không thể hủy lượt agent.");
    }
  };

  const {
    queue,
    queueRef,
    queuePaused,
    setQueuePaused,
    commitQueue,
    handleSendQueuedNow,
    handleEditQueued,
    handleRemoveQueued,
    handleMoveQueued,
  } = useSendQueue({
    activeThreadId: activeThread?.id,
    historyReadyFor,
    runningTurnId,
    sending,
    startPrompt,
    onCancelTurn: handleCancelTurn,
    onEditPrompt: (text) => {
      setInputPrompt((cur) => (cur.trim() ? `${cur}\n${text}` : text));
      textareaRef.current?.focus();
    },
  });

  const {
    pickerMode,
    setPickerMode,
    pickerIndex,
    setPickerIndex,
    pickerItems,
    pickerQuery,
    pickerListRef,
    handleInputChange,
    handlePickerKey,
    handleSelectPickerItem,
  } = useMentionPicker({
    projectFiles,
    playbooks,
    inputPrompt,
    setInputPrompt,
    textareaRef,
  });

  const handleSend = async () => {
    const userText = inputPrompt.trim();
    if (!userText || !activeThread) return;
    setInputPrompt("");
    setPickerMode(null);

    if (runningTurnId || sending) {
      commitQueue(activeThread.id, [...queueRef.current, newQueuedSend(userText)]);
      return;
    }
    const started = await startPrompt(userText);
    if (started) setQueuePaused(false);
    else setInputPrompt((cur) => cur || userText);
  };

  const handleModelChange = async (newModel: string) => {
    if (!activeThread) return;
    setSavingModel(true);
    try {
      const val = newModel.trim() ? newModel.trim() : null;
      await studioApi.setThreadModel(activeThread.id, val);
      setSelectedModel(newModel);
      activeThread.model = val || undefined;
    } catch (e: any) {
      setError(e?.message || "Không thể cập nhật model.");
    } finally {
      setSavingModel(false);
    }
  };

  const handleRetryTurn = async (turnId?: string) => {
    if (!activeThread) return;
    // Find the user prompt for this turn or the last user prompt
    let promptToRetry = "";
    if (turnId) {
      const userMsg = messages.find((m) => m.role === "user" && m.turnId === turnId);
      if (userMsg?.text) promptToRetry = userMsg.text;
    }
    if (!promptToRetry) {
      const lastUser = [...messages].reverse().find((m) => m.role === "user" && m.text);
      if (lastUser) promptToRetry = lastUser.text;
    }
    if (promptToRetry) {
      await startPrompt(promptToRetry);
    }
  };

  const handleStartEditResend = (prompt: string, turnId?: string) => {
    if (!turnId) {
      // Just populate composer if turn has no id
      setInputPrompt(prompt);
      textareaRef.current?.focus();
      return;
    }
    const agentTurn = messages.find((m) => m.role === "agent" && m.turnId === turnId);
    const hasFiles = Boolean(agentTurn?.fileChanges?.length);
    if (hasFiles) {
      // Need confirmation before undoing files
      setEditResendTargetTurnId(turnId);
      setEditResendPrompt(prompt);
    } else {
      // No files changed: directly undo turn and populate prompt in composer
      handleConfirmEditResend(turnId, prompt);
    }
  };

  const handleConfirmEditResend = async (turnId: string, prompt?: string) => {
    const textToEdit = prompt || editResendPrompt;
    setUndoing(true);
    try {
      await studioApi.undoTurn({ turnId });
      setEditResendTargetTurnId(null);
      setMessages((prev) => prev.filter((m) => m.turnId !== turnId));
      setInputPrompt(textToEdit);
      textareaRef.current?.focus();
      onUndoTurnSuccess?.();
    } catch (err: any) {
      setError(err?.message || "Không thể hoàn tác lượt để sửa tin nhắn.");
    } finally {
      setUndoing(false);
    }
  };

  const handleUndoTurn = async (turnId: string) => {
    setUndoing(true);
    try {
      await studioApi.undoTurn({ turnId });
      setUndoTurnTarget(null);
      setMessages((prev) => prev.filter((m) => m.turnId !== turnId));
      setUndoToast("Đã hoàn tác lượt thành công! Các tệp đã được khôi phục về điểm kiểm soát trước đó.");
      setTimeout(() => setUndoToast(null), 4000);
      onUndoTurnSuccess?.();
    } catch (err: any) {
      setError(err?.message || "Không thể hoàn tác lượt.");
    } finally {
      setUndoing(false);
    }
  };

  // Listen to command palette undo event
  React.useEffect(() => {
    const handleUndoLastTurnEvent = () => {
      // Find the last agent turn with a turnId
      const lastAgentTurn = [...messages].reverse().find((m) => m.role === "agent" && m.turnId);
      if (lastAgentTurn && lastAgentTurn.turnId) {
        setUndoTurnTarget(lastAgentTurn.turnId);
      }
    };

    const hasUndoableTurn = messages.some((m) => m.role === "agent" && Boolean(m.turnId));
    window.dispatchEvent(
      new CustomEvent("studio:thread-state", {
        detail: { hasUndoableTurn, threadId: activeThread?.id },
      })
    );

    window.addEventListener("studio:undo-last-turn", handleUndoLastTurnEvent);
    return () => window.removeEventListener("studio:undo-last-turn", handleUndoLastTurnEvent);
  }, [messages, activeThread?.id]);

  const handlePermissionRespond = async (requestId: string, allow: boolean) => {
    try {
      await studioApi.respondPermission({ requestId, allow });
      setMessages((prev) =>
        prev.map((m) => ({
          ...m,
          permissionRequests: m.permissionRequests?.map((pr) =>
            pr.requestId === requestId ? { ...pr, resolved: true } : pr
          ),
        }))
      );
    } catch (err: any) {
      setError(err?.message || "Không thể phản hồi yêu cầu cấp quyền.");
    }
  };

  const toggleToolCallExpanded = (messageIndex: number, toolCallId: string) => {
    setMessages((prev) =>
      prev.map((msg, i) =>
        i === messageIndex
          ? {
              ...msg,
              toolCalls: msg.toolCalls?.map((tc) =>
                tc.id === toolCallId ? { ...tc, expanded: !tc.expanded } : tc
              ),
            }
          : msg
      )
    );
  };

  const lastUserMsgIdx = React.useMemo(() => {
    for (let i = messages.length - 1; i >= 0; i--) {
      if (messages[i].role === "user") return i;
    }
    return -1;
  }, [messages]);

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        backgroundColor: "var(--cs-bg-panel)",
        borderRight: "1px solid var(--cs-border-subtle)",
        position: "relative",
      }}
    >
      {/* Thread Header */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "var(--cs-space-2) var(--cs-space-4)",
          borderBottom: "1px solid var(--cs-border-subtle)",
          backgroundColor: "var(--cs-bg-raised)",
          fontSize: "var(--cs-text-sm)",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 8, flex: 1, minWidth: 0 }}>
          <span style={{ fontWeight: 600, color: "var(--cs-fg-primary)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {activeThread?.title || "Hội thoại chính"}
          </span>
          <Badge variant={activeThread?.engine === "claude" ? "info" : "neutral"}>
            {activeThread?.engine === "claude" ? "Claude" : "agy"}
          </Badge>
          {availableModels.length > 0 && (
            <div style={{ width: 170, marginLeft: 4 }}>
              <Select
                size="sm"
                value={selectedModel}
                disabled={Boolean(runningTurnId) || savingModel}
                options={[
                  { value: "", label: "(Mặc định của engine)" },
                  ...availableModels.map((m) => ({ value: m.value, label: m.label })),
                ]}
                onChange={(e: React.ChangeEvent<HTMLSelectElement>) => handleModelChange(e.target.value)}
                title="Đổi model cho các lượt tiếp theo"
              />
            </div>
          )}
        </div>

        {runningTurnId && (
          <Button
            size="sm"
            variant="destructive"
            onClick={handleCancelTurn}
            leftIcon={<Icon name="x" size={13} />}
          >
            Hủy lượt
          </Button>
        )}
      </div>

      {/* Messages Stream Body */}
      <div
        ref={scroll.ref}
        style={{
          flex: 1,
          overflowY: "auto",
          padding: "var(--cs-space-4)",
          display: "flex",
          flexDirection: "column",
          gap: "var(--cs-space-4)",
        }}
      >
        {error && (
          <Callout variant="danger" title="Lỗi lượt xử lý">
            {error}
          </Callout>
        )}

        {messages.length === 0 ? (
          <div
            style={{
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              padding: "var(--cs-space-8) var(--cs-space-4)",
              color: "var(--cs-fg-muted)",
              textAlign: "center",
              gap: 8,
            }}
          >
            <Icon name="bot" size={32} color="var(--cs-fg-muted)" />
            <span style={{ fontSize: "var(--cs-text-sm)" }}>
              Bắt đầu giao việc cho agent trong dự án này.
            </span>
            <span style={{ fontSize: "var(--cs-text-xs)" }}>
              Dùng @ để dẫn tệp, / để áp dụng quy trình playbook.
            </span>
          </div>
        ) : (
          messages.map((msg, msgIdx) =>
            msg.role === "user" ? (
              <div
                key={msg.id}
                style={{
                  display: "flex",
                  flexDirection: "column",
                  gap: 6,
                  alignSelf: "flex-end",
                  maxWidth: "85%",
                }}
              >
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    gap: 8,
                    fontSize: "var(--cs-text-xs)",
                    color: "var(--cs-fg-muted)",
                  }}
                >
                  <span style={{ fontWeight: 600 }}>Bạn</span>
                  {msgIdx === lastUserMsgIdx && !runningTurnId && (
                    <button
                      type="button"
                      onClick={() => {
                        const associatedTurnId = msg.turnId || messages[msgIdx + 1]?.turnId;
                        handleStartEditResend(msg.text, associatedTurnId);
                      }}
                      style={{
                        background: "none",
                        border: "none",
                        color: "var(--cs-fg-secondary)",
                        cursor: "pointer",
                        display: "inline-flex",
                        alignItems: "center",
                        gap: 4,
                        fontSize: "var(--cs-text-xs)",
                        padding: "2px 6px",
                        borderRadius: "var(--cs-radius-control)",
                      }}
                      title="Sửa và gửi lại tin nhắn này (hoàn tác lượt agent tương ứng nếu có)"
                    >
                      <Icon name="writing" size={11} />
                      <span>Sửa</span>
                    </button>
                  )}
                </div>
                <div
                  style={{
                    padding: "var(--cs-space-3)",
                    backgroundColor: "var(--cs-selection)",
                    color: "var(--cs-fg-primary)",
                    border: "1px solid var(--cs-border-subtle)",
                    borderRadius: "var(--cs-radius-control)",
                    fontSize: "var(--cs-text-sm)",
                    lineHeight: 1.5,
                    whiteSpace: "pre-wrap",
                    wordBreak: "break-word",
                  }}
                >
                  {msg.text}
                </div>
              </div>
            ) : (
              <AgentMessage
                key={msg.id}
                msg={msg}
                msgIdx={msgIdx}
                expandedToolGroups={expandedToolGroups}
                toggleToolGroup={toggleToolGroup}
                toggleToolCallExpanded={toggleToolCallExpanded}
                onSelectFile={onSelectFile}
                onPermissionRespond={handlePermissionRespond}
                onRerunPrompt={(p) => setInputPrompt(p)}
                onRetryTurn={handleRetryTurn}
                onOpenPublishDraft={async () => {
                  try {
                    const list = await studioApi.publishList(projectSlug);
                    const req = list.find((r) => r.status === "awaiting_confirmation") || list[0];
                    if (req) {
                      setSelectedPublishRequest(req);
                      setPublishDialogOpen(true);
                    }
                  } catch {
                    // ignore
                  }
                }}
                onNavigateSettings={onNavigateSettings}
                onUndoClick={(tId) => setUndoTurnTarget(tId)}
                isAnyTurnRunning={Boolean(runningTurnId)}
              />
            )
          )
        )}
      </div>

      {scroll.showJumpToLatest && (
        <div style={{ position: "relative", height: 0 }}>
          <button
            type="button"
            onClick={scroll.jumpToLatest}
            style={{
              position: "absolute",
              bottom: 12,
              left: "50%",
              transform: "translateX(-50%)",
              zIndex: 5,
              display: "inline-flex",
              alignItems: "center",
              gap: 6,
              padding: "4px 12px",
              fontSize: "var(--cs-text-xs)",
              color: "var(--cs-fg-primary)",
              backgroundColor: "var(--cs-bg-raised)",
              border: "1px solid var(--cs-border-strong)",
              borderRadius: 999,
              boxShadow: "var(--cs-shadow-overlay)",
              cursor: "pointer",
            }}
          >
            ↓ Về tin mới nhất
          </button>
        </div>
      )}

      {/* Mention Pickers Popup */}
      <MentionPicker
        pickerMode={pickerMode}
        pickerIndex={pickerIndex}
        pickerItems={pickerItems}
        pickerQuery={pickerQuery}
        pickerListRef={pickerListRef}
        onSelect={handleSelectPickerItem}
        onHoverIndex={(idx) => setPickerIndex(idx)}
      />

      {/* Composer Input Area */}
      <Composer
        inputPrompt={inputPrompt}
        textareaRef={textareaRef}
        runningTurnId={runningTurnId}
        sending={sending}
        queue={queue}
        queuePaused={queuePaused}
        hasActiveThread={Boolean(activeThread)}
        pendingPublishRequests={pendingPublishRequests}
        onInputChange={handleInputChange}
        onKeyDown={(e) => {
          if (handlePickerKey(e)) return;
          if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            handleSend();
          }
        }}
        onSend={handleSend}
        onOpenPublishRequest={(req) => {
          setSelectedPublishRequest(req);
          setPublishDialogOpen(true);
        }}
        onSendQueuedNow={handleSendQueuedNow}
        onEditQueued={handleEditQueued}
        onRemoveQueued={handleRemoveQueued}
        onMoveQueued={handleMoveQueued}
        onResumeQueue={() => setQueuePaused(false)}
      />

      {/* Undo Confirm Dialog */}
      <UndoConfirmDialog
        targetTurnId={undoTurnTarget}
        messages={messages}
        undoing={undoing}
        onClose={() => setUndoTurnTarget(null)}
        onConfirm={handleUndoTurn}
      />

      {/* Edit & Resend Undo Confirm Dialog */}
      <UndoConfirmDialog
        targetTurnId={editResendTargetTurnId}
        messages={messages}
        undoing={undoing}
        title="Sửa và gửi lại tin nhắn?"
        confirmLabel="Hoàn tác & Sửa"
        onClose={() => setEditResendTargetTurnId(null)}
        onConfirm={(turnId) => handleConfirmEditResend(turnId)}
      />

      {/* Undo Result Toast */}
      {undoToast && (
        <div
          style={{
            position: "absolute",
            top: 16,
            left: "50%",
            transform: "translateX(-50%)",
            backgroundColor: "var(--cs-bg-panel)",
            border: "1px solid var(--cs-success, #10b981)",
            boxShadow: "var(--cs-shadow-overlay)",
            borderRadius: "var(--cs-radius-control)",
            padding: "8px 16px",
            zIndex: 100,
            display: "flex",
            alignItems: "center",
            gap: 8,
            fontSize: "var(--cs-text-xs)",
            color: "var(--cs-fg-primary)",
          }}
        >
          <span style={{ color: "var(--cs-success, #10b981)", fontWeight: 700 }}>✓</span>
          <span>{undoToast}</span>
          <button
            type="button"
            onClick={() => setUndoToast(null)}
            style={{
              background: "none",
              border: "none",
              color: "var(--cs-fg-muted)",
              cursor: "pointer",
              marginLeft: 4,
              fontSize: 14,
              lineHeight: 1,
            }}
          >
            ×
          </button>
        </div>
      )}

      {/* Publish Dialog */}
      <PublishDialog
        open={publishDialogOpen}
        onClose={() => {
          setPublishDialogOpen(false);
          setSelectedPublishRequest(null);
          loadPublishRequests();
        }}
        projectSlug={projectSlug}
        initialRequest={selectedPublishRequest}
        onPublished={() => {
          loadPublishRequests();
        }}
      />
    </div>
  );
};
