# Kết nối YouTube để đăng video từ Local Content Studio

Studio đăng video lên kênh YouTube **của chính bạn** bằng Google OAuth. Bạn tự tạo một "OAuth client" miễn phí trên
Google Cloud, rồi nạp vào Studio. Làm một lần, khoảng 10 phút.

Khoá chỉ lưu trong Windows Credential Manager trên máy bạn, không bao giờ gửi đi đâu khác ngoài Google.

## Trước khi bắt đầu

- Một tài khoản Google **đã có kênh YouTube**. Chưa có thì vào youtube.com → ảnh đại diện → **Tạo kênh**.
- Studio đang mở một workspace.

## Bước 1 — Tạo project trên Google Cloud

1. Vào <https://console.cloud.google.com>, đăng nhập bằng tài khoản Google sở hữu kênh.
2. Thanh trên cùng → chọn project → **New project** → đặt tên (vd `Local Content Studio`) → **Create**.

## Bước 2 — Bật YouTube Data API v3

1. **APIs & Services → Library**.
2. Tìm **YouTube Data API v3** → **Enable**.

## Bước 3 — Màn đồng ý (OAuth consent) và người thử

1. **Google Auth Platform** (giao diện cũ: *APIs & Services → OAuth consent screen*) → **Get started**.
2. Tên ứng dụng: tuỳ ý (vd `Local Content Studio`); email hỗ trợ: email của bạn.
3. Audience / User type: **External**.
4. Mục **Audience → Test users** → **Add users** → nhập **đúng email tài khoản sở hữu kênh** → **Save**.
5. Giữ trạng thái **Testing**; không cần "Publish app" khi chỉ dùng cho kênh của mình.

## Bước 4 — Tạo OAuth client

1. **Credentials → Create credentials → OAuth client ID** (nếu hỏi loại dữ liệu, chọn **User data**).
2. Application type: **Desktop app** → đặt tên → **Create**.
3. Bấm **Download JSON**, được tệp `client_secret_….json`.

## Bước 5 — Nạp vào Studio và kết nối

1. Studio → **Tools & Cài đặt (`Ctrl+5`) → Tài khoản đăng bài** (mục **YouTube**).
2. Nạp tệp JSON tải về từ Google Cloud, hoặc điền tay 2 khoá `YOUTUBE_CLIENT_ID` và `YOUTUBE_CLIENT_SECRET`.
3. Bấm **Kết nối YouTube**. Trình duyệt mở trang Google:
   - chọn đúng tài khoản sở hữu kênh;
   - màn "Google chưa xác minh ứng dụng này" → **Tiếp tục** (bình thường vì app ở chế độ Testing và do chính bạn tạo);
   - tick các quyền YouTube → **Tiếp tục**.
4. Trình duyệt báo kết nối thành công → quay lại Studio. Bạn có 5 phút cho bước này; **đừng tắt Studio** khi đang đăng nhập.
5. Xoá tệp `client_secret_….json` trong Downloads (khoá đã nằm an toàn trong Windows Credential Manager).

## Đăng video

Thành phẩm (Outputs) → **Đăng…** → chọn YouTube, điền tiêu đề/mô tả/thẻ (có bộ đếm ký tự theo kênh), chọn chế độ hiển thị
(mặc định **Riêng tư**) → **Tiếp tục xác nhận** → kiểm lại tệp và quyền riêng tư → **Xác nhận đăng lên YouTube**.

- Agent chỉ được **chuẩn bị bản nháp**; chỉ bạn mới bấm đăng.
- Bản đã đăng không thể gửi lại; bản lỗi chỉ thử lại khi bạn bấm.

## Lỗi thường gặp

| Bạn thấy | Nguyên nhân | Cách sửa |
| --- | --- | --- |
| Google: **"Lỗi 403: access_denied"**, "chưa hoàn tất quy trình xác minh của Google" | Email chưa có trong Test users | Bước 3.4: thêm đúng email vào **Test users**, rồi kết nối lại |
| Google: **"Lỗi 400: invalid_request — Required parameter is missing: response_type"** | Bản Studio cũ mở URL bị cắt (đã sửa) | Cập nhật Studio |
| Trình duyệt: **"127.0.0.1 refused to connect"** sau khi bấm Tiếp tục | Studio đã tắt hoặc quá 5 phút trước khi Google chuyển về | Mở Studio, bấm **Kết nối YouTube** lại và làm liền một mạch |
| Studio: "Hết thời gian chờ xác thực YouTube" | Không hoàn tất đăng nhập trong 5 phút | Bấm Kết nối lại |
| Studio: **`youtubeSignupRequired`** / "Unauthorized" khi đăng | Tài khoản chưa có kênh YouTube | youtube.com → **Tạo kênh**, rồi bấm thử lại bản đăng |
| Studio: "Tên khoá bí mật không hợp lệ: YOUTUBE_CLIENT_ID" | Bản Studio cũ chưa cho phép khoá YouTube (đã sửa) | Cập nhật Studio |
| Google: "redirect_uri_mismatch" | Tạo nhầm loại client (Web application) | Tạo lại client loại **Desktop app** |
| Đăng xong nhưng video bị khoá "Riêng tư" dù chọn Công khai | Project chưa qua kiểm duyệt API của YouTube: video từ project chưa xác minh bị giữ riêng tư | Dùng cho kênh riêng thì để Riêng tư rồi tự đổi trên YouTube Studio; muốn công khai tự động cần xin YouTube API audit |

## Gỡ kết nối

Xoá `YOUTUBE_REFRESH_TOKEN` (và 2 khoá client nếu muốn) trong Studio, và thu hồi quyền tại
<https://myaccount.google.com/permissions> → Local Content Studio → **Xoá quyền truy cập**.
