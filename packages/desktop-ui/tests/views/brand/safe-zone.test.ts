import { describe, expect, it } from "vitest";

describe("SafeZone aspect ratio and box dimension math", () => {
  const BOX = 180;
  function calcDimensions(width: number, height: number) {
    const ratio = width / Math.max(1, height);
    const w = ratio >= 1 ? BOX : Math.round(BOX * ratio);
    const h = ratio >= 1 ? Math.round(BOX / ratio) : BOX;
    return { ratio, w, h };
  }

  it("calculates correct dimensions for landscape 16:9", () => {
    const { ratio, w, h } = calcDimensions(1920, 1080);
    expect(ratio).toBeCloseTo(16 / 9, 2);
    expect(w).toBe(BOX);
    expect(h).toBe(Math.round(180 / (1920 / 1080))); // ~101px
  });

  it("calculates correct dimensions for portrait 9:16", () => {
    const { ratio, w, h } = calcDimensions(1080, 1920);
    expect(ratio).toBeCloseTo(9 / 16, 2);
    expect(w).toBe(Math.round(180 * (1080 / 1920))); // ~101px
    expect(h).toBe(BOX);
  });

  it("calculates correct dimensions for square 1:1", () => {
    const { ratio, w, h } = calcDimensions(1080, 1080);
    expect(ratio).toBe(1);
    expect(w).toBe(BOX);
    expect(h).toBe(BOX);
  });

  it("handles zero or degenerate height without dividing by zero", () => {
    const { ratio, w, h } = calcDimensions(1920, 0);
    expect(ratio).toBe(1920);
    expect(w).toBe(BOX);
    expect(h).toBe(0);
  });
});
