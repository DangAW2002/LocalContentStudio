# Chế độ Web Cá nhân (Personal Web Mode) — Local Content Studio

Chế độ Web Cá nhân cho phép bạn truy cập và điều khiển Local Content Studio từ trình duyệt web (trên chính máy tính này, hoặc từ điện thoại/máy tính bảng/laptop trong cùng mạng Wi-Fi/LAN).

Tất cả tác vụ, luồng hội thoại AI, job render và tệp tin lưu trữ đều tiếp tục chạy trực tiếp trên máy tính chính của bạn. Đây là chế độ dành riêng cho **một chủ sở hữu duy nhất** (single-owner), không yêu cầu tài khoản đám mây hay dịch vụ trung gian nào.

---

## 1. Cách kích hoạt trong Studio

1. Mở ứng dụng Studio trên máy tính.
2. Điều hướng tới **Tools & Cài đặt** (hoặc nhấn `Ctrl+5`) → Chọn tab **Truy cập Web**.
3. Bấm **Thiết lập mật khẩu**:
   - Nhập mật khẩu chủ sở hữu (tối thiểu 6 ký tự).
   - Mật khẩu được mã hóa an toàn bằng thuật toán **Argon2id** và lưu trực tiếp trong cơ sở dữ liệu `app.db` của máy.
4. Bấm **Bật Chế độ Web**:
   - Dịch vụ web cá nhân sẽ khởi chạy trên cổng mặc định `3000`.
   - Trình duyệt trên máy tính có thể truy cập ngay tại: `http://localhost:3000`.

---

## 2. Kết nối từ Điện thoại hoặc Laptop qua Wi-Fi/LAN (HTTPS)

1. Trong tab **Truy cập Web**, bật tuỳ chọn: **Cho phép truy cập từ mạng LAN (HTTPS Tự ký)**.
   - Studio sẽ chuyển sang lắng nghe trên `0.0.0.0` với kết nối **HTTPS bảo mật** qua chứng chỉ tự ký (self-signed TLS).
   - Địa chỉ LAN sẽ xuất hiện dưới dạng: `https://192.168.x.x:3000`.
2. **Quét mã QR**:
   - Trên màn hình Cài đặt sẽ hiển thị một mã QR tự động chứa đường dẫn `https://...`.
   - Mở ứng dụng Camera trên điện thoại hoặc máy tính bảng kết nối cùng mạng Wi-Fi, quét mã để mở ngay trang Studio.
3. **Chấp nhận Cảnh báo Chứng chỉ Tự ký trên Trình duyệt Di động**:
   - Vì chứng chỉ SSL được sinh tự động cục bộ bởi Studio (chứng chỉ tự ký cá nhân cho IP LAN và localhost), trình duyệt trên điện thoại (Chrome, Safari, Firefox) sẽ hiển thị cảnh báo bảo mật lần đầu tiên (e.g. *"Kết nối của bạn không phải là kết nối riêng tư"* hoặc *"Chứng chỉ không đáng tin cậy"*).
   - **Đối chiếu Vân tay SHA-256 (SHA-256 Fingerprint):**
     - Bấm vào biểu tượng ổ khóa hoặc nút *Xem chứng chỉ* trên trình duyệt điện thoại.
     - So sánh chuỗi **Vân tay SHA-256** (dạng `AA:BB:CC:...`) hiển thị trên điện thoại với chuỗi vân tay được hiển thị trong tab **Truy cập Web** trên máy tính. Khi hai chuỗi trùng khớp 100%, bạn chắc chắn đang kết nối an toàn tới máy tính của chính mình.
   - Bấm **Nâng cao (Advanced)** → Chọn **Tiếp tục truy cập (Proceed/Accept Risk)** để vào trang đăng nhập Studio.
4. Nhập mật khẩu chủ sở hữu đã thiết lập để đăng nhập.

> [!WARNING]
> **Lưu ý an toàn mạng Wi-Fi:** Khi bật kết nối LAN, bất kỳ ai kết nối vào mạng Wi-Fi của bạn đều có thể thấy trang đăng nhập của Studio. Hãy luôn đặt mật khẩu đủ mạnh và chỉ bật kết nối LAN trên mạng Wi-Fi tin cậy (nhà riêng).

---

## 3. Các Cơ chế An ninh Nghiêm ngặt

Vì Studio trao quyền cho các agent AI thực thi công cụ và quản lý tệp tin, lớp bảo mật của chế độ web được thiết kế theo chuẩn nghiêm ngặt:

1. **Phiên đăng nhập an toàn (HttpOnly Cookie):**
   - Phiên đăng nhập được quản lý bằng cookie `studio_session` với các cờ `HttpOnly`, `SameSite=Strict`, và `Path=/`. Trình duyệt web không thể đọc hoặc làm rò rỉ cookie này qua script phía client. Khi kết nối qua HTTPS LAN, cờ `Secure` tự động được kích hoạt để đảm bảo cookie không bao giờ truyền qua kênh không mã hóa.
2. **Chống dò mật khẩu (Rate Limiting):**
   - Nhập sai mật khẩu liên tiếp 5 lần sẽ bị khóa đăng nhập tạm thời trong 60 giây.
3. **Xác thực quyền quản trị (Sudo Mode):**
   - Các hành động có tác động lớn hoặc làm thay đổi hệ thống ngoài (đăng video YouTube `publish_confirm`, cài đặt/gỡ công cụ `tool_install_confirm`, tải mô hình/binary giọng đọc `tts_model_install`, cấu hình khoá bí mật `secret_set`/`secret_delete`, phục hồi checkpoint hoặc hoàn tác lượt `turn_undo`) bắt buộc bạn phải nhập lại mật khẩu chủ sở hữu trong vòng 10 phút gần nhất.
4. **Chống tấn công CSRF & DNS Rebinding:**
   - Mọi yêu cầu `POST` bắt buộc phải có header `x-studio-csrf: 1`.
   - Máy chủ kiểm tra nghiêm ngặt header `Origin` và `Host`. Mọi yêu cầu từ domain bên ngoài hoặc không khớp với loopback/LAN IP đều bị từ chối ngay lập tức (HTTP 403 Forbidden). Cả scheme `http://` và `https://` tương ứng với chế độ phục vụ đều được kiểm tra chặt chẽ.
   - Không hỗ trợ CORS cho bên ngoài.
5. **Giới hạn Dung lượng Tải Lên & JSON Có Thể Cấu Hình (Body Limits):**
   - Tuyến tải tệp `/api/upload` mặc định giới hạn tối đa 200 MB (có thể tùy chỉnh lên đến trần cứng 1024 MB / 1 GB trong Cài đặt).
   - Tuyến lệnh JSON `/api/invoke/*` và các endpoint API giới hạn 1 MB (trần cứng 10 MB) nhằm ngăn ngừa tấn công DoS tràn bộ nhớ.
   - Khi vượt quá giới hạn, hệ thống trả về mã lỗi `HTTP 413 Payload Too Large`.
6. **Bảo mật Khoá Bí Mật & Khoá Riêng TLS (Zero Leaks):**
   - Các khoá API (Anthropic, Gemini, OpenAI, YouTube client secret...) chỉ cho phép ghi một chiều (`write-only`) vào Credential Manager.
   - Khoá riêng TLS (`web_key.pem`) được lưu trong thư mục dữ liệu ứng dụng của người dùng (`STUDIO_APP_DIR/tls/`), không bao giờ nằm trong workspace, không xuất hiện trong nhật ký log hay gói sao lưu export.
7. **Xem tệp an toàn (Safe File Scoping):**
   - API xem trước tệp `/api/file` chỉ cho phép đọc các tệp nằm bên trong workspace hiện tại; mọi nỗ lực vượt quyền (path traversal như `../`) đều bị chặn đứng và ghi nhật ký cảnh báo.

---

## 4. Trải nghiệm trên Trình duyệt Di động & Web

- **Giao diện đáp ứng (Responsive):**
  - Màn hình Dự án trên màn hình nhỏ (điện thoại) tự động chuyển sang chế độ chuyển tab mượt mà giữa **Hội thoại (Thread)**, **Stage (Khu vực xem trước)** và **Thành phẩm (Outputs)**.
  - Trên màn hình điện thoại (≤ 480px) và màn hình hẹp (< 600px), thanh tiêu cự (top bar) tự động thu gọn thanh tìm kiếm thành icon button, rút gọn breadcrumb và ẩn gợi ý phím tắt (`Ctrl+K`, `Ctrl+J`) để không tràn layout.
  - Thanh trạng thái (status bar) tự động điều chỉnh và ưu tiên hiển thị thông tin workspace cùng số tác vụ đang chạy.
- **Hỗ trợ Giọng đọc ngoại tuyến (TTS):** Giao diện web hỗ trợ đầy đủ các thao tác xem trạng thái nhà cung cấp TTS (VieNeu-TTS, Piper, Windows SAPI), chọn giọng đọc mặc định, nghe thử trực tiếp trên trình duyệt và tải mô hình giọng đọc (có xác thực Sudo).
- **Phát đa phương tiện tối ưu:** API `/api/file` hỗ trợ đầy đủ chuẩn HTTP `Range Requests`, cho phép tua nhanh video 4K/FullHD và nghe trực tuyến audio trên cả Safari di động và Chrome.
- **Cập nhật thời gian thực:** Kết nối Server-Sent Events (SSE) `/api/events` nhận và hiển thị phản hồi streaming từng chữ của AI và tiến độ công việc trong nền không có độ trễ.
- **Tải tệp tin:** Khi cần nạp ảnh/video trên trình duyệt (thay vì cửa sổ chọn tệp hệ thống của máy tính), hệ thống cung cấp endpoint upload tạm thời an toàn hỗ trợ kích thước cấu hình linh hoạt.

---

## 5. Khoảng trống đã biết & Hướng phát triển

- **Cảnh báo chứng chỉ tự ký trên thiết bị mới:** Vì chứng chỉ tự sinh nội bộ, người dùng cần xác nhận một lần trên mỗi trình duyệt điện thoại/máy tính bảng.
- **Tắt máy tính:** Vì Studio chạy hoàn toàn trên máy tính cá nhân của bạn, khi máy tính chuyển sang chế độ ngủ (Sleep) hoặc tắt nguồn, kết nối web từ điện thoại sẽ tạm thời gián đoạn.
