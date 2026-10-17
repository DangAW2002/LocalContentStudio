---
name: promo-pipeline
description: Quy trình 8 giai đoạn làm video giới thiệu/ra mắt sản phẩm chất lượng điện ảnh bằng code (hiểu sản phẩm → hướng hình ảnh → ánh xạ tính năng sang cảnh → storyboard → tư liệu → dựng từng cảnh → âm thanh → nghiệm thu độc lập). Dùng khi người dùng muốn làm promo, launch video, giới thiệu app/website.
---

# Quy trình video giới thiệu sản phẩm (Studio)

Dựa trên pipeline của video-shotcraft (Apache-2.0); tài liệu gốc tiếng Trung ở `references/`. Phần dưới là cách áp
vào Studio — khi gốc và Studio khác nhau, **theo Studio**.

## Chế độ

- **Tự sáng tác (mặc định trong Studio):** agent tự quyết hướng hình ảnh, ánh xạ cảnh, storyboard, tư liệu, âm thanh và
  làm liền một mạch tới thành phẩm; ghi các quyết định chính vào `projects/<slug>/notes.md`. Chỉ hỏi khi thiếu đầu vào
  không thể suy ra an toàn. Yêu cầu rõ ràng của người dùng luôn là ràng buộc.
- **Đồng sáng tác:** chỉ khi người dùng muốn tham gia. Đọc `references/guided-free-creation.md`; dừng xác nhận ở: tóm tắt
  sản phẩm, quyết định yêu cầu, hướng hình ảnh, ánh xạ cảnh, storyboard cuối (mỗi lượt hỏi 1–3 câu). Người dùng nói
  "tự quyết đi" → chuyển sang tự sáng tác và ghi lại.
- Người dùng nêu tên thẻ cảnh (skill `shot-recipes`) → đó là ràng buộc cảnh, không tự động là đồng sáng tác.

## 8 giai đoạn (chi tiết: `references/pipeline.md`)

0. **Hiểu sản phẩm và ràng buộc:** định vị, tính năng chính, trạng thái trình diễn được, rủi ro dữ liệu. Đọc Brand
   (`brand_get`) và kênh đích (`channel_get`: kích thước, thời lượng tối đa, khung an toàn).
1. **Hướng hình ảnh và khung mẫu:** ngôn ngữ hình ảnh **mọc từ sản phẩm**. Ưu tiên token của Studio Brand; nếu có
   website/app thật thì trích thêm font, cỡ chữ, lưới, khoảng cách, bo góc, màu từ design system/computed styles. Làm
   1–2 khung mẫu (artifact `design`, render PNG) trước khi dựng chuyển động.
2. **Ánh xạ tính năng → cảnh:** chọn thẻ từ `shot-recipes/INDEX.md` theo đường cong năng lượng
   (`references/promo-energy-arc.md`). Ánh xạ chỉ là chọn ngữ pháp chuyển động, chưa phải storyboard.
3. **Storyboard:** thứ tự cảnh, thời lượng (theo khung), hình cụ thể, trạng thái trang, nguồn tư liệu, phụ đề, chuyển
   cảnh, SFX. Dành sẵn khung cho các nhịp "thở".
4. **Tư liệu cuối:** trang thật phải dùng **ảnh chụp thật** (trình duyệt headless, 2x, cắt theo phần tử, bảng toạ độ).
   Dữ liệu khách hàng/cá nhân/nội bộ/khoá/realtime → thay bằng dữ liệu hư cấu, khoá lại trước khi chụp. Ảnh/video AI
   (`capability_run` image/video.generate) chỉ cho phần không phải giao diện thật.
5. **Dựng từng cảnh:** mỗi cảnh = thẻ + demo tham chiếu, chuyển sang HyperFrames (artifact `motion`,
   `projects/<slug>/artifacts/<id>/`). Render khung tĩnh để tự xem sau mỗi cảnh; tất định tuyệt đối.
6. **Âm thanh:** skill `motion-aesthetics` (`sound-design.md`, `music-beat-sync.md`); giọng đọc qua `tts.synthesize@1`.
   Ghép bằng artifact `timeline` (FFmpeg) nếu cần nhiều nguồn.
7. **Nghiệm thu:** render toàn bộ → trích khung → `check_run` (spec kênh, brand) → tự kiểm theo `aesthetic-rules.md`.
   Sau đó **kiểm tra độc lập**: nếu engine hỗ trợ subagent, giao cho một subagent ngữ cảnh sạch với đầu vào: thành
   phẩm, khung trích, tóm tắt sản phẩm, quyết định, hướng hình ảnh, ánh xạ, thẻ đã chọn, storyboard, quy tắc thẩm mỹ;
   theo `references/final-review.md`, báo cáo từng mục có số khung làm bằng chứng. Không có subagent → tự làm đúng
   checklist đó và nói rõ là tự kiểm. Người làm luôn thiên vị sản phẩm của mình: không giao bản chưa kiểm.

## Khác với bản gốc

- Không cài skill bằng `npx skills add`, không symlink vào thư mục skill toàn cục.
- Không mở "Motion Workbench", không xuất JianYing: chỉnh sửa sau giao làm trên Stage của Studio.
- Không nhắc người dùng @ tác giả hay gửi showcase; không đăng mạng xã hội (Studio có publisher riêng, luôn cần xác nhận).
- Mọi thao tác tệp nằm trong workspace; checkpoint của Studio tự tạo trước mỗi lượt.
