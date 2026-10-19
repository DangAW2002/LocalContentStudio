import React from "react";

/**
 * Close a popup the way VS Code's context views do: Escape, or a pointer press outside every
 * element in `refs` (the popup and the button that opened it, so the toggle click is not counted
 * as "outside").
 */
export function useDismiss(
  open: boolean,
  onDismiss: () => void,
  refs: ReadonlyArray<React.RefObject<HTMLElement | null>>,
): void {
  const onDismissRef = React.useRef(onDismiss);
  onDismissRef.current = onDismiss;
  const refsRef = React.useRef(refs);
  refsRef.current = refs;

  React.useEffect(() => {
    if (!open) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onDismissRef.current();
      }
    };
    const onPointerDown = (e: PointerEvent) => {
      const target = e.target as Node | null;
      if (!target) return;
      if (refsRef.current.some((r) => r.current?.contains(target))) return;
      onDismissRef.current();
    };
    // Capture phase so a stopPropagation inside the page cannot keep the popup open.
    window.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("pointerdown", onPointerDown, true);
    return () => {
      window.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("pointerdown", onPointerDown, true);
    };
  }, [open]);
}
