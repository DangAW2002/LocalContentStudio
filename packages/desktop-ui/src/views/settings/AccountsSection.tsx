import React from "react";
import { Icon } from "../../components/Icon";
import { Badge, Button, Callout, EmptyState, Input, Spinner } from "@studio/design-system";
import { studioApi, parseYouTubeError } from "../../api/client";
import type {
  YouTubeChannelAccount,
  YoutubeClientStatus,
  YouTubeErrorDetail,
} from "../../api/types";
import { YouTubeGuideModal } from "../../components/YouTubeGuideModal";

export const AccountsSection: React.FC = () => {
  const [ytClientStatus, setYtClientStatus] = React.useState<YoutubeClientStatus | null>(null);
  const [ytChannels, setYtChannels] = React.useState<YouTubeChannelAccount[]>([]);
  const [ytImportMode, setYtImportMode] = React.useState<"file" | "manual">("file");
  const [ytClientIdInput, setYtClientIdInput] = React.useState("");
  const [ytClientSecretInput, setYtClientSecretInput] = React.useState("");
  const [ytImportBusy, setYtImportBusy] = React.useState(false);
  const [ytImportError, setYtImportError] = React.useState<string | null>(null);
  const [ytImportSuccess, setYtImportSuccess] = React.useState<string | null>(null);
  const [ytConnecting, setYtConnecting] = React.useState(false);
  const [ytCountdown, setYtCountdown] = React.useState(300);
  const [ytConnectError, setYtConnectError] = React.useState<YouTubeErrorDetail | null>(null);
  const [ytConnectSuccess, setYtConnectSuccess] = React.useState<string | null>(null);
  const [ytDisconnectingId, setYtDisconnectingId] = React.useState<string | null>(null);
  const [guideModalOpen, setGuideModalOpen] = React.useState(false);
  const ytFileInputRef = React.useRef<HTMLInputElement | null>(null);

  const loadAccounts = React.useCallback(async () => {
    try {
      const [status, channels] = await Promise.all([
        studioApi.youtubeClientStatus().catch(() => null),
        studioApi.youtubeChannelsList().catch(() => []),
      ]);
      setYtClientStatus(status);
      setYtChannels(channels);
    } catch {
      // ignore
    }
  }, []);

  React.useEffect(() => {
    loadAccounts();
  }, [loadAccounts]);

  React.useEffect(() => {
    let timer: any = null;
    if (ytConnecting && ytCountdown > 0) {
      timer = setInterval(() => {
        setYtCountdown((prev) => Math.max(0, prev - 1));
      }, 1000);
    }
    return () => {
      if (timer) clearInterval(timer);
    };
  }, [ytConnecting, ytCountdown]);

  const handleImportJsonFile = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    setYtImportBusy(true);
    setYtImportError(null);
    setYtImportSuccess(null);
    const reader = new FileReader();
    reader.onload = async (evt) => {
      const content = evt.target?.result as string;
      if (!content) {
        setYtImportError("Tệp tải lên rỗng.");
        setYtImportBusy(false);
        return;
      }
      try {
        const res = await studioApi.youtubeClientImport({ json: content });
        setYtImportSuccess(`Đã nạp OAuth client thành công! (…${res.clientIdLast6})`);
        const status = await studioApi.youtubeClientStatus();
        setYtClientStatus(status);
      } catch (err: any) {
        const parsed = parseYouTubeError(err);
        setYtImportError(parsed ? parsed.messageVi : (err?.message || "Không thể nạp tệp client secret."));
      } finally {
        setYtImportBusy(false);
        if (ytFileInputRef.current) ytFileInputRef.current.value = "";
      }
    };
    reader.onerror = () => {
      setYtImportError("Không đọc được tệp JSON.");
      setYtImportBusy(false);
    };
    reader.readAsText(file);
  };

  const handleImportManual = async () => {
    if (!ytClientIdInput.trim() || !ytClientSecretInput.trim()) {
      setYtImportError("Vui lòng nhập cả Client ID và Client Secret.");
      return;
    }
    setYtImportBusy(true);
    setYtImportError(null);
    setYtImportSuccess(null);
    try {
      const res = await studioApi.youtubeClientImport({
        clientId: ytClientIdInput.trim(),
        clientSecret: ytClientSecretInput.trim(),
      });
      setYtImportSuccess(`Đã nạp OAuth client thành công! (…${res.clientIdLast6})`);
      setYtClientIdInput("");
      setYtClientSecretInput("");
      const status = await studioApi.youtubeClientStatus();
      setYtClientStatus(status);
    } catch (err: any) {
      const parsed = parseYouTubeError(err);
      setYtImportError(parsed ? parsed.messageVi : (err?.message || "Không thể lưu Client ID/Secret."));
    } finally {
      setYtImportBusy(false);
    }
  };

  const handleConnectYouTube = async () => {
    setYtConnecting(true);
    setYtCountdown(300);
    setYtConnectError(null);
    setYtConnectSuccess(null);
    try {
      const account = await studioApi.youtubeConnect();
      setYtConnectSuccess(`Kết nối thành công kênh "${account.title}" (${account.handle ? "@" + account.handle : account.id})!`);
      const updatedList = await studioApi.youtubeChannelsList();
      setYtChannels(updatedList);
    } catch (err: any) {
      const parsed = parseYouTubeError(err);
      if (parsed) {
        setYtConnectError(parsed);
      } else {
        setYtConnectError({
          code: "unknown",
          messageVi: err?.message || "Không thể kết nối YouTube",
          fixVi: "Vui lòng kiểm tra kết nối mạng và thử lại.",
        });
      }
    } finally {
      setYtConnecting(false);
    }
  };

  const handleDisconnectChannel = async (channelId: string) => {
    setYtDisconnectingId(channelId);
    try {
      await studioApi.youtubeChannelDisconnect(channelId);
      const updatedList = await studioApi.youtubeChannelsList();
      setYtChannels(updatedList);
    } catch (err: any) {
      alert(err?.message || "Không thể ngắt kết nối kênh.");
    } finally {
      setYtDisconnectingId(null);
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-4)" }}>
      {/* Header */}
      <div style={{ display: "flex", alignItems: "flex-start", justifyContent: "space-between", gap: 16 }}>
        <div>
          <h2 style={{ margin: "0 0 4px 0", fontSize: "var(--cs-text-md)", fontWeight: 700 }}>
            Tài khoản Đăng bài (YouTube)
          </h2>
          <p style={{ margin: 0, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", lineHeight: 1.5 }}>
            Quản lý OAuth client và các kênh xuất bản nội dung YouTube. Khóa xác thực được lưu trữ mã hoá an toàn trong Windows Credential Manager trên máy bạn.
          </p>
        </div>
        <Button
          size="sm"
          variant="secondary"
          onClick={() => setGuideModalOpen(true)}
          leftIcon={<Icon name="help" size={13} />}
        >
          Xem hướng dẫn thiết lập
        </Button>
      </div>

      {/* SECTION 1: Nạp OAuth client */}
      <div
        style={{
          padding: "var(--cs-space-4)",
          backgroundColor: "var(--cs-bg-panel)",
          border: "1px solid var(--cs-border-subtle)",
          borderRadius: "var(--cs-radius-control)",
          display: "flex",
          flexDirection: "column",
          gap: "var(--cs-space-3)",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <span style={{ fontWeight: 700, fontSize: "var(--cs-text-sm)" }}>
              1. Nạp Google OAuth Client
            </span>
            <Badge variant={ytClientStatus?.configured ? "success" : "warning"}>
              {ytClientStatus?.configured
                ? `Đã nạp (…${ytClientStatus.clientIdLast6 || "******"})`
                : "Chưa nạp"}
            </Badge>
          </div>
        </div>

        <p style={{ margin: 0, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", lineHeight: 1.5 }}>
          Studio sử dụng OAuth Client của chính bạn từ Google Cloud Console để tải video lên YouTube. Không bao giờ gửi mã khóa lên máy chủ bên ngoài.
        </p>

        {/* Import method toggle */}
        <div style={{ display: "flex", gap: "var(--cs-space-4)", fontSize: "var(--cs-text-xs)", marginTop: 4 }}>
          <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer" }}>
            <input
              type="radio"
              name="ytImportMode"
              value="file"
              checked={ytImportMode === "file"}
              onChange={() => setYtImportMode("file")}
            />
            Chọn tệp client_secret_*.json (Khuyên dùng)
          </label>
          <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer" }}>
            <input
              type="radio"
              name="ytImportMode"
              value="manual"
              checked={ytImportMode === "manual"}
              onChange={() => setYtImportMode("manual")}
            />
            Nhập Client ID & Secret thủ công
          </label>
        </div>

        {ytImportMode === "file" ? (
          <div style={{ display: "flex", flexDirection: "column", gap: 8, marginTop: 4 }}>
            <input
              ref={ytFileInputRef}
              type="file"
              accept=".json,application/json"
              onChange={handleImportJsonFile}
              style={{ display: "none" }}
            />
            <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
              <Button
                size="sm"
                variant="primary"
                onClick={() => ytFileInputRef.current?.click()}
                busy={ytImportBusy}
                busyText="Đang nạp tệp..."
                leftIcon={<Icon name="upload" size={13} />}
              >
                Chọn tệp client_secret_*.json
              </Button>
              <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
                Tệp JSON tải về từ mục Credentials của Google Cloud Console (loại Desktop app).
              </span>
            </div>
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 10, marginTop: 4 }}>
            <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
              <label style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600 }}>Client ID:</label>
              <Input
                value={ytClientIdInput}
                onChange={(e: any) => setYtClientIdInput(e.target.value)}
                placeholder="...apps.googleusercontent.com"
              />
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
              <label style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600 }}>Client Secret:</label>
              <Input
                type="password"
                value={ytClientSecretInput}
                onChange={(e: any) => setYtClientSecretInput(e.target.value)}
                placeholder="GOCSPX-..."
              />
            </div>
            <div style={{ display: "flex", justifyContent: "flex-end" }}>
              <Button
                size="sm"
                variant="primary"
                onClick={handleImportManual}
                busy={ytImportBusy}
                busyText="Đang lưu..."
              >
                Lưu OAuth Client
              </Button>
            </div>
          </div>
        )}

        {ytImportError && (
          <Callout variant="danger" style={{ marginTop: 6 }}>
            {ytImportError}
          </Callout>
        )}

        {ytImportSuccess && (
          <Callout variant="success" style={{ marginTop: 6 }}>
            {ytImportSuccess}
          </Callout>
        )}
      </div>

      {/* SECTION 2: Danh sách kênh & Kết nối */}
      <div
        style={{
          padding: "var(--cs-space-4)",
          backgroundColor: "var(--cs-bg-panel)",
          border: "1px solid var(--cs-border-subtle)",
          borderRadius: "var(--cs-radius-control)",
          display: "flex",
          flexDirection: "column",
          gap: "var(--cs-space-3)",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <span style={{ fontWeight: 700, fontSize: "var(--cs-text-sm)" }}>
              2. Kênh YouTube đã kết nối ({ytChannels.length})
            </span>
          </div>

          {!ytConnecting && (
            <Button
              size="sm"
              variant="primary"
              onClick={handleConnectYouTube}
              disabled={!ytClientStatus?.configured}
              leftIcon={<Icon name="link" size={13} />}
            >
              Kết nối YouTube
            </Button>
          )}
        </div>

        {!ytClientStatus?.configured && (
          <Callout variant="neutral">
            Vui lòng nạp Google OAuth Client ở bước 1 để kích hoạt chức năng kết nối YouTube.
          </Callout>
        )}

        {/* Visible 5-minute countdown during connect */}
        {ytConnecting && (
          <div
            style={{
              padding: "var(--cs-space-3) var(--cs-space-4)",
              backgroundColor: "var(--cs-bg-raised)",
              border: "1px solid var(--cs-border-subtle)",
              borderRadius: "var(--cs-radius-control)",
              display: "flex",
              flexDirection: "column",
              gap: 8,
            }}
          >
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <Spinner size={16} />
                <span style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)" }}>
                  Đang chờ hoàn tất đăng nhập trong trình duyệt...
                </span>
              </div>
              <span
                style={{
                  fontFamily: "var(--cs-font-mono)",
                  fontSize: "var(--cs-text-sm)",
                  fontWeight: 700,
                  color: "var(--cs-accent)",
                }}
              >
                {Math.floor(ytCountdown / 60)}:{(ytCountdown % 60).toString().padStart(2, "0")}
              </span>
            </div>
            <Callout variant="warning">
              Đừng tắt Studio khi đang đăng nhập. Trình duyệt mặc định đang mở trang Google để bạn cấp quyền truy cập.
            </Callout>
          </div>
        )}

        {/* Structured error callout */}
        {ytConnectError && (
          <Callout variant="danger" title={ytConnectError.messageVi}>
            <p style={{ margin: "0 0 8px 0" }}>{ytConnectError.fixVi}</p>
            <div style={{ display: "flex", gap: 8, marginTop: 8 }}>
              <Button size="sm" variant="secondary" onClick={handleConnectYouTube}>
                Thử lại
              </Button>
              <Button size="sm" variant="secondary" onClick={() => setGuideModalOpen(true)}>
                Mở hướng dẫn
              </Button>
            </div>
          </Callout>
        )}

        {ytConnectSuccess && (
          <Callout variant="success">
            {ytConnectSuccess}
          </Callout>
        )}

        {/* Channels List */}
        {ytChannels.length === 0 ? (
          <EmptyState
            icon={<Icon name="video" size={28} />}
            title="Chưa có kênh YouTube nào được kết nối"
            description="Sau khi nạp OAuth client, bấm 'Kết nối YouTube' để liên kết kênh của bạn."
          />
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-2)" }}>
            {ytChannels.map((chan) => (
              <div
                key={chan.id}
                style={{
                  padding: "var(--cs-space-3) var(--cs-space-4)",
                  backgroundColor: "var(--cs-bg-raised)",
                  border: "1px solid var(--cs-border-subtle)",
                  borderRadius: "var(--cs-radius-control)",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                  {chan.avatarUrl ? (
                    <img
                      src={chan.avatarUrl}
                      alt={chan.title}
                      style={{ width: 40, height: 40, borderRadius: "50%", objectFit: "cover" }}
                    />
                  ) : (
                    <div
                      style={{
                        width: 40,
                        height: 40,
                        borderRadius: "50%",
                        backgroundColor: "var(--cs-bg-panel)",
                        border: "1px solid var(--cs-border-subtle)",
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                      }}
                    >
                      <Icon name="video" size={18} color="var(--cs-fg-muted)" />
                    </div>
                  )}
                  <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                      <span style={{ fontWeight: 700, fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-primary)" }}>
                        {chan.title}
                      </span>
                      {chan.handle && (
                        <Badge variant="neutral">
                          @{chan.handle.replace(/^@/, "")}
                        </Badge>
                      )}
                    </div>
                    <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)", fontFamily: "var(--cs-font-mono)" }}>
                      ID: {chan.id}
                    </span>
                  </div>
                </div>

                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() => handleDisconnectChannel(chan.id)}
                  busy={ytDisconnectingId === chan.id}
                  busyText="Đang ngắt..."
                  leftIcon={<Icon name="trash" size={13} />}
                >
                  Ngắt kết nối
                </Button>
              </div>
            ))}
          </div>
        )}

        {/* Revoke link info */}
        <div style={{ marginTop: 4 }}>
          <Callout variant="info">
            Để thu hồi hoàn toàn quyền truy cập hoặc xóa ứng dụng khỏi Google Account, hãy mở liên kết:{" "}
            <a
              href="https://myaccount.google.com/permissions"
              target="_blank"
              rel="noreferrer"
              style={{ color: "var(--cs-accent)", fontWeight: 600, textDecoration: "underline" }}
            >
              myaccount.google.com/permissions
            </a>
          </Callout>
        </div>
      </div>

      <YouTubeGuideModal open={guideModalOpen} onClose={() => setGuideModalOpen(false)} />
    </div>
  );
};
