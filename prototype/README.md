# Prototype giao diện Local Content Studio

Prototype tĩnh, bấm thử được, dùng để duyệt thiết kế trước khi đưa vào `packages/desktop-ui`. Không cần build: mở `index.html` bằng trình duyệt (Chrome/Edge), hoặc chạy `npx serve prototype`.

Dữ liệu trong prototype là dữ liệu mẫu. Không có lời gọi mạng hay backend nào.

## Thanh PROTOTYPE (dưới giữa màn hình)

- **Trạng thái**: Có dữ liệu / Đang tải / Trống / Lỗi / Bị chặn. Mỗi màn có cách hiển thị và hướng xử lý riêng cho từng trạng thái. "Bị chặn" còn chuyển ứng dụng sang chế độ không dùng AI và chưa kết nối Social.
- **Sáng / Tối**.
- **Bề rộng**: Toàn màn, 1280, 768, 390, 320 (giả lập bằng container query).

## Phạm vi

| Màn | Nội dung chính |
| --- | --- |
| Tổng quan | Cần xử lý, chỉ số, bảng dự án, hoạt động, tình trạng hệ thống |
| Nghiên cứu | Thu thập URL có giới hạn, nguồn, trích dẫn, cấu trúc, hình ảnh; chặn SSRF |
| Thư viện | Bộ lọc, lưới tài nguyên, cách ly, xác nhận quyền (chỉ người dùng), chi tiết |
| Thương hiệu | Phiên bản nháp/phát hành, màu + tương phản, chữ, logo, giọng điệu, kênh, so sánh |
| Soạn thảo | Biến thể kênh, trích dẫn, đếm ký tự, xem trước mô phỏng, QA, AI có so sánh trước/sau |
| Thiết kế | Công cụ, canvas chọn/bỏ chọn/dịch bằng phím, lớp, trang, inspector chỉnh được |
| Chuyển động | Storyboard, trình phát proxy, timeline nhiều track, cấu hình render |
| Tự động hóa & AI | Quy trình có cổng duyệt, lần chạy + checkpoint, quyền công cụ, CLI/MCP, nhật ký |
| Bàn giao | Các mốc độc lập có nguồn xác nhận, biên nhận, đối soát, checklist sẵn sàng |
| Cài đặt | Chung, AI, Social, lưu trữ & sao lưu, năng lực hệ thống, plugin, giới thiệu |

Overlay: command palette (Ctrl+K), tạo dự án (4 bước), xuất, nhập tệp, xác nhận quyền, xác minh Windows Hello, lưu revision, phím tắt (?), công bố dữ liệu gửi AI, Task Center (Ctrl+J), toast, menu ngữ cảnh.

## Quy ước thiết kế

Quy tắc đầy đủ và có tính ràng buộc nằm ở `.aw/documentation/design-system.md`. Tóm tắt:

- Token theo `.aw/documentation/design-system.md`, có điều chỉnh (xem đầu `css/tokens.css`). Bo góc: 0 cho cấu trúc, 2px cho control.
- Không hiển thị enum thô: mọi trạng thái đi qua `UI.status` (nhãn tiếng Việt + icon + mô tả trong tooltip).
- Tooltip hiện khi hover và khi focus bằng bàn phím; thao tác không khả dụng luôn kèm lý do hiển thị cạnh nút.
- Trạng thái đang xử lý (đang gửi, đang chờ) dùng màu thông tin, không dùng xanh lá.
- Không dùng tổ hợp Ctrl+Shift để tránh xung đột bộ gõ tiếng Việt.
