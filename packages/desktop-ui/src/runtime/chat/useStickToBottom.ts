/**
 * DOM wiring for the stick-to-bottom intent machine, extracted from open-design's ChatPane
 * (https://github.com/nexu-io/open-design, apps/web/src/components/ChatPane.tsx — Apache-2.0).
 * The anchor-to-top / tail-spacer feature of upstream is not ported.
 */
import React from "react";
import {
  nextFollowIntent,
  upwardGestureCanEscapeBottom,
  type FollowIntent,
  type ScrollSample,
  type WheelWitness,
} from "./stick-to-bottom";
import { shouldShowJumpToLatest } from "./jump-to-latest";

/** Bám đáy chỉ cần viết khi còn cách đáy hơn 1px (tránh tự sinh scroll event vô ích). */
const FOLLOW_PIN_TOLERANCE_PX = 1;

function readSample(el: HTMLElement): ScrollSample {
  const clientHeight = el.clientHeight;
  return {
    scrollTop: el.scrollTop,
    scrollHeight: Math.max(clientHeight, el.scrollHeight),
    clientHeight,
  };
}

export interface StickToBottom {
  /** Gắn vào khung cuộn. */
  ref: React.RefObject<HTMLDivElement>;
  /** Hiện nút "Về tin mới nhất". */
  showJumpToLatest: boolean;
  /** Bật lại bám đáy và nhảy xuống ngay (nút "về mới nhất", gửi tin, đổi hội thoại). */
  jumpToLatest: () => void;
}

/**
 * `resetKey` đổi (vd. đổi hội thoại) thì bật lại bám đáy. Chiều cao nội dung được theo dõi bằng
 * ResizeObserver trên khung và từng con trực tiếp, nên không cần truyền danh sách tin vào.
 */
export function useStickToBottom(resetKey?: unknown): StickToBottom {
  const ref = React.useRef<HTMLDivElement>(null);
  const intentRef = React.useRef<FollowIntent>({ following: true, escaped: false });
  const lastSampleRef = React.useRef<ScrollSample>({ scrollTop: 0, scrollHeight: 0, clientHeight: 0 });
  const wheelWitnessRef = React.useRef<WheelWitness | null>(null);
  const wheelWitnessFrameRef = React.useRef<number | null>(null);
  const [showJumpToLatest, setShowJumpToLatest] = React.useState(false);

  const rememberSample = React.useCallback((el: HTMLElement) => {
    lastSampleRef.current = readSample(el);
  }, []);

  const resetWheelWitness = React.useCallback(() => {
    wheelWitnessRef.current = null;
    const frame = wheelWitnessFrameRef.current;
    wheelWitnessFrameRef.current = null;
    if (frame !== null) cancelAnimationFrame(frame);
  }, []);

  /** Đưa ý định lên màn hình: đang bám thì dính đáy (TỨC THỜI), rồi cập nhật nút "về mới nhất". */
  const syncFollowState = React.useCallback(() => {
    const el = ref.current;
    if (!el) return;
    if (intentRef.current.following) {
      const maxTop = Math.max(0, el.scrollHeight - el.clientHeight);
      if (maxTop - el.scrollTop > FOLLOW_PIN_TOLERANCE_PX) {
        el.scrollTop = el.scrollHeight;
        rememberSample(el);
      }
    }
    const sample = readSample(el);
    setShowJumpToLatest((prev) => {
      const next = shouldShowJumpToLatest({
        distance: Math.max(0, sample.scrollHeight - sample.scrollTop - sample.clientHeight),
        clientHeight: sample.clientHeight,
        scrollHeight: sample.scrollHeight,
        shown: prev,
        following: intentRef.current.following,
      });
      // Trả lại đúng giá trị cũ khi không đổi để chuỗi scroll event lúc stream không gây re-render.
      return prev === next ? prev : next;
    });
  }, [rememberSample]);

  const jumpToLatest = React.useCallback(() => {
    intentRef.current = { following: true, escaped: false };
    syncFollowState();
  }, [syncFollowState]);

  // Thao tác người dùng: cuộn, bánh xe, chạm.
  React.useEffect(() => {
    const el = ref.current;
    if (!el) return;

    const releaseFollow = () => {
      intentRef.current = { following: false, escaped: true };
    };

    const armWheelWitnessExpiry = () => {
      if (wheelWitnessFrameRef.current !== null) return;
      wheelWitnessFrameRef.current = requestAnimationFrame(() => {
        wheelWitnessFrameRef.current = null;
        wheelWitnessRef.current = null;
      });
    };

    const recordWheelWitness = (deltaY: number) => {
      if (deltaY === 0) return;
      const atScrollTop = el.scrollTop;
      const current = wheelWitnessRef.current;
      const witness =
        current !== null && current.atScrollTop === atScrollTop
          ? current
          : { downwardEvents: 0, upwardEvents: 0, atScrollTop };
      if (deltaY > 0) witness.downwardEvents += 1;
      else witness.upwardEvents += 1;
      wheelWitnessRef.current = witness;
      armWheelWitnessExpiry();
    };

    const onScroll = () => {
      const sample = readSample(el);
      intentRef.current = nextFollowIntent(intentRef.current, lastSampleRef.current, sample, wheelWitnessRef.current);
      lastSampleRef.current = sample;
      // Biên nhận chỉ dùng cho đúng một dịch chuyển.
      resetWheelWitness();
      syncFollowState();
    };

    // Lăn lên = nhả ngay, không chờ scroll event (khi stream nhanh trình duyệt có thể nuốt mất nó).
    const onWheel = (event: WheelEvent) => {
      // Ghi hướng trước khi thoát sớm: nấc lăn xuống chính là thứ kích hoạt compositor snap-back.
      recordWheelWitness(event.deltaY);
      if (event.deltaY >= 0) return;
      if (!upwardGestureCanEscapeBottom(readSample(el))) return;
      const { following, escaped } = intentRef.current;
      if (!following && escaped) return;
      releaseFollow();
      syncFollowState();
    };

    // Cảm ứng: kéo ngón tay xuống (xem nội dung cũ hơn) thì nhả.
    let touchStartY: number | null = null;
    const onTouchStart = (event: TouchEvent) => {
      touchStartY = event.touches[0]?.clientY ?? null;
    };
    const onTouchMove = (event: TouchEvent) => {
      if (touchStartY === null) return;
      const y = event.touches[0]?.clientY;
      if (y === undefined) return;
      if (y - touchStartY > 8 && upwardGestureCanEscapeBottom(readSample(el))) {
        releaseFollow();
        syncFollowState();
        touchStartY = null;
      }
    };

    // Kênh nhập khác (thanh cuộn, bàn phím) làm biên nhận bánh xe cũ mất hiệu lực.
    const onOtherInput = () => resetWheelWitness();

    rememberSample(el);
    el.addEventListener("scroll", onScroll);
    el.addEventListener("wheel", onWheel, { passive: true });
    el.addEventListener("touchstart", onTouchStart, { passive: true });
    el.addEventListener("touchmove", onTouchMove, { passive: true });
    el.addEventListener("pointerdown", onOtherInput, { passive: true });
    el.addEventListener("keydown", onOtherInput, { passive: true });
    return () => {
      el.removeEventListener("scroll", onScroll);
      el.removeEventListener("wheel", onWheel);
      el.removeEventListener("touchstart", onTouchStart);
      el.removeEventListener("touchmove", onTouchMove);
      el.removeEventListener("pointerdown", onOtherInput);
      el.removeEventListener("keydown", onOtherInput);
      resetWheelWitness();
    };
  }, [rememberSample, resetWheelWitness, syncFollowState]);

  // Hình học đổi (nội dung lớn lên khi stream, panel đổi cỡ): gom vào một frame. Không chạm ý định.
  React.useEffect(() => {
    const el = ref.current;
    if (!el || typeof ResizeObserver === "undefined") return;

    let frame: number | null = null;
    const schedule = () => {
      if (frame !== null) return;
      frame = requestAnimationFrame(() => {
        frame = null;
        syncFollowState();
        // Đổi bố cục đổi luôn mốc so sánh cho lần cuộn kế tiếp; không làm mới thì lần cuộn thật
        // tiếp theo mang scrollHeight cũ và bị coi là chỉnh bố cục.
        if (ref.current) rememberSample(ref.current);
      });
    };

    const resizeObserver = new ResizeObserver(schedule);
    resizeObserver.observe(el);
    const observed = new Set<Element>();
    const syncChildren = () => {
      const current = new Set(Array.from(el.children));
      for (const child of current) {
        if (!observed.has(child)) {
          resizeObserver.observe(child);
          observed.add(child);
        }
      }
      for (const child of observed) {
        if (!current.has(child)) {
          resizeObserver.unobserve(child);
          observed.delete(child);
        }
      }
    };
    syncChildren();
    const mutationObserver = new MutationObserver(() => {
      syncChildren();
      schedule();
    });
    mutationObserver.observe(el, { childList: true });

    return () => {
      if (frame !== null) cancelAnimationFrame(frame);
      resizeObserver.disconnect();
      mutationObserver.disconnect();
    };
  }, [rememberSample, syncFollowState]);

  // Đổi hội thoại: bám đáy lại.
  React.useEffect(() => {
    jumpToLatest();
  }, [resetKey, jumpToLatest]);

  return { ref, showJumpToLatest, jumpToLatest };
}
