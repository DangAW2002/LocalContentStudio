import React from "react";
import { Icon } from "../../components/Icon";
import { Button, Badge, Callout, Kbd, Dialog } from "@studio/design-system";
import { studioApi } from "../../api/client";
import type {
  EngineTurnEvent,
  ProjectFileEntry,
  ScopeItem,
  ThreadSummary,
} from "../../api/types";

export interface ThreadMessage {
  id: string;
  role: "user" | "agent";
  text: string;
  thinking?: string;
  toolCalls?: Array<{
    id: string;
    name: string;
    input: unknown;
    output?: string;
    isError?: boolean;
    expanded?: boolean;
  }>;
  fileChanges?: Array<{ path: string; op: "create" | "modify" | "delete" }>;
  permissionRequests?: Array<{ requestId: string; tool: string; input: unknown; resolved?: boolean }>;
  deniedActions?: Array<{ tool: string; input: unknown }>;
  usage?: {
    inputTokens: number;
    outputTokens: number;
    cacheReadTokens: number;
    costUsd?: number;
  };
  turnId?: string;
  isStreaming?: boolean;
}

export interface ThreadPaneProps {
  projectSlug: string;
  activeThread?: ThreadSummary;
  projectFiles: ProjectFileEntry[];
  playbooks: ScopeItem[];
  onSelectFile: (path: string) => void;
}

/**
 * Pure reducer: fold one turn event into the message list. Used for the live stream and for history.
 * Every agent-side event lands in the agent message of its turn (created on demand).
 */
export function applyTurnEvent(prev: ThreadMessage[], payload: EngineTurnEvent): ThreadMessage[] {
  const event = payload.event;
  if (event.type === "UserPrompt") {
    const last = prev[prev.length - 1];
    // The UI already appended the prompt optimistically: attach the turn id instead of duplicating.
    if (last && last.role === "user" && !last.turnId && last.text === event.text) {
      return [...prev.slice(0, -1), { ...last, turnId: payload.turnId }];
    }
    return [...prev, { id: `user-${payload.turnId}`, role: "user", text: event.text, turnId: payload.turnId }];
  }
  if (event.type === "Session") return prev;

  let list = prev;
  let idx = list.findIndex((m) => m.role === "agent" && m.turnId === payload.turnId);
  if (idx === -1) {
    list = [...list, { id: `turn-${payload.turnId}`, role: "agent", text: "", turnId: payload.turnId, isStreaming: true }];
    idx = list.length - 1;
  }
  const msg = list[idx];
  let next: ThreadMessage = msg;
  switch (event.type) {
    case "TextDelta":
      next = { ...msg, text: msg.text + event.text };
      break;
    case "Text":
      next = { ...msg, text: event.text };
      break;
    case "Thinking":
      next = { ...msg, thinking: (msg.thinking || "") + event.text };
      break;
    case "ToolCall":
      next = { ...msg, toolCalls: [...(msg.toolCalls || []), { id: event.id, name: event.name, input: event.input, expanded: false }] };
      break;
    case "ToolResult":
      next = {
        ...msg,
        toolCalls: (msg.toolCalls || []).map((tc) =>
          tc.id === event.id ? { ...tc, output: event.outputPreview, isError: event.isError } : tc
        ),
      };
      break;
    case "FileChange":
      next = { ...msg, fileChanges: [...(msg.fileChanges || []), { path: event.path, op: event.op }] };
      break;
    case "PermissionRequest":
      next = { ...msg, permissionRequests: [...(msg.permissionRequests || []), { requestId: event.requestId, tool: event.tool, input: event.input }] };
      break;
    case "PermissionDenied":
      next = { ...msg, deniedActions: [...(msg.deniedActions || []), { tool: event.tool, input: event.input }] };
      break;
    case "Usage":
      next = {
        ...msg,
        usage: {
          inputTokens: event.inputTokens,
          outputTokens: event.outputTokens,
          cacheReadTokens: event.cacheReadTokens,
          costUsd: event.costUsd,
        },
      };
      break;
    case "Done":
      next = { ...msg, isStreaming: false, text: msg.text || event.text || "" };
      break;
    case "Error":
      next = { ...msg, isStreaming: false, text: msg.text ? `${msg.text}\n\n⚠ ${event.message}` : `⚠ ${event.message}` };
      break;
  }
  const out = list.slice();
  out[idx] = next;
  return out;
}

export const ThreadPane: React.FC<ThreadPaneProps> = ({
  projectSlug,
  activeThread,
  projectFiles,
  playbooks,
  onSelectFile,
}) => {
  const [messages, setMessages] = React.useState<ThreadMessage[]>([]);
  const [inputPrompt, setInputPrompt] = React.useState("");
  const [runningTurnId, setRunningTurnId] = React.useState<string | null>(null);
  const [pickerMode, setPickerMode] = React.useState<"@" | "/" | null>(null);
  const [pickerIndex, setPickerIndex] = React.useState(0);
  const [undoTurnTarget, setUndoTurnTarget] = React.useState<string | null>(null);
  const [error, setError] = React.useState<string | null>(null);

  const messagesEndRef = React.useRef<HTMLDivElement>(null);
  const textareaRef = React.useRef<HTMLTextAreaElement>(null);

  // Scroll to bottom on new messages
  React.useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  // Load persisted history of the active thread (same reducer as the live stream).
  React.useEffect(() => {
    let cancelled = false;
    setMessages([]);
    setRunningTurnId(null);
    if (!activeThread) return;
    (async () => {
      try {
        let folded: ThreadMessage[] = [];
        let cursor: string | undefined;
        for (let page = 0; page < 50; page++) {
          const res = await studioApi.getThreadEvents({ threadId: activeThread.id, cursor, limit: 500 });
          for (const ev of res.events) folded = applyTurnEvent(folded, ev);
          if (!res.hasMore || !res.nextCursor) break;
          cursor = res.nextCursor;
        }
        if (!cancelled) setMessages(folded.map((m) => ({ ...m, isStreaming: false })));
      } catch (err: any) {
        if (!cancelled) setError(err?.message || "Không tải được lịch sử hội thoại.");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [activeThread?.id]);

  // Listen to turn-event
  React.useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      try {
        unlisten = await studioApi.onTurnEvent((payload) => {
          if (!activeThread || payload.threadId !== activeThread.id) return;
          setMessages((prev) => applyTurnEvent(prev, payload));
          if (payload.event.type === "Done") setRunningTurnId(null);
          if (payload.event.type === "Error") {
            setRunningTurnId(null);
            setError(payload.event.message);
          }
        });
      } catch (err: any) {
        // Core absent
      }
    };

    setupListener();
    return () => {
      if (unlisten) unlisten();
    };
  }, [activeThread]);

  // Handle send message
  const handleSend = async () => {
    if (!inputPrompt.trim() || !activeThread || runningTurnId) return;

    const userText = inputPrompt.trim();
    setInputPrompt("");
    setPickerMode(null);
    setError(null);

    // Append user message
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
      setRunningTurnId(res.turnId);
    } catch (err: any) {
      setError(err?.message || "Không thể bắt đầu lượt agent.");
    }
  };

  // Handle cancel
  const handleCancelTurn = async () => {
    if (!runningTurnId) return;
    try {
      await studioApi.cancelTurn({ turnId: runningTurnId });
      setRunningTurnId(null);
    } catch (err: any) {
      setError(err?.message || "Không thể hủy lượt agent.");
    }
  };

  // Handle undo turn
  const handleUndoTurn = async (turnId: string) => {
    try {
      await studioApi.undoTurn({ turnId });
      setUndoTurnTarget(null);
      // Remove undone messages
      setMessages((prev) => prev.filter((m) => m.turnId !== turnId));
    } catch (err: any) {
      setError(err?.message || "Không thể hoàn tác lượt.");
    }
  };

  // Handle permission response
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

  // Handle picker trigger
  const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = e.target.value;
    setInputPrompt(val);

    const lastChar = val.slice(-1);
    if (lastChar === "@") {
      setPickerMode("@");
      setPickerIndex(0);
    } else if (lastChar === "/") {
      setPickerMode("/");
      setPickerIndex(0);
    } else if (val.endsWith(" ") || val === "") {
      setPickerMode(null);
    }
  };

  const handleSelectPickerItem = (itemText: string) => {
    if (pickerMode === "@") {
      setInputPrompt((prev) => prev.slice(0, -1) + `@${itemText} `);
    } else if (pickerMode === "/") {
      setInputPrompt((prev) => prev.slice(0, -1) + `/${itemText} `);
    }
    setPickerMode(null);
    textareaRef.current?.focus();
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
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <span style={{ fontWeight: 600, color: "var(--cs-fg-primary)" }}>
            {activeThread?.title || "Hội thoại chính"}
          </span>
          <Badge variant={activeThread?.engine === "claude" ? "info" : "neutral"}>
            {activeThread?.engine === "claude" ? "Claude" : "agy"}
          </Badge>
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
          messages.map((msg, msgIdx) => (
            <div
              key={msg.id}
              style={{
                display: "flex",
                flexDirection: "column",
                gap: 6,
                alignSelf: msg.role === "user" ? "flex-end" : "flex-start",
                maxWidth: msg.role === "user" ? "85%" : "100%",
                width: msg.role === "agent" ? "100%" : "auto",
              }}
            >
              {/* Message Header */}
              <div style={{ display: "flex", alignItems: "center", gap: 6, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
                <span style={{ fontWeight: 600 }}>{msg.role === "user" ? "Bạn" : "Agent"}</span>
              </div>

              {/* Message Box */}
              <div
                style={{
                  padding: "var(--cs-space-3)",
                  backgroundColor: msg.role === "user" ? "var(--cs-selection)" : "var(--cs-bg-raised)",
                  color: "var(--cs-fg-primary)",
                  border: "1px solid var(--cs-border-subtle)",
                  borderRadius: "var(--cs-radius-control)",
                  fontSize: "var(--cs-text-sm)",
                  lineHeight: 1.5,
                  whiteSpace: "pre-wrap",
                  wordBreak: "break-word",
                }}
              >
                {/* Thinking section (if present) */}
                {msg.thinking && (
                  <details
                    style={{
                      marginBottom: 8,
                      padding: "4px 8px",
                      backgroundColor: "var(--cs-bg-panel)",
                      border: "1px solid var(--cs-border-subtle)",
                      fontSize: "var(--cs-text-xs)",
                      color: "var(--cs-fg-secondary)",
                    }}
                  >
                    <summary style={{ cursor: "pointer", fontWeight: 600 }}>
                      Suy nghĩ của agent...
                    </summary>
                    <div style={{ marginTop: 4, whiteSpace: "pre-wrap", fontFamily: "var(--cs-font-mono)" }}>
                      {msg.thinking}
                    </div>
                  </details>
                )}

                {/* Main Text */}
                {msg.text}

                {/* Collapsed Tool Calls */}
                {msg.toolCalls && msg.toolCalls.length > 0 && (
                  <div style={{ display: "flex", flexDirection: "column", gap: 6, marginTop: 8 }}>
                    {msg.toolCalls.map((tc) => (
                      <div
                        key={tc.id}
                        style={{
                          border: "1px solid var(--cs-border-subtle)",
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
                            <Icon name={tc.isError ? "x-circle" : "terminal"} size={13} color={tc.isError ? "var(--cs-danger)" : "var(--cs-accent)"} />
                            <span style={{ fontWeight: 600, fontFamily: "var(--cs-font-mono)" }}>
                              {tc.name}
                            </span>
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
                                <pre style={{ margin: "2px 0", fontSize: 11, fontFamily: "var(--cs-font-mono)", overflowX: "auto" }}>
                                  {tc.output}
                                </pre>
                              </div>
                            )}
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                )}

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
                      <Button size="sm" variant="primary" onClick={() => handlePermissionRespond(pr.requestId, true)}>
                        Cho phép
                      </Button>
                      <Button size="sm" variant="secondary" onClick={() => handlePermissionRespond(pr.requestId, false)}>
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
                      onClick={() => {
                        // Re-run prompt
                        setInputPrompt(`Cho phép và chạy lại ${da.tool}`);
                      }}
                    >
                      Cho phép và chạy lại
                    </Button>
                  </div>
                ))}
              </div>

              {/* Per-Turn Footer (Usage / Undo) */}
              {msg.role === "agent" && (
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    fontSize: "var(--cs-text-xs)",
                    color: "var(--cs-fg-muted)",
                    padding: "0 4px",
                  }}
                >
                  {/* Usage / Cost */}
                  <div>
                    {msg.usage ? (
                      msg.usage.costUsd !== undefined ? (
                        <span>
                          ${msg.usage.costUsd.toFixed(4)} · {(msg.usage.inputTokens + msg.usage.outputTokens).toLocaleString()} tokens
                        </span>
                      ) : (
                        <span>
                          {msg.usage.inputTokens.toLocaleString()} in · {msg.usage.outputTokens.toLocaleString()} out
                        </span>
                      )
                    ) : null}
                  </div>

                  {/* Undo Button */}
                  {msg.turnId && !msg.isStreaming && (
                    <button
                      type="button"
                      onClick={() => setUndoTurnTarget(msg.turnId!)}
                      style={{
                        background: "none",
                        border: "none",
                        padding: "2px 4px",
                        color: "var(--cs-fg-secondary)",
                        cursor: "pointer",
                        display: "inline-flex",
                        alignItems: "center",
                        gap: 4,
                        fontSize: "var(--cs-text-xs)",
                        borderRadius: "var(--cs-radius-control)",
                      }}
                      title="Hoàn tác các file đã sửa trong lượt này"
                    >
                      <Icon name="undo" size={12} />
                      <span>Hoàn tác lượt này</span>
                    </button>
                  )}
                </div>
              )}
            </div>
          ))
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Mention Pickers Popup */}
      {pickerMode && (
        <div
          style={{
            position: "absolute",
            bottom: 120,
            left: "var(--cs-space-4)",
            right: "var(--cs-space-4)",
            backgroundColor: "var(--cs-bg-panel)",
            border: "1px solid var(--cs-border-strong)",
            boxShadow: "var(--cs-shadow-overlay)",
            borderRadius: "var(--cs-radius-control)",
            maxHeight: 180,
            overflowY: "auto",
            zIndex: 15,
            padding: "4px 0",
          }}
        >
          <div style={{ padding: "4px 10px", fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)", borderBottom: "1px solid var(--cs-border-subtle)" }}>
            {pickerMode === "@" ? "Chọn tệp trong dự án:" : "Chọn quy trình playbook:"}
          </div>
          {pickerMode === "@" ? (
            projectFiles.length === 0 ? (
              <div style={{ padding: "8px 10px", fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
                Chưa có tệp nào trong dự án
              </div>
            ) : (
              projectFiles.map((f) => (
                <div
                  key={f.path}
                  onClick={() => handleSelectPickerItem(f.path)}
                  style={{
                    padding: "6px 10px",
                    cursor: "pointer",
                    display: "flex",
                    alignItems: "center",
                    gap: 6,
                    fontSize: "var(--cs-text-xs)",
                    fontFamily: "var(--cs-font-mono)",
                  }}
                  onMouseEnter={(e) => (e.currentTarget.style.backgroundColor = "var(--cs-bg-hover)")}
                  onMouseLeave={(e) => (e.currentTarget.style.backgroundColor = "transparent")}
                >
                  <Icon name="file" size={13} color="var(--cs-accent)" />
                  <span>{f.path}</span>
                </div>
              ))
            )
          ) : (
            playbooks.length === 0 ? (
              <div style={{ padding: "8px 10px", fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
                Chưa có playbook nào trong scope chain
              </div>
            ) : (
              playbooks.map((p) => (
                <div
                  key={p.id}
                  onClick={() => handleSelectPickerItem(p.name)}
                  style={{
                    padding: "6px 10px",
                    cursor: "pointer",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    fontSize: "var(--cs-text-xs)",
                  }}
                  onMouseEnter={(e) => (e.currentTarget.style.backgroundColor = "var(--cs-bg-hover)")}
                  onMouseLeave={(e) => (e.currentTarget.style.backgroundColor = "transparent")}
                >
                  <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                    <Icon name="sparkles" size={13} color="var(--cs-accent)" />
                    <span style={{ fontWeight: 600 }}>{p.name}</span>
                  </div>
                  <span style={{ color: "var(--cs-fg-muted)", fontSize: 11 }}>{p.layer}</span>
                </div>
              ))
            )
          )}
        </div>
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
            onChange={handleInputChange}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                handleSend();
              }
            }}
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
            disabled={!inputPrompt.trim() || Boolean(runningTurnId)}
            busy={Boolean(runningTurnId)}
            busyText="Agent đang xử lý..."
            onClick={handleSend}
            leftIcon={<Icon name="arrow-right" size={14} />}
          >
            Gửi
          </Button>
        </div>
      </div>

      {/* Undo Confirm Dialog */}
      {undoTurnTarget && (
        <Dialog
          title="Xác nhận hoàn tác lượt này?"
          open={Boolean(undoTurnTarget)}
          onClose={() => setUndoTurnTarget(null)}
          actions={
            <>
              <Button variant="secondary" onClick={() => setUndoTurnTarget(null)}>
                Hủy
              </Button>
              <Button
                variant="destructive"
                onClick={() => handleUndoTurn(undoTurnTarget)}
              >
                Hoàn tác lượt
              </Button>
            </>
          }
        >
          <p style={{ margin: 0, fontSize: "var(--cs-text-sm)", lineHeight: 1.5 }}>
            Thao tác này sẽ khôi phục lại toàn bộ tệp source bị thay đổi bởi lượt agent này về điểm kiểm soát (checkpoint) ngay trước đó.
          </p>
        </Dialog>
      )}
    </div>
  );
};
