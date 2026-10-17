import React from "react";
import { Icon } from "../../components/Icon";
import { Badge, Button, Callout, Dialog, Input, Spinner } from "@studio/design-system";
import { studioApi, type WebStatus } from "../../api/client";

export const WebAccessSection: React.FC = () => {
  const [status, setStatus] = React.useState<WebStatus | null>(null);
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);
  const [successMsg, setSuccessMsg] = React.useState<string | null>(null);

  // Port configuration
  const [portInput, setPortInput] = React.useState<string>("3000");
  const [portBusy, setPortBusy] = React.useState(false);

  // Password modal
  const [passwordModalOpen, setPasswordModalOpen] = React.useState(false);
  const [newPassword, setNewPassword] = React.useState("");
  const [confirmPassword, setConfirmPassword] = React.useState("");
  const [passwordError, setPasswordError] = React.useState<string | null>(null);
  const [passwordBusy, setPasswordBusy] = React.useState(false);

  // LAN toggle busy
  const [toggleBusy, setToggleBusy] = React.useState(false);

  // Copy feedback
  const [copiedKey, setCopiedKey] = React.useState<string | null>(null);

  const loadStatus = React.useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const res = await studioApi.webStatus();
      setStatus(res);
      setPortInput(String(res.port));
    } catch (err: any) {
      setError(err?.message || "Không thể tải trạng thái Chế độ Web.");
    } finally {
      setLoading(false);
    }
  }, []);

  React.useEffect(() => {
    loadStatus();
  }, [loadStatus]);

  const handleToggleEnabled = async () => {
    if (!status) return;

    // If enabling but no password is set, prompt to create password first
    if (!status.enabled && !status.has_password) {
      setPasswordError("Vui lòng đặt mật khẩu chủ sở hữu trước khi bật Chế độ Web.");
      setPasswordModalOpen(true);
      return;
    }

    setToggleBusy(true);
    setError(null);
    setSuccessMsg(null);
    try {
      const res = await studioApi.webConfigure({ enabled: !status.enabled });
      setStatus(res);
      setSuccessMsg(
        res.enabled
          ? "Đã bật Chế độ Web Cá nhân thành công."
          : "Đã tắt Chế độ Web Cá nhân."
      );
    } catch (err: any) {
      setError(err?.message || "Không thể thay đổi trạng thái Chế độ Web.");
    } finally {
      setToggleBusy(false);
    }
  };

  const handleToggleLan = async () => {
    if (!status) return;

    setToggleBusy(true);
    setError(null);
    setSuccessMsg(null);
    try {
      const res = await studioApi.webConfigure({ lan_allowed: !status.lan_allowed });
      setStatus(res);
      setSuccessMsg(
        res.lan_allowed
          ? "Đã cho phép truy cập từ mạng LAN."
          : "Đã giới hạn truy cập chỉ trên máy tính này (127.0.0.1)."
      );
    } catch (err: any) {
      setError(err?.message || "Không thể thay đổi quyền truy cập LAN.");
    } finally {
      setToggleBusy(false);
    }
  };

  const handleSavePort = async () => {
    const p = parseInt(portInput.trim(), 10);
    if (isNaN(p) || p < 1024 || p > 65535) {
      setError("Cổng (Port) không hợp lệ. Vui lòng nhập số trong khoảng từ 1024 đến 65535.");
      return;
    }

    setPortBusy(true);
    setError(null);
    setSuccessMsg(null);
    try {
      const res = await studioApi.webConfigure({ port: p });
      setStatus(res);
      setSuccessMsg(`Đã cập nhật cổng dịch vụ thành ${p}.`);
    } catch (err: any) {
      setError(err?.message || "Không thể cập nhật cổng.");
    } finally {
      setPortBusy(false);
    }
  };

  const handleSavePassword = async () => {
    if (!newPassword || newPassword.length < 6) {
      setPasswordError("Mật khẩu phải có độ dài tối thiểu 6 ký tự.");
      return;
    }
    if (newPassword !== confirmPassword) {
      setPasswordError("Mật khẩu xác nhận không khớp.");
      return;
    }

    setPasswordBusy(true);
    setPasswordError(null);
    try {
      // Save password and enable if user was in enabling flow
      const res = await studioApi.webConfigure({
        password: newPassword,
        enabled: status ? (status.enabled || !status.has_password) : true,
      });
      setStatus(res);
      setPasswordModalOpen(false);
      setNewPassword("");
      setConfirmPassword("");
      setSuccessMsg("Đã lưu mật khẩu chủ sở hữu (mã hóa an toàn bằng Argon2id)!");
    } catch (err: any) {
      setPasswordError(err?.message || "Không thể lưu mật khẩu.");
    } finally {
      setPasswordBusy(false);
    }
  };

  const copyToClipboard = (key: string, text: string) => {
    if (navigator?.clipboard?.writeText) {
      navigator.clipboard.writeText(text);
      setCopiedKey(key);
      setTimeout(() => setCopiedKey(null), 2000);
    }
  };

  if (loading) {
    return (
      <div style={{ display: "flex", alignItems: "center", justifyContent: "center", padding: 48, gap: 12 }}>
        <Spinner size={20} />
        <span style={{ fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-secondary)" }}>
          Đang tải trạng thái Chế độ Web...
        </span>
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-4)" }}>
      {/* Intro Callout */}
      <Callout variant="neutral" title="Chế độ Web Cá nhân (Personal Web Mode)">
        Sử dụng Local Content Studio từ trình duyệt web trên máy tính này hoặc điện thoại/laptop trong cùng mạng Wi-Fi/LAN.
        Mọi dữ liệu, tệp tin và lệnh thực thi AI đều diễn ra cục bộ trên máy tính của bạn — không qua cloud hay tài khoản trung gian.
      </Callout>

      {error && (
        <Callout variant="danger" title="Lỗi thao tác">
          {error}
        </Callout>
      )}

      {successMsg && (
        <Callout variant="success" title="Thành công">
          {successMsg}
        </Callout>
      )}

      {/* CARD 1: Master Enable / Status */}
      <div
        style={{
          padding: "var(--cs-space-4)",
          backgroundColor: "var(--cs-bg-panel)",
          border: "1px solid var(--cs-border-subtle)",
          borderRadius: "var(--cs-radius-control)",
          display: "flex",
          flexDirection: "column",
          gap: 16,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <Icon name="globe" size={20} color="var(--cs-accent)" />
            <div>
              <div style={{ fontWeight: 700, fontSize: "var(--cs-text-md)" }}>
                Máy chủ Web Cá nhân
              </div>
              <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
                {status?.enabled
                  ? `Đang chạy trên cổng ${status.port} • ${status.active_sessions} phiên hoạt động`
                  : "Dịch vụ đang tắt (chỉ hoạt động qua ứng dụng máy tính)"}
              </div>
            </div>
          </div>

          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <Badge variant={status?.enabled ? "success" : "neutral"}>
              {status?.enabled ? "Đang bật" : "Đã tắt"}
            </Badge>
            <Button
              variant={status?.enabled ? "destructive" : "primary"}
              onClick={handleToggleEnabled}
              busy={toggleBusy}
              busyText="Đang chuyển..."
            >
              {status?.enabled ? "Tắt dịch vụ Web" : "Bật Chế độ Web"}
            </Button>
          </div>
        </div>
      </div>

      {/* CARD 2: Security & Password */}
      <div
        style={{
          padding: "var(--cs-space-4)",
          backgroundColor: "var(--cs-bg-panel)",
          border: "1px solid var(--cs-border-subtle)",
          borderRadius: "var(--cs-radius-control)",
          display: "flex",
          flexDirection: "column",
          gap: 12,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <Icon name="shield-check" size={18} color="var(--cs-accent)" />
            <div>
              <div style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)" }}>
                Mật khẩu Chủ sở hữu (Owner Password)
              </div>
              <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
                {status?.has_password
                  ? "Đã cấu hình • Băm Argon2id lưu trong cơ sở dữ liệu cục bộ"
                  : "Chưa cấu hình mật khẩu (Bắt buộc phải đặt trước khi bật web)"}
              </div>
            </div>
          </div>

          <Button
            size="sm"
            variant="secondary"
            onClick={() => {
              setPasswordError(null);
              setPasswordModalOpen(true);
            }}
            leftIcon={<Icon name="lock" size={13} />}
          >
            {status?.has_password ? "Đổi mật khẩu" : "Thiết lập mật khẩu"}
          </Button>
        </div>

        <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)", lineHeight: 1.5 }}>
          🛡️ <strong>Nguyên tắc an ninh:</strong> Mọi kết nối web đều cần đăng nhập để lấy phiên HttpOnly SameSite=Strict.
          Để bảo vệ máy tính của bạn trước việc vô tình kích hoạt các hành động nguy hiểm, các thao tác nhạy cảm
          (đăng video YouTube, cài công cụ, cấu hình khoá API, hoàn tác lượt) yêu cầu nhập lại mật khẩu (Sudo) trong vòng 10 phút.
        </div>
      </div>

      {/* CARD 3: Network, LAN & QR Code */}
      <div
        style={{
          padding: "var(--cs-space-4)",
          backgroundColor: "var(--cs-bg-panel)",
          border: "1px solid var(--cs-border-subtle)",
          borderRadius: "var(--cs-radius-control)",
          display: "flex",
          flexDirection: "column",
          gap: 16,
        }}
      >
        <div style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)" }}>
          Cấu hình Mạng & Kết nối LAN
        </div>

        {/* LAN Toggle */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "var(--cs-space-3)",
            backgroundColor: "var(--cs-bg-raised)",
            borderRadius: "var(--cs-radius-control)",
            border: "1px solid var(--cs-border-subtle)",
          }}
        >
          <div>
            <div style={{ fontWeight: 600, fontSize: "var(--cs-text-xs)" }}>
              Cho phép truy cập từ mạng LAN
            </div>
            <div style={{ fontSize: 11, color: "var(--cs-fg-muted)" }}>
              {status?.lan_allowed
                ? "Lắng nghe trên 0.0.0.0 — Các thiết bị khác trong cùng mạng Wi-Fi có thể kết nối"
                : "Chỉ lắng nghe trên 127.0.0.1 — Chỉ trình duyệt trên máy tính này mới mở được"}
            </div>
          </div>

          <Button
            size="sm"
            variant={status?.lan_allowed ? "secondary" : "primary"}
            onClick={handleToggleLan}
            busy={toggleBusy}
          >
            {status?.lan_allowed ? "Tắt kết nối LAN" : "Bật kết nối LAN"}
          </Button>
        </div>

        {status?.lan_allowed && (
          <Callout variant="warning" title="Cảnh báo an toàn mạng Wi-Fi/LAN">
            Khi bật tính năng này, bất kỳ ai kết nối chung mạng Wi-Fi của bạn đều có thể thấy trang đăng nhập của Studio.
            Hãy đảm bảo bạn đang sử dụng mạng Wi-Fi tin cậy (nhà riêng) và mật khẩu chủ sở hữu đủ dài và an toàn.
          </Callout>
        )}

        {/* Port Input */}
        <div style={{ display: "flex", alignItems: "flex-end", gap: 10, maxWidth: 320 }}>
          <div style={{ flex: 1 }}>
            <label style={{ display: "block", fontSize: "var(--cs-text-xs)", fontWeight: 600, marginBottom: 4 }}>
              Cổng dịch vụ (Port)
            </label>
            <Input
              type="number"
              value={portInput}
              onChange={(e) => setPortInput(e.target.value)}
              placeholder="3000"
            />
          </div>
          <Button
            size="sm"
            variant="secondary"
            onClick={handleSavePort}
            busy={portBusy}
            disabled={portInput === String(status?.port)}
          >
            Lưu cổng
          </Button>
        </div>

        {/* URLs & QR Code */}
        {status?.enabled && (
          <div
            style={{
              marginTop: 8,
              paddingTop: 16,
              borderTop: "1px solid var(--cs-border-subtle)",
              display: "grid",
              gridTemplateColumns: status.lan_allowed && status.qr_code_svg ? "1fr auto" : "1fr",
              gap: 24,
              alignItems: "center",
            }}
          >
            <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              <div>
                <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)", display: "block", marginBottom: 2 }}>
                  Địa chỉ trên máy này (Localhost):
                </span>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <code style={{ fontSize: "var(--cs-text-sm)", backgroundColor: "var(--cs-bg-raised)", padding: "4px 8px", borderRadius: 4 }}>
                    {status.local_url || `http://localhost:${status.port}`}
                  </code>
                  <Button
                    size="sm"
                    variant="quiet"
                    onClick={() => copyToClipboard("local", status.local_url || `http://localhost:${status.port}`)}
                    leftIcon={<Icon name={copiedKey === "local" ? "check" : "copy"} size={13} />}
                  >
                    {copiedKey === "local" ? "Đã chép" : "Sao chép"}
                  </Button>
                </div>
              </div>

              {status.lan_allowed && (
                <div>
                  <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)", display: "block", marginBottom: 2 }}>
                    Địa chỉ kết nối từ điện thoại / máy tính khác qua LAN:
                  </span>
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <code style={{ fontSize: "var(--cs-text-sm)", fontWeight: 700, color: "var(--cs-accent)", backgroundColor: "var(--cs-bg-raised)", padding: "4px 8px", borderRadius: 4 }}>
                      {status.lan_url || (status.lan_ip ? `http://${status.lan_ip}:${status.port}` : "Đang kiểm tra IP LAN...")}
                    </code>
                    {status.lan_url && (
                      <Button
                        size="sm"
                        variant="quiet"
                        onClick={() => copyToClipboard("lan", status.lan_url!)}
                        leftIcon={<Icon name={copiedKey === "lan" ? "check" : "copy"} size={13} />}
                      >
                        {copiedKey === "lan" ? "Đã chép" : "Sao chép"}
                      </Button>
                    )}
                  </div>
                </div>
              )}
            </div>

            {/* QR Code */}
            {status.lan_allowed && status.qr_code_svg && (
              <div
                style={{
                  display: "flex",
                  flexDirection: "column",
                  alignItems: "center",
                  gap: 8,
                  padding: 12,
                  backgroundColor: "var(--cs-bg-raised)",
                  border: "1px solid var(--cs-border-subtle)",
                  borderRadius: "var(--cs-radius-control)",
                }}
              >
                <div
                  style={{
                    width: 160,
                    height: 160,
                    backgroundColor: "#ffffff",
                    padding: 8,
                    borderRadius: 6,
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                  }}
                  dangerouslySetInnerHTML={{ __html: status.qr_code_svg }}
                />
                <span style={{ fontSize: 11, color: "var(--cs-fg-muted)", textAlign: "center" }}>
                  📱 Quét bằng camera điện thoại
                </span>
              </div>
            )}
          </div>
        )}
      </div>

      {/* PASSWORD SETUP / CHANGE MODAL */}
      <Dialog
        open={passwordModalOpen}
        onClose={() => setPasswordModalOpen(false)}
        title={status?.has_password ? "Đổi mật khẩu Chủ sở hữu" : "Đặt mật khẩu Chế độ Web"}
        actions={
          <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
            <Button variant="secondary" onClick={() => setPasswordModalOpen(false)}>
              Hủy
            </Button>
            <Button
              variant="primary"
              onClick={handleSavePassword}
              busy={passwordBusy}
              busyText="Đang lưu..."
            >
              Lưu mật khẩu
            </Button>
          </div>
        }
      >
        <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
          <p style={{ margin: 0, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", lineHeight: 1.5 }}>
            Mật khẩu này dùng để đăng nhập vào giao diện web Studio từ trình duyệt khác và xác thực quyền quản trị (Sudo) cho các lệnh nhạy cảm.
          </p>

          {passwordError && (
            <Callout variant="danger" title="Lỗi">
              {passwordError}
            </Callout>
          )}

          <div>
            <label style={{ display: "block", fontSize: "var(--cs-text-xs)", fontWeight: 600, marginBottom: 4 }}>
              Mật khẩu mới (tối thiểu 6 ký tự)
            </label>
            <Input
              type="password"
              value={newPassword}
              onChange={(e) => setNewPassword(e.target.value)}
              placeholder="••••••••"
              autoFocus
            />
          </div>

          <div>
            <label style={{ display: "block", fontSize: "var(--cs-text-xs)", fontWeight: 600, marginBottom: 4 }}>
              Xác nhận mật khẩu mới
            </label>
            <Input
              type="password"
              value={confirmPassword}
              onChange={(e) => setConfirmPassword(e.target.value)}
              placeholder="••••••••"
            />
          </div>
        </div>
      </Dialog>
    </div>
  );
};
