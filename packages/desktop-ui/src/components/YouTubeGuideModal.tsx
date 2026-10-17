import React from "react";
import { Icon } from "./Icon";
import { Button, Badge, Callout } from "@studio/design-system";

export interface YouTubeGuideModalProps {
  open: boolean;
  onClose: () => void;
}

export const YouTubeGuideModal: React.FC<YouTubeGuideModalProps> = ({ open, onClose }) => {
  if (!open) return null;

  return (
    <div
      style={{
        position: "fixed",
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        backgroundColor: "rgba(0, 0, 0, 0.65)",
        backdropFilter: "blur(4px)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 1000,
        padding: "var(--cs-space-4)",
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="yt-guide-title"
        style={{
          width: "100%",
          maxWidth: 720,
          maxHeight: "90vh",
          backgroundColor: "var(--cs-bg-panel)",
          borderRadius: "var(--cs-radius-lg)",
          border: "1px solid var(--cs-border-subtle)",
          boxShadow: "var(--cs-shadow-modal)",
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}
      >
        {/* Header */}
        <div
          style={{
            padding: "var(--cs-space-4) var(--cs-space-5)",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            borderBottom: "1px solid var(--cs-border-subtle)",
            backgroundColor: "var(--cs-bg-raised)",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 10, fontWeight: 600, fontSize: "var(--cs-text-md)" }}>
            <Icon name="check" size={20} color="#ff0000" />
            <span id="yt-guide-title">Hướng dẫn kết nối YouTube & Đăng bài</span>
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="Đóng"
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

        {/* Content body */}
        <div
          style={{
            padding: "var(--cs-space-5)",
            overflowY: "auto",
            display: "flex",
            flexDirection: "column",
            gap: "var(--cs-space-4)",
            fontSize: "var(--cs-text-sm)",
            lineHeight: 1.6,
            color: "var(--cs-fg-primary)",
          }}
        >
          <Callout variant="neutral">
            Studio đăng video lên kênh YouTube <strong>của chính bạn</strong> bằng Google OAuth. Bạn tự tạo một "OAuth client" miễn phí trên Google Cloud rồi nạp vào Studio (khoảng 10 phút, làm 1 lần duy nhất). Khóa chỉ lưu trong Windows Credential Manager trên máy bạn.
          </Callout>

          {/* Section: Trước khi bắt đầu */}
          <div>
            <h3 style={{ fontSize: "var(--cs-text-base)", fontWeight: 600, marginBottom: 6 }}>Trước khi bắt đầu</h3>
            <ul style={{ paddingLeft: 20, margin: 0 }}>
              <li>Một tài khoản Google <strong>đã có kênh YouTube</strong>. Nếu chưa có, vào <code>youtube.com</code> → ảnh đại diện → <strong>Tạo kênh</strong>.</li>
              <li>Studio đang mở một workspace.</li>
            </ul>
          </div>

          {/* Step 1 */}
          <div style={{ borderTop: "1px solid var(--cs-border-subtle)", paddingTop: 12 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
              <Badge variant="info">Bước 1</Badge>
              <h3 style={{ fontSize: "var(--cs-text-base)", fontWeight: 600, margin: 0 }}>Tạo project trên Google Cloud</h3>
            </div>
            <ol style={{ paddingLeft: 20, margin: 0 }}>
              <li>Vào <a href="https://console.cloud.google.com" target="_blank" rel="noreferrer" style={{ color: "var(--cs-accent)" }}>https://console.cloud.google.com</a>, đăng nhập tài khoản Google sở hữu kênh.</li>
              <li>Thanh trên cùng → chọn project → <strong>New project</strong> → đặt tên (vd <code>Local Content Studio</code>) → <strong>Create</strong>.</li>
            </ol>
          </div>

          {/* Step 2 */}
          <div style={{ borderTop: "1px solid var(--cs-border-subtle)", paddingTop: 12 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
              <Badge variant="info">Bước 2</Badge>
              <h3 style={{ fontSize: "var(--cs-text-base)", fontWeight: 600, margin: 0 }}>Bật YouTube Data API v3</h3>
            </div>
            <ol style={{ paddingLeft: 20, margin: 0 }}>
              <li>Menu bên trái → <strong>APIs & Services → Library</strong>.</li>
              <li>Tìm kiếm <strong>YouTube Data API v3</strong> → bấm <strong>Enable</strong>.</li>
            </ol>
          </div>

          {/* Step 3 */}
          <div style={{ borderTop: "1px solid var(--cs-border-subtle)", paddingTop: 12 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
              <Badge variant="info">Bước 3</Badge>
              <h3 style={{ fontSize: "var(--cs-text-base)", fontWeight: 600, margin: 0 }}>Màn hình đồng ý (OAuth consent screen) & Người thử (Test users)</h3>
            </div>
            <ol style={{ paddingLeft: 20, margin: 0 }}>
              <li><strong>Google Auth Platform</strong> (hoặc <em>APIs & Services → OAuth consent screen</em>) → <strong>Get started</strong>.</li>
              <li>Tên ứng dụng: tuỳ ý (vd <code>Local Content Studio</code>); email hỗ trợ: email của bạn.</li>
              <li>User type: <strong>External</strong>.</li>
              <li>Mục <strong>Audience → Test users</strong> → <strong>Add users</strong> → nhập <strong>đúng email tài khoản sở hữu kênh</strong> → <strong>Save</strong>.</li>
              <li>Giữ trạng thái <strong>Testing</strong> (không cần Publish app khi chỉ dùng cho kênh của mình).</li>
            </ol>
          </div>

          {/* Step 4 */}
          <div style={{ borderTop: "1px solid var(--cs-border-subtle)", paddingTop: 12 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
              <Badge variant="info">Bước 4</Badge>
              <h3 style={{ fontSize: "var(--cs-text-base)", fontWeight: 600, margin: 0 }}>Tạo OAuth client</h3>
            </div>
            <ol style={{ paddingLeft: 20, margin: 0 }}>
              <li>Menu <strong>Credentials → Create credentials → OAuth client ID</strong>.</li>
              <li>Application type: <strong>Desktop app</strong> (tuyệt đối không chọn Web application) → đặt tên → <strong>Create</strong>.</li>
              <li>Bấm <strong>Download JSON</strong>, bạn được tệp <code>client_secret_….json</code>.</li>
            </ol>
          </div>

          {/* Step 5 */}
          <div style={{ borderTop: "1px solid var(--cs-border-subtle)", paddingTop: 12 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
              <Badge variant="info">Bước 5</Badge>
              <h3 style={{ fontSize: "var(--cs-text-base)", fontWeight: 600, margin: 0 }}>Nạp vào Studio và kết nối</h3>
            </div>
            <ol style={{ paddingLeft: 20, margin: 0 }}>
              <li>Studio → <strong>Cài đặt → Tài khoản đăng bài</strong>.</li>
              <li>Bấm chọn tệp <code>client_secret_….json</code> vừa tải, hoặc điền Client ID và Client Secret.</li>
              <li>Bấm <strong>Kết nối YouTube</strong>. Trình duyệt mở trang đăng nhập Google:
                <ul style={{ marginTop: 4 }}>
                  <li>Chọn đúng tài khoản sở hữu kênh;</li>
                  <li>Màn "Google chưa xác minh ứng dụng này" → bấm <strong>Tiếp tục</strong> (an toàn vì app do chính bạn tạo);</li>
                  <li>Tick các quyền YouTube → bấm <strong>Tiếp tục</strong>.</li>
                </ul>
              </li>
              <li>Trình duyệt báo kết nối thành công → quay lại Studio. Bạn có 5 phút; <strong>đừng tắt Studio khi đang đăng nhập</strong>.</li>
            </ol>
          </div>

          {/* Common Errors */}
          <div style={{ borderTop: "1px solid var(--cs-border-subtle)", paddingTop: 12 }}>
            <h3 style={{ fontSize: "var(--cs-text-base)", fontWeight: 600, marginBottom: 8 }}>Bảng lỗi thường gặp & Cách xử lý</h3>
            <div style={{ overflowX: "auto" }}>
              <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 13 }}>
                <thead>
                  <tr style={{ borderBottom: "1px solid var(--cs-border-subtle)", textAlign: "left" }}>
                    <th style={{ padding: "6px 8px" }}>Hiện tượng</th>
                    <th style={{ padding: "6px 8px" }}>Nguyên nhân</th>
                    <th style={{ padding: "6px 8px" }}>Cách sửa</th>
                  </tr>
                </thead>
                <tbody>
                  <tr style={{ borderBottom: "1px solid var(--cs-border-subtle)" }}>
                    <td style={{ padding: "6px 8px", fontWeight: 500 }}>403: access_denied</td>
                    <td style={{ padding: "6px 8px" }}>Email chưa có trong Test users</td>
                    <td style={{ padding: "6px 8px" }}>Thêm đúng email vào <strong>Test users</strong> của OAuth consent screen rồi kết nối lại.</td>
                  </tr>
                  <tr style={{ borderBottom: "1px solid var(--cs-border-subtle)" }}>
                    <td style={{ padding: "6px 8px", fontWeight: 500 }}>youtubeSignupRequired</td>
                    <td style={{ padding: "6px 8px" }}>Tài khoản Google chưa có kênh</td>
                    <td style={{ padding: "6px 8px" }}>Vào youtube.com → <strong>Tạo kênh</strong>, rồi thử lại.</td>
                  </tr>
                  <tr style={{ borderBottom: "1px solid var(--cs-border-subtle)" }}>
                    <td style={{ padding: "6px 8px", fontWeight: 500 }}>redirect_uri_mismatch</td>
                    <td style={{ padding: "6px 8px" }}>Tạo nhầm client Web</td>
                    <td style={{ padding: "6px 8px" }}>Tạo lại client loại <strong>Desktop app</strong>.</td>
                  </tr>
                  <tr style={{ borderBottom: "1px solid var(--cs-border-subtle)" }}>
                    <td style={{ padding: "6px 8px", fontWeight: 500 }}>quotaExceeded</td>
                    <td style={{ padding: "6px 8px" }}>Hết hạn mức YouTube API ngày</td>
                    <td style={{ padding: "6px 8px" }}>Mặc định ~6 lượt tải/ngày. Đợi sang ngày hôm sau.</td>
                  </tr>
                  <tr style={{ borderBottom: "1px solid var(--cs-border-subtle)" }}>
                    <td style={{ padding: "6px 8px", fontWeight: 500 }}>Hết thời gian chờ (timeout)</td>
                    <td style={{ padding: "6px 8px" }}>Quá 5 phút hoặc đã tắt Studio</td>
                    <td style={{ padding: "6px 8px" }}>Bấm Kết nối lại và làm liền một mạch trong 5 phút.</td>
                  </tr>
                  <tr>
                    <td style={{ padding: "6px 8px", fontWeight: 500 }}>Video bị giữ Riêng tư</td>
                    <td style={{ padding: "6px 8px" }}>Project Google chưa kiểm duyệt API</td>
                    <td style={{ padding: "6px 8px" }}>YouTube tự giữ riêng tư với project cá nhân chưa audit; bạn có thể tự đổi sang Công khai trên YouTube Studio.</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>

          {/* Revocation */}
          <div style={{ borderTop: "1px solid var(--cs-border-subtle)", paddingTop: 12 }}>
            <h3 style={{ fontSize: "var(--cs-text-base)", fontWeight: 600, marginBottom: 6 }}>Gỡ kết nối và thu hồi quyền</h3>
            <p style={{ margin: 0 }}>
              Để gỡ tài khoản: bấm nút <strong>Ngắt kết nối</strong> cạnh kênh trong Cài đặt Studio. Sau đó truy cập{" "}
              <a href="https://myaccount.google.com/permissions" target="_blank" rel="noreferrer" style={{ color: "var(--cs-accent)" }}>
                https://myaccount.google.com/permissions
              </a>{" "}
              để xóa quyền truy cập của Local Content Studio.
            </p>
          </div>
        </div>

        {/* Footer */}
        <div
          style={{
            padding: "var(--cs-space-3) var(--cs-space-5)",
            borderTop: "1px solid var(--cs-border-subtle)",
            backgroundColor: "var(--cs-bg-raised)",
            display: "flex",
            justifyContent: "flex-end",
          }}
        >
          <Button variant="primary" onClick={onClose}>
            Đã hiểu & Đóng
          </Button>
        </div>
      </div>
    </div>
  );
};
