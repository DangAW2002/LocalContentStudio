---
name: "Carousel Bài Đăng Nhiều Ảnh"
description: "Quy trình thiết kế bộ slide carousel 4:5 hoặc 1:1 cho Instagram và LinkedIn"
kind: playbook
artifacts:
  - "slides"
  - "copy"
steps:
  - id: "structure_outline"
    name: "Lập dàn ý các slide"
    capabilities: ["llm.generate@1"]
    description: "Slide 1 tiêu đề kích thích, slide 2-6 chia sẻ giá trị, slide cuối CTA."
  - id: "design_html_slides"
    name: "Dựng slide HTML/CSS"
    capabilities: ["render.html_png@1"]
    description: "Áp dụng CSS variables từ bảng màu và font của brand."
  - id: "write_caption"
    name: "Soạn bài viết kèm theo"
    capabilities: ["llm.generate@1"]
    description: "Viết bài dẫn dắt kèm danh sách hashtag chọn lọc."
checkers:
  - "dimensions"
  - "palette"
  - "contrast"
  - "logo_presence"
  - "text_limits"
ask_user_at:
  - "Duyệt nội dung các trang slide trước khi xuất ảnh"
recipes:
  - "carousel-export"
---

# Hướng dẫn chi tiết thiết kế Carousel

1. **Chuẩn kích thước**: Sử dụng tỷ lệ 4:5 (1080×1350) cho Instagram hoặc 1:1 cho bài đăng tổng hợp.
2. **Thương hiệu**: Giữ nhất quán màu nền và font chữ xuyên suốt các slide.
3. **Độ tương phản**: Đảm bảo tỷ lệ WCAG AA ≥ 4.5:1 để độc giả dễ đọc trên thiết bị di động.
