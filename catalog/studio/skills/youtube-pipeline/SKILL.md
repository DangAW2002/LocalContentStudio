---
name: youtube-pipeline
description: Quy trình sản xuất video YouTube hoàn chỉnh từ đầu đến cuối trong Studio (kịch bản → voice TTS → b-roll media.import / AI gen → timeline_build → render video → check tiêu chuẩn kênh → thumbnail design → metadata tiêu đề, mô tả, thẻ tags). Dùng khi tạo nội dung video YouTube chuẩn phong cách cá nhân/thương hiệu.
---

# Quy trình sản xuất video YouTube đầu-cuối (Studio Playbook)

Playbook điều phối toàn bộ quy trình làm video YouTube chuẩn chất lượng cao, tích hợp chặt chẽ với Brand & Channel specs của Studio.

## Nguyên tắc vận hành

- **File là nguồn sự thật:** Mọi kịch bản (`text.md`), timeline (`timeline.json`), thumbnail (`index.html`) và metadata được lưu trong `projects/<slug>/`.
- **Chỉ sử dụng Studio MCP tools có sẵn:**
  - `brand_get`, `channel_get` để lấy thông tin nhận diện thương hiệu và tiêu chuẩn kênh YouTube (`channels/youtube.yaml`).
  - `capability_run` để chạy các tác vụ: `tts.synthesize@1` (giọng đọc), `media.import@1` (tải b-roll / video tư liệu / file cục bộ), `image.generate@1` (sinh ảnh nền/minh họa), `video.generate@1` (sinh b-roll video ngắn), `media.inspect@1` (kiểm tra chi tiết media, thời lượng, loudness LUFS, khung hình mẫu).
  - **Dùng `media.inspect` thay vì Bash ffprobe:** Khi cần kiểm tra thông số kỹ thuật của clip video hoặc audio (thời lượng, kích thước, fps, loudness ebur128, khung hình mẫu), luôn gọi capability `media.inspect@1` qua `capability_run` với tham số `{ "path": "<đường dẫn tương đối>" }`, tuyệt đối không gọi Bash chạy ffprobe ngoài terminal.
  - `artifact_list`, `artifact_get` để xem artifact; tạo/sửa artifact `timeline` bằng công cụ `timeline_build`; tạo/sửa `design`, `text` bằng cách ghi trực tiếp
    `projects/<slug>/artifacts/<id>/artifact.yaml` + tệp nguồn (`index.html`, `text.md`).
  - `timeline_build` để tự động tính toán thời gian, căn khớp voice/b-roll, chia phụ đề và dựng `timeline.json`.
  - `artifact_render` để xuất video MP4 hoặc ảnh PNG thumbnail.
  - `check_run` (sau khi render) để kiểm tra tuân thủ kích thước, thời lượng, safe-zone, bảng màu và từ cấm.
- **Khai báo liên kết đồ thị (`inputs`) và phân tích ảnh hưởng (`graph_impact`):**
  - Mọi artifact phụ thuộc phải khai báo trường `inputs` trong `artifact.yaml` để Studio theo dõi đồ thị nội dung và tính toán lỗi thời:
    - Artifact `timeline` dùng `inputs` trỏ tới kịch bản (`artifact:<script-id>`), giọng đọc voice (`artifact:<voice-id>`) và các clip/media tư liệu (`asset:<path>` hoặc `artifact:<clip-id>`).
    - Artifact `design` thumbnail dùng `inputs` trỏ tới kịch bản (`artifact:<script-id>`) hoặc nhân vật tham chiếu (`{ ref: "artifact:<nv-id>", role: "reference" }`).
  - Trước khi sửa đổi một artifact nguồn (như kịch bản hay giọng đọc), luôn gọi MCP tool `graph_impact(ref: "...")` để biết danh sách các sản phẩm phía dưới bị ảnh hưởng theo thứ tự topo và ước tính chi phí render/sinh lại.
- **Tự động & an toàn:** Luôn tôn trọng checkpoint CAS tự động của Studio trước mỗi lượt; không tự tiện ghi đè tệp ngoài workspace.
- **Ghi/sửa tệp bằng tool Write/Edit, đọc bằng Read** (`artifact.yaml`, `index.html`, `text.md`, `metadata.yaml`, `timeline.json`):
  không dùng Bash (`cat > … <<EOF`, `sed -i`, `python -c`) — mỗi lệnh Bash như vậy bắt người dùng duyệt quyền.
  Bash chỉ dùng khi thật sự cần (ví dụ `cp` ảnh vào thư mục artifact).
- **Tạo lại bản nháp đăng:** gọi lại `publish_prepare` cho cùng artifact/kênh sẽ cập nhật bản nháp đang mở, không tạo bản mới.

## 7 giai đoạn thực hiện

### 1. Kịch bản & Cấu trúc (Scripting)
- Đọc thông tin Brand (`brand_get`) và quy chuẩn kênh YouTube (`channel_get`).
- Soạn kịch bản chi tiết theo cấu trúc video YouTube chuẩn:
  - **Hook (0-15s):** Nêu bật vấn đề, thu hút sự chú ý ngay lập tức.
  - **Giới thiệu & Lợi ích (15-45s):** Giải thích người xem sẽ nhận được gì.
  - **Thân bài (Nội dung chính):** Chia thành các phần (chapters) rõ ràng.
  - **Call to Action (CTA) & Outro:** Kêu gọi đăng ký kênh, gợi ý video tiếp theo.
- Lưu kịch bản vào artifact loại `text` hoặc tệp `projects/<slug>/script.md`.

### 2. Giọng đọc (Voice & TTS)
- Tách từng đoạn lời thoại theo nhịp câu phù hợp.
- Tạo giọng đọc qua capability `tts.synthesize@1`:
  - Gọi `capability_run` với công cụ `tts.windows-sapi` (hoặc provider khả dụng).
  - Đảm bảo đầu ra âm thanh định dạng `.wav` hoặc `.mp3` được ghi nhận trong thư mục asset của lượt/dự án.
  - Ghi nhận thời lượng chính xác của từng file âm thanh qua `duration_seconds` trả về sẵn (hoặc kiểm tra bằng `media.inspect@1`) để căn khớp timeline.

### 3. Tư liệu hình ảnh & B-Roll (Media Gathering & Gen)
- Thu thập hoặc sinh tư liệu hình ảnh/video bổ trợ (B-roll):
  - **Tải từ web hoặc nhập tệp sẵn có:** Dùng `media.import@1` (`url` hoặc `path`) để tải video/audio tư liệu vào dự án.
  - **Sinh ảnh minh hoạ AI:** Dùng `image.generate@1` (`google.imagen` hoặc `openai.images`) với tỷ lệ 16:9 (`width: 1280, height: 720` hoặc `1920x1080`).
  - **Sinh video AI:** Dùng `video.generate@1` (`google.veo` hoặc `openai.videos`) cho các phân cảnh chuyển động ấn tượng.
  - **Kiểm tra tệp tư liệu:** Dùng `media.inspect@1` để xác thực độ phân giải, fps và âm thanh của b-roll đã tải.

### 4. Dựng dòng thời gian (Timeline Assembly)
- Dùng công cụ Studio MCP `timeline_build` để tự động tạo artifact timeline (`artifact.yaml` + `timeline.json`). Không cần viết Python qua Bash hay tự tính toán thời lượng thủ công:
  - `timeline_build` tự động probe thời lượng từng tệp âm thanh (WAV/MP3), sắp xếp các phân cảnh nối tiếp nhau với các khoảng đệm mượt mà (`lead_s`, `gap_s`, `tail_s`).
  - Tự động chia lời thoại kịch bản thành phụ đề cân đối theo câu và độ dài (tối đa 2 dòng ~42 ký tự mỗi dòng), gán mốc thời gian chính xác khớp với giọng đọc.
  - Hỗ trợ tùy biến định dạng phụ đề qua `caption_style` (hoặc trường `style` trên track `captions` trong `timeline.json`): `font` (tên font chữ), `font_size` (cỡ chữ px, mặc định ~4.5% chiều cao khung hình), `color` (màu chữ #RRGGBB), `outline` (độ dày viền px), `box` (bật/tắt hộp nền), `box_color` (màu hộp nền, vd `#000000` hoặc `black@0.6`), `margin_bottom` (khoảng cách mép đáy px, mặc định ~8% chiều cao), `max_width_ratio` (tỷ lệ rộng tối đa 0-1). Cả khối phụ đề nhiều dòng được giữ trọn vẹn trong khung hình (`y=h-text_h-margin`).
  - Hỗ trợ lớp phủ (overlays) hiển thị xuyên suốt video (ví dụ logo brand định dạng PNG) hoặc theo khoảng thời gian tùy chọn.
  - Tự động thiết lập kích thước và khung hình theo chuẩn của kênh YouTube (1920x1080@30fps).
- **Lưu ý định dạng SVG:** FFmpeg không giải mã trực tiếp SVG trong timeline. Hãy render tệp SVG logo sang PNG qua design artifact trước khi truyền vào `overlays` của `timeline_build`.
- **Ví dụ gọi công cụ `timeline_build` (kèm `caption_style` và overlay logo):**

  ```json
  {
    "project": "ca-phe-phin",
    "artifact": "video",
    "segments": [
      {
        "audio": "projects/ca-phe-phin/assets/vo/vo-01.wav",
        "text": "Vì sao một ly cà phê phin lại cần nhỏ giọt thật chậm?",
        "visual": "projects/ca-phe-phin/renders/scene-01/output.png"
      },
      {
        "audio": "projects/ca-phe-phin/assets/vo/vo-02.wav",
        "text": "Bí quyết nằm ở nhiệt độ nước và độ nén vừa phải của phin nhôm.",
        "visual": "projects/ca-phe-phin/renders/scene-02/output.png"
      }
    ],
    "caption_style": {
      "font": "segoeui",
      "font_size": 48,
      "color": "#ffffff",
      "outline": 0,
      "box": true,
      "box_color": "black@0.6",
      "margin_bottom": 86,
      "max_width_ratio": 0.8
    },
    "overlays": [
      {
        "source": "projects/ca-phe-phin/renders/logo-badge/output.png",
        "x": 60,
        "y": 60,
        "width": 180,
        "opacity": 0.9
      }
    ],
    "brand": "demo",
    "channels": ["youtube"],
    "lead_s": 0.5,
    "gap_s": 0.4,
    "tail_s": 1.5,
    "transition": "fade"
  }
  ```
- Kết quả trả về gồm: `{ "artifact": "video", "duration_s": 65.2, "segments": [{"start": 0.0, "duration": 8.5}, ...], "captions_count": 8 }`. Không cần can thiệp tay vào `timeline.json`. Sau bước này, chuyển ngay sang bước 5 để kết xuất video.

### 5. Kết xuất & Kiểm tra chất lượng (Render & Validation)
- Thực hiện kết xuất video qua `artifact_render` cho artifact timeline:
  - FFmpeg ghép nối video, audio và phụ đề ra định dạng MP4 H.264/AAC.
  - Âm thanh tự chuẩn hoá theo `loudness_lufs` của kênh trong `artifact.yaml` (youtube: −14 LUFS) — không cần tự chỉnh
    `volume` hay đo bằng Bash. Muốn mức khác thì ghi `"loudness_lufs": -16` ở gốc `timeline.json`.
- Chạy kiểm tra quy chuẩn kênh bằng `check_run`:
  - Đảm bảo video đạt chuẩn 16:9, thời lượng không vượt giới hạn kênh, logo brand hiện diện đầy đủ trên overlay track, không vi phạm từ cấm trong giọng đọc.
  - Dùng `media.inspect@1` để xem lại các khung hình mẫu (10%, 50%, 90%) và loudness tích hợp sau khi render nếu muốn tự đánh giá trước khi đăng.
  - Nếu phát hiện lỗi (finding), tự động điều chỉnh lại `timeline.json` hoặc audio/text và render lại.

### 6. Thiết kế ảnh bìa (Thumbnail Design)
- Tạo artifact `design` (`index.html`) cho thumbnail kích thước 1280x720:
  - Áp dụng màu sắc chủ đạo từ Brand (`brand_get`).
  - Đặt chữ tiêu đề to, tương phản cao, tối đa 4-6 từ dễ đọc trên thiết bị di động.
  - Sử dụng các lớp `data-layer` (bao gồm `data-layer="logo"` nếu muốn gắn logo badge) để người dùng có thể tinh chỉnh trực tiếp trên Stage.
- Render thumbnail ra ảnh PNG qua `artifact_render`.
- Chạy `check_run` kiểm tra thumbnail đạt chuẩn kênh (tỷ lệ 16:9, dung lượng < 2MB, safe zones tránh che khuất bởi timestamp ở góc dưới bên phải).

### 7. Xuất bản & Metadata (Publishing Preparation)
- Chuẩn bị gói thông tin metadata đăng tải theo đúng giới hạn của YouTube:
  - **Tiêu đề (Title):** Hấp dẫn, kích thích tò mò, độ dài tối đa 100 ký tự (khuyến nghị 50-70 ký tự để không bị cắt bớt).
  - **Mô tả (Description):** Tóm tắt nội dung, mốc thời gian (timestamps/chapters), liên kết hữu ích, hashtags.
  - **Thẻ từ khoá (Tags):** Các từ khoá cốt lõi liên quan đến chủ đề video.
- Lưu lại metadata vào `projects/<slug>/metadata.yaml` để người dùng sẵn sàng tải lên hoặc sử dụng publisher tool.
