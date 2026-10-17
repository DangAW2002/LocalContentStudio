---
name: shot-recipes
description: Thư viện 124 thẻ công thức cảnh quay điện ảnh (máy quay 2.5D/3D, chữ, giao diện xuất hiện, số liệu, chuyển cảnh, nhịp, ánh sáng, kết) kèm tham số đã tinh chỉnh và mã tham chiếu. Dùng khi dựng cảnh motion cho promo, explainer, Shorts, hoặc khi người dùng nêu tên một thẻ.
---

# Thẻ công thức cảnh quay (Studio)

Bộ thẻ sao chép từ video-shotcraft (Apache-2.0, xem `NOTICE.md` ở gốc gói). Mỗi thẻ là **một ngữ pháp chuyển động**
đã được chỉnh qua nhiều lần làm lại, không phải template hình ảnh.

## Cách dùng

1. **Chọn thẻ:** đọc `INDEX.md` (tên, tóm tắt, thời lượng, năng lượng theo 10 nhóm). Chọn theo đường cong năng lượng
   của cả video (xem skill `promo-pipeline` → `references/promo-energy-arc.md`), không chọn theo độ "ngầu".
2. **Đọc thẻ đầy đủ** `cards/<nhóm>/<tên>.md`: ý đồ, lõi chuyển động, **bảng tham số + cảm giác khi chỉnh**,
   **lỗi đã biết**, và mục "参考实现" (đường dẫn demo).
3. **Đọc mã tham chiếu** ở `demos/<nhóm>/<tên>/` — đây là nguồn sự thật về easing, tỉ lệ thời gian, thời điểm và cách né
   lỗi. Thẻ chỉ là ngữ nghĩa. Viết lại từ tên thẻ = vứt bỏ toàn bộ phần tinh chỉnh.
4. **Thích nghi, không hạ chuẩn:** được đổi nội dung, màu, chữ, thời lượng theo brand; tham số nằm trong "已知坑/命门"
   (lỗi đã biết / điểm sống còn) **không được hạ cấp**.

## Chuyển sang renderer của Studio

- Renderer chính là **HyperFrames** (HTML + GSAP, `render.html_video@1`). Demo là Remotion TSX: chuyển
  `interpolate`/`spring`/`Easing.bezier` sang timeline GSAP với cùng khung thời gian (frame ÷ fps → giây) và cùng
  đường cong bezier; `useCurrentFrame` → vị trí playhead; `Sequence from/durationInFrames` → vị trí trên timeline.
- Chỉ dùng Remotion trực tiếp khi module `render.remotion` đang bật (giấy phép Remotion cần người dùng xác nhận).
- Component dùng lại ở `lib/` (PageCam camera 2.5D, ClipCard, DigitRoll, Caption, FlatPanel, helpers rand/camera/motion):
  chép logic, không import.
- **Tất định:** cấm `Date.now()`/`Math.random()`; ngẫu nhiên giả dùng seed cố định (mulberry32, seed suy từ index).

## Hình ảnh lấy từ Brand, không lấy từ demo

Logo, chữ "video-shotcraft", màu graphite/champagne trong demo là của upstream. Luôn thay bằng Brand của Studio
(`brand_get`: màu, font, logo, giọng) và kích thước/khung an toàn của kênh (`channel_get`). Chỉ kế thừa cấu trúc
cảnh, ngữ pháp chuyển động, nhịp và tham số đã chỉnh.

## Quy tắc ngắn khi ghép cảnh

- Mỗi cảnh chỉ kể **một** chuyển động chính; một thủ pháp (bay vào/xếp chồng/lật trang) chỉ làm nhân vật chính một lần
  trong cả video.
- Thông tin quan trọng hiện xong phải **thở**: chữ thương hiệu đứng yên ≥ 1 s, nhóm chuyển động kết thúc nghỉ 0,5 s.
- Rung/va đập toàn khung (camera slam) tối đa 3 lần cả video. Chi tiết: skill `motion-aesthetics`.
