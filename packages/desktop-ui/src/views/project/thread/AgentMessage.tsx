import React from "react";
import { Icon } from "../../../components/Icon";
import { Badge, Button, Callout } from "@studio/design-system";
import { renderMarkdown } from "../../../runtime/markdown";
import {
  groupConsecutiveToolCalls,
  isMcpLoadFailure,
  type ThreadMessage,
} from "../../../runtime/chat/turn-reducer";
import { ThinkingIndicator } from "./ThinkingIndicator";
import { UsageSummary } from "./UsageSummary";

export interface AgentMessageProps {
  msg: ThreadMessage;
  msgIdx: number;
  expandedToolGroups: Set<string>;
  toggleToolGroup: (groupKey: string) => void;
  toggleToolCallExpanded: (msgIdx: number, toolCallId: string) => void;
  onSelectFile: (path: string) => void;
  onPermissionRespond: (requestId: string, allow: boolean) => void;
  onRerunPrompt?: (prompt: string) => void;
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
  onRerunPrompt,
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
      {/* Message Header */}
      <div style={{ display: "flex", alignItems: "center", gap: 6, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
        <span style={{ fontWeight: 600 }}>Agent</span>
      </div>

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

        {/* Main Text */}
        {msg.text && (
          <div className="cs-md" style={{ whiteSpace: "normal" }}>
            {renderMarkdown(msg.text, { syntaxHighlight: !msg.isStreaming })}
          </div>
        )}

        {/* Danger Callout inside bubble on error */}
        {msg.errorMessage && (
          <div style={{ marginTop: msg.text ? 8 : 0 }}>
            <Callout variant="danger">
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

        {/* Collapsed Tool Calls */}
        {msg.toolCalls && msg.toolCalls.length > 0 && (() => {
          const groups = groupConsecutiveToolCalls(msg.toolCalls, msg.id || String(msgIdx));
          const hasMcpErr = msg.toolCalls.some((tc) => isMcpLoadFailure(tc.output));

          return (
            <div style={{ display: "flex", flexDirection: "column", gap: 6, marginTop: 8 }}>
              {groups.map((group) => {
                const isMulti = group.calls.length > 1;
                const isExpanded = expandedToolGroups.has(group.key);

                if (!isMulti) {
                  const tc = group.calls[0];
                  const mcpFail = isMcpLoadFailure(tc.output);

                  return (
                    <div
                      key={tc.id}
                      style={{
                        border: tc.isError ? "1px solid var(--cs-danger)" : "1px solid var(--cs-border-subtle)",
                        backgroundColor: "var(--cs-bg-panel)",
                        borderRadius: "var(--cs-radius-control)",
                        fontSize: "var(--cs-text-xs)",
                        overflow: "hidden",
                      }}
                    >
                      <div
                        onClick={() => toggleToolCallExpanded(msgIdx, tc.id)}
                        style={{
                          display: "flex",
                          alignItems: "center",
                          justifyContent: "space-between",
                          padding: "6px 10px",
                          cursor: "pointer",
                          backgroundColor: "var(--cs-bg-raised)",
                          userSelect: "none",
                        }}
                      >
                        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                          <Icon
                            name={tc.isError ? "x-circle" : "terminal"}
                            size={13}
                            color={tc.isError ? "var(--cs-danger)" : "var(--cs-accent)"}
                          />
                          <span style={{ fontWeight: 600, fontFamily: "var(--cs-font-mono)" }}>
                            {tc.name}
                          </span>
                          {tc.isError && <Badge variant="danger">Lỗi</Badge>}
                        </div>
                        <Icon name={tc.expanded ? "chev-up" : "chev-down"} size={12} color="var(--cs-fg-muted)" />
                      </div>
                      {tc.expanded && (
                        <div style={{ padding: "8px 10px", display: "flex", flexDirection: "column", gap: 6 }}>
                          <div>
                            <span style={{ color: "var(--cs-fg-muted)" }}>Đầu vào:</span>
                            <pre style={{ margin: "2px 0", fontSize: 11, fontFamily: "var(--cs-font-mono)", overflowX: "auto" }}>
                              {JSON.stringify(tc.input, null, 2)}
                            </pre>
                          </div>
                          {tc.output && (
                            <div>
                              <span style={{ color: "var(--cs-fg-muted)" }}>Đầu ra:</span>
                              <pre style={{ margin: "2px 0", fontSize: 11, fontFamily: "var(--cs-font-mono)", overflowX: "auto", color: tc.isError ? "var(--cs-danger)" : undefined }}>
                                {tc.output}
                              </pre>
                            </div>
                          )}
                          {mcpFail && (
                            <div style={{ marginTop: 4 }}>
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
                    </div>
                  );
                }

                // Multi-call group (e.g. view_file ×14)
                return (
                  <div
                    key={group.key}
                    style={{
                      border: group.hasErrors ? "1px solid var(--cs-danger)" : "1px solid var(--cs-border-subtle)",
                      backgroundColor: "var(--cs-bg-panel)",
                      borderRadius: "var(--cs-radius-control)",
                      fontSize: "var(--cs-text-xs)",
                      overflow: "hidden",
                    }}
                  >
                    <div
                      onClick={() => toggleToolGroup(group.key)}
                      style={{
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "space-between",
                        padding: "6px 10px",
                        cursor: "pointer",
                        backgroundColor: "var(--cs-bg-raised)",
                        userSelect: "none",
                      }}
                    >
                      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                        <Icon
                          name={group.hasErrors ? "x-circle" : "terminal"}
                          size={13}
                          color={group.hasErrors ? "var(--cs-danger)" : "var(--cs-accent)"}
                        />
                        <span style={{ fontWeight: 600, fontFamily: "var(--cs-font-mono)" }}>
                          {group.name} ×{group.calls.length}
                        </span>
                        {group.hasErrors && (
                          <Badge variant="danger">
                            {group.errorCount} lỗi
                          </Badge>
                        )}
                      </div>
                      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                        <span style={{ fontSize: 11, color: "var(--cs-fg-muted)" }}>
                          {isExpanded ? "Thu gọn" : "Xem chi tiết"}
                        </span>
                        <Icon name={isExpanded ? "chev-up" : "chev-down"} size={12} color="var(--cs-fg-muted)" />
                      </div>
                    </div>

                    {isExpanded && (
                      <div style={{ padding: "6px 10px", display: "flex", flexDirection: "column", gap: 6, backgroundColor: "var(--cs-bg-app)", borderTop: "1px solid var(--cs-border-subtle)" }}>
                        {group.calls.map((tc, cIdx) => {
                          const mcpFail = isMcpLoadFailure(tc.output);
                          const inputSummary =
                            typeof tc.input === "object" && tc.input !== null
                              ? String((tc.input as any).path || (tc.input as any).file || (tc.input as any).filePath || "")
                              : "";

                          return (
                            <div
                              key={tc.id}
                              style={{
                                border: tc.isError ? "1px solid var(--cs-danger)" : "1px solid var(--cs-border-subtle)",
                                backgroundColor: "var(--cs-bg-panel)",
                                borderRadius: "var(--cs-radius-control)",
                                fontSize: "var(--cs-text-xs)",
                                overflow: "hidden",
                              }}
                            >
                              <div
                                onClick={() => toggleToolCallExpanded(msgIdx, tc.id)}
                                style={{
                                  display: "flex",
                                  alignItems: "center",
                                  justifyContent: "space-between",
                                  padding: "5px 8px",
                                  cursor: "pointer",
                                  backgroundColor: "var(--cs-bg-raised)",
                                  userSelect: "none",
                                }}
                              >
                                <div style={{ display: "flex", alignItems: "center", gap: 6, minWidth: 0 }}>
                                  <Icon
                                    name={tc.isError ? "x-circle" : "terminal"}
                                    size={12}
                                    color={tc.isError ? "var(--cs-danger)" : "var(--cs-accent)"}
                                  />
                                  <span style={{ fontWeight: 600, fontFamily: "var(--cs-font-mono)", fontSize: 11 }}>
                                    #{cIdx + 1}
                                  </span>
                                  {inputSummary && (
                                    <span style={{ color: "var(--cs-fg-muted)", fontFamily: "var(--cs-font-mono)", fontSize: 11, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                                      {inputSummary}
                                    </span>
                                  )}
                                  {tc.isError && <Badge variant="danger">Lỗi</Badge>}
                                </div>
                                <Icon name={tc.expanded ? "chev-up" : "chev-down"} size={11} color="var(--cs-fg-muted)" />
                              </div>

                              {tc.expanded && (
                                <div style={{ padding: "8px 10px", display: "flex", flexDirection: "column", gap: 6 }}>
                                  <div>
                                    <span style={{ color: "var(--cs-fg-muted)" }}>Đầu vào:</span>
                                    <pre style={{ margin: "2px 0", fontSize: 11, fontFamily: "var(--cs-font-mono)", overflowX: "auto" }}>
                                      {JSON.stringify(tc.input, null, 2)}
                                    </pre>
                                  </div>
                                  {tc.output && (
                                    <div>
                                      <span style={{ color: "var(--cs-fg-muted)" }}>Đầu ra:</span>
                                      <pre style={{ margin: "2px 0", fontSize: 11, fontFamily: "var(--cs-font-mono)", overflowX: "auto", color: tc.isError ? "var(--cs-danger)" : undefined }}>
                                        {tc.output}
                                      </pre>
                                    </div>
                                  )}
                                  {mcpFail && (
                                    <div style={{ marginTop: 4 }}>
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
                            </div>
                          );
                        })}
                      </div>
                    )}
                  </div>
                );
              })}

              {/* Clear banner for MCP load failure */}
              {hasMcpErr && (
                <div style={{ marginTop: 4 }}>
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
          );
        })()}

        {/* File Change Chips */}
        {msg.fileChanges && msg.fileChanges.length > 0 && (
          <div style={{ display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap", marginTop: 8 }}>
            <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
              Thay đổi tệp:
            </span>
            {msg.fileChanges.map((fc, i) => (
              <button
                key={i}
                type="button"
                onClick={() => onSelectFile(fc.path)}
                style={{
                  display: "inline-flex",
                  alignItems: "center",
                  gap: 4,
                  padding: "2px 6px",
                  borderRadius: "var(--cs-radius-control)",
                  backgroundColor: "var(--cs-bg-panel)",
                  border: "1px solid var(--cs-border-subtle)",
                  fontSize: "var(--cs-text-xs)",
                  fontFamily: "var(--cs-font-mono)",
                  cursor: "pointer",
                  color: fc.op === "create" ? "var(--cs-success)" : fc.op === "delete" ? "var(--cs-danger)" : "var(--cs-accent)",
                }}
                title="Bấm để xem trên Stage"
              >
                <span>{fc.op === "create" ? "+" : fc.op === "delete" ? "-" : "~"}</span>
                <span>{fc.path}</span>
              </button>
            ))}
          </div>
        )}

        {/* Permission Requests */}
        {msg.permissionRequests?.map((pr) => !pr.resolved && (
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
        ))}

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
          {msg.usage ? <UsageSummary usage={msg.usage} /> : null}
        </div>

        {msg.turnId && !msg.isStreaming && (
          <button
            type="button"
            disabled={isAnyTurnRunning}
            onClick={() => onUndoClick?.(msg.turnId!)}
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
