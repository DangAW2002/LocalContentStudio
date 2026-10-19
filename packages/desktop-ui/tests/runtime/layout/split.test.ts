// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from "vitest";
import {
  DEFAULT_PROJECT_LAYOUT,
  dragPane,
  loadProjectLayout,
  paneMax,
  saveProjectLayout,
  stepPane,
} from "../../../src/runtime/layout/split";

const limits = { min: 280, max: 600 };

describe("split pane drag (VS Code snap semantics)", () => {
  it("clamps within min/max while visible", () => {
    expect(dragPane({ size: 380, visible: true }, 450, limits)).toEqual({ size: 450, visible: true });
    expect(dragPane({ size: 380, visible: true }, 900, limits)).toEqual({ size: 600, visible: true });
    // Between half-min and min: held at min, not hidden yet.
    expect(dragPane({ size: 380, visible: true }, 200, limits)).toEqual({ size: 280, visible: true });
  });

  it("snaps hidden below half the minimum and keeps the previous size", () => {
    expect(dragPane({ size: 380, visible: true }, 139, limits)).toEqual({ size: 380, visible: false });
  });

  it("snaps back once dragged past half the minimum", () => {
    expect(dragPane({ size: 380, visible: false }, 100, limits)).toEqual({ size: 380, visible: false });
    expect(dragPane({ size: 380, visible: false }, 140, limits)).toEqual({ size: 280, visible: true });
  });

  it("never lets a side pane squeeze the centre below its minimum", () => {
    expect(paneMax(720, 1200, 320, 320)).toBe(560);
    expect(paneMax(720, 2000, 320, 320)).toBe(720);
    expect(paneMax(720, 500, 320, 320)).toBe(0);
    // A max below min still yields min (the centre then scrolls rather than the pane vanishing).
    expect(dragPane({ size: 380, visible: true }, 500, { min: 280, max: 100 })).toEqual({ size: 280, visible: true });
  });

  it("keyboard steps clamp and leave hidden panes alone", () => {
    expect(stepPane({ size: 380, visible: true }, 16, limits)).toEqual({ size: 396, visible: true });
    expect(stepPane({ size: 290, visible: true }, -64, limits)).toEqual({ size: 280, visible: true });
    const hidden = { size: 380, visible: false };
    expect(stepPane(hidden, 16, limits)).toBe(hidden);
  });
});

describe("project layout persistence", () => {
  beforeEach(() => localStorage.clear());

  it("round-trips and falls back on bad data", () => {
    expect(loadProjectLayout()).toEqual(DEFAULT_PROJECT_LAYOUT);
    const layout = { thread: { size: 420, visible: false }, outputs: { size: 300, visible: true } };
    saveProjectLayout(layout);
    expect(loadProjectLayout()).toEqual(layout);
    localStorage.setItem("lcs.layout.project.v1", JSON.stringify({ thread: { size: "x" } }));
    expect(loadProjectLayout()).toEqual(DEFAULT_PROJECT_LAYOUT);
  });
});
