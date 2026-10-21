# Nhật ký thay đổi (Changelog) — Local Content Studio

Tất cả các thay đổi đáng chú ý của Local Content Studio được ghi lại trong tài liệu này theo ngữ cảnh người dùng cuối.

---

## [0.2.0] — 2026-10-21

Bản phát hành lớn thứ hai của Local Content Studio tập trung vào độ tin cậy của quy trình sáng tạo nội dung tự động bằng AI, quản lý đa kênh, mở rộng trải nghiệm điều khiển và hoàn thiện chuỗi sản xuất video YouTube từ đầu đến cuối.

### Lưu ý quan trọng khi nâng cấp từ 0.1.0
- **Tự động sao lưu và chuyển đổi dữ liệu (Migration)**: Khi bạn mở workspace tạo từ bản 0.1.0 bằng phiên bản 0.2.0, hệ thống sẽ tự động tạo một tệp sao lưu nguyên vẹn cơ sở dữ liệu (`.studio/backups/studio-pre-v<số>-<thời điểm>.db`) trước khi nâng cấp cấu trúc dữ liệu SQLite lên phiên bản mới nhất. Toàn bộ hội thoại (thread), lượt trao đổi (turns), tệp định nghĩa tài sản (`artifact.yaml`), ảnh và video đã render đều được giữ nguyên vẹn 100%. Quá trình này diễn ra hoàn toàn tự động và an toàn.
- **Bộ cài đặt chưa ký số (Cảnh báo SmartScreen)**: Bộ cài đặt Windows (.exe) của phiên bản 0.2.0 chưa mua chứng chỉ ký số thương mại (nhằm tránh phát sinh chi phí đăng ký hằng năm). Khi cài đặt trên Windows 10/11, trình bảo vệ Windows SmartScreen có thể hiển thị thông báo *"Windows protected your PC"* hoặc *"Trình bảo vệ Windows đã bảo vệ PC của bạn"*. Bạn chỉ cần bấm vào liên kết **More info** (Thông tin thêm) và bấm nút **Run anyway** (Vẫn chạy). Bạn có thể kiểm tra tính toàn vẹn của tệp cài bằng mã băm SHA-256 được công bố đi kèm trong tệp `SHA256SUMS.txt`.

---

### Tính năng mới & Cải tiến nổi bật trong 0.2.0

#### 1. Quản lý Thương hiệu, Kênh & Quy trình theo Phạm vi (Brand, Channels & Playbooks)
- **Kế thừa & Phân cấp (App → Org → Workspace)**: Cấu hình thương hiệu (`brand.yaml`), quy chuẩn kênh (`channel.yaml`) và quy trình sáng tạo (`playbook.yaml`) nay có thể được dùng chung ở cấp Ứng dụng/Tổ chức hoặc ghi đè tùy chỉnh cho từng Workspace dự án.
- **Tự động sao chép tài sản khi ghi đè**: Khi tạo bản ghi đè (override) trong workspace, ứng dụng tự động sao chép logo, font chữ và hình ảnh liên quan vào thư mục dự án cục bộ, đảm bảo tính độc lập và toàn vẹn của dự án.
- **Kiểm tra quy chuẩn thời gian thực**: Trình soạn thảo cung cấp thông báo lỗi và cảnh báo bằng tiếng Việt trực quan (tỉ lệ khung hình, vùng an toàn safe-zone, giới hạn ký tự tiêu đề/mô tả).

#### 2. Ma trận Kênh phân phối (Channel Matrix)
- **Đối soát đa kênh trong tab Outputs**: Bảng ma trận trực quan hiển thị trạng thái kiểm tra của từng tài nguyên nội dung (ảnh thumbnail, video) so với từng kênh mục tiêu (YouTube, Facebook, TikTok...).
- **Bộ nhớ đệm thông minh (Cache SQLite)**: Kết quả kiểm tra được lưu đệm tự động theo mã băm render và phiên bản kênh; không cần tính toán lại nếu nội dung không đổi.
- **Hỗ trợ Agent sửa lỗi**: Bấm vào bất kỳ ô vi phạm tiêu chuẩn nào để xem nguyên nhân chi tiết và nhấn *"Nhờ agent sửa"* để tự động đưa yêu cầu khắc phục vào khung chat của trợ lý AI.

#### 3. Bảng lệnh Thông minh (Command Palette Ctrl+K)
- **Truy cập nhanh mọi thao tác**: Nhấn tổ hợp phím `Ctrl+K` để mở bảng điều khiển tìm kiếm mờ (fuzzy search) hỗ trợ gõ tiếng Việt có dấu và không dấu.
- **Tiền tố điều hướng tiện lợi**:
  - `>`: Tìm và thực thi nhanh các lệnh hệ thống.
  - `@`: Chuyển nhanh giữa các cuộc trò chuyện (threads).
  - `#`: Mở trực tiếp các tệp tin trong dự án.
  - `?`: Tra cứu danh sách phím tắt hữu ích.
- **Ghi nhớ lệnh gần đây (MRU)**: Tự động đưa 10 lệnh bạn thường dùng lên đầu danh sách.
- **Thanh trạng thái tương tác**: Nhấp trực tiếp vào các chỉ số trên thanh trạng thái (Status Bar) để mở nhanh bảng kiểm tra engine Claude/agy, dung lượng ổ đĩa hay danh sách tác vụ.

#### 4. Hội thoại Agent Tin cậy & Kiểm soát Ngân sách Chi tiêu
- **Thử lại & Sửa tin nhắn dễ dàng**: Hỗ trợ nút *"Thử lại"* khi một lượt trả lời của agent gặp lỗi kết nối và nút *"Sửa & gửi lại"* tin nhắn gần nhất (tự động khôi phục điểm phục hồi checkpoint an toàn).
- **Chuyển đổi Model AI linh hoạt**: Có thể thay đổi linh hoạt model AI ngay giữa cuộc hội thoại thông qua danh sách chọn model ở đầu khung chat.
- **Giới hạn ngân sách ngày & tháng**: Đặt mức chi phí tối đa (USD) mỗi ngày hoặc mỗi tháng. Khi chạm trần, mọi lệnh tốn phí (sinh ảnh/video/giọng đọc) đều hỏi bạn trước khi chạy; chi phí hôm nay hiện trên thanh trạng thái.
- **Theo dõi tiến trình AI trực quan**: Hiển thị số lượng token suy nghĩ (thinking tokens) theo thời gian thực và mô tả từng bước thực thi công cụ của trợ lý AI bằng tiếng Việt dễ hiểu.

#### 5. Chuỗi sản xuất Video YouTube Tự động Đầu-cuối
- **Dựng video tự động với `timeline_build`**: Trợ lý AI có thể tự động phân tích thời lượng giọng đọc, sắp xếp cảnh quay nền, chia phụ đề ngắt câu thông minh (dưới 42 ký tự/dòng) khớp chính xác từng mili-giây.
- **Chuẩn hóa âm lượng chuẩn phát sóng (Loudnorm)**: Tự động tinh chỉnh âm thanh video về chuẩn YouTube (-14 LUFS), giúp âm thanh rõ ràng, không bị rè hoặc quá nhỏ.
- **Lớp phủ logo**: Đặt logo thương hiệu (PNG) đè lên video ở vị trí, kích thước và độ mờ tuỳ chọn; trình kiểm tra kênh nhận ra logo trên lớp phủ.
- **Bản nháp đăng riêng tư**: Agent chuẩn bị bản nháp (video, thumbnail, tiêu đề, mô tả, tag) và kiểm theo giới hạn của YouTube; chỉ khi bạn bấm xác nhận, video mới được tải lên ở chế độ riêng tư. Bản nháp còn lỗi không thể xác nhận.

#### 6. Thư viện Tài sản Tập trung (Asset Library)
- **Quản lý đa nguồn**: Xem toàn bộ hình ảnh, âm thanh, video và tài liệu từ các dự án tại một nơi duy nhất.
- **Thông tin đi kèm**: Ảnh/video do AI sinh hiện kèm prompt, nhà cung cấp, model và chi phí; mọi tệp hiện kích thước và định dạng.
- **Kéo thả & Hoàn tác**: Kéo thả tệp từ máy tính để thêm nhanh vào thư viện (tối đa 200 MB/tệp, chặn tệp thực thi); xóa tệp có xác nhận và hoàn tác được.
- **Sử dụng trực tiếp trong chat**: Dễ dàng đính kèm tài sản từ thư viện vào ngữ cảnh trò chuyện với trợ lý AI.

#### 7. Truy cập Giao diện Web Bảo mật qua Mạng LAN (HTTPS)
- **Điều khiển từ điện thoại / máy tính bảng**: Truy cập Local Content Studio từ trình duyệt của bất kỳ thiết bị nào trong cùng mạng Wi-Fi/LAN thông qua giao thức HTTPS bảo mật với chứng chỉ TLS tự ký.
- **Quét mã QR tiện lợi**: Hiển thị mã QR kết nối nhanh trong phần Cài đặt > Truy cập Web.
- **Mật khẩu chủ & Chế độ Sudo**: Mọi truy cập web cần mật khẩu chủ; các thao tác nhạy cảm (đăng video, cài công cụ, khoá API, hoàn tác) yêu cầu nhập lại mật khẩu trong 10 phút. Có thể đối chiếu vân tay SHA-256 của chứng chỉ trong Cài đặt.

#### 8. Hỗ trợ Đăng bài Trang Facebook (Facebook Page Publisher)
- **Tích hợp OAuth an toàn**: Kết nối an toàn với Fanpage của bạn, mã truy cập được lưu trữ bảo mật trong Windows Credential Manager.
- **Chế độ kiểm duyệt trước khi đăng**: Mặc định tạo bài viết ở chế độ chưa công khai (unpublished/nháp) để bạn kiểm duyệt chất lượng hình ảnh, nội dung trước khi bấm xuất bản.
- **Gói xuất bản ngoại tuyến**: Hỗ trợ tải về gói dữ liệu hoàn chỉnh để đăng thủ công nếu bạn không muốn liên kết tài khoản trực tiếp.

#### 9. Tinh chỉnh Giao diện & Hiệu năng
- **Bố cục co giãn thông minh**: Tối ưu hiển thị hoàn hảo trên các màn hình có độ rộng từ 768px, 900px, 1100px đến 1440px.
- **Phím tắt an toàn với bộ gõ tiếng Việt**: Tất cả các phím tắt hệ thống được thiết kế tránh tổ hợp phím xung đột với Unikey và EVKey.
- **Task Center tập trung (Ctrl+J)**: Theo dõi tiến độ chạy nền của các tác vụ render video, nén âm thanh và tải lên với báo cáo lỗi chi tiết.

---

## [0.1.0] — 2026-10-14

Bản phát hành đầu tiên đánh dấu kiến trúc Desktop AI-first độc lập cho cá nhân:
- Nhân hệ thống viết bằng Rust và Tauri v2, hỗ trợ Windows 10/11 x64.
- Giao diện người dùng React và hệ thống Design System chuẩn WCAG AA.
- Tương thích và tích hợp sâu với các engine AI cục bộ qua giao thức dòng lệnh (Claude Code CLI, agy CLI) và Model Context Protocol (MCP).
- Quản lý Workspace dự án độc lập, lưu trữ dữ liệu hoàn toàn cục bộ trên máy với SQLite WAL và hệ thống Content-Addressable Storage (CAS).
- Hỗ trợ công cụ kết xuất đa phương tiện cơ bản (HTML sang hình ảnh, FFmpeg render video storyboard).
