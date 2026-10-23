# Quản lý Sơ đồ Nội dung & Series — Local Content Studio

Tab **Sơ đồ** trong màn hình Dự án (nhấn `Ctrl+2` để mở Dự án, sau đó nhấp tab **Sơ đồ**) hiển thị mạng lưới các ấn phẩm (artifacts) trong dự án: quan hệ phụ thuộc giữa các node, trạng thái cập nhật hay lỗi thời ("cũ"), khóa ấn phẩm (`pinned`), hồ sơ tham chiếu (`reference`) và quy trình sản xuất theo Series nhiều tập.

![Tổng quan Sơ đồ Nội dung](anh/12-graph-overview-light.png)

---

## 1. Các Node & Cạnh Phụ Thuộc (Nodes & Edges)

Mỗi ấn phẩm nội dung là một thẻ **Node**, liên kết với nhau qua các đường **Cạnh (Edges)** có định hướng.

### Phân loại Cạnh (Roles)

Hệ thống hỗ trợ 3 kiểu liên kết phụ thuộc:

| Kiểu liên kết | Định dạng đường | Lan truyền trạng thái cũ | Mục đích sử dụng |
| :--- | :--- | :--- | :--- |
| `uses` | Nét liền | **Có** (đổi mã băm render) | Node đích sử dụng trực tiếp tệp nguồn của Node gốc (ví dụ: Timeline dùng tệp âm thanh giọng đọc và hình ảnh B-roll). |
| `reference` | Nét đứt | **Không** | Cung cấp ảnh mẫu, giọng nói chuẩn hoặc hồ sơ ngữ cảnh cho AI; không kích hoạt kết xuất lại khi hồ sơ thay đổi. |
| `variant_of` | Nét chấm | **Có** (khi bản gốc đổi) | Ấn phẩm phái sinh từ bản gốc (ví dụ: Video ngắn dọc 9:16 biến thể từ video ngang 16:9). |

### Trạng thái của Node

Mỗi node hiển thị nhãn phân loại (Kịch bản, Thiết kế, Âm thanh, Timeline, Video, Hồ sơ) và huy hiệu trạng thái:
- **Mới / Sẵn sàng (Xanh lục):** Nội dung và bản render trên đĩa khớp với dữ liệu đầu vào hiện tại.
- **Cũ (Vàng cam):** Khi một node đầu vào (`input`) thay đổi nội dung, mã băm đầu vào (`render_cache_key`) không còn khớp với bản kết xuất gần nhất. Trạng thái cũ tự động lan truyền xuôi dòng qua các cạnh `uses` và `variant_of`.
- **Chưa render (Xám):** Ấn phẩm mới tạo cấu hình, chưa có tệp kết xuất thành phẩm.

### Cách Render lại Node

1. **Render trực tiếp:** Nhấp chuột phải vào node → chọn **Render lại**.
2. **Xem trên Stage:** Nhấp đúp vào node (hoặc chọn **Mở trên Stage** trong menu chuột phải) để chuyển sang tab Stage, sau đó nhấn nút **Render** trên thanh công cụ.
3. **Cập nhật xuôi dòng:** Chọn **Cập nhật phía dưới** để dựng lại các node phụ thuộc theo thứ tự topo.

---

## 2. Khóa Node (`pinned: true`)

Khi ấn phẩm đã hoàn thiện (ví dụ: kịch bản đã duyệt hoặc thumbnail ưng ý), bạn có thể bật **Khóa node** để tránh bị AI hoặc tiến trình tự động ghi đè.

![Menu ngữ cảnh chuột phải trên Node](anh/13-graph-node-menu-light.png)

### Cơ chế hoạt động của Node Khóa

1. **Thao tác:** Nhấp chuột phải vào node → chọn **Khóa node (pinned)**. Node hiển thị biểu tượng ổ khóa màu vàng cam.
2. **Chặn công cụ ngoài & AI:** Máy chủ Studio MCP từ chối các yêu cầu `artifact_render` hoặc tiến trình ghi đè vào thư mục của artifact đó kèm thông báo:
   > *"Artifact {id} đang khoá — mở khoá trong tab Sơ đồ trước khi render/gen lại."*
3. **Bỏ qua khi cập nhật xuôi dòng:** Tiến trình cập nhật tự động bỏ qua các node đang khóa.
4. **Cảnh báo và Hoàn tác sau lượt chat:** Nếu AI thay đổi tệp thuộc node bị khóa trong một lượt trò chuyện, hệ thống hiển thị cảnh báo trên khung Thread kèm nút **Hoàn tác phần này** để khôi phục tệp từ checkpoint trước lượt.
5. **Mở khóa:** Nhấp chuột phải vào node → chọn **Mở khóa node**.

---

## 3. Kéo Nối Cạnh & Kế Hoạch Cập Nhật Phía Dưới

Bạn có thể chỉnh sửa liên kết phụ thuộc trực tiếp trên canvas.

### Thao tác Kéo Nối Cạnh
1. Rê chuột vào chấm tròn (handle) ở cạnh phải của node nguồn.
2. Kéo đường nối sang chấm tròn ở cạnh trái của node đích rồi thả chuột.
3. Hộp thoại **Chọn vai trò liên kết** xuất hiện: chọn giữa `uses`, `reference`, hoặc `variant_of`.
4. Hệ thống kiểm tra chu trình: nếu liên kết mới tạo thành vòng lặp (A → B → C → A), hệ thống từ chối và hiển thị thông báo lỗi.
5. Thao tác nối cạnh hoặc xóa cạnh (chọn cạnh rồi nhấn `Delete`, `Backspace` hoặc nút **Xóa cạnh** trên thanh công cụ) tự động lưu vào tệp `artifact.yaml` và tạo một checkpoint khôi phục trong Workspace.

### Hộp Thoại Kế Hoạch Cập Nhật Phía Dưới

Khi thay đổi nội dung ở node đầu nguồn:
- Nhấp chuột phải vào node → chọn **Cập nhật phía dưới**.
- Studio phân tích thứ tự topo của đồ thị và mở hộp thoại kế hoạch:

![Hộp thoại Kế hoạch Cập nhật Phía dưới](anh/14-graph-cost-dialog-light.png)

- **Bước miễn phí:** Render HTML sang ảnh, HyperFrames và FFmpeg chạy cục bộ trên máy.
- **Bước tốn phí:** Các tác vụ gọi API đám mây (sinh ảnh qua `image.generate@1`, sinh video qua `video.generate@1`) hiển thị chi phí ước tính (USD).
- Hộp thoại cung cấp nút **Chạy bước miễn phí ({số bước})** hoặc **Chạy cả bước tốn phí ({số bước}) (≈ ${chi phí})**. Node bị khóa (`pinned`) được đánh dấu bỏ qua.

---

## 4. Hồ Sơ Tham Chiếu (`kind: reference`)

Hồ sơ tham chiếu dùng để giữ tính nhất quán về phong cách mỹ thuật và nhân vật qua các tập nội dung.

### Cấu trúc Hồ sơ Tham chiếu
Hồ sơ được lưu dưới dạng artifact có `kind: reference`:
- Tệp `profile.md`: Mô tả nhân vật, trang phục, phong cách ánh sáng hoặc giọng đọc.
- Tệp ảnh mẫu (`params.images`): Ví dụ `an-1.png`, `an-2.png` chụp chân dung ở nhiều góc nhìn.
- Hạt giống ngẫu nhiên (`params.seed`): Giữ tính nhất quán khi sinh ảnh AI.

### Thao tác với Hồ sơ
- **Đặt làm hồ sơ:** Nhấp chuột phải vào bất kỳ artifact hoặc asset hình ảnh nào trên sơ đồ → chọn **Đặt làm hồ sơ**. Hệ thống chuyển đổi ấn phẩm thành node tham chiếu (`kind: reference`).
- **Gắn hồ sơ…:** Nhấp chuột phải vào node cần tham chiếu phong cách → chọn **Gắn hồ sơ…**. Chọn hồ sơ từ danh sách để tạo cạnh liên kết kiểu `reference`.

---

## 5. Quản lý Series & Bộ Kiểm Tra Nhất Quán (`SERIES_*`)

Studio hỗ trợ sản xuất nội dung theo chuỗi nhiều tập với cấu hình lưu tại `series/<slug>/series.yaml`.

![Hộp thoại Tạo tập từ Series](anh/15-graph-series-light.png)

### Tạo Series & Tạo Tập Mới
1. **Khai báo Series:** Định nghĩa bộ Kit, Playbook và danh sách hồ sơ nhân vật dùng chung.
2. **Tạo tập mới:** Bấm nút **+ Tập từ series** trên thanh tiêu đề Dự án. Hệ thống tạo thư mục dự án tập mới và tự động liên kết cạnh `reference` tới các hồ sơ tham chiếu của Series.

### Bộ Kiểm Tra Tiêu Chuẩn Series

Khi kiểm định qua Ma trận Kênh, hệ thống đối chiếu chéo giữa các tập trong cùng Series:

| Mã kiểm định | Tên hiển thị | Nguyên nhân & Khắc phục |
| :--- | :--- | :--- |
| `SERIES_INTRO_MISMATCH` | Khác clip mở đầu (intro) | Clip đầu tiên trên timeline không cùng nguồn tư liệu với các tập khác trong series. |
| `SERIES_OUTRO_MISMATCH` | Khác clip kết thúc (outro) | Clip kết thúc video khác so với quy chuẩn của series. |
| `SERIES_FONT_MISMATCH` | Khác font tiêu đề thumbnail | Phông chữ thiết kế trên thumbnail không đồng nhất với các tập trước. |
| `SERIES_VOICE_MISMATCH` | Khác giọng đọc TTS | Giọng đọc AI khác mô hình hoặc người đọc đã chọn cho series. |
| `SERIES_REFERENCE_MISSING` | Chưa gắn hồ sơ tham chiếu | Tập chưa có cạnh nối `reference` tới hồ sơ nhân vật của series. Nhấp chuột phải vào node và chọn **Gắn hồ sơ…** để liên kết. |

---

## 6. Giao Diện Sáng / Tối (Theme)

Canvas Sơ đồ nội dung tự động cập nhật màu nền và đường lưới theo chủ đề giao diện:
- Đổi giao diện: Nhấn nút biểu tượng mặt trời / mặt trăng ở góc phải thanh tiêu đề hoặc mở Bảng lệnh (`Ctrl+K`) và gõ lệnh `theme.toggle` (không có phím tắt riêng). Phím `Ctrl+J` dùng để mở trung tâm tác vụ (Task Center).

![Sơ đồ nội dung ở Chế độ Tối](anh/16-graph-overview-dark.png)

---

## Phím Tắt & Thao Tác Canvas

- **Cuộn chuột:** Phóng to / thu nhỏ canvas.
- **Kéo chuột trên nền trống:** Di chuyển vùng nhìn (`pan`).
- **Nhấp chuột vào node:** Chọn node trên canvas.
- **Nút "Gửi cho agent":** Chèn danh sách các node đang chọn dạng `@<id>` vào ô chat Thread.
- **Phím `Delete` hoặc `Backspace`:** Xóa cạnh liên kết đang chọn.
