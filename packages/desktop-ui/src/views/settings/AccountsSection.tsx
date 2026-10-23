import React from "react";
import { Icon } from "../../components/Icon";
import { Badge, Button, Callout, EmptyState, Input, Spinner } from "@studio/design-system";
import { studioApi, parseYouTubeError } from "../../api/client";
import type {
  YouTubeChannelAccount,
  YoutubeClientStatus,
  YouTubeErrorDetail,
  FacebookPageAccount,
  FacebookClientStatus,
} from "../../api/types";
import { YouTubeGuideModal } from "../../components/YouTubeGuideModal";

export const AccountsSection: React.FC = () => {
  // Platform tab: "facebook" | "youtube" | "tiktok" | "instagram"
  const [activePlatform, setActivePlatform] = React.useState<"facebook" | "youtube" | "tiktok" | "instagram">("facebook");

  // YouTube states
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

  // Facebook states
  const [fbClientStatus, setFbClientStatus] = React.useState<FacebookClientStatus | null>(null);
  const [fbPages, setFbPages] = React.useState<FacebookPageAccount[]>([]);
  const [fbAppIdInput, setFbAppIdInput] = React.useState("");
  const [fbAppSecretInput, setFbAppSecretInput] = React.useState("");
  const [fbConfigBusy, setFbConfigBusy] = React.useState(false);
  const [fbConfigError, setFbConfigError] = React.useState<string | null>(null);
  const [fbConfigSuccess, setFbConfigSuccess] = React.useState<string | null>(null);
  const [fbConnecting, setFbConnecting] = React.useState(false);
  const [fbCountdown, setFbCountdown] = React.useState(300);
  const [fbConnectError, setFbConnectError] = React.useState<YouTubeErrorDetail | null>(null);
  const [fbConnectSuccess, setFbConnectSuccess] = React.useState<string | null>(null);
  const [fbDisconnectingId, setFbDisconnectingId] = React.useState<string | null>(null);
  const [fbGuideOpen, setFbGuideOpen] = React.useState(false);

  const loadAccounts = React.useCallback(async () => {
    try {
      const [yStatus, yChannels, fStatus, fPages] = await Promise.all([
        studioApi.youtubeClientStatus().catch(() => null),
        studioApi.youtubeChannelsList().catch(() => []),
        studioApi.facebookClientStatus().catch(() => null),
        studioApi.facebookPagesList().catch(() => []),
      ]);
      setYtClientStatus(yStatus);
      setYtChannels(yChannels);
      setFbClientStatus(fStatus);
      setFbPages(fPages);
    } catch {
      // ignore
    }
  }, []);

  React.useEffect(() => {
    loadAccounts();
  }, [loadAccounts]);

  // Timers for OAuth countdown
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

  React.useEffect(() => {
    let timer: any = null;
    if (fbConnecting && fbCountdown > 0) {
      timer = setInterval(() => {
        setFbCountdown((prev) => Math.max(0, prev - 1));
      }, 1000);
    }
    return () => {
      if (timer) clearInterval(timer);
    };
  }, [fbConnecting, fbCountdown]);

  // Facebook handlers
  const handleConfigFacebook = async () => {
    if (!fbAppIdInput.trim() || !fbAppSecretInput.trim()) {
      setFbConfigError("Vui lòng nhập cả App ID và App Secret.");
      return;
    }
    setFbConfigBusy(true);
    setFbConfigError(null);
    setFbConfigSuccess(null);
    try {
      const res = await studioApi.facebookClientConfig(fbAppIdInput.trim(), fbAppSecretInput.trim());
      setFbConfigSuccess(`Đã lưu cấu hình Meta App thành công! (…${res.appIdLast4})`);
      setFbAppIdInput("");
      setFbAppSecretInput("");
      const status = await studioApi.facebookClientStatus();
      setFbClientStatus(status);
    } catch (err: any) {
      const parsed = parseYouTubeError(err);
      setFbConfigError(parsed ? parsed.messageVi : (err?.message || "Không thể lưu thông tin Meta App."));
    } finally {
      setFbConfigBusy(false);
    }
  };

  const handleConnectFacebook = async () => {
    setFbConnecting(true);
    setFbCountdown(300);
    setFbConnectError(null);
    setFbConnectSuccess(null);
    try {
      const pages = await studioApi.facebookConnect();
      setFbConnectSuccess(`Kết nối thành công ${pages.length} Facebook Page!`);
      const updated = await studioApi.facebookPagesList();
      setFbPages(updated);
    } catch (err: any) {
      const parsed = parseYouTubeError(err);
      if (parsed) {
        setFbConnectError(parsed);
      } else {
        setFbConnectError({
          code: "unknown",
          messageVi: err?.message || "Không thể kết nối Facebook Page",
          fixVi: "Vui lòng kiểm tra lại cấu hình App ID/Secret và thử lại.",
        });
      }
    } finally {
      setFbConnecting(false);
    }
  };

  const handleDisconnectFacebookPage = async (pageId: string) => {
    setFbDisconnectingId(pageId);
    try {
      await studioApi.facebookPageDisconnect(pageId);
      const updated = await studioApi.facebookPagesList();
      setFbPages(updated);
    } catch (err: any) {
      alert(err?.message || "Không thể ngắt kết nối Facebook Page.");
    } finally {
      setFbDisconnectingId(null);
    }
  };

  // YouTube handlers
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
      {/* Platform Selector Tabs */}
      <div style={{ display: "flex", gap: "var(--cs-space-2)", borderBottom: "1px solid var(--cs-border-subtle)", paddingBottom: "var(--cs-space-2)" }}>
        <Button
          
          variant={activePlatform === "facebook" ? "primary" : "secondary"}
          onClick={() => setActivePlatform("facebook")}
          leftIcon={<Icon name="globe" size={14} />}
        >
          Facebook Page {fbPages.length > 0 && `(${fbPages.length})`}
        </Button>
        <Button
          
          variant={activePlatform === "youtube" ? "primary" : "secondary"}
          onClick={() => setActivePlatform("youtube")}
          leftIcon={<Icon name="video" size={14} />}
        >
          YouTube {ytChannels.length > 0 && `(${ytChannels.length})`}
        </Button>
        <Button
          
          variant={activePlatform === "tiktok" ? "primary" : "secondary"}
          onClick={() => setActivePlatform("tiktok")}
        >
          TikTok
        </Button>
        <Button
          
          variant={activePlatform === "instagram" ? "primary" : "secondary"}
          onClick={() => setActivePlatform("instagram")}
        >
          Instagram
        </Button>
      </div>

      {/* FACEBOOK TAB */}
      {activePlatform === "facebook" && (
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-4)" }}>
          {/* Header */}
          <div style={{ display: "flex", alignItems: "flex-start", justifyContent: "space-between", gap: 16 }}>
            <div>
              <h2 style={{ margin: "0 0 4px 0", fontSize: "var(--cs-text-md)", fontWeight: 700 }}>
                Tài khoản Đăng bài (Facebook Page)
              </h2>
              <p style={{ margin: 0, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", lineHeight: 1.5 }}>
                Đăng bài viết ảnh và video trực tiếp lên Facebook Page thông qua Meta Graph API (v25.0).
                Thông tin App Secret và Page Token được bảo vệ trong Windows Credential Manager.
              </p>
            </div>
            <Button
              
              variant="secondary"
              onClick={() => setFbGuideOpen(true)}
              leftIcon={<Icon name="help" size={13} />}
            >
              Xem checklist U-TAIKHOAN
            </Button>
          </div>

          {/* Checklist Modal */}
          {fbGuideOpen && (
            <div
              style={{
                position: "fixed",
                inset: 0,
                backgroundColor: "rgba(0,0,0,0.5)",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                zIndex: 1000,
                padding: "var(--cs-space-4)",
              }}
              onClick={() => setFbGuideOpen(false)}
            >
              <div
                style={{
                  backgroundColor: "var(--cs-bg-panel)",
                  borderRadius: "var(--cs-radius-modal)",
                  border: "1px solid var(--cs-border-subtle)",
                  maxWidth: 580,
                  width: "100%",
                  maxHeight: "85vh",
                  overflowY: "auto",
                  padding: "var(--cs-space-5)",
                  display: "flex",
                  flexDirection: "column",
                  gap: "var(--cs-space-3)",
                }}
                onClick={(e) => e.stopPropagation()}
              >
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <h3 style={{ margin: 0, fontSize: "var(--cs-text-md)", fontWeight: 700 }}>
                    Checklist thiết lập Facebook Page (U-TAIKHOAN)
                  </h3>
                  <Button  variant="secondary" onClick={() => setFbGuideOpen(false)}>
                    Đóng
                  </Button>
                </div>

                <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", lineHeight: 1.6, display: "flex", flexDirection: "column", gap: 10 }}>
                  <p style={{ margin: 0 }}>
                    <strong>1. Redirect URI trên Meta Developer Portal:</strong><br />
                    Meta bắt buộc HTTPS cho URI chuyển hướng, trừ <code>localhost</code>: khi App ở chế độ <strong>Phát triển</strong>, URI dưới đây được tự cho phép, không cần thêm.<br />
                    Nếu Meta vẫn báo URL bị chặn, vào <strong>Facebook Login</strong> → <strong>Cài đặt</strong> và thêm vào ô <em>Valid OAuth Redirect URIs</em>:
                  </p>
                  <pre style={{ background: "var(--cs-bg-raised)", padding: "8px 12px", borderRadius: 4, margin: 0, fontFamily: "var(--cs-font-mono)" }}>
                    http://localhost:8911/callback
                  </pre>
                  <p style={{ margin: 0 }}>
                    <strong>2. Lấy App ID & App Secret:</strong><br />
                    Trong menu trái, vào <strong>Cài đặt ứng dụng</strong> → <strong>Cơ bản</strong>. Sao chép App ID và App Secret dán vào Studio.
                  </p>
                  <p style={{ margin: 0 }}>
                    <strong>3. Quyền hạn yêu cầu (Scopes):</strong><br />
                    <code>pages_show_list, business_management, pages_manage_posts, pages_read_engagement</code>.<br />
                    Trong Development Mode, tài khoản Facebook của bạn cần là Quản trị viên của Meta App và quản trị viên của Facebook Page cần đăng.
                  </p>
                </div>
              </div>
            </div>
          )}

          {/* SECTION 1: Cấu hình App ID/Secret */}
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
                  1. Cấu hình Meta App (Facebook)
                </span>
                <Badge variant={fbClientStatus?.configured ? "success" : "warning"}>
                  {fbClientStatus?.configured
                    ? `Đã cấu hình (…${fbClientStatus.appIdLast4 || "****"})`
                    : "Chưa cấu hình"}
                </Badge>
              </div>
            </div>

            <p style={{ margin: 0, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", lineHeight: 1.5 }}>
              Nhập App ID và App Secret từ Meta for Developers. Khóa được lưu trữ an toàn trong Windows Credential Manager.
            </p>

            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12, marginTop: 4 }}>
              <div>
                <label style={{ display: "block", fontSize: "var(--cs-text-xs)", fontWeight: 600, marginBottom: 4 }}>
                  App ID
                </label>
                <Input
                  
                  placeholder="VD: 1234567890123456"
                  value={fbAppIdInput}
                  onChange={(e) => setFbAppIdInput(e.target.value)}
                />
              </div>
              <div>
                <label style={{ display: "block", fontSize: "var(--cs-text-xs)", fontWeight: 600, marginBottom: 4 }}>
                  App Secret
                </label>
                <Input
                  
                  type="password"
                  placeholder="Nhập App Secret"
                  value={fbAppSecretInput}
                  onChange={(e) => setFbAppSecretInput(e.target.value)}
                />
              </div>
            </div>

            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <Button
                
                variant="primary"
                onClick={handleConfigFacebook}
                busy={fbConfigBusy}
                busyText="Đang lưu..."
                disabled={!fbAppIdInput.trim() || !fbAppSecretInput.trim()}
              >
                Lưu thông tin ứng dụng
              </Button>
            </div>

            {fbConfigError && (
              <Callout variant="danger">
                {fbConfigError}
              </Callout>
            )}

            {fbConfigSuccess && (
              <Callout variant="success">
                {fbConfigSuccess}
              </Callout>
            )}
          </div>

          {/* SECTION 2: Kết nối Facebook Page */}
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
                  2. Kết nối Facebook Page
                </span>
                <Badge variant={fbPages.length > 0 ? "success" : "neutral"}>
                  {fbPages.length > 0 ? `Đã kết nối ${fbPages.length} trang` : "Chưa kết nối"}
                </Badge>
              </div>

              <Button
                
                variant="primary"
                onClick={handleConnectFacebook}
                busy={fbConnecting}
                busyText={`Đang chờ trình duyệt (${fbCountdown}s)...`}
                disabled={!fbClientStatus?.configured || fbConnecting}
                leftIcon={<Icon name="link" size={13} />}
              >
                {fbPages.length > 0 ? "Thêm / Kết nối lại Page" : "Kết nối Facebook Page"}
              </Button>
            </div>

            {fbConnecting && (
              <Callout variant="info">
                <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                  <Spinner size={16} />
                  <div>
                    Trình duyệt đang mở để bạn xác thực tài khoản Facebook. Sau khi cấp quyền cho các Page, hãy quay lại đây.
                    <br />
                    <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
                      Cổng xác thực cục bộ: <code>http://localhost:8911/callback</code>. Còn {fbCountdown} giây...
                    </span>
                  </div>
                </div>
              </Callout>
            )}

            {fbConnectError && (
              <Callout variant="danger" title={fbConnectError.messageVi}>
                <p style={{ margin: "0 0 8px 0" }}>{fbConnectError.fixVi}</p>
                <div style={{ display: "flex", gap: 8, marginTop: 8 }}>
                  <Button  variant="secondary" onClick={handleConnectFacebook}>
                    Thử lại
                  </Button>
                  <Button  variant="secondary" onClick={() => setFbGuideOpen(true)}>
                    Mở hướng dẫn U-TAIKHOAN
                  </Button>
                </div>
              </Callout>
            )}

            {fbConnectSuccess && (
              <Callout variant="success">
                {fbConnectSuccess}
              </Callout>
            )}

            {/* Pages List */}
            {fbPages.length === 0 ? (
              <EmptyState
                icon={<Icon name="globe" size={28} />}
                title="Chưa có Facebook Page nào được kết nối"
                description="Sau khi lưu App ID/Secret, bấm 'Kết nối Facebook Page' để liên kết các trang bạn quản lý."
              />
            ) : (
              <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-2)" }}>
                {fbPages.map((page) => (
                  <div
                    key={page.id}
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
                      {page.avatarUrl ? (
                        <img
                          src={page.avatarUrl}
                          alt={page.name}
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
                          <Icon name="globe" size={18} color="var(--cs-fg-muted)" />
                        </div>
                      )}
                      <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                          <span style={{ fontWeight: 700, fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-primary)" }}>
                            {page.name}
                          </span>
                          {page.category && (
                            <Badge variant="neutral">
                              {page.category}
                            </Badge>
                          )}
                        </div>
                        <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)", fontFamily: "var(--cs-font-mono)" }}>
                          Page ID: {page.id}
                        </span>
                      </div>
                    </div>

                    <Button
                      
                      variant="secondary"
                      onClick={() => handleDisconnectFacebookPage(page.id)}
                      busy={fbDisconnectingId === page.id}
                      busyText="Đang ngắt..."
                      leftIcon={<Icon name="trash" size={13} />}
                    >
                      Ngắt kết nối
                    </Button>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      )}

      {/* YOUTUBE TAB */}
      {activePlatform === "youtube" && (
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
                    
                    variant="primary"
                    onClick={() => ytFileInputRef.current?.click()}
                    busy={ytImportBusy}
                    busyText="Đang nạp tệp..."
                    leftIcon={<Icon name="upload" size={13} />}
                  >
                    Chọn tệp client_secret_*.json
                  </Button>
                </div>
              </div>
            ) : (
              <div style={{ display: "flex", flexDirection: "column", gap: 8, marginTop: 4 }}>
                <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
                  <div>
                    <label style={{ display: "block", fontSize: "var(--cs-text-xs)", fontWeight: 600, marginBottom: 4 }}>
                      Client ID
                    </label>
                    <Input
                      
                      placeholder="VD: 123456789-xyz.apps.googleusercontent.com"
                      value={ytClientIdInput}
                      onChange={(e) => setYtClientIdInput(e.target.value)}
                    />
                  </div>
                  <div>
                    <label style={{ display: "block", fontSize: "var(--cs-text-xs)", fontWeight: 600, marginBottom: 4 }}>
                      Client Secret
                    </label>
                    <Input
                      
                      type="password"
                      placeholder="Nhập Client Secret"
                      value={ytClientSecretInput}
                      onChange={(e) => setYtClientSecretInput(e.target.value)}
                    />
                  </div>
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <Button
                    
                    variant="primary"
                    onClick={handleImportManual}
                    busy={ytImportBusy}
                    busyText="Đang lưu..."
                    disabled={!ytClientIdInput.trim() || !ytClientSecretInput.trim()}
                  >
                    Lưu thông tin Client
                  </Button>
                </div>
              </div>
            )}

            {ytImportError && (
              <Callout variant="danger">
                {ytImportError}
              </Callout>
            )}

            {ytImportSuccess && (
              <Callout variant="success">
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
                  2. Kênh YouTube đã kết nối
                </span>
                <Badge variant={ytChannels.length > 0 ? "success" : "neutral"}>
                  {ytChannels.length > 0 ? `Đã kết nối ${ytChannels.length} kênh` : "Chưa kết nối"}
                </Badge>
              </div>

              <Button
                
                variant="primary"
                onClick={handleConnectYouTube}
                busy={ytConnecting}
                busyText={`Đang chờ trình duyệt (${ytCountdown}s)...`}
                disabled={!ytClientStatus?.configured || ytConnecting}
                leftIcon={<Icon name="link" size={13} />}
              >
                {ytChannels.length > 0 ? "Thêm / Kết nối lại kênh" : "Kết nối YouTube"}
              </Button>
            </div>

            {ytConnecting && (
              <Callout variant="info">
                <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                  <Spinner size={16} />
                  <div>
                    Trình duyệt đang mở để bạn chọn tài khoản Google và cấp quyền tải lên YouTube.
                    <br />
                    <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
                      Sau khi cấp quyền thành công, màn hình sẽ tự động cập nhật. Còn {ytCountdown} giây...
                    </span>
                  </div>
                </div>
              </Callout>
            )}

            {ytConnectError && (
              <Callout variant="danger" title={ytConnectError.messageVi}>
                <p style={{ margin: "0 0 8px 0" }}>{ytConnectError.fixVi}</p>
                <div style={{ display: "flex", gap: 8, marginTop: 8 }}>
                  <Button  variant="secondary" onClick={handleConnectYouTube}>
                    Thử lại
                  </Button>
                  <Button  variant="secondary" onClick={() => setGuideModalOpen(true)}>
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
      )}

      {/* TIKTOK & INSTAGRAM TABS (Chưa kết nối — dùng gói xuất) */}
      {(activePlatform === "tiktok" || activePlatform === "instagram") && (
        <div
          style={{
            padding: "var(--cs-space-5)",
            backgroundColor: "var(--cs-bg-panel)",
            border: "1px solid var(--cs-border-subtle)",
            borderRadius: "var(--cs-radius-control)",
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            textAlign: "center",
            gap: "var(--cs-space-3)",
            minHeight: 240,
          }}
        >
          <Icon name="folder" size={36} color="var(--cs-fg-muted)" />
          <div>
            <h3 style={{ margin: "0 0 6px 0", fontSize: "var(--cs-text-md)", fontWeight: 700 }}>
              {activePlatform === "tiktok" ? "TikTok" : "Instagram"}: Chưa kết nối — dùng gói xuất
            </h3>
            <p style={{ margin: 0, fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-secondary)", maxWidth: 500, lineHeight: 1.5 }}>
              Nền tảng {activePlatform === "tiktok" ? "TikTok" : "Instagram"} chưa được mở quyền API đăng tải trực tiếp.
              Trong mục Xuất bản, hệ thống sẽ tự động xuất một <strong>gói thư mục</strong> chứa đầy đủ file video/ảnh,
              ảnh thu nhỏ, tệp siêu dữ liệu JSON và chú thích để bạn đăng bài thủ công.
            </p>
          </div>
          <Badge variant="warning">
            Chưa kết nối — dùng gói xuất
          </Badge>
        </div>
      )}
    </div>
  );
};
