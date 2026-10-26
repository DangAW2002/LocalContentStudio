import React from "react";
import { Icon } from "../../../components/Icon";
import { Badge, Button, Callout } from "@studio/design-system";
import {
  groupConsecutiveToolCalls,
  isMcpLoadFailure,
  type ToolCallItem,
} from "../../../runtime/chat/turn-reducer";
import { describeStep } from "../../../runtime/chat/activity-label";

export interface ToolStepsProps {
  toolCalls: ToolCallItem[];
  turnId?: string;
  msgId?: string;
  msgIdx: number;
  isStreaming?: boolean;
  errorMessage?: string;
  expandedToolGroups: Set<string>;
  toggleToolGroup: (groupKey: string) => void;
  toggleToolCallExpanded: (msgIdx: number, toolCallId: string) => void;
  onNavigateEngines?: () => void;
}

export const ToolSteps: React.FC<ToolStepsProps> = ({
  toolCalls,
  turnId,
  msgId,
  msgIdx,
  isStreaming = false,
  errorMessage,
  expandedToolGroups,
  toggleToolGroup,
  toggleToolCallExpanded,
  onNavigateEngines,
}) => {
  if (!toolCalls || toolCalls.length === 0) return null;

  const stepCount = toolCalls.length;
  const errorCount = toolCalls.filter((tc) => tc.isError).length;
  const lastTool = toolCalls[toolCalls.length - 1];

  // Auto-expand when the turn is done, the turn errored, and the last tool failed
  const shouldAutoExpand = !isStreaming && Boolean(errorMessage) && Boolean(lastTool?.isError);

  const [userExpanded, setUserExpanded] = React.useState<boolean | null>(null);
  const isOpen = userExpanded !== null ? userExpanded : shouldAutoExpand;

  const handleGoToEngines = () => {
    if (onNavigateEngines) {
      onNavigateEngines();
    } else {
      window.dispatchEvent(
        new CustomEvent("studio:navigate", {
          detail: { area: "settings", tab: "engines" },
        })
      );
    }
  };

  const groups = groupConsecutiveToolCalls(toolCalls, msgId || String(msgIdx));
  const hasMcpErr = toolCalls.some((tc) => isMcpLoadFailure(tc.output));

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: 6,
        marginTop: 8,
      }}
    >
      {/* Collapsed summary row: "▸ 22 bước · 3 lỗi" */}
      <div
        role="button"
        tabIndex={0}
        data-testid="tool-steps-summary"
        onClick={() => setUserExpanded(!isOpen)}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            setUserExpanded(!isOpen);
          }
        }}
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "6px 10px",
          borderRadius: "var(--cs-radius-control)",
          backgroundColor: "var(--cs-bg-panel)",
          border: "1px solid var(--cs-border-subtle)",
          cursor: "pointer",
          userSelect: "none",
          fontSize: "var(--cs-text-xs)",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <span style={{ fontSize: 11, color: "var(--cs-fg-muted)", fontFamily: "monospace" }}>
            {isOpen ? "▾" : "▸"}
          </span>
          <span style={{ fontWeight: 600, color: "var(--cs-fg-secondary)" }}>
            {stepCount} bước
          </span>
          {errorCount > 0 && (
            <span style={{ color: "var(--cs-danger)", fontWeight: 600 }}>
              · {errorCount} lỗi
            </span>
          )}
        </div>
        <span style={{ fontSize: 11, color: "var(--cs-fg-muted)" }}>
          {isOpen ? "Thu gọn" : "Chi tiết"}
        </span>
      </div>

      {/* When streaming: show only the latest step (1 line) beneath the summary row */}
      {isStreaming && !isOpen && lastTool && (
        <div
          data-testid="tool-steps-latest"
          style={{
            display: "flex",
            alignItems: "center",
            gap: 6,
            padding: "4px 8px",
            fontSize: "var(--cs-text-xs)",
            color: lastTool.isError ? "var(--cs-danger)" : "var(--cs-fg-secondary)",
            backgroundColor: "var(--cs-bg-panel)",
            borderRadius: "var(--cs-radius-control)",
            border: lastTool.isError
              ? "1px solid var(--cs-danger)"
              : "1px solid var(--cs-border-subtle)",
            overflow: "hidden",
          }}
        >
          <Icon
            name={lastTool.isError ? "x-circle" : "terminal"}
            size={12}
            color={lastTool.isError ? "var(--cs-danger)" : "var(--cs-accent)"}
          />
          <span
            style={{
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
              fontWeight: 500,
              flex: 1,
            }}
          >
            {describeStep(lastTool.name, lastTool.input)}
          </span>
          {lastTool.isError && <Badge variant="danger">Lỗi</Badge>}
        </div>
      )}

      {/* Expanded list of grouped tool steps */}
      {isOpen && (
        <div
          data-testid="tool-steps-expanded"
          style={{ display: "flex", flexDirection: "column", gap: 6 }}
        >
          {groups.map((group) => {
            const isMulti = group.calls.length > 1;
            const isGroupExpanded =
              expandedToolGroups.has(group.key) ||
              (shouldAutoExpand && group.calls.some((c) => c.id === lastTool?.id));

            if (!isMulti) {
              const tc = group.calls[0];
              const isCallExpanded =
                tc.expanded || (shouldAutoExpand && tc.id === lastTool?.id);
              const mcpFail = isMcpLoadFailure(tc.output);

              return (
                <div
                  key={tc.id}
                  style={{
                    border: tc.isError
                      ? "1px solid var(--cs-danger)"
                      : "1px solid var(--cs-border-subtle)",
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
                    <div style={{ display: "flex", alignItems: "center", gap: 6, minWidth: 0 }}>
                      <Icon
                        name={tc.isError ? "x-circle" : "terminal"}
                        size={13}
                        color={tc.isError ? "var(--cs-danger)" : "var(--cs-accent)"}
                      />
                      <span
                        style={{
                          fontWeight: 600,
                          overflow: "hidden",
                          textOverflow: "ellipsis",
                          whiteSpace: "nowrap",
                        }}
                      >
                        {describeStep(tc.name, tc.input)}
                      </span>
                      {tc.isError && <Badge variant="danger">Lỗi</Badge>}
                    </div>
                    <Icon
                      name={isCallExpanded ? "chev-up" : "chev-down"}
                      size={12}
                      color="var(--cs-fg-muted)"
                    />
                  </div>
                  {isCallExpanded && (
                    <div
                      style={{
                        padding: "8px 10px",
                        display: "flex",
                        flexDirection: "column",
                        gap: 6,
                      }}
                    >
                      <div>
                        <span style={{ color: "var(--cs-fg-muted)" }}>Đầu vào:</span>
                        <pre
                          style={{
                            margin: "2px 0",
                            fontSize: 11,
                            fontFamily: "var(--cs-font-mono)",
                            overflowX: "auto",
                          }}
                        >
                          {JSON.stringify(tc.input, null, 2)}
                        </pre>
                      </div>
                      {tc.output && (
                        <div>
                          <span style={{ color: "var(--cs-fg-muted)" }}>Đầu ra:</span>
                          <pre
                            style={{
                              margin: "2px 0",
                              fontSize: 11,
                              fontFamily: "var(--cs-font-mono)",
                              overflowX: "auto",
                              color: tc.isError ? "var(--cs-danger)" : undefined,
                            }}
                          >
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
                  border: group.hasErrors
                    ? "1px solid var(--cs-danger)"
                    : "1px solid var(--cs-border-subtle)",
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
                    <span style={{ fontWeight: 600 }}>
                      {describeStep(group.name)} ×{group.calls.length}
                    </span>
                    {group.hasErrors && (
                      <Badge variant="danger">{group.errorCount} lỗi</Badge>
                    )}
                  </div>
                  <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                    <span style={{ fontSize: 11, color: "var(--cs-fg-muted)" }}>
                      {isGroupExpanded ? "Thu gọn" : "Xem chi tiết"}
                    </span>
                    <Icon
                      name={isGroupExpanded ? "chev-up" : "chev-down"}
                      size={12}
                      color="var(--cs-fg-muted)"
                    />
                  </div>
                </div>

                {isGroupExpanded && (
                  <div
                    style={{
                      padding: "6px 10px",
                      display: "flex",
                      flexDirection: "column",
                      gap: 6,
                      backgroundColor: "var(--cs-bg-app)",
                      borderTop: "1px solid var(--cs-border-subtle)",
                    }}
                  >
                    {group.calls.map((tc, cIdx) => {
                      const isCallExpanded =
                        tc.expanded || (shouldAutoExpand && tc.id === lastTool?.id);
                      const mcpFail = isMcpLoadFailure(tc.output);

                      return (
                        <div
                          key={tc.id}
                          style={{
                            border: tc.isError
                              ? "1px solid var(--cs-danger)"
                              : "1px solid var(--cs-border-subtle)",
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
                            <div
                              style={{
                                display: "flex",
                                alignItems: "center",
                                gap: 6,
                                minWidth: 0,
                              }}
                            >
                              <Icon
                                name={tc.isError ? "x-circle" : "terminal"}
                                size={12}
                                color={tc.isError ? "var(--cs-danger)" : "var(--cs-accent)"}
                              />
                              <span
                                style={{
                                  fontWeight: 600,
                                  fontFamily: "var(--cs-font-mono)",
                                  fontSize: 11,
                                }}
                              >
                                #{cIdx + 1}
                              </span>
                              <span
                                style={{
                                  color: "var(--cs-fg-secondary)",
                                  fontSize: 11,
                                  overflow: "hidden",
                                  textOverflow: "ellipsis",
                                  whiteSpace: "nowrap",
                                }}
                              >
                                {describeStep(tc.name, tc.input)}
                              </span>
                              {tc.isError && <Badge variant="danger">Lỗi</Badge>}
                            </div>
                            <Icon
                              name={isCallExpanded ? "chev-up" : "chev-down"}
                              size={11}
                              color="var(--cs-fg-muted)"
                            />
                          </div>

                          {isCallExpanded && (
                            <div
                              style={{
                                padding: "8px 10px",
                                display: "flex",
                                flexDirection: "column",
                                gap: 6,
                              }}
                            >
                              <div>
                                <span style={{ color: "var(--cs-fg-muted)" }}>Đầu vào:</span>
                                <pre
                                  style={{
                                    margin: "2px 0",
                                    fontSize: 11,
                                    fontFamily: "var(--cs-font-mono)",
                                    overflowX: "auto",
                                  }}
                                >
                                  {JSON.stringify(tc.input, null, 2)}
                                </pre>
                              </div>
                              {tc.output && (
                                <div>
                                  <span style={{ color: "var(--cs-fg-muted)" }}>Đầu ra:</span>
                                  <pre
                                    style={{
                                      margin: "2px 0",
                                      fontSize: 11,
                                      fontFamily: "var(--cs-font-mono)",
                                      overflowX: "auto",
                                      color: tc.isError ? "var(--cs-danger)" : undefined,
                                    }}
                                  >
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

          {/* MCP load failure general banner if error present */}
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
      )}
    </div>
  );
};
