# Hướng dẫn chạy Studio Web Headless trên Máy chủ Linux (dang-server)

Tài liệu hướng dẫn triển khai Local Content Studio ở chế độ **Headless (không giao diện đồ họa cửa sổ)** trên máy chủ Linux (Debian 13 trixie / dang-server) làm dịch vụ nền (systemd user service), truy cập và làm việc qua trình duyệt web trên điện thoại hoặc máy tính cá nhân qua mạng riêng ảo **Tailscale**.

---

## 1. Kiến trúc & Đặc điểm chế độ Headless

- **Không cần màn hình / X11 / Wayland:** Binary `studio-tauri --headless` khởi chạy trực tiếp lõi xử lý Rust, Studio MCP server, bộ lập lịch tác vụ và máy chủ web HTTP/HTTPS nhúng sẵn giao diện React.
- **Dữ liệu & Cấu hình:** Lưu trong thư mục `STUDIO_APP_DIR`. Service đặt `STUDIO_APP_DIR=~/.local/share/local-content-studio` (SSD). Nếu chạy lệnh tay mà **không** đặt biến này, binary dùng `~/.localcontentstudio` — khác thư mục của service, nên mọi lệnh tay ở dưới đều phải đặt `STUDIO_APP_DIR` giống service.
- **Khoá bí mật bền vững (FileSecretStore):** Trên Linux, các API key (Google, OpenAI, Facebook, YouTube) được lưu dạng văn bản thường (không mã hoá) trong `secrets.json`, ghi nguyên tử, quyền `0600` (chỉ tài khoản `dang` đọc/ghi). Nếu file hỏng, Studio cất nó sang `secrets.json.corrupt-<thời điểm>` thay vì ghi đè.
- **Đăng nhập & Sudo:** Được bảo vệ bởi mật khẩu chủ sở hữu (băm Argon2id). Lệnh quản trị nhạy cảm yêu cầu xác thực Sudo 10 phút.

---

## 2. Chuẩn bị môi trường máy chủ

### 2.1. Quyền duy trì tiến trình nền khi đăng xuất (Linger)
Để dịch vụ chạy nền ngay cả khi bạn không mở phiên SSH, chạy lệnh một lần:
```bash
sudo loginctl enable-linger dang
```

### 2.2. Kiểm tra toolchain
Đảm bảo các công cụ sau đã có trong tài khoản `dang`:
- **Rust:** `rustc --version` (khuyến nghị 1.99+)
- **Node.js & pnpm:** `node -v` (v24+), `pnpm -v` (v11+)
- **FFmpeg:** `ffmpeg -version` (hỗ trợ trích xuất, render, audio inspect)
- **Tailscale:** `tailscale status`

---

## 3. Biên dịch và cài đặt Studio

Lần đầu, sau khi laptop đã `git push server studio-v2`, clone từ bare repo trên server rồi build:
```bash
git clone -b studio-v2 ~/git/LocalContentStudio.git ~/src/LocalContentStudio
cd ~/src/LocalContentStudio
bash scripts/server/build-linux.sh
```

Kịch bản này sẽ:
1. Cài đặt các gói phụ thuộc Node qua `pnpm install --frozen-lockfile`.
2. Build Design System (`@studio/design-system`).
3. Build Desktop UI (`@studio/desktop-ui`), sinh thư mục `dist/` để nhúng thẳng vào binary Rust.
4. Biên dịch `studio-tauri` ở chế độ release với `CARGO_BUILD_JOBS=4` (phù hợp CPU 4 nhân của dang-server, tránh quá tải RAM).
5. Tự động cài đặt binary vào `~/.local/bin/studio-tauri`.

---

## 4. Thiết lập mật khẩu Web ban đầu

Chế độ web yêu cầu mật khẩu chủ sở hữu (tối thiểu 8 ký tự). Mật khẩu chỉ được nạp qua luồng tiêu chuẩn **stdin**, không bao giờ truyền qua tham số dòng lệnh `argv` hay lưu vào lịch sử bash:

```bash
# Gõ mật khẩu không hiện lên màn hình, không lưu vào file hay lịch sử bash:
read -rs -p "Mật khẩu web: " P; echo
printf '%s\n' "$P" | STUDIO_APP_DIR=~/.local/share/local-content-studio ~/.local/bin/studio-tauri --set-web-password
unset P
```

Phải đặt `STUDIO_APP_DIR` giống service; thiếu biến này mật khẩu sẽ ghi vào thư mục khác và trang web báo "Chưa thiết lập mật khẩu chủ sở hữu".

Sau khi hoàn tất, Studio sẽ ghi băm Argon2id vào cơ sở dữ liệu `app.db` và tự động kích hoạt cờ `web:enabled = true`.

---

## 5. Cài đặt và quản lý dịch vụ Systemd (User Unit)

Sử dụng systemd user unit giúp dịch vụ khởi động cùng hệ thống mà không cần đặc quyền `root`:

```bash
# 1. Tạo thư mục cấu hình systemd user nếu chưa có
mkdir -p ~/.config/systemd/user

# 2. Sao chép tệp cấu hình service
cp scripts/server/studio-web.service ~/.config/systemd/user/

# 3. Nạp lại systemd daemon
systemctl --user daemon-reload

# 4. Kích hoạt và khởi động dịch vụ
systemctl --user enable --now studio-web

# 5. Kiểm tra trạng thái dịch vụ
systemctl --user status studio-web

# 6. Xem nhật ký hoạt động thời gian thực (journald)
journalctl --user -u studio-web -f
```

---

## 6. Truy cập từ Điện thoại qua Tailscale (Khuyến nghị)

Phương án an toàn và mượt mà nhất là sử dụng **Tailscale Serve** kết hợp với chế độ loopback của Studio. Phương án này cung cấp chứng chỉ HTTPS Let's Encrypt hoàn toàn miễn phí và tự động từ Tailscale, trình duyệt trên điện thoại không hiện bất kỳ cảnh báo bảo mật nào:

### 6.1. Bật Tailscale Serve
Chạy lệnh sau trên máy chủ (một lần duy nhất):
```bash
tailscale serve --bg --https=443 http://127.0.0.1:3000
```

Nếu báo thiếu quyền, cho tài khoản `dang` quản lý Tailscale một lần (cần sudo): `sudo tailscale set --operator=dang`.

> [!TIP]
> Đảm bảo tính năng **MagicDNS** và **HTTPS Certificates** đã được bật trong bảng điều khiển Tailscale Admin Console.

### 6.2. Cấu hình cờ tin cậy tên miền Tailscale trong service
Mở tệp `~/.config/systemd/user/studio-web.service` và điều chỉnh dòng `ExecStart` với tên máy chủ Tailscale của bạn:
```ini
ExecStart=%h/.local/bin/studio-tauri --headless --port 3000 --behind-https-proxy --allowed-host dang-server.<tailnet-name>.ts.net
```
Sau đó nạp lại:
```bash
systemctl --user daemon-reload
systemctl --user restart studio-web
```

### 6.3. Mở trên điện thoại hoặc máy tính khác
1. Mở ứng dụng Tailscale trên điện thoại và bật kết nối VPN.
2. Mở trình duyệt web (Safari, Chrome, Firefox) và truy cập:
   `https://dang-server.<tailnet-name>.ts.net`
3. Nhập mật khẩu chủ sở hữu đã thiết lập ở Mục 4 để đăng nhập.
4. Mọi tính năng quản lý dự án, chat với agent (Claude, Antigravity), xem timeline, chỉnh sửa kịch bản đều khả dụng trực tiếp trên màn hình điện thoại.

---

## 7. Cập nhật phiên bản mới

Khi có mã nguồn mới, thực hiện các bước sau trên máy chủ:

```bash
cd ~/src/LocalContentStudio
git pull

# Biên dịch lại binary và nhúng UI mới
bash scripts/server/build-linux.sh

# Khởi động lại dịch vụ
systemctl --user restart studio-web
```

Dịch vụ sẽ khởi động lại mượt mà trong vòng 2–3 giây. Dữ liệu workspace, SQLite `app.db` và các API key trong `secrets.json` hoàn toàn được bảo toàn.

---

## 8. Khắc phục sự cố

| Hiện tượng | Nguyên nhân | Cách xử lý |
| :--- | :--- | :--- |
| `HTTP 403 Forbidden: Origin/Host không hợp lệ` | Truy cập qua tên miền chưa nằm trong danh sách `--allowed-host` | Thêm cờ `--allowed-host <tên-miền>` vào dòng `ExecStart` trong `studio-web.service`, sau đó reload service. |
| Mất phiên làm việc sau mỗi lần tải lại trang | Trình duyệt chặn cookie không an toàn qua HTTPS proxy | Đảm bảo đã thêm cờ `--behind-https-proxy` để kích hoạt cờ `Secure` trên cookie phiên làm việc. |
| Dịch vụ tự tắt sau khi ngắt phiên SSH | Chưa bật `linger` cho tài khoản `dang` | Chạy lệnh `sudo loginctl enable-linger dang`. |
| Quên mật khẩu web, hoặc web báo "Chưa thiết lập mật khẩu" | Mật khẩu chưa đặt hoặc đặt nhầm thư mục (thiếu `STUDIO_APP_DIR`) | Làm lại Mục 4 qua SSH, nhớ đặt `STUDIO_APP_DIR` giống service. |

---

## 9. Tích hợp liên tục (CI) & Kiểm tra tự động

Mọi commit đẩy lên các nhánh chính (`studio-v2`, `master`) và Pull Request đều được kiểm tra tự động qua GitHub Actions trên cả hai môi trường:
- **Linux (Ubuntu 24.04):** Chạy toàn bộ cổng chất lượng (`cargo fmt`, `clippy -D warnings`, unit test, UI build/test, biên dịch release `studio-tauri` và `studio-mcp`).
- **Windows (windows-latest):** Chạy unit test workspace và UI test trên Windows.

### Theo dõi kết quả CI qua GitHub CLI (`gh`)

Bạn có thể kiểm tra trực tiếp trạng thái các lượt chạy CI từ terminal:

```bash
# Xem danh sách các lần chạy CI gần nhất
gh run list

# Theo dõi tiến trình lượt chạy CI đang diễn ra
gh run watch
```

