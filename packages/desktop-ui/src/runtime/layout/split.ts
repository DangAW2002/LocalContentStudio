/**
 * Side-pane sizing with VS Code's split-view semantics
 * (vscode src/vs/base/browser/ui/splitview/splitview.ts, MIT):
 *  · a pane snaps hidden once it is dragged below half its minimum size, and snaps back as soon
 *    as it is dragged past that point again (`limitDelta = minDelta ± minimumSize / 2`);
 *  · a hidden pane keeps its last size so showing it again restores it;
 *  · the centre view keeps its own minimum, which caps how wide a side pane may grow.
 */

export interface PaneState {
  size: number;
  visible: boolean;
}

export interface PaneLimits {
  min: number;
  max: number;
}

function clamp(value: number, low: number, high: number): number {
  return Math.min(high, Math.max(low, value));
}

/** The largest a side pane may be without squeezing the centre below its minimum. */
export function paneMax(paneMaxSize: number, containerWidth: number, otherPaneSize: number, centerMin: number): number {
  return Math.max(0, Math.min(paneMaxSize, containerWidth - otherPaneSize - centerMin));
}

/** New pane state for a drag that would make the pane `proposed` pixels wide. */
export function dragPane(start: PaneState, proposed: number, limits: PaneLimits): PaneState {
  if (proposed < limits.min / 2) {
    // Snapped away: remember the size it had before the drag began.
    return { size: start.size, visible: false };
  }
  const max = Math.max(limits.min, limits.max);
  return { size: clamp(proposed, limits.min, max), visible: true };
}

/** Keyboard resizing (arrow keys on a focused sash); never snaps, only clamps. */
export function stepPane(start: PaneState, delta: number, limits: PaneLimits): PaneState {
  if (!start.visible) return start;
  return { size: clamp(start.size + delta, limits.min, Math.max(limits.min, limits.max)), visible: true };
}

/** Persisted layout of the project view's side panes. */
export interface ProjectLayout {
  thread: PaneState;
  outputs: PaneState;
}

export const DEFAULT_PROJECT_LAYOUT: ProjectLayout = {
  thread: { size: 380, visible: true },
  outputs: { size: 320, visible: true },
};

const LAYOUT_KEY = "lcs.layout.project.v1";

function validPane(value: unknown, fallback: PaneState): PaneState {
  const v = value as Partial<PaneState> | null;
  if (!v || typeof v.size !== "number" || !Number.isFinite(v.size) || typeof v.visible !== "boolean") {
    return fallback;
  }
  return { size: v.size, visible: v.visible };
}

export function loadProjectLayout(): ProjectLayout {
  try {
    const raw = JSON.parse(localStorage.getItem(LAYOUT_KEY) || "null");
    return {
      thread: validPane(raw?.thread, DEFAULT_PROJECT_LAYOUT.thread),
      outputs: validPane(raw?.outputs, DEFAULT_PROJECT_LAYOUT.outputs),
    };
  } catch {
    return DEFAULT_PROJECT_LAYOUT;
  }
}

export function saveProjectLayout(layout: ProjectLayout): void {
  try {
    localStorage.setItem(LAYOUT_KEY, JSON.stringify(layout));
  } catch {
    // Not persisted; the layout still works for this session.
  }
}
