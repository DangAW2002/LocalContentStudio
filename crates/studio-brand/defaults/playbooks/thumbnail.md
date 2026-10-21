---
name: "Thumbnail YouTube Chuyên Nghiệp"
description: "Quy trình thiết kế ảnh thu nhỏ (thumbnail) 16:9 thu hút tỷ lệ nhấp (CTR) cao"
kind: playbook
artifacts:
  - "thumbnail"
steps:
  - id: "concept_ideation"
    name: "Lên ý tưởng bố cục"
    capabilities: ["llm.generate@1"]
    description: "Chọn ảnh chủ thể rõ nét, văn bản tối đa 3-5 từ gây tò mò."
  - id: "render_graphic"
    name: "Render ảnh bìa"
    capabilities: ["render.html_png@1"]
    description: "Render HTML/CSS ra file PNG 1280×720 (chuẩn thumbnail YouTube, dưới 2 MB)."
checkers:
  - "dimensions"
  - "palette"
  - "contrast"
  - "logo_presence"
ask_user_at:
  - "Chọn phương án thumbnail ưng ý nhất"
recipes:
  - "thumb-render"
---

# Hướng dẫn chi tiết thiết kế Thumbnail

1. **Kích thước**: 1280×720 (16:9) — đặt `width: 1280`, `height: 720` trong `artifact.yaml`; tệp PNG dưới 2 MB.
2. **Quy tắc logo**: Đặt logo brand ở góc trên trái, kích thước tối thiểu 48px, không che mặt nhân vật chính.
3. **Màu sắc**: Sử dụng màu nhấn (accent) từ bảng màu brand để làm nổi bật từ khóa chính.
