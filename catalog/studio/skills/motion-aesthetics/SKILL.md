---
name: motion-aesthetics
description: Quy tắc thẩm mỹ chuyển động dạng án lệ (nhịp, chất cảm/máy quay/bố cục, âm thanh, chữ, quy trình), thiết kế âm thanh với SFX và cắt theo phách nhạc. Dùng khi dựng hoặc tự kiểm một video motion/promo trước khi giao.
---

# Thẩm mỹ chuyển động (Studio)

Tài liệu gốc từ video-shotcraft (Apache-2.0) nằm ở `references/` (tiếng Trung):

| Tệp | Khi nào đọc |
| --- | --- |
| `references/aesthetic-rules.md` | Khi lên timeline và **bắt buộc** khi tự kiểm trước khi giao |
| `references/sound-design.md` | Khi thêm SFX/nhạc nền |
| `references/music-beat-sync.md` | Khi người dùng đã chọn nhạc có nhịp mạnh — đọc **trước** khi lên storyboard |

## Cách đọc quy tắc

Mỗi quy tắc = một câu làm được ngay + án lệ (lời người dùng thật khi bắt làm lại) + câu hỏi tự kiểm. Mã nhóm:
**R** nhịp, **Q** chất cảm/máy quay/bố cục, **S** âm thanh, **C** chữ, **P** quy trình. Được cố ý vi phạm khi phong cách
cần, nhưng phải ghi lý do vào ghi chú của artifact.

## Điều cốt lõi (tóm tắt)

- **Nhịp (R):** thông tin quan trọng hiện xong phải đứng yên; chữ thương hiệu ≥ 1 s; mở đầu cho chủ thể ≥ 3 s. Tốc độ đến
  từ **gia tốc** chứ không phải chạy đều nhanh; nhóm phần tử gắn một ẩn dụ vật lý (chia bài) và kết thúc nghỉ 0,5 s.
  Phản hồi lịch sử chỉ một chiều: "chậm lại, dừng lâu hơn" — chưa từng có "quá chậm".
- **Chất cảm (Q):** điện ảnh đến từ máy quay, ánh sáng, nhịp và âm thanh phối hợp — không từ hiệu ứng khoe kỹ thuật.
  Một nhân vật chính với cung động tác trọn vẹn (soi → đẩy gần → lơ lửng → về chỗ). Glow/glint rải hàng loạt = rẻ tiền;
  một điểm sáng chất lượng thì được.
- **Âm thanh (S):** câu âm thanh riser → impact → sparkle; có hành động trên hình (bấm, bật) thì có tiếng; âm "bíp" tổng
  hợp phải nghe thử từng cái.
- **Phách nhạc:** nhạc nhịp mạnh thì mọi chuyển cảnh và động tác rơi đúng phách; phân tích BPM/pha trước, viết timeline
  theo phách, render xong đo lại độ lệch điểm cắt ≤ 3 khung. Có nhạc nền thì giao 2 bản: có nhạc và không nhạc (giữ SFX).

## Âm thanh trong Studio

Không có sẵn thư viện SFX của upstream (một số tệp chưa rõ nguồn nên không chép). Lấy âm thanh qua Studio Library
(`asset_list`) hoặc capability (`capability_search` → `audio.sfx.generate@1`, `music.generate@1`, `tts.synthesize@1`).
Ghi nguồn/giấy phép của từng tệp âm thanh vào ghi chú artifact.

## Tự kiểm trước khi giao

Trích khung bằng FFmpeg ở các mốc quan trọng, rồi đi qua `aesthetic-rules.md` từng mục, ghi `mã ✓` hoặc
`mã ✗ (cảnh/khung)`. Kết hợp `check_run` của Studio (kích thước, thời lượng, khung an toàn, màu brand). Sửa hết ✗ rồi mới báo xong.
