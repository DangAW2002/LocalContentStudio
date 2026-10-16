---
name: "Shorts & Reels Video Ngắn"
description: "Quy trình làm video dọc ngắn dưới 60 giây cho YouTube Shorts, TikTok và Reels"
kind: playbook
artifacts:
  - "script"
  - "video"
  - "captions"
steps:
  - id: "write_hook_script"
    name: "Viết kịch bản cô đọng"
    capabilities: ["llm.generate@1"]
    description: "Tối đa 130-150 từ, đảm bảo thời lượng dưới 60 giây."
  - id: "generate_voice"
    name: "Tạo voice AI"
    capabilities: ["tts.synthesize@1"]
    description: "Chọn giọng đọc nhịp điệu nhanh, rõ ràng."
  - id: "transcribe_captions"
    name: "Sinh phụ đề từng chữ"
    capabilities: ["stt.transcribe@1"]
    description: "Tạo file phụ đề khớp nhịp nói để hiện chữ chạy trên video."
  - id: "render_motion"
    name: "Render motion video"
    capabilities: ["render.html_video@1"]
    description: "Render video dọc 1080×1920 với hiệu ứng chữ và hình ảnh động."
checkers:
  - "dimensions"
  - "duration"
  - "safe_zone"
  - "captions_required"
  - "voice_words"
ask_user_at:
  - "Duyệt kịch bản trước khi render video"
recipes:
  - "shorts-fast"
---

# Hướng dẫn chi tiết quy trình Video Ngắn

1. **Giới hạn thời lượng**: Video phải ≤ 60s đối với Shorts. Checker `duration` sẽ báo lỗi nếu vượt quá.
2. **Safe zone**: Đặt chữ và chủ thể ở giữa màn hình (tránh 20% mép dưới vì nút điều khiển và caption của nền tảng).
3. **Phụ đề bắt buộc**: Video ngắn cần phụ đề động (`captions_required` bắt buộc có track sub hoặc file .srt).
