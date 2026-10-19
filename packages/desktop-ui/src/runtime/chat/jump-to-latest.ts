/**
 * Ported from open-design (https://github.com/nexu-io/open-design),
 * apps/web/src/runtime/chat/jump-to-latest.ts — Apache-2.0. Logic unchanged; comments condensed.
 *
 * Khi nào hiện nút "Về tin mới nhất". Ngưỡng theo chiều cao viewport chứ không ghim pixel: "ở khá
 * xa phía trên" trên panel 400px và 1200px không cùng số pixel. Hiện và ẩn dùng hai ngưỡng (trễ)
 * để không nhấp nháy khi dừng gần ngưỡng.
 */

const SHOW_RATIO = 0.75;
const HIDE_RATIO = 0.5;
const MIN_SHOW_PX = 320;
const MAX_SHOW_PX = 1200;

function clamp(value: number, low: number, high: number): number {
  return Math.min(high, Math.max(low, value));
}

export interface JumpToLatestInput {
  /** scrollHeight - scrollTop - clientHeight. */
  distance: number;
  clientHeight: number;
  scrollHeight?: number;
  /** Đang hiện hay không — dùng cho độ trễ. */
  shown: boolean;
  /** Đang bám đáy thì không có "về mới nhất" — bạn đang ở mới nhất. */
  following?: boolean;
}

export function shouldShowJumpToLatest({
  distance,
  clientHeight,
  scrollHeight,
  shown,
  following,
}: JumpToLatestInput): boolean {
  if (following) return false;
  const height = Number.isFinite(clientHeight) && clientHeight > 0 ? clientHeight : 0;
  // Không cuộn được thì không có gì để về (1px chừa cho làm tròn dưới pixel).
  if (scrollHeight != null && Number.isFinite(scrollHeight) && scrollHeight <= height + 1) {
    return false;
  }
  const showAt = clamp(height * SHOW_RATIO, MIN_SHOW_PX, MAX_SHOW_PX);
  if (!shown) return distance > showAt;
  const hideAt = clamp(height * HIDE_RATIO, MIN_SHOW_PX / 2, MAX_SHOW_PX);
  return distance > hideAt;
}
