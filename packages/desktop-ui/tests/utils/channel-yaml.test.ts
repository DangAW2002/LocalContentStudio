import { describe, expect, it } from "vitest";
import {
  computeAspectRatio,
  parseChannelYaml,
  stringifyChannelYaml,
} from "../../src/utils/channelYaml";

describe("channelYaml utils", () => {
  describe("computeAspectRatio", () => {
    it("computes 16:9 for landscape 1920x1080 and 1280x720", () => {
      expect(computeAspectRatio(1920, 1080)).toBe("16:9");
      expect(computeAspectRatio(1280, 720)).toBe("16:9");
    });

    it("computes 9:16 for portrait 1080x1920 and 720x1280", () => {
      expect(computeAspectRatio(1080, 1920)).toBe("9:16");
      expect(computeAspectRatio(720, 1280)).toBe("9:16");
    });

    it("computes 1:1 for square 1080x1080", () => {
      expect(computeAspectRatio(1080, 1080)).toBe("1:1");
    });

    it("computes 4:5 for vertical post 1080x1350", () => {
      expect(computeAspectRatio(1080, 1350)).toBe("4:5");
    });

    it("computes 4:3 and 3:2 standard ratios", () => {
      expect(computeAspectRatio(1440, 1080)).toBe("4:3");
      expect(computeAspectRatio(1080, 1440)).toBe("3:4");
      expect(computeAspectRatio(1500, 1000)).toBe("3:2");
    });

    it("handles zero/negative or non-standard dimensions gracefully", () => {
      expect(computeAspectRatio(0, 1080)).toBe("1:1");
      expect(computeAspectRatio(1200, 628)).toBe("1.91:1");
      expect(computeAspectRatio(1200, 500)).toBe("12:5");
    });
  });

  describe("parseChannelYaml and stringifyChannelYaml", () => {
    const sampleYaml = `
id: tiktok
name: "TikTok Channel"
description: "TikTok short form videos"
formats:
  - id: short
    name: "Video 9:16"
    width: 1080
    height: 1920
    ratio: "9:16"
    fps: 30
    duration_max_s: 60
    safe_zone:
      top: 0.1
      bottom: 0.2
      left: 0.05
      right: 0.15
limits:
  title_max_chars: 100
  desc_max_chars: 2200
  captions_required: true
files:
  max_size_mb: 287
`;

    it("parses YAML into ChannelSpec", () => {
      const spec = parseChannelYaml(sampleYaml, "fallback-id");
      expect(spec.id).toBe("tiktok");
      expect(spec.title).toBe("TikTok Channel");
      expect(spec.formats).toHaveLength(1);
      expect(spec.formats[0].width).toBe(1080);
      expect(spec.formats[0].height).toBe(1920);
      expect(spec.formats[0].safe_zone.bottom).toBe(0.2);
      expect(spec.text.title_max).toBe(100);
      expect(spec.captions_required).toBe(true);
      expect(spec.file.max_size_mb).toBe(287);
    });

    it("stringifies ChannelSpec back to valid YAML", () => {
      const spec = parseChannelYaml(sampleYaml, "fallback-id");
      const yamlStr = stringifyChannelYaml(spec);
      expect(yamlStr).toContain("title: \"TikTok Channel\"");
      expect(yamlStr).toContain("width: 1080");
      expect(yamlStr).toContain("height: 1920");
      expect(yamlStr).toContain("bottom: 0.2");
      expect(yamlStr).toContain("title_max: 100");
    });
  });
});
