# Sổ tay Hướng dẫn Sử dụng — Local Content Studio

Chào mừng bạn đến với **Local Content Studio** — studio sáng tạo nội dung cá nhân, cục bộ trên máy tính (desktop) kết hợp sức mạnh điều phối của các AI Agent.

Bộ tài liệu hướng dẫn này được thiết kế dành cho một chủ sở hữu duy nhất (single-owner), với các chỉ dẫn trực quan, từng bước đánh số rõ ràng, chỉ rõ những gì bạn thấy trên màn hình và cách xử lý nhanh khi gặp sự cố.

---

## Danh mục tài liệu hướng dẫn

### Khởi đầu & Thiết lập cơ bản
1. **[Cài đặt & Nâng cấp Studio](./cai-dat.md)**
   Cài đặt môi trường ứng dụng trên máy tính Windows, kiểm tra các công cụ hệ thống cần thiết và cập nhật phiên bản mới.
2. **[Bảng lệnh và Phím tắt](./bang-lenh-phim-tat.md)**
   Làm chủ phím tắt `Ctrl+K`, các tiền tố tìm kiếm nhanh (`>`, `@`, `#`, `?`), danh mục phím tắt hệ thống và thanh trạng thái tương tác.

### Quản lý Thương hiệu & Tiêu chuẩn
3. **[Quản lý Brand, Kênh và Quy trình](./brand-kenh-quy-trinh.md)**
   Thiết lập nhận diện thương hiệu (bảng màu, font chữ, logo), quy chuẩn định dạng kênh phát hành, vùng an toàn (Safe Zone) và quy tắc kế thừa App → Org → Workspace.
4. **[Kiểm tra Tiêu chuẩn Kênh (Ma trận Outputs)](./kiem-tra-kenh.md)**
   Sử dụng ma trận ấn phẩm × kênh trong bảng Outputs, tra cứu ý nghĩa các mã lỗi kiểm định và tính năng "Nhờ agent sửa" chỉ với một nhấp chuột.

### Tương tác với AI & Quản lý Tài nguyên
5. **[Hội thoại AI và Kiểm soát Ngân sách](./thread-va-ngan-sach.md)**
   Thao tác với luồng chat thông minh: thử lại lượt lỗi, sửa và gửi lại tin nhắn, đổi mô hình AI giữa thread, hoàn tác trạng thái dự án (checkpoint) và thiết lập hạn mức ngân sách ngày/tháng.
6. **[Quản lý Thư viện Tài sản](./thu-vien.md)**
   Quản lý tập trung kho ảnh, video, âm thanh: kéo-thả nhập tệp, xem lại câu lệnh Prompt và chi phí AI, quy tắc giới hạn dung lượng 200 MB và chặn tệp nguy hiểm.

### Sản xuất & Xuất bản Đa nền tảng
7. **[Quy trình Sản xuất Video YouTube](./video-youtube.md)**
   Chuỗi 7 bước tự động hóa trọn gói: viết kịch bản, đọc lời thoại tiếng Việt (TTS), chuẩn bị hình ảnh B-roll, dựng dòng thời gian (timeline), chuẩn hóa âm lượng chuẩn −14 LUFS, chèn logo thương hiệu và tạo thumbnail.
8. **[Hướng dẫn Kết nối YouTube OAuth](./ket-noi-youtube.md)**
   Từng bước tạo ứng dụng Google Cloud cá nhân, nạp Client ID/Secret và kết nối kênh để xuất bản video ở chế độ riêng tư.
9. **[Kết nối và Đăng bài lên Facebook Page](./dang-facebook.md)**
   Thiết lập Meta App, kết nối Facebook Page qua cơ chế loopback an toàn, nguyên tắc đăng bài không công khai (nháp) và gói xuất thư mục khi chưa kết nối API.

### Truy cập Di động & Mạng nội bộ
10. **[Chế độ Web Cá nhân (LAN / Wi-Fi)](./che-do-web.md)**
    Bật kết nối HTTPS tự ký, đối chiếu mã vân tay SHA-256 an toàn từ điện thoại, mật khẩu chủ sở hữu (Argon2id), giới hạn dung lượng tải lên và cơ chế xác thực Sudo 10 phút.

---

> [!NOTE]
> Tab **Sơ đồ** trong Project (xem sản phẩm nào phụ thuộc vào sản phẩm nào và cái nào đã cũ) mới có từ bản 0.2.0; hướng dẫn riêng sẽ bổ sung sau.
