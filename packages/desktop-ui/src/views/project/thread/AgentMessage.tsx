import React from "react";
import { Icon } from "../../../components/Icon";
import { Button, Callout } from "@studio/design-system";
import { renderMarkdown } from "../../../runtime/markdown";
import {
  isMcpLoadFailure,
  type ThreadMessage,
} from "../../../runtime/chat/turn-reducer";
import { ThinkingIndicator } from "./ThinkingIndicator";
import { AskUserQuestionCard } from "./AskUserQuestionCard";
import { UsageSummary } from "./UsageSummary";
import { ResultCard } from "./ResultCard";
import { ToolSteps } from "./ToolSteps";

export interface AgentMessageProps {
  msg: ThreadMessage;
  msgIdx: number;
  expandedToolGroups: Set<string>;
  toggleToolGroup: (groupKey: string) => void;
  toggleToolCallExpanded: (msgIdx: number, toolCallId: string) => void;
  onSelectFile: (path: string) => void;
  onPermissionRespond: (requestId: string, allow: boolean) => void;
  onQuestionRespond?: (requestId: string, answers: Record<string, string | string[]>) => void;
  onRerunPrompt?: (prompt: string) => void;
  onRetryTurn?: (turnId?: string) => void;
  onOpenPublishDraft?: () => void;
  onNavigateSettings?: (tab?: string) => void;
  onUndoClick?: (turnId: string) => void;
  isAnyTurnRunning: boolean;
}

export const AgentMessage: React.FC<AgentMessageProps> = ({
  msg,
  msgIdx,
  expandedToolGroups,
  toggleToolGroup,
  toggleToolCallExpanded,
  onSelectFile,
  onPermissionRespond,
  onQuestionRespond,
  onRerunPrompt,
  onRetryTurn,
  onOpenPublishDraft,
  onNavigateSettings,
  onUndoClick,
  isAnyTurnRunning,
}) => {
  const handleGoToEngines = () => {
    if (onNavigateSettings) {
      onNavigateSettings("engines");
    } else {
      window.dispatchEvent(
        new CustomEvent("studio:navigate", {
          detail: { area: "settings", tab: "engines" },
        })
      );
    }
  };

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: 6,
        alignSelf: "flex-start",
        maxWidth: "100%",
        width: "100%",
      }}
    >
      {/* Message Box */}
      <div
        style={{
          padding: "var(--cs-space-3)",
          backgroundColor: "var(--cs-bg-raised)",
          color: "var(--cs-fg-primary)",
          border: "1px solid var(--cs-border-subtle)",
          borderRadius: "var(--cs-radius-control)",
          fontSize: "var(--cs-text-sm)",
          lineHeight: 1.5,
          whiteSpace: "pre-wrap",
          wordBreak: "break-word",
        }}
      >
        {/* Thinking / working indicator */}
        <ThinkingIndicator msg={msg} />

        {/* A finished turn with nothing to show must still say what happened. */}
        {!msg.isStreaming &&
          !msg.text &&
          !msg.errorMessage &&
          !msg.toolCalls?.length &&
          !msg.fileChanges?.length &&
          !msg.permissionRequests?.length &&
          !msg.deniedActions?.length && (
            <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
              {msg.doneStatus === "cancelled" ? "Đã huỷ lượt." : "Agent kết thúc mà không trả lời."}
            </div>
          )}

        {/* Main Text */}
        {msg.text && (
          <div className="cs-md" style={{ whiteSpace: "normal" }}>
            {renderMarkdown(msg.text, { syntaxHighlight: !msg.isStreaming })}
          </div>
        )}

        {/* Danger Callout inside bubble on error */}
        {msg.errorMessage && (
          <div style={{ marginTop: msg.text ? 8 : 0 }}>
            <Callout
              variant="danger"
              actions={
                onRetryTurn && !isAnyTurnRunning ? (
                  <Button
                    size="sm"
                    variant="primary"
                    onClick={() => onRetryTurn(msg.turnId)}
                    leftIcon={<Icon name="refresh" size={12} />}
                  >
                    Thử lại
                  </Button>
                ) : undefined
              }
            >
              <div style={{ fontWeight: 600 }}>Engine báo lỗi:</div>
              <div style={{ marginTop: 2 }}>{msg.errorMessage}</div>
            </Callout>
            {isMcpLoadFailure(msg.errorMessage) && (
              <div style={{ marginTop: 8 }}>
                <Callout
                  variant="danger"
                  title="Lỗi kết nối Studio MCP"
                  actions={
                    <Button
                      size="sm"
                      variant="primary"
                      onClick={handleGoToEngines}
                      leftIcon={<Icon name="settings" size={13} />}
                    >
                      Kết nối agy với Studio
                    </Button>
                  }
                >
                  agy chưa kết nối được Studio — mở Tools & Cài đặt → Engines → Kết nối agy với Studio
                </Callout>
              </div>
            )}
          </div>
        )}

        {/* Collapsed Tool Calls (Task D: ToolSteps summary & progressive disclosure) */}
        {msg.toolCalls && msg.toolCalls.length > 0 && (
          <ToolSteps
            toolCalls={msg.toolCalls}
            turnId={msg.turnId}
            msgId={msg.id}
            msgIdx={msgIdx}
            isStreaming={msg.isStreaming}
            errorMessage={msg.errorMessage}
            expandedToolGroups={expandedToolGroups}
            toggleToolGroup={toggleToolGroup}
            toggleToolCallExpanded={toggleToolCallExpanded}
            onNavigateEngines={handleGoToEngines}
          />
        )}

        {/* Results Card */}
        {msg.fileChanges && msg.fileChanges.length > 0 && (
          <ResultCard fileChanges={msg.fileChanges} onSelectFile={onSelectFile} />
        )}

        {/* Permission Requests & AskUserQuestion Cards */}
        {msg.permissionRequests?.map((pr) => {
          const isAskQuestion = pr.tool === "AskUserQuestion" || pr.tool.endsWith("AskUserQuestion");
          if (isAskQuestion) {
            return (
              <AskUserQuestionCard
                key={pr.requestId}
                requestId={pr.requestId}
                tool={pr.tool}
                input={pr.input}
                resolved={pr.resolved}
                answers={pr.answers}
                onRespond={(answers) => onQuestionRespond?.(pr.requestId, answers)}
              />
            );
          }
          if (pr.resolved) return null;
          return (
            <div
              key={pr.requestId}
              style={{
                marginTop: 8,
                padding: "var(--cs-space-3)",
                border: "1px solid var(--cs-warning)",
                backgroundColor: "var(--cs-status-warning-bg)",
                borderRadius: "var(--cs-radius-control)",
                display: "flex",
                flexDirection: "column",
                gap: 6,
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: 6, fontWeight: 600, color: "var(--cs-warning)" }}>
                <Icon name="shield-alert" size={16} />
                <span>Yêu cầu cấp quyền chạy công cụ</span>
              </div>
              <div style={{ fontSize: "var(--cs-text-xs)" }}>
                Agent muốn thực thi: <code>{pr.tool}</code>
              </div>
              <div style={{ display: "flex", gap: "var(--cs-space-2)", marginTop: 4 }}>
                <Button size="sm" variant="primary" onClick={() => onPermissionRespond(pr.requestId, true)}>
                  Cho phép
                </Button>
                <Button size="sm" variant="secondary" onClick={() => onPermissionRespond(pr.requestId, false)}>
                  Từ chối
                </Button>
              </div>
            </div>
          );
        })}

        {/* Denied Actions */}
        {msg.deniedActions?.map((da, i) => (
          <div
            key={i}
            style={{
              marginTop: 8,
              padding: "8px",
              border: "1px solid var(--cs-danger)",
              backgroundColor: "var(--cs-status-danger-bg)",
              borderRadius: "var(--cs-radius-control)",
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
            }}
          >
            <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-danger)" }}>
              Thao tác <code>{da.tool}</code> bị từ chối.
            </span>
            <Button
              size="sm"
              variant="danger-outline"
              onClick={() => onRerunPrompt?.(`Cho phép và chạy lại ${da.tool}`)}
            >
              Cho phép và chạy lại
            </Button>
          </div>
        ))}

        {/* Agent Publish Draft In-Turn Banner */}
        {msg.toolCalls?.some((tc) => tc.name.includes("publish_prepare")) && (
          <div
            style={{
              marginTop: 8,
              padding: "8px 12px",
              backgroundColor: "var(--cs-status-info-bg)",
              border: "1px solid var(--cs-accent)",
              borderRadius: "var(--cs-radius-control)",
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              gap: 8,
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: 6, fontSize: "var(--cs-text-xs)", color: "var(--cs-accent)" }}>
              <Icon name="publishing" size={14} color="var(--cs-accent)" />
              <span style={{ fontWeight: 600 }}>Agent đã chuẩn bị bản đăng — xem lại và xác nhận</span>
            </div>
            <Button
              size="sm"
              variant="primary"
              onClick={onOpenPublishDraft}
            >
              Xem lại & xác nhận
            </Button>
          </div>
        )}
      </div>

      {/* Pinned Artifact Warning Banner */}
      {msg.pinnedWarning && (
        <div
          style={{
            marginTop: 8,
            padding: "10px 12px",
            backgroundColor: "var(--cs-status-warning-bg)",
            border: "1px solid var(--cs-warning)",
            borderRadius: "var(--cs-radius-control)",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            gap: 8,
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 6, fontSize: "var(--cs-text-xs)", color: "var(--cs-warning)" }}>
            <Icon name="shield-alert" size={15} color="var(--cs-warning)" />
            <span style={{ fontWeight: 600 }}>
              {`Artifact khoá "${msg.pinnedWarning.artifactId}" đã bị thay đổi file trong lượt này.`}
            </span>
          </div>
          <Button
            size="sm"
            variant="secondary"
            onClick={async () => {
              const proj = window.location.pathname.split("/").pop() || "";
              try {
                await (window as any).__studioApi_restorePinned?.(msg.pinnedWarning!.artifactId, msg.pinnedWarning!.checkpointId);
              } catch {
                // Fallback or trigger via event
              }
              window.dispatchEvent(
                new CustomEvent("studio:restore-pinned", {
                  detail: msg.pinnedWarning,
                })
              );
            }}
          >
            Hoàn tác phần này
          </Button>
        </div>
      )}

      {/* Per-Turn Footer (Usage / Undo) */}
      <div
        style={{
          display: "flex",
          alignItems: "flex-start",
          justifyContent: "space-between",
          gap: 8,
          fontSize: "var(--cs-text-xs)",
          color: "var(--cs-fg-muted)",
          padding: "0 4px",
        }}
      >
        <div style={{ flex: 1, minWidth: 0 }}>
          {msg.usage ? <UsageSummary usage={msg.usage} model={msg.model} /> : null}
        </div>

        {msg.turnId && !msg.isStreaming && (
          <button
            type="button"
            disabled={isAnyTurnRunning}
            onClick={() => onUndoClick?.(msg.turnId!)}
            aria-label="Hoàn tác các file đã sửa trong lượt này"
            style={{
              background: "none",
              border: "none",
              padding: "2px 4px",
              color: isAnyTurnRunning ? "var(--cs-fg-muted)" : "var(--cs-fg-secondary)",
              cursor: isAnyTurnRunning ? "not-allowed" : "pointer",
              opacity: isAnyTurnRunning ? 0.4 : 1,
              display: "inline-flex",
              alignItems: "center",
              gap: 4,
              fontSize: "var(--cs-text-xs)",
              borderRadius: "var(--cs-radius-control)",
              whiteSpace: "nowrap",
              flexShrink: 0,
            }}
            title={
              isAnyTurnRunning
                ? "Không thể hoàn tác khi agent đang chạy một lượt khác"
                : "Hoàn tác các file đã sửa trong lượt này"
            }
          >
            <Icon name="undo" size={12} />
            <span>Hoàn tác</span>
          </button>
        )}
      </div>
    </div>
  );
};
