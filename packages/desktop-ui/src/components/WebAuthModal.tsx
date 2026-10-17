import React from "react";
import { Dialog, Button, Input, Callout } from "@studio/design-system";
import { Icon } from "./Icon";
import {
  isTauri,
  checkAuthStatus,
  login,
  verifySudo,
  registerSudoHandler,
} from "../api/client";

export interface WebAuthModalProps {
  onLoginSuccess?: () => void;
}

export const WebAuthModal: React.FC<WebAuthModalProps> = ({ onLoginSuccess }) => {
  // If running in desktop Tauri, web auth modal is never needed
  if (isTauri()) {
    return null;
  }

  // Login Modal State
  const [showLogin, setShowLogin] = React.useState(false);
  const [loginPassword, setLoginPassword] = React.useState("");
  const [loginError, setLoginError] = React.useState<string | null>(null);
  const [loginBusy, setLoginBusy] = React.useState(false);

  // Sudo Modal State
  const [showSudo, setShowSudo] = React.useState(false);
  const [sudoCommand, setSudoCommand] = React.useState<string>("");
  const [sudoPassword, setSudoPassword] = React.useState("");
  const [sudoError, setSudoError] = React.useState<string | null>(null);
  const [sudoBusy, setSudoBusy] = React.useState(false);
  const sudoResolverRef = React.useRef<((ok: boolean) => void) | null>(null);

  // Check auth status on initial mount in browser
  React.useEffect(() => {
    checkAuthStatus().then((status) => {
      if (!status.authenticated) {
        setShowLogin(true);
      }
    });

    const handleAuthRequired = () => {
      setShowLogin(true);
    };

    window.addEventListener("studio-auth-required", handleAuthRequired);
    return () => {
      window.removeEventListener("studio-auth-required", handleAuthRequired);
    };
  }, []);

  // Register Sudo prompt handler with transport bridge
  React.useEffect(() => {
    registerSudoHandler(async (cmd: string) => {
      setSudoCommand(cmd);
      setSudoPassword("");
      setSudoError(null);
      setShowSudo(true);

      return new Promise<boolean>((resolve) => {
        sudoResolverRef.current = resolve;
      });
    });

    return () => {
      registerSudoHandler(null);
    };
  }, []);

  const handleLoginSubmit = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    if (!loginPassword) return;

    setLoginBusy(true);
    setLoginError(null);
    const res = await login(loginPassword);
    setLoginBusy(false);

    if (res.success) {
      setShowLogin(false);
      setLoginPassword("");
      if (typeof window !== "undefined") {
        window.dispatchEvent(new CustomEvent("studio:login-success"));
      }
      if (onLoginSuccess) {
        onLoginSuccess();
      }
    } else {
      setLoginError(res.message || "Mật khẩu không chính xác.");
    }
  };

  const handleSudoSubmit = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    if (!sudoPassword) return;

    setSudoBusy(true);
    setSudoError(null);
    const res = await verifySudo(sudoPassword);
    setSudoBusy(false);

    if (res.success) {
      setShowSudo(false);
      setSudoPassword("");
      if (sudoResolverRef.current) {
        sudoResolverRef.current(true);
        sudoResolverRef.current = null;
      }
    } else {
      setSudoError(res.message || "Mật khẩu xác thực không đúng.");
    }
  };

  const handleSudoCancel = () => {
    setShowSudo(false);
    setSudoPassword("");
    if (sudoResolverRef.current) {
      sudoResolverRef.current(false);
      sudoResolverRef.current = null;
    }
  };

  return (
    <>
      {/* 1. Full-screen Owner Login Modal */}
      {showLogin && (
        <div
          style={{
            position: "fixed",
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            backgroundColor: "rgba(0, 0, 0, 0.75)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            zIndex: 99999,
            padding: 20,
            backdropFilter: "blur(6px)",
          }}
        >
          <div
            style={{
              width: "100%",
              maxWidth: 420,
              backgroundColor: "var(--cs-bg-panel)",
              border: "1px solid var(--cs-border-subtle)",
              borderRadius: "var(--cs-radius-panel)",
              padding: "var(--cs-space-6)",
              display: "flex",
              flexDirection: "column",
              gap: 20,
              boxShadow: "0 12px 32px rgba(0, 0, 0, 0.4)",
            }}
          >
            <div style={{ textAlign: "center" }}>
              <div
                style={{
                  width: 48,
                  height: 48,
                  borderRadius: 12,
                  backgroundColor: "var(--cs-bg-raised)",
                  display: "inline-flex",
                  alignItems: "center",
                  justifyContent: "center",
                  marginBottom: 12,
                  color: "var(--cs-accent)",
                }}
              >
                <Icon name="lock" size={24} />
              </div>
              <h2 style={{ margin: "0 0 6px 0", fontSize: "var(--cs-text-lg)", fontWeight: 700 }}>
                Chế độ Web Cá nhân
              </h2>
              <p style={{ margin: 0, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", lineHeight: 1.5 }}>
                Đăng nhập bằng mật khẩu chủ sở hữu đã thiết lập trên ứng dụng máy tính.
              </p>
            </div>

            {loginError && (
              <Callout variant="danger" title="Lỗi đăng nhập">
                {loginError}
              </Callout>
            )}

            <form onSubmit={handleLoginSubmit} style={{ display: "flex", flexDirection: "column", gap: 16 }}>
              <div>
                <label style={{ display: "block", fontSize: "var(--cs-text-xs)", fontWeight: 600, marginBottom: 6 }}>
                  Mật khẩu Chủ sở hữu
                </label>
                <Input
                  type="password"
                  value={loginPassword}
                  onChange={(e) => setLoginPassword(e.target.value)}
                  placeholder="••••••••"
                  autoFocus
                />
              </div>

              <Button
                type="submit"
                variant="primary"
                busy={loginBusy}
                busyText="Đang xác thực..."
                style={{ width: "100%", justifyContent: "center" }}
              >
                Đăng nhập
              </Button>
            </form>
          </div>
        </div>
      )}

      {/* 2. Sudo Authentication Modal */}
      <Dialog
        open={showSudo}
        onClose={handleSudoCancel}
        title="Xác thực quyền Quản trị (Sudo Mode)"
        actions={
          <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
            <Button variant="secondary" onClick={handleSudoCancel}>
              Hủy bỏ
            </Button>
            <Button
              variant="primary"
              onClick={() => handleSudoSubmit()}
              busy={sudoBusy}
              busyText="Đang xác thực..."
            >
              Tiếp tục thao tác
            </Button>
          </div>
        }
      >
        <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
          <p style={{ margin: 0, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", lineHeight: 1.5 }}>
            Thao tác <code>{sudoCommand}</code> có tác động quan trọng (xuất bản, sửa đổi công cụ hoặc khoá bí mật).
            Vui lòng nhập lại mật khẩu chủ sở hữu để tiếp tục (phiên sudo sẽ có hiệu lực trong 10 phút).
          </p>

          {sudoError && (
            <Callout variant="danger" title="Lỗi">
              {sudoError}
            </Callout>
          )}

          <form onSubmit={handleSudoSubmit}>
            <div>
              <label style={{ display: "block", fontSize: "var(--cs-text-xs)", fontWeight: 600, marginBottom: 4 }}>
                Mật khẩu Chủ sở hữu
              </label>
              <Input
                type="password"
                value={sudoPassword}
                onChange={(e) => setSudoPassword(e.target.value)}
                placeholder="••••••••"
                autoFocus
              />
            </div>
          </form>
        </div>
      </Dialog>
    </>
  );
};
