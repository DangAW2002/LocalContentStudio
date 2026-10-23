# Hướng dẫn Sử dụng — Local Content Studio

Chào mừng bạn đến với tài liệu hướng dẫn sử dụng chính thức của **Local Content Studio** — studio sáng tạo nội dung cá nhân chạy trực tiếp trên máy tính của bạn (Tauri 2 + Rust + React), ưu tiên AI và bảo vệ quyền riêng tư tuyệt đối.

Tất cả các tài liệu dưới đây được biên soạn dành cho chủ sở hữu cá nhân, trình bày trực quan kèm hình ảnh chụp thực tế và các bước thực hiện chi tiết.

---

## Danh mục Hướng dẫn theo Chủ đề

### Khởi động & Thao tác Cơ bản
1. **[Cài đặt và Khởi chạy ứng dụng](./cai-dat.md)**
   Yêu cầu hệ thống, tải bộ cài đặt Windows, cấp quyền và khởi động ứng dụng lần đầu.
2. **[Bảng lệnh `Ctrl+K` và Phím tắt](./bang-lenh-phim-tat.md)**
   Làm chủ phím tắt `Ctrl+K`, các tiền tố tìm kiếm nhanh (`>`, `@`, `#`, `?`), danh mục phím tắt hệ thống và thanh trạng thái tương tác.

### Quản lý Thương hiệu & Tiêu chuẩn
3. **[Quản lý Brand, Kênh và Quy trình](./brand-kenh-quy-trinh.md)**
   Thiết lập nhận diện thương hiệu (bảng màu, font chữ, logo), quy chuẩn định dạng kênh phát hành, vùng an toàn (Safe Zone) và quy tắc kế thừa App → Org → Workspace.
4. **[Kiểm tra Tiêu chuẩn Kênh (Ma trận Outputs)](./kiem-tra-kenh.md)**
   Sử dụng ma trận ấn phẩm × kênh trong bảng Outputs, tra cứu ý nghĩa các mã lỗi kiểm định và tính năng "Nhờ agent sửa" chỉ với một nhấp chuột.

### Quản lý Dự án, Sơ đồ & Series
5. **[Quản lý Sơ đồ Nội dung & Series](./so-do-noi-dung.md)**
   Theo dõi mạng lưới ấn phẩm trong tab Sơ đồ, nhận biết ấn phẩm lỗi thời ("cũ") và render lại, khóa node an toàn (`pinned`), kéo nối khai báo input, hộp thoại duyệt chi phí, hồ sơ tham chiếu (`kind: reference`), sản xuất Series nhiều tập và bộ kiểm tra nhất quán (`SERIES_*`).

### Tương tác với AI & Quản lý Tài nguyên
6. **[Hội thoại AI và Kiểm soát Ngân sách](./thread-va-ngan-sach.md)**
   Thao tác với luồng chat thông minh: thử lại lượt lỗi, sửa và gửi lại tin nhắn, đổi mô hình AI giữa thread, hoàn tác trạng thái dự án (checkpoint) và thiết lập hạn mức ngân sách ngày/tháng.
7. **[Quản lý Thư viện Tài sản](./thu-vien.md)**
   Quản lý tập trung kho ảnh, video, âm thanh: kéo-thả nhập tệp, xem lại câu lệnh Prompt và chi phí AI, quy tắc giới hạn dung lượng 200 MB và chặn tệp nguy hiểm.

### Sản xuất & Xuất bản Đa nền tảng
8. **[Quy trình Sản xuất Video YouTube](./video-youtube.md)**
   Chuỗi 7 bước tự động hóa trọn gói: viết kịch bản, đọc lời thoại tiếng Việt (TTS), chuẩn bị hình ảnh B-roll, dựng dòng thời gian (`timeline_build`), tùy biến kiểu dáng phụ đề (`caption_style`), chuẩn hóa âm lượng chuẩn −14 LUFS, chèn logo thương hiệu và tạo thumbnail.
9. **[Hướng dẫn Kết nối YouTube OAuth](./ket-noi-youtube.md)**
   Từng bước tạo ứng dụng Google Cloud cá nhân, nạp Client ID/Secret và kết nối kênh để xuất bản video ở chế độ riêng tư.
10. **[Kết nối và Đăng bài lên Facebook Page](./dang-facebook.md)**
    Thiết lập Meta App, kết nối Facebook Page qua cơ chế loopback an toàn, nguyên tắc đăng bài không công khai (nháp) và gói xuất thư mục khi chưa kết nối API.

### Truy cập Di động & Mạng nội bộ
11. **[Chế độ Web Cá nhân (LAN / Wi-Fi)](./che-do-web.md)**
    Bật kết nối HTTPS tự ký, đối chiếu mã vân tay SHA-256 an toàn từ điện thoại, mật khẩu chủ sở hữu (Argon2id), giới hạn dung lượng tải lên và cơ chế xác thực Sudo 10 phút.
12. **[Triển khai Web Headless trên Máy chủ Linux (dang-server)](./may-chu-linux.md)**
    Chạy Studio dạng dịch vụ nền không giao diện (`studio-tauri --headless`), lưu trữ khoá an toàn với phân quyền `0600`, kết nối qua mạng riêng ảo Tailscale Serve HTTPS và thao tác qua trình duyệt điện thoại.
