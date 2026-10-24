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
- **Provider đúng:** Gọi capability `tts.synthesize@1` qua `capability_run` với provider khả dụng: `tts.piper` (mặc định tối ưu tốc độ CPU, giọng `vi_VN-vais1000-medium`) hoặc `tts.vieneu` (chất lượng cao, các giọng `Thanh Bình`, `Mai Anh`, `Minh Đức`, `Trúc Ly` hoặc alias `Bình`, `Mai`, `Nam`, `Nữ`). Không dùng provider legacy.
- **Quy tắc viết lời thuyết minh tự nhiên:**
  - **Độ dài câu:** Mỗi câu ngắn gọn, tối đa ≤ 20 từ để giữ hơi thở tự nhiên.
  - **Ngắt nhịp lấy hơi:** Đặt dấu phẩy `,` ở đúng chỗ ngắt nghỉ lấy hơi; mỗi câu chỉ truyền đạt một ý trọn vẹn; tránh dùng ngoặc đơn/kép `( )`, `[ ]` hay ký hiệu rườm rà.
  - **Phân chia segment:** Mỗi phân cảnh (segment) chỉ nên dài 1–2 câu tương ứng với một tệp âm thanh WAV.
  - **Thẻ ngắt chủ động:** Dùng thẻ `[ngắt x]` (ví dụ `[ngắt 0.7]` giây) trước các ý nhấn mạnh quan trọng hoặc ngay sau câu hỏi để tạo khoảng dừng thuyết phục người xem.
  - **Tốc độ nói (`speed`):** Mặc định hệ thống tự động thiết lập `speed: 0.76` cho tiếng Việt khi không truyền `speed` (kết hợp chuẩn hoá từ ngữ, ngắt dấu câu và thuật toán bù tốc độ thật bằng atempo để đạt nhịp chuẩn ≈ 3,2 tiếng/giây). Nếu muốn chỉnh tay, khuyến nghị thiết lập `speed` từ 0.75 – 0.90.
  - **Kiểm soát nhịp độ sau tổng hợp:** Đọc trực tiếp hai trường `syllables_per_second` và `effective_speed` được hệ thống tính toán và trả về sẵn trong kết quả TTS. Nhịp thuyết minh chuẩn đạt khoảng 3,0 – 3,4 tiếng/giây (mục tiêu mặc định ≈ 3,2 tiếng/giây). Chỉ khi `syllables_per_second` > 3,5 tiếng/giây mới cần giảm `speed` xuống 0,72 – 0,75 và tổng hợp lại một lần; nếu vẫn nhanh, viết lại câu ngắn hơn hoặc thêm dấu phẩy/`[ngắt x]` thay vì giảm tiếp.
- **Lưu trữ asset:** Đảm bảo đầu ra âm thanh định dạng `.wav` được lưu trữ vào thư mục asset của project (`projects/<slug>/assets/vo/`) để sẵn sàng cho bước dựng timeline.

### 3. Lập danh sách cảnh & Tư liệu (Shot List, Media & B-Roll)
- **Quy tắc nhịp hình (Shot List):** Tránh video bị cảm giác "trình chiếu slide" tĩnh:
  - **Nhịp đổi cảnh:** Thay đổi hình ảnh/video mỗi 3–5 giây để duy trì sự chú ý của người xem.
  - **Mật độ cảnh theo lời thoại:** Mỗi câu thoại ≥ 1 cảnh; với câu dài (> 6 giây), bắt buộc chia thành ≥ 2 cảnh (`visuals`).
  - **Tỷ lệ xen kẽ video B-roll:** Xen kẽ ≥ 30% tổng số cảnh là video chuyển động (dùng `video.generate@1` qua Veo/Sora hoặc `media.import@1` từ tư liệu/b-roll có sẵn) khi ngân sách cho phép.
  - **Chuyển động cho ảnh tĩnh:** Tất cả các cảnh ảnh tĩnh còn lại đều phải có hiệu ứng chuyển động Ken Burns (`motion: "auto"` hoặc chỉ định `zoom_in`, `zoom_out`, `pan_left`, `pan_right`...).
  - **Đa dạng thị giác:** Tránh lặp lại cùng một ảnh giữa các cảnh liên tiếp.
- **Thu thập & sinh tư liệu:**
  - **Tải hoặc nhập video tư liệu:** Dùng `media.import@1` (`url` hoặc `path`) để tải video B-roll vào dự án.
  - **Sinh ảnh minh hoạ AI:** Dùng `image.generate@1` (`google.imagen` hoặc `openai.images`) với tỷ lệ 16:9 (`1920x1080` hoặc `1280x720`).
  - **Sinh video AI:** Dùng `video.generate@1` (`google.veo` hoặc `openai.videos`) cho các phân cảnh chuyển động ấn tượng.
  - **Kiểm tra tệp tư liệu:** Dùng `media.inspect@1` để xác thực độ phân giải, fps và âm thanh của b-roll đã tải.

### 4. Dựng dòng thời gian (Timeline Assembly)
- Dùng công cụ Studio MCP `timeline_build` để tự động tạo artifact timeline (`artifact.yaml` + `timeline.json`). Không cần viết Python qua Bash hay tự tính toán thời lượng thủ công:
  - `timeline_build` tự động probe thời lượng từng tệp âm thanh (WAV/MP3), sắp xếp các phân cảnh nối tiếp nhau với các khoảng đệm mượt mà (`lead_s`, `gap_s`, `tail_s`).
  - **Hỗ trợ nhiều cảnh (`visuals`) trong 1 phân cảnh:** Tự động chia đều thời lượng phân cảnh cho các cảnh (hoặc theo mảng `durations` tùy chọn). Cảnh báo (`warnings`) nếu cảnh tĩnh dài > `max_still_s` (mặc định 6s) và chỉ có 1 visual.
  - **Chuyển động ảnh tĩnh (Ken Burns):** Mặc định tự động gán `motion: "auto"` (luân phiên tất định `zoom_in`, `zoom_out`, `pan_left`, `pan_right`) cho ảnh tĩnh, render mượt mà bằng FFmpeg scale 2× + zoompan ở 1920x1080@30fps. Cảnh video giữ nguyên chuyển động gốc.
  - **Chuyển cảnh thông minh:** Chuyển cảnh giữa các phân cảnh hỗ trợ `transition: "fade"` (mặc định 0.5s), `"cut"` hoặc `"none"`. Giữa các cảnh con trong cùng một segment mặc định là `cut` dứt khoát.
  - Tự động chia lời thoại kịch bản thành phụ đề cân đối theo câu và độ dài (tối đa 2 dòng ~42 ký tự mỗi dòng), gán mốc thời gian chính xác khớp với giọng đọc.
  - Hỗ trợ tùy biến định dạng phụ đề qua `caption_style`: `font`, `font_size`, `color`, `outline`, `box`, `box_color`, `margin_bottom`, `max_width_ratio`.
  - Hỗ trợ lớp phủ (overlays) hiển thị xuyên suốt video (ví dụ logo brand PNG) hoặc theo khoảng thời gian tùy chọn.
- **Lưu ý định dạng SVG:** FFmpeg không giải mã trực tiếp SVG trong timeline. Hãy render tệp SVG logo sang PNG qua design artifact trước khi truyền vào `overlays` của `timeline_build`.
- **Ví dụ gọi công cụ `timeline_build` (kèm `visuals`, `motion`, `caption_style` và overlay logo):**

  ```json
  {
    "project": "ca-phe-phin",
    "artifact": "video",
    "segments": [
      {
        "audio": "projects/ca-phe-phin/assets/vo/vo-01.wav",
        "text": "Vì sao một ly cà phê phin lại cần nhỏ giọt thật chậm?",
        "visuals": [
          {
            "source": "projects/ca-phe-phin/renders/scene-01a/output.png",
            "motion": "zoom_in",
            "motion_strength": 0.08
          },
          {
            "source": "projects/ca-phe-phin/assets/broll/phin-close-up.mp4",
            "in_point": 0.0,
            "out_point": 3.0
          }
        ]
      },
      {
        "audio": "projects/ca-phe-phin/assets/vo/vo-02.wav",
        "text": "Bí quyết nằm ở nhiệt độ nước và độ nén vừa phải của phin nhôm.",
        "visual": "projects/ca-phe-phin/renders/scene-02/output.png",
        "motion": "pan_right"
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
- Kết quả trả về gồm: `{ "artifact": "video", "duration_s": 65.2, "segments": [{"start": 0.0, "duration": 8.5}, ...], "captions_count": 8, "warnings": [] }`. Không cần can thiệp tay vào `timeline.json`. Sau bước này, chuyển ngay sang bước 5 để kết xuất video.

### 5. Kết xuất & Kiểm tra chất lượng (Render & Validation)
- Thực hiện kết xuất video qua `artifact_render` cho artifact timeline:
  - FFmpeg ghép nối video, audio và phụ đề ra định dạng MP4 H.264/AAC.
  - Âm thanh tự chuẩn hoá hai lượt (two-pass loudnorm) theo `loudness_lufs` của kênh trong `artifact.yaml` (youtube: −14 LUFS) — không cần tự chỉnh
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

### 8. Hồ sơ & series (Consistency & Series Profiles)
- **Hồ sơ tham chiếu (reference):** Các nhân vật, phong cách hình ảnh hoặc giọng đọc cố định được lưu dưới dạng artifact kind `reference` (chứa `profile.md` và các ảnh mẫu trong thư mục artifact).
- **Sinh nội dung nhất quán:** Khi gọi `capability_run` cho `image.generate@1`, `video.generate@1` hoặc `tts.synthesize@1`, truyền tham số `reference: "<id hồ sơ>"` (hoặc khai báo cạnh `{ ref: "artifact:<id>", role: "reference" }` trong `artifact.yaml`). Provider sẽ tự động tiếp nhận ảnh tham chiếu (`reference_images`), seed và voice ghim sẵn, đảm bảo hình ảnh và giọng đọc nhất quán xuyên suốt.
- **Series:** Nhóm các tập video dùng chung hồ sơ và recipe (kit + modules + playbook). Gọi MCP tool `series_get` để lấy thông tin series và danh sách hồ sơ đang áp dụng. Khi kiểm tra chất lượng qua `check_run`, bộ kiểm tra nhất quán (`consistency`) sẽ so sánh intro/outro, font tiêu đề và giọng đọc giữa các tập cùng series.
