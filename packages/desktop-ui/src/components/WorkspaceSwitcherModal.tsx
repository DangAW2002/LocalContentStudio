import React from "react";
import { Icon } from "./Icon";
import { Button, Input, Callout } from "@studio/design-system";
import { cleanDisplayPath } from "../utils/path";
import type { WorkspaceSummary } from "../api/types";
import { studioApi } from "../api/client";
import { isTauri } from "../api/tauri-bridge";

export interface WorkspaceSwitcherModalProps {
  open: boolean;
  onClose: () => void;
  currentWorkspace?: WorkspaceSummary;
  recentWorkspaces: WorkspaceSummary[];
  onOpenWorkspace: (path: string) => Promise<void>;
  onCreateWorkspace: (path: string, name: string) => Promise<void>;
}

export const WorkspaceSwitcherModal: React.FC<WorkspaceSwitcherModalProps> = ({
  open,
  onClose,
  currentWorkspace,
  recentWorkspaces,
  onOpenWorkspace,
  onCreateWorkspace,
}) => {
  const [tab, setTab] = React.useState<"recent" | "create" | "open">("recent");
  const [openPath, setOpenPath] = React.useState("");
  const [createPath, setCreatePath] = React.useState("");
  const [createName, setCreateName] = React.useState("");
  const [error, setError] = React.useState<string | null>(null);
  const [notice, setNotice] = React.useState<string | null>(null);
  const [busy, setBusy] = React.useState(false);
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

  const folderName = (path: string) => {
    const parts = path.replace(/\\/g, "/").split("/").filter(Boolean);
    return parts[parts.length - 1] ?? "";
  };

  const handleOpen = async (path: string) => {
    try {
      setBusy(true);
      setError(null);
      setNotice(null);
      await onOpenWorkspace(path);
      onClose();
    } catch (err: any) {
      const message: string = err?.message || "";
      if (message.includes("Workspace not found")) {
        // Folder exists (or not) but has no studio.yaml: offer to initialise it.
        setCreatePath(path);
        if (!createName.trim()) setCreateName(folderName(path));
        setTab("create");
        setNotice("Thư mục này chưa phải workspace. Đặt tên rồi bấm \"Tạo và mở\" để khởi tạo workspace tại đây.");
      } else {
        setError(message || "Không thể mở workspace.");
      }
    } finally {
      setBusy(false);
    }
  };

  const handleCreate = async () => {
    if (!createPath.trim() || !createName.trim()) {
      setError("Vui lòng nhập đầy đủ đường dẫn thư mục và tên workspace.");
      return;
    }
    try {
      setBusy(true);
      setError(null);
      await onCreateWorkspace(createPath.trim(), createName.trim());
      onClose();
    } catch (err: any) {
      setError(err?.message || "Không thể tạo workspace mới.");
    } finally {
      setBusy(false);
    }
  };

  const handleBrowseOpen = async () => {
    if (!isTauri()) {
      setError("Hộp thoại chọn thư mục chỉ hỗ trợ trên ứng dụng desktop. Vui lòng nhập đường dẫn thư mục trực tiếp.");
      return;
    }
    try {
      const picked = await studioApi.pickFolder();
      if (picked) {
        setOpenPath(picked);
      }
    } catch (err: any) {
      setError(err?.message || "Không thể mở hộp thoại chọn thư mục.");
    }
  };

  const handleBrowseCreate = async () => {
    if (!isTauri()) {
      setError("Hộp thoại chọn thư mục chỉ hỗ trợ trên ứng dụng desktop. Vui lòng nhập đường dẫn thư mục trực tiếp.");
      return;
    }
    try {
      const picked = await studioApi.pickFolder();
      if (picked) {
        setCreatePath(picked);
        if (!createName.trim()) {
          const last = folderName(picked);
          if (last) {
            setCreateName(last);
          }
        }
      }
    } catch (err: any) {
      setError(err?.message || "Không thể mở hộp thoại chọn thư mục.");
    }
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Chuyển đổi Workspace"
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
          width: 580,
          maxWidth: "92vw",
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
            <span>Quản lý & Chuyển đổi Workspace</span>
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

        {/* Tab Buttons */}
        <div
          style={{
            display: "flex",
            borderBottom: "1px solid var(--cs-border-subtle)",
            padding: "0 var(--cs-space-4)",
            gap: "var(--cs-space-3)",
          }}
        >
          <button
            type="button"
            onClick={() => setTab("recent")}
            style={{
              padding: "10px 4px",
              background: "none",
              border: "none",
              borderBottom: tab === "recent" ? "2px solid var(--cs-accent)" : "2px solid transparent",
              color: tab === "recent" ? "var(--cs-accent)" : "var(--cs-fg-secondary)",
              fontWeight: tab === "recent" ? 600 : 500,
              cursor: "pointer",
              fontSize: "var(--cs-text-sm)",
            }}
          >
            Gần đây
          </button>
          <button
            type="button"
            onClick={() => setTab("open")}
            style={{
              padding: "10px 4px",
              background: "none",
              border: "none",
              borderBottom: tab === "open" ? "2px solid var(--cs-accent)" : "2px solid transparent",
              color: tab === "open" ? "var(--cs-accent)" : "var(--cs-fg-secondary)",
              fontWeight: tab === "open" ? 600 : 500,
              cursor: "pointer",
              fontSize: "var(--cs-text-sm)",
            }}
          >
            Mở thư mục...
          </button>
          <button
            type="button"
            onClick={() => setTab("create")}
            style={{
              padding: "10px 4px",
              background: "none",
              border: "none",
              borderBottom: tab === "create" ? "2px solid var(--cs-accent)" : "2px solid transparent",
              color: tab === "create" ? "var(--cs-accent)" : "var(--cs-fg-secondary)",
              fontWeight: tab === "create" ? 600 : 500,
              cursor: "pointer",
              fontSize: "var(--cs-text-sm)",
            }}
          >
            Tạo mới...
          </button>
        </div>

        {/* Content */}
        <div style={{ padding: "var(--cs-space-4)", display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
          {error && (
            <Callout variant="danger" title="Không thành công">
              {error}
            </Callout>
          )}
          {notice && !error && (
            <Callout variant="info" title="Chưa có workspace">
              {notice}
            </Callout>
          )}

          {tab === "recent" && (
            <div style={{ display: "flex", flexDirection: "column", gap: 6, maxHeight: 280, overflowY: "auto" }}>
              {recentWorkspaces.length === 0 ? (
                <div style={{ padding: "var(--cs-space-4)", textAlign: "center", color: "var(--cs-fg-muted)", fontSize: "var(--cs-text-sm)" }}>
                  Chưa có workspace nào trong danh sách gần đây.
                </div>
              ) : (
                recentWorkspaces.map((ws) => {
                  const isCurrent = currentWorkspace?.path === ws.path;
                  return (
                    <div
                      key={ws.path}
                      onClick={() => !isCurrent && handleOpen(ws.path)}
                      style={{
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "space-between",
                        padding: "8px 12px",
                        backgroundColor: isCurrent ? "var(--cs-selection)" : "var(--cs-bg-raised)",
                        border: "1px solid var(--cs-border-subtle)",
                        borderRadius: "var(--cs-radius-control)",
                        cursor: isCurrent ? "default" : "pointer",
                      }}
                    >
                      <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                        <span style={{ fontWeight: 600, color: "var(--cs-fg-primary)", fontSize: "var(--cs-text-sm)" }}>
                          {ws.name}
                        </span>
                        <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)", fontFamily: "var(--cs-font-mono)" }}>
                          {cleanDisplayPath(ws.path)}
                        </span>
                      </div>
                      {isCurrent ? (
                        <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-accent)", fontWeight: 600 }}>
                          Hiện tại
                        </span>
                      ) : (
                        <Button size="sm" variant="secondary" onClick={() => handleOpen(ws.path)}>
                          Mở
                        </Button>
                      )}
                    </div>
                  );
                })
              )}
            </div>
          )}

          {tab === "open" && (
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
              <div style={{ display: "flex", gap: "var(--cs-space-2)", alignItems: "flex-end" }}>
                <div style={{ flex: 1 }}>
                  <Input
                    label="Đường dẫn thư mục workspace"
                    placeholder="Ví dụ: D:/Studio/ca-nhan/kenh-review"
                    value={openPath}
                    onChange={(e) => setOpenPath(e.target.value)}
                  />
                </div>
                <Button
                  type="button"
                  variant="secondary"
                  onClick={handleBrowseOpen}
                  leftIcon={<Icon name="folder" size={14} />}
                  style={{ flexShrink: 0, height: "var(--cs-control-height)", display: isTauri() ? undefined : "none" }}
                >
                  Chọn thư mục...
                </Button>
              </div>
              <div style={{ display: "flex", justifyContent: "flex-end", gap: "var(--cs-space-2)" }}>
                <Button variant="secondary" onClick={onClose}>
                  Hủy
                </Button>
                <Button
                  variant="primary"
                  busy={busy}
                  disabled={!openPath.trim()}
                  onClick={() => handleOpen(openPath.trim())}
                >
                  Mở workspace
                </Button>
              </div>
            </div>
          )}

          {tab === "create" && (
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
              <Input
                label="Tên Workspace"
                placeholder="Ví dụ: Kênh Công Nghệ"
                value={createName}
                onChange={(e) => setCreateName(e.target.value)}
              />
              <div style={{ display: "flex", gap: "var(--cs-space-2)", alignItems: "flex-end" }}>
                <div style={{ flex: 1 }}>
                  <Input
                    label="Đường dẫn thư mục"
                    placeholder="Ví dụ: D:/Studio/cong-nghe"
                    value={createPath}
                    onChange={(e) => setCreatePath(e.target.value)}
                  />
                </div>
                <Button
                  type="button"
                  variant="secondary"
                  onClick={handleBrowseCreate}
                  leftIcon={<Icon name="folder" size={14} />}
                  style={{ flexShrink: 0, height: "var(--cs-control-height)", display: isTauri() ? undefined : "none" }}
                >
                  Chọn thư mục...
                </Button>
              </div>
              <div style={{ display: "flex", justifyContent: "flex-end", gap: "var(--cs-space-2)" }}>
                <Button variant="secondary" onClick={onClose}>
                  Hủy
                </Button>
                <Button
                  variant="primary"
                  busy={busy}
                  disabled={!createName.trim() || !createPath.trim()}
                  onClick={handleCreate}
                >
                  Tạo và mở
                </Button>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
