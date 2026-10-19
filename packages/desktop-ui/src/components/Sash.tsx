import React from "react";

export interface SashProps {
  /** Accessible name, e.g. "Đổi độ rộng khung hội thoại". */
  label: string;
  /** Drag began (capture the starting sizes). */
  onDragStart: () => void;
  /** Pointer moved `dx` pixels horizontally since the drag began. */
  onDrag: (dx: number) => void;
  onDragEnd?: () => void;
  /** Double-click: back to the default size (VS Code sash `onDidReset`). */
  onReset?: () => void;
  /** Keyboard resize by `dx` pixels (arrow keys on the focused sash). */
  onStep?: (dx: number) => void;
}

/**
 * Vertical drag handle between two panes, modeled on VS Code's Sash: pointer-captured drag,
 * double-click to reset, delayed hover highlight (see .cs-resizer in ui.css), arrow keys to nudge.
 */
export const Sash: React.FC<SashProps> = ({ label, onDragStart, onDrag, onDragEnd, onReset, onStep }) => {
  const [dragging, setDragging] = React.useState(false);
  const startXRef = React.useRef(0);

  const end = (el: HTMLElement, pointerId: number) => {
    if (el.hasPointerCapture(pointerId)) el.releasePointerCapture(pointerId);
    setDragging(false);
    delete document.body.dataset.resizing;
    onDragEnd?.();
  };

  return (
    <div
      className="cs-resizer"
      role="separator"
      aria-orientation="vertical"
      aria-label={label}
      tabIndex={0}
      data-dragging={dragging ? "true" : undefined}
      onPointerDown={(e) => {
        if (e.button !== 0) return;
        e.preventDefault();
        e.currentTarget.setPointerCapture(e.pointerId);
        startXRef.current = e.clientX;
        setDragging(true);
        document.body.dataset.resizing = "true";
        onDragStart();
      }}
      onPointerMove={(e) => {
        if (!dragging) return;
        onDrag(e.clientX - startXRef.current);
      }}
      onPointerUp={(e) => end(e.currentTarget, e.pointerId)}
      onPointerCancel={(e) => end(e.currentTarget, e.pointerId)}
      onDoubleClick={() => onReset?.()}
      onKeyDown={(e) => {
        if (!onStep) return;
        const step = e.shiftKey ? 64 : 16;
        if (e.key === "ArrowLeft") {
          e.preventDefault();
          onStep(-step);
        } else if (e.key === "ArrowRight") {
          e.preventDefault();
          onStep(step);
        }
      }}
    />
  );
};
