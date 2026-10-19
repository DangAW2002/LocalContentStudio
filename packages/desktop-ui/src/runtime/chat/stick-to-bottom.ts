/**
 * Ported from open-design (https://github.com/nexu-io/open-design),
 * apps/web/src/runtime/chat/stick-to-bottom.ts — Apache-2.0. Logic unchanged; comments condensed.
 * Upstream itself ports the core of `use-stick-to-bottom` (MIT) plus one assistant-ui criterion.
 *
 * "Có đang theo output mới nhất không" là một Ý ĐỊNH, không phải đại lượng suy ra từ vị trí.
 *
 * Cách cũ `pinned = distance < 80` tự khoá khi stream: ghi scrollTop về đáy → distance = 0 →
 * suy ra "vẫn theo" → lại ghi về đáy. Người dùng cuộn lên một nấc (~40px) đã bị kéo về trước
 * khi kịp đủ 80px, nên không thể cuộn lên được.
 *
 * Ở đây ý định CHỈ đổi theo thao tác người dùng. Nội dung cao/thấp đi, việc ta tự ghi scrollTop,
 * panel đổi kích thước đều không được chạm vào nó.
 *
 * Ba đại lượng, không phải một:
 *   · `following` — có nên bám đáy lúc này.
 *   · `escaped`   — người dùng đã chủ động thoát. Thiếu nó thì "cuộn lên một chút" bị kéo lại ngay.
 *   · khoảng cách tới đáy — hình học thuần, tính lại mỗi lần, không lưu.
 * Thoát chỉ cần thực sự rời đáy; quay lại phải CHỦ ĐỘNG cuộn xuống và chạm đáy.
 *
 * Phân biệt "người dùng cuộn" với "nội dung gây ra": cuộn lên := vị trí giảm VÀ scrollHeight
 * không đổi. Dịch chuyển do nội dung (trình duyệt kẹp, scroll anchoring) luôn kèm scrollHeight đổi.
 * Sau mỗi lần TỰ ghi scrollTop phải lấy mẫu lại làm mốc, nếu không hướng cuộn kế tiếp sẽ bị tính ngược.
 *
 * Không đặt `overflow-anchor: none`: chính scroll anchoring gốc giữ vị trí đọc khi nội dung phía
 * trên reflow; tiêu chí scrollHeight ở trên đảm bảo ta không đọc nhầm nó thành thao tác người dùng.
 */

export interface ScrollSample {
  scrollTop: number;
  scrollHeight: number;
  clientHeight: number;
}

export interface FollowIntent {
  /** Có nên bám output mới nhất lúc này. */
  following: boolean;
  /** Người dùng đã chủ động cuộn đi — chỉ cuộn xuống chủ động mới xoá được. */
  escaped: boolean;
}

/**
 * Nhỏ tới mức này vẫn tính là "sát đáy". Không lấy 1px: màn hình devicePixelRatio 2 làm tròn
 * scrollTop thiếu 1px so với đáy thật nên tiêu chí chặt không bao giờ đúng (assistant-ui #4141).
 */
export const AT_BOTTOM_TOLERANCE_PX = 8;

/** Dải "gần đáy" tính theo tỉ lệ viewport — panel hẹp và rộng không cùng một số pixel. */
const RESUME_BAND_RATIO = 0.12;
const RESUME_BAND_MIN_PX = 40;
const RESUME_BAND_MAX_PX = 120;

function clamp(value: number, low: number, high: number): number {
  return Math.min(high, Math.max(low, value));
}

export function resolveResumeBand(clientHeight: number): number {
  const height = Number.isFinite(clientHeight) && clientHeight > 0 ? clientHeight : 0;
  return clamp(height * RESUME_BAND_RATIO, RESUME_BAND_MIN_PX, RESUME_BAND_MAX_PX);
}

/** Còn cách đáy nội dung bao xa. Nội dung thấp hơn viewport thì là 0. */
export function distanceFromBottom({ scrollTop, scrollHeight, clientHeight }: ScrollSample): number {
  return Math.max(0, scrollHeight - scrollTop - clientHeight);
}

/** Nội dung không đầy một màn hình, hoặc đang dính đáy. */
export function isAtBottom(sample: ScrollSample): boolean {
  if (sample.scrollHeight <= sample.clientHeight) return true;
  return distanceFromBottom(sample) <= AT_BOTTOM_TOLERANCE_PX;
}

/** Đủ gần để bám lại. */
export function isNearBottom(sample: ScrollSample): boolean {
  if (sample.scrollHeight <= sample.clientHeight) return true;
  return distanceFromBottom(sample) <= resolveResumeBand(sample.clientHeight);
}

function maxScrollTopOf(sample: ScrollSample): number {
  return Math.max(0, sample.scrollHeight - sample.clientHeight);
}

/**
 * Một cử chỉ "nhìn lại" (lăn chuột lên, kéo ngón tay xuống) trên hình học này CÓ THỂ đưa người
 * dùng rời đáy không.
 *
 * Cần riêng vì khi stream nhanh, nếu cùng frame ta đã ghi scrollTop thì trình duyệt huỷ luôn lần
 * lăn đó, không phát scroll event — nên phải nhả ở wheel/touch, TRƯỚC khi có dịch chuyển.
 * Phía đó không có dịch chuyển để đo, nên phải tự loại các cử chỉ không thể rời đáy:
 *   · maxScrollTop <= 8px — không có vị trí nào tính là "không ở đáy" (đo thực tế: hội thoại ngắn
 *     tràn đúng 6px do padding; tiêu chí `scrollHeight > clientHeight` sẽ nhả nhầm ở đây).
 *   · scrollTop <= 0 — đã ở đỉnh.
 * Đây là điều kiện cần, không đủ; quyết định thật vẫn ở `nextFollowIntent`.
 */
export function upwardGestureCanEscapeBottom(sample: ScrollSample): boolean {
  if (sample.scrollTop <= 0) return false;
  return maxScrollTopOf(sample) > AT_BOTTOM_TOLERANCE_PX;
}

/**
 * Hướng bánh xe chuột trong đoạn dịch chuyển này. Đếm thay vì lưu nấc cuối: một lần vuốt touchpad
 * có thể phát hàng chục nấc giữa hai scroll event và đổi chiều giữa chừng.
 *
 * Mỗi "biên nhận" chỉ giải thích MỘT dịch chuyển: `atScrollTop` phải đúng bằng điểm bắt đầu của
 * dịch chuyển, lệch là vô hiệu. Bên gọi còn phải xoá sau khi dùng và cho hết hạn sau một frame.
 */
export interface WheelWitness {
  downwardEvents: number;
  upwardEvents: number;
  atScrollTop: number;
}

/**
 * Lần "vị trí giảm" này là compositor kẹp vị trí về giới hạn cũ của nó, không phải người dùng cuộn
 * lên. Chromium lưu riêng một bản "khung này cuộn được tới đâu" để lăn chuột không chờ main thread,
 * và bản đó có thể kẹt ở giá trị cũ: bấm "về mới nhất" → tới đáy thật; chạm bánh xe → bị ném về giới
 * hạn cũ, scrollHeight không đổi → bị đọc nhầm là cuộn lên và tắt bám đáy.
 *
 * Tiêu chí: bánh xe CHỈ hướng xuống + vị trí lại chạy lên. Người dùng cuộn lên thì hoặc có nấc lên,
 * hoặc không có bánh xe (thanh cuộn / bàn phím / chạm). Thà bỏ sót còn hơn đoán thừa — đoán thừa sẽ
 * hàn chết chế độ bám đáy.
 */
export function isCompositorSnapBack(
  previous: ScrollSample,
  next: ScrollSample,
  wheel: WheelWitness | null | undefined,
): boolean {
  if (wheel == null) return false;
  if (wheel.upwardEvents > 0) return false;
  if (wheel.downwardEvents <= 0) return false;
  if (wheel.atScrollTop !== previous.scrollTop) return false;
  if (next.scrollHeight !== previous.scrollHeight) return false;
  if (next.clientHeight !== previous.clientHeight) return false;
  return previous.scrollTop - next.scrollTop > AT_BOTTOM_TOLERANCE_PX;
}

/**
 * Sau một lần cuộn, ý định bám đáy thành gì.
 *
 * CHỈ ở đây (và vài thao tác rõ ràng: bấm "về mới nhất", gửi tin, đổi hội thoại) mới được đổi ý định.
 *
 * Bất biến bên gọi: tự cuộn thì luôn TỨC THỜI (`behavior: 'auto'`). `smooth` phát ra một loạt scroll
 * event trung gian nằm phía bên kia mốc — ở đây chúng chính là "người dùng cuộn".
 */
export function nextFollowIntent(
  current: FollowIntent,
  previous: ScrollSample,
  next: ScrollSample,
  wheel?: WheelWitness | null,
): FollowIntent {
  const layoutStable =
    next.scrollHeight === previous.scrollHeight && next.clientHeight === previous.clientHeight;
  const scrolledUp = next.scrollTop < previous.scrollTop && layoutStable;
  /*
   * Cuộn xuống KHÔNG đòi bố cục đứng yên, chỉ đòi "đáy không tiến lại gần người dùng". Khi stream,
   * nội dung cao lên mỗi frame; đòi layoutStable thì lần cuộn về đáy trùng frame nội dung lớn lên sẽ
   * bị bỏ, người dùng không sao bám lại được. Nới tới "maxScrollTop không giảm" là chứng minh được:
   * nội dung cao lên ⇒ đáy rời xa, nên "vị trí tăng và chạm đáy" nghĩa là người dùng thật sự cuộn xuống;
   * nội dung thấp đi ⇒ đáy tự tiến lại — vẫn bị chặn.
   */
  const bottomDidNotApproach = maxScrollTopOf(next) >= maxScrollTopOf(previous);
  const scrolledDown = next.scrollTop > previous.scrollTop && bottomDidNotApproach;

  let { following, escaped } = current;
  if (scrolledUp && !isAtBottom(next) && !isCompositorSnapBack(previous, next, wheel)) {
    escaped = true;
    following = false;
  }
  // Xoá escaped và bật lại following phải là CÙNG một lần người dùng cuộn xuống tới đáy.
  if (scrolledDown && isAtBottom(next)) {
    escaped = false;
    following = true;
  }
  return { following, escaped };
}
