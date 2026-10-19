import { describe, expect, it } from "vitest";
import { validateTimelineDoc } from "../../../src/views/project/TimelineEditor";

describe("validateTimelineDoc", () => {
  it("validates correct timeline doc structure", () => {
    const validDoc = {
      version: 1,
      width: 1920,
      height: 1080,
      fps: 30,
      tracks: [
        {
          id: "track-1",
          kind: "video",
          clips: [
            {
              id: "clip-1",
              source: "video.mp4",
              start: 0,
              duration: 5,
              in_point: 0,
              out_point: 5,
            },
          ],
        },
      ],
    };
    expect(validateTimelineDoc(validDoc)).toBe(true);
  });

  it("rejects non-object or missing top-level fields", () => {
    expect(validateTimelineDoc(null)).toBe(false);
    expect(validateTimelineDoc("string")).toBe(false);
    expect(validateTimelineDoc({ version: 0, width: 1920, height: 1080, fps: 30, tracks: [] })).toBe(false);
    expect(validateTimelineDoc({ version: 1, width: 0, height: 1080, fps: 30, tracks: [] })).toBe(false);
    expect(validateTimelineDoc({ version: 1, width: 1920, height: -1, fps: 30, tracks: [] })).toBe(false);
    expect(validateTimelineDoc({ version: 1, width: 1920, height: 1080, fps: 0, tracks: [] })).toBe(false);
    expect(validateTimelineDoc({ version: 1, width: 1920, height: 1080, fps: 30, tracks: null })).toBe(false);
  });

  it("validates track kind and clips", () => {
    const base = {
      version: 1,
      width: 1080,
      height: 1920,
      fps: 60,
    };

    expect(validateTimelineDoc({ ...base, tracks: [{ id: "t1", kind: "invalid-kind", clips: [] }] })).toBe(false);
    expect(validateTimelineDoc({ ...base, tracks: [{ id: "t1", kind: "caption", clips: "not-array" }] })).toBe(false);

    // Invalid clip duration < 0
    expect(
      validateTimelineDoc({
        ...base,
        tracks: [
          {
            id: "t1",
            kind: "audio",
            clips: [{ id: "c1", source: "a.mp3", start: 0, duration: -1 }],
          },
        ],
      })
    ).toBe(false);

    // Invalid clip start < 0
    expect(
      validateTimelineDoc({
        ...base,
        tracks: [
          {
            id: "t1",
            kind: "audio",
            clips: [{ id: "c1", source: "a.mp3", start: -5, duration: 10 }],
          },
        ],
      })
    ).toBe(false);
  });
});
