---
name: youtube-pipeline
description: Quy trình sản xuất video YouTube hoàn chỉnh từ đầu đến cuối trong Studio (kịch bản → voice TTS → b-roll media.import / AI gen → timeline → render video → check tiêu chuẩn kênh → thumbnail design → metadata tiêu đề, mô tả, thẻ tags). Dùng khi tạo nội dung video YouTube chuẩn phong cách cá nhân/thương hiệu.
---

# Quy trình sản xuất video YouTube đầu-cuối (Studio Playbook)

Playbook điều phối toàn bộ quy trình làm video YouTube chuẩn chất lượng cao, tích hợp chặt chẽ với Brand & Channel specs của Studio.

## Nguyên tắc vận hành

- **File là nguồn sự thật:** Mọi kịch bản (`text.md`), timeline (`timeline.json`), thumbnail (`index.html`) và metadata được lưu trong `projects/<slug>/`.
- **Chỉ sử dụng Studio MCP tools có sẵn:**
  - `brand_get`, `channel_get` để lấy thông tin nhận diện thương hiệu và tiêu chuẩn kênh YouTube (`channels/youtube.yaml`).
  - `capability_run` để chạy các tác vụ: `tts.synthesize@1` (giọng đọc), `media.import@1` (tải b-roll / video tư liệu / file cục bộ), `image.generate@1` (sinh ảnh nền/minh họa), `video.generate@1` (sinh b-roll video ngắn).
  - `artifact_list`, `artifact_get` để xem artifact; tạo/sửa artifact `timeline`, `design`, `text` bằng cách ghi trực tiếp
    `projects/<slug>/artifacts/<id>/artifact.yaml` + tệp nguồn (`timeline.json`, `index.html`, `text.md`).
  - `artifact_render` để xuất video MP4 hoặc ảnh PNG thumbnail.
  - `check_run` (sau khi render) để kiểm tra tuân thủ kích thước, thời lượng, safe-zone, bảng màu và từ cấm.
- **Tự động & an toàn:** Luôn tôn trọng checkpoint CAS tự động của Studio trước mỗi lượt; không tự tiện ghi đè tệp ngoài workspace.

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
  - Ghi nhận thời lượng chính xác của từng file âm thanh để căn khớp timeline.

### 3. Tư liệu hình ảnh & B-Roll (Media Gathering & Gen)
- Thu thập hoặc sinh tư liệu hình ảnh/video bổ trợ (B-roll):
  - **Tải từ web hoặc nhập tệp sẵn có:** Dùng `media.import@1` (`url` hoặc `path`) để tải video/audio tư liệu vào dự án.
  - **Sinh ảnh minh hoạ AI:** Dùng `image.generate@1` (`google.imagen` hoặc `openai.images`) với tỷ lệ 16:9 (`width: 1280, height: 720` hoặc `1920x1080`).
  - **Sinh video AI:** Dùng `video.generate@1` (`google.veo` hoặc `openai.videos`) cho các phân cảnh chuyển động ấn tượng.

### 4. Dựng dòng thời gian (Timeline Assembly)
- Khởi tạo hoặc cập nhật artifact loại `timeline` (`timeline.json`) (ghi `artifact.yaml` + `timeline.json` trong `projects/<slug>/artifacts/<id>/`):
  - `width`: 1920, `height`: 1080 (hoặc 1280x720), `fps`: 30.0.
  - **Track video:** Danh sách clips hình ảnh và video B-roll, gán đúng `start` và `duration`.
  - **Track audio:** Ghép các đoạn voiceover từ bước 2 và nhạc nền nhẹ nhàng (nếu có).
  - **Track caption (phụ đề):** Đặt lời thoại khớp với âm thanh hiển thị theo mốc thời gian.
- Kiểm tra tính hợp lệ của cấu trúc `timeline.json`. Mẫu tối thiểu hợp lệ (đường dẫn `source` tính từ thư mục
  artifact `projects/<slug>/artifacts/<id>/`; mỗi track/clip bắt buộc có `id`, mỗi clip bắt buộc có `source` + `duration`):

  ```json
  {
    "width": 1920, "height": 1080, "fps": 30,
    "tracks": [
      { "id": "video", "kind": "video", "clips": [
        { "id": "s1", "source": "../../renders/scene-01/<hash>/output.png", "start": 0, "duration": 8.2, "transition": "fade", "transition_duration": 0.5 },
        { "id": "s2", "source": "color:#1E88E5", "start": 8.2, "duration": 3 }
      ]},
      { "id": "voice", "kind": "audio", "clips": [
        { "id": "vo1", "source": "../../assets/vo/vo-01.wav", "start": 0, "duration": 8.2 }
      ]},
      { "id": "subs", "kind": "captions", "clips": [
        { "id": "t1", "source": "Vì sao một ly cà phê nhỏ giọt chậm…", "start": 0, "duration": 8.2 }
      ]}
    ]
  }
  ```
- Thời lượng giọng đọc: `tts.synthesize@1` trả `duration_seconds` cho tệp WAV — dùng số này, không cần chạy ffprobe.

### 5. Kết xuất & Kiểm tra chất lượng (Render & Validation)
- Thực hiện kết xuất video qua `artifact_render` cho artifact timeline:
  - FFmpeg ghép nối video, audio và phụ đề ra định dạng MP4 H.264/AAC.
- Chạy kiểm tra quy chuẩn kênh bằng `check_run`:
  - Đảm bảo video đạt chuẩn 16:9, thời lượng không vượt giới hạn kênh, không vi phạm từ cấm trong giọng đọc.
  - Nếu phát hiện lỗi (finding), tự động điều chỉnh lại `timeline.json` hoặc audio/text và render lại.

### 6. Thiết kế ảnh bìa (Thumbnail Design)
- Tạo artifact `design` (`index.html`) cho thumbnail kích thước 1280x720:
  - Áp dụng màu sắc chủ đạo từ Brand (`brand_get`).
  - Đặt chữ tiêu đề to, tương phản cao, tối đa 4-6 từ dễ đọc trên thiết bị di động.
  - Sử dụng các lớp `data-layer` để người dùng có thể tinh chỉnh trực tiếp trên Stage.
- Render thumbnail ra ảnh PNG qua `artifact_render`.
- Chạy `check_run` kiểm tra thumbnail đạt chuẩn kênh (tỷ lệ 16:9, dung lượng < 2MB, safe zones tránh che khuất bởi timestamp ở góc dưới bên phải).

### 7. Xuất bản & Metadata (Publishing Preparation)
- Chuẩn bị gói thông tin metadata đăng tải theo đúng giới hạn của YouTube:
  - **Tiêu đề (Title):** Hấp dẫn, kích thích tò mò, độ dài tối đa 100 ký tự (khuyến nghị 50-70 ký tự để không bị cắt bớt).
  - **Mô tả (Description):** Tóm tắt nội dung, mốc thời gian (timestamps/chapters), liên kết hữu ích, hashtags.
  - **Thẻ từ khoá (Tags):** Các từ khoá cốt lõi liên quan đến chủ đề video.
- Lưu lại metadata vào `projects/<slug>/metadata.yaml` để người dùng sẵn sàng tải lên hoặc sử dụng publisher tool.
