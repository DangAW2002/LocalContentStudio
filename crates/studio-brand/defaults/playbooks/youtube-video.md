---
name: "YouTube Video Chuẩn"
description: "Quy trình sản xuất trọn gói một video YouTube ngang dài kèm thumbnail và metadata"
kind: playbook
artifacts:
  - "script"
  - "video"
  - "thumbnail"
  - "metadata"
steps:
  - id: "research_and_script"
    name: "Nghiên cứu & Kịch bản"
    capabilities: ["llm.generate@1"]
    description: "Soạn đề cương, móc câu (hook), nội dung chính và lời kêu gọi hành động."
  - id: "voice_synthesis"
    name: "Thu âm giọng đọc"
    capabilities: ["tts.synthesize@1"]
    description: "Chuyển kịch bản thành giọng đọc truyền cảm ứng với voice guide của brand."
  - id: "transcribe_subtitles"
    name: "Tạo phụ đề"
    capabilities: ["stt.transcribe@1"]
    description: "Trích xuất phụ đề tự động theo file âm thanh voice."
  - id: "assemble_timeline"
    name: "Dựng timeline video"
    capabilities: ["render.timeline_video@1"]
    description: "Ghép voice, b-roll, nhạc nền và các đoạn lower-third vào timeline."
  - id: "design_thumbnail"
    name: "Thiết kế Thumbnail"
    capabilities: ["render.html_png@1"]
    description: "Tạo thumbnail 1920×1080 bắt mắt, đúng font và màu sắc của brand."
  - id: "generate_metadata"
    name: "Viết tiêu đề & mô tả"
    capabilities: ["llm.generate@1"]
    description: "Tối ưu SEO tiêu đề, mô tả và chapter cho YouTube."
checkers:
  - "dimensions"
  - "palette"
  - "contrast"
  - "logo_presence"
  - "text_limits"
  - "voice_words"
ask_user_at:
  - "Duyệt kịch bản trước khi thu âm voice"
  - "Xác nhận thumbnail và tiêu đề cuối cùng trước khi đăng"
recipes:
  - "youtube-longform"
---

# Hướng dẫn chi tiết quy trình sản xuất YouTube Video

1. **Khởi tạo kịch bản**: Đảm bảo 30 giây đầu tiên có hook lôi cuốn người xem.
2. **Kiểm tra từ ngữ**: Chạy checker `voice_words` để loại bỏ các từ cấm kỵ hoặc sáo rỗng.
3. **Thu âm**: Dùng năng lực `tts.synthesize@1` tạo file âm thanh chất lượng cao.
4. **Phụ đề**: Chạy `stt.transcribe@1` để xuất file `.srt`.
5. **Dựng video**: Đồng bộ hình ảnh và chèn intro/outro từ tài nguyên brand.
6. **Thumbnail & Check**: Dùng HTML/CSS render ra ảnh PNG, kiểm tra tương phản và logo.
