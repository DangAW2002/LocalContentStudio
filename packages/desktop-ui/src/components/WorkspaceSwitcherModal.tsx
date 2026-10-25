import React from "react";
import { Icon } from "./Icon";
import { Button, Input, Callout } from "@studio/design-system";
import { cleanDisplayPath } from "../utils/path";
import type { WorkspaceSummary, WorkspaceItem } from "../api/types";
import { studioApi } from "../api/client";

export interface WorkspaceSwitcherModalProps {
  open: boolean;
  onClose: () => void;
  currentWorkspace?: WorkspaceSummary;
  recentWorkspaces: WorkspaceSummary[];
  onOpenWorkspace: (path: string) => Promise<void>;
  onCreateWorkspace: (path: string, name: string) => Promise<void>;
}

/** Folder-safe slug of a workspace name ("Kênh Công Nghệ" → "kenh-cong-nghe"). */
export const slugifyName = (name: string) =>
  name
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .replace(/đ/gi, "d")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");

export const WorkspaceSwitcherModal: React.FC<WorkspaceSwitcherModalProps> = ({
  open,
  onClose,
  currentWorkspace,
  recentWorkspaces,
  onOpenWorkspace,
  onCreateWorkspace,
}) => {
  const [rootPath, setRootPath] = React.useState<string>("");
  const [workspaces, setWorkspaces] = React.useState<WorkspaceItem[]>([]);
  const [loading, setLoading] = React.useState(true);
  const [createName, setCreateName] = React.useState("");
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const [notice, setNotice] = React.useState<string | null>(null);

  // Advanced: open external folder or change root storage
  const [showOtherFolder, setShowOtherFolder] = React.useState(false);
  const [otherPath, setOtherPath] = React.useState("");
  const [editingRoot, setEditingRoot] = React.useState(false);
  const [customRootInput, setCustomRootInput] = React.useState("");

  const refreshWorkspaces = React.useCallback(async () => {
    setLoading(true);
    try {
      const res = await studioApi.getWorkspacesRoot();
      setRootPath(res.root);
      setWorkspaces(res.workspaces);
    } catch {
      // Fallback to recentWorkspaces if workspaces_root_get fails
      const fallbackList: WorkspaceItem[] = recentWorkspaces.map((ws) => ({
        slug: slugifyName(ws.name) || "workspace",
        name: ws.name,
        path: ws.path,
        isWorkspace: true,
        isCurrent: Boolean(ws.isCurrent || (currentWorkspace && currentWorkspace.path === ws.path)),
        lastOpenedAt: ws.lastOpenedAt ?? null,
      }));
      setWorkspaces(fallbackList);
    } finally {
      setLoading(false);
    }
  }, [recentWorkspaces, currentWorkspace]);

  React.useEffect(() => {
    if (open) {
      void refreshWorkspaces();
      setCreateName("");
      setError(null);
      setNotice(null);
      setShowOtherFolder(false);
      setEditingRoot(false);
    }
  }, [open, refreshWorkspaces]);

  const prevActiveElementRef = React.useRef<HTMLElement | null>(null);

  React.useEffect(() => {
    if (open) {
      if (document.activeElement instanceof HTMLElement) {
        prevActiveElementRef.current = document.activeElement;
      }
      const handleKeyDown = (e: KeyboardEvent) => {
        if (e.key === "Escape") {
          onClose();
        }
      };
      window.addEventListener("keydown", handleKeyDown);
      return () => window.removeEventListener("keydown", handleKeyDown);
    } else if (prevActiveElementRef.current) {
      prevActiveElementRef.current.focus?.();
      prevActiveElementRef.current = null;
    }
  }, [open, onClose]);

  if (!open) return null;

  const handleOpen = async (path: string) => {
    try {
      setBusy(true);
      setError(null);
      setNotice(null);
      await onOpenWorkspace(path);
      onClose();
    } catch (err: any) {
      setError(err?.message || "Không thể mở workspace.");
    } finally {
      setBusy(false);
    }
  };

  const handleCreate = async (name: string, customPath?: string) => {
    const trimmed = name.trim();
    if (!trimmed) {
      setError("Vui lòng nhập tên Workspace.");
      return;
    }
    try {
      setBusy(true);
      setError(null);
      // Empty path triggers the backend to create directly under the sandboxed workspaces root
      await onCreateWorkspace(customPath || "", trimmed);
      onClose();
    } catch (err: any) {
      setError(err?.message || "Không thể tạo workspace mới.");
    } finally {
      setBusy(false);
    }
  };

  const handleSaveCustomRoot = async () => {
    if (!customRootInput.trim()) return;
    try {
      setBusy(true);
      setError(null);
      const res = await studioApi.setWorkspacesRoot(customRootInput.trim());
      setRootPath(res.root);
      setWorkspaces(res.workspaces);
      setEditingRoot(false);
    } catch (err: any) {
      setError(err?.message || "Không thể thay đổi thư mục lưu trữ.");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Quản lý Workspace"
      onClick={onClose}
      style={{
        position: "fixed",
        inset: 0,
        backgroundColor: "var(--cs-overlay)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 50,
      }}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          width: 600,
          maxWidth: "94vw",
          backgroundColor: "var(--cs-bg-panel)",
          border: "1px solid var(--cs-border-strong)",
          boxShadow: "var(--cs-shadow-overlay)",
          borderRadius: "var(--cs-radius-none)",
          overflow: "hidden",
          display: "flex",
          flexDirection: "column",
        }}
      >
        {/* Header */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "var(--cs-space-3) var(--cs-space-4)",
            borderBottom: "1px solid var(--cs-border-subtle)",
            backgroundColor: "var(--cs-bg-raised)",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 8, fontWeight: 600, fontSize: "var(--cs-text-md)" }}>
            <Icon name="folder" size={18} color="var(--cs-accent)" />
            <span>Không gian làm việc (Workspaces)</span>
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="Đóng (Esc)"
            style={{
              background: "none",
              border: "none",
              padding: 4,
              cursor: "pointer",
              color: "var(--cs-fg-secondary)",
              display: "inline-flex",
            }}
          >
            <Icon name="x" size={16} />
          </button>
        </div>

        {/* Quick Create Section */}
        <div
          style={{
            padding: "var(--cs-space-3) var(--cs-space-4)",
            backgroundColor: "var(--cs-bg-raised)",
            borderBottom: "1px solid var(--cs-border-subtle)",
            display: "flex",
            flexDirection: "column",
            gap: "var(--cs-space-2)",
          }}
        >
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600, color: "var(--cs-fg-secondary)", textTransform: "uppercase", letterSpacing: "0.05em" }}>
              Tạo Workspace Mới
            </span>
            <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
              Lưu tự động vào kho Studio
            </span>
          </div>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              if (createName.trim() && !busy) {
                void handleCreate(createName);
              }
            }}
            style={{ display: "flex", gap: "var(--cs-space-2)", alignItems: "flex-end" }}
          >
            <div style={{ flex: 1 }}>
              <Input
                placeholder="Nhập tên workspace (ví dụ: Kênh Công Nghệ, Studio Vlog)..."
                value={createName}
                autoFocus
                onChange={(e) => setCreateName(e.target.value)}
              />
            </div>
            <Button
              type="submit"
              variant="primary"
              busy={busy}
              disabled={!createName.trim()}
              leftIcon={<Icon name="plus" size={14} />}
              style={{ flexShrink: 0, height: "var(--cs-control-height)" }}
            >
              Tạo Workspace
            </Button>
          </form>
        </div>

        {/* Content & Workspaces List */}
        <div style={{ padding: "var(--cs-space-4)", display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
          {error && (
            <Callout variant="danger" title="Không thành công">
              {error}
            </Callout>
          )}
          {notice && !error && (
            <Callout variant="info" title="Thông báo">
              {notice}
            </Callout>
          )}

          {/* Workspaces List Header */}
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600, color: "var(--cs-fg-secondary)", textTransform: "uppercase", letterSpacing: "0.05em" }}>
              Danh sách Workspace ({workspaces.length})
            </span>
            {loading && (
              <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
                Đang quét kho lưu trữ...
              </span>
            )}
          </div>

          {/* Workspaces List Container */}
          <div style={{ display: "flex", flexDirection: "column", gap: 8, maxHeight: 300, overflowY: "auto", paddingRight: 2 }}>
            {workspaces.length === 0 && !loading ? (
              <div
                style={{
                  padding: "var(--cs-space-5) var(--cs-space-4)",
                  textAlign: "center",
                  backgroundColor: "var(--cs-bg-raised)",
                  border: "1px dashed var(--cs-border-subtle)",
                  borderRadius: "var(--cs-radius-control)",
                  display: "flex",
                  flexDirection: "column",
                  alignItems: "center",
                  gap: "var(--cs-space-2)",
                }}
              >
                <Icon name="folder" size={32} color="var(--cs-fg-muted)" />
                <div style={{ fontWeight: 600, color: "var(--cs-fg-primary)", fontSize: "var(--cs-text-sm)" }}>
                  Chưa có workspace nào trong kho Studio.
                </div>
                <div style={{ color: "var(--cs-fg-muted)", fontSize: "var(--cs-text-xs)", maxWidth: 320 }}>
                  Gõ tên vào ô phía trên hoặc bấm nút bên dưới để tạo nhanh workspace cá nhân đầu tiên.
                </div>
                <Button
                  size="sm"
                  variant="primary"
                  busy={busy}
                  onClick={() => void handleCreate("Studio Cá Nhân")}
                  leftIcon={<Icon name="sparkles" size={14} />}
                  style={{ marginTop: 6 }}
                >
                  Tạo nhanh "Studio Cá Nhân"
                </Button>
              </div>
            ) : (
              workspaces.map((ws) => {
                const isCurrent = ws.isCurrent || currentWorkspace?.path === ws.path;
                return (
                  <div
                    key={ws.path}
                    onClick={() => !isCurrent && void handleOpen(ws.path)}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "space-between",
                      padding: "10px 14px",
                      backgroundColor: isCurrent ? "var(--cs-selection)" : "var(--cs-bg-raised)",
                      border: "1px solid",
                      borderColor: isCurrent ? "var(--cs-accent)" : "var(--cs-border-subtle)",
                      borderRadius: "var(--cs-radius-control)",
                      cursor: isCurrent ? "default" : "pointer",
                      transition: "background-color 0.15s ease, border-color 0.15s ease",
                    }}
                  >
                    <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                      <div
                        style={{
                          width: 32,
                          height: 32,
                          borderRadius: "var(--cs-radius-control)",
                          backgroundColor: isCurrent ? "var(--cs-accent)" : "var(--cs-bg-panel)",
                          border: isCurrent ? "none" : "1px solid var(--cs-border-subtle)",
                          display: "flex",
                          alignItems: "center",
                          justifyContent: "center",
                          color: isCurrent ? "white" : "var(--cs-fg-secondary)",
                          flexShrink: 0,
                        }}
                      >
                        <Icon name={isCurrent ? "sparkles" : "folder"} size={16} />
                      </div>
                      <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                        <span style={{ fontWeight: 600, color: "var(--cs-fg-primary)", fontSize: "var(--cs-text-sm)" }}>
                          {ws.name}
                        </span>
                        <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)", fontFamily: "var(--cs-font-mono)" }}>
                          {ws.slug || cleanDisplayPath(ws.path)}
                        </span>
                      </div>
                    </div>
                    <div>
                      {isCurrent ? (
                        <span
                          style={{
                            fontSize: "var(--cs-text-xs)",
                            color: "var(--cs-accent)",
                            fontWeight: 600,
                            backgroundColor: "var(--cs-bg-panel)",
                            border: "1px solid var(--cs-accent)",
                            padding: "3px 8px",
                            borderRadius: 4,
                          }}
                        >
                          Đang mở
                        </span>
                      ) : (
                        <Button
                          size="sm"
                          variant="secondary"
                          onClick={(e) => {
                            e.stopPropagation();
                            void handleOpen(ws.path);
                          }}
                        >
                          Chuyển đến
                        </Button>
                      )}
                    </div>
                  </div>
                );
              })
            )}
          </div>

          {/* Optional: Open custom directory fallback */}
          {showOtherFolder && (
            <div
              style={{
                marginTop: "var(--cs-space-2)",
                padding: "var(--cs-space-3)",
                backgroundColor: "var(--cs-bg-raised)",
                border: "1px dashed var(--cs-border-subtle)",
                borderRadius: "var(--cs-radius-control)",
                display: "flex",
                flexDirection: "column",
                gap: "var(--cs-space-2)",
              }}
            >
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <span style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600, color: "var(--cs-fg-secondary)" }}>
                  Mở workspace từ đường dẫn riêng ngoài kho Studio
                </span>
                <button
                  type="button"
                  onClick={() => setShowOtherFolder(false)}
                  style={{ background: "none", border: "none", color: "var(--cs-fg-muted)", cursor: "pointer", fontSize: "var(--cs-text-xs)" }}
                >
                  Đóng
                </button>
              </div>
              <div style={{ display: "flex", gap: "var(--cs-space-2)", alignItems: "flex-end" }}>
                <div style={{ flex: 1 }}>
                  <Input
                    placeholder="Đường dẫn thư mục..."
                    value={otherPath}
                    onChange={(e) => setOtherPath(e.target.value)}
                  />
                </div>
                <Button
                  size="sm"
                  variant="secondary"
                  disabled={!otherPath.trim()}
                  busy={busy}
                  onClick={() => void handleOpen(otherPath.trim())}
                >
                  Mở
                </Button>
              </div>
            </div>
          )}
        </div>

        {/* Footer: Isolated Studio Storage Info */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "10px var(--cs-space-4)",
            borderTop: "1px solid var(--cs-border-subtle)",
            backgroundColor: "var(--cs-bg-raised)",
            fontSize: "var(--cs-text-xs)",
            color: "var(--cs-fg-muted)",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
            <Icon name="folder" size={12} color="var(--cs-fg-secondary)" />
            <span>Kho lưu trữ Studio:</span>
            {!editingRoot ? (
              <>
                <span style={{ fontFamily: "var(--cs-font-mono)", color: "var(--cs-fg-primary)", fontWeight: 500 }}>
                  {cleanDisplayPath(rootPath || "~/Studio")}
                </span>
                <button
                  type="button"
                  onClick={() => {
                    setCustomRootInput(rootPath);
                    setEditingRoot(true);
                  }}
                  style={{
                    background: "none",
                    border: "none",
                    color: "var(--cs-accent)",
                    cursor: "pointer",
                    textDecoration: "underline",
                    padding: 0,
                    fontSize: "var(--cs-text-xs)",
                  }}
                >
                  Đổi kho...
                </button>
              </>
            ) : (
              <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                <input
                  type="text"
                  value={customRootInput}
                  onChange={(e) => setCustomRootInput(e.target.value)}
                  placeholder="Đường dẫn kho Studio mới..."
                  style={{
                    padding: "2px 6px",
                    fontSize: "var(--cs-text-xs)",
                    backgroundColor: "var(--cs-bg-subtle)",
                    border: "1px solid var(--cs-border-subtle)",
                    borderRadius: 4,
                    color: "var(--cs-fg-primary)",
                  }}
                />
                <Button size="sm" variant="primary" onClick={() => void handleSaveCustomRoot()} busy={busy}>
                  Lưu
                </Button>
                <Button size="sm" variant="secondary" onClick={() => setEditingRoot(false)}>
                  Hủy
                </Button>
              </div>
            )}
          </div>

          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            {!showOtherFolder && (
              <button
                type="button"
                onClick={() => setShowOtherFolder(true)}
                style={{
                  background: "none",
                  border: "none",
                  color: "var(--cs-fg-secondary)",
                  cursor: "pointer",
                  fontSize: "var(--cs-text-xs)",
                }}
              >
                Mở thư mục khác...
              </button>
            )}
            <Button size="sm" variant="secondary" onClick={onClose}>
              Đóng
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
};
