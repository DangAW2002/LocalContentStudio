import { describe, expect, it } from "vitest";
import {
  parseBrandYaml,
  stringifyBrandYaml,
  isValidHexColor,
  DEFAULT_BRAND_SPEC,
  type BrandSpec,
} from "../../src/utils/brandYaml";

describe("brandYaml parser & serializer", () => {
  it("validates hex color strings accurately", () => {
    expect(isValidHexColor("#fff")).toBe(true);
    expect(isValidHexColor("#FFFFFF")).toBe(true);
    expect(isValidHexColor("#1e88e5")).toBe(true);
    expect(isValidHexColor("#1e88e5ff")).toBe(true);
    expect(isValidHexColor("1e88e5")).toBe(false);
    expect(isValidHexColor("#12345")).toBe(false);
    expect(isValidHexColor("#gggggg")).toBe(false);
    expect(isValidHexColor("")).toBe(false);
  });

  it("parses minimal valid brand YAML", () => {
    const yaml = `
name: "Brand Mới"
slogan: "Sáng tạo không giới hạn"
colors:
  primary: "#0055ff"
  accent: "#ffaa00"
  bg: "#111111"
  text: "#eeeeee"
  extra:
    - "#123456"
fonts:
  heading: "Roboto"
  body: "Open Sans"
logos:
  full: "assets/logo.png"
  mark: "assets/mark.png"
  min_size_px: 64
  clear_space_ratio: 0.3
voice: "custom-voice.md"
voice_words:
  prefer:
    - "nhanh chóng"
  avoid:
    - "chậm trễ"
rules:
  - "Luôn dùng logo trên nền tương phản"
version: "brand@1"
`;
    const spec = parseBrandYaml(yaml);
    expect(spec.name).toBe("Brand Mới");
    expect(spec.slogan).toBe("Sáng tạo không giới hạn");
    expect(spec.colors.primary).toBe("#0055ff");
    expect(spec.colors.extra).toContain("#123456");
    expect(spec.fonts.heading).toBe("Roboto");
    expect(spec.logos.min_size_px).toBe(64);
    expect(spec.voice_words.prefer).toEqual(["nhanh chóng"]);
    expect(spec.voice_words.avoid).toEqual(["chậm trễ"]);
    expect(spec.rules).toEqual(["Luôn dùng logo trên nền tương phản"]);
  });

  it("roundtrips BrandSpec through stringify and parse", () => {
    const spec: BrandSpec = {
      ...DEFAULT_BRAND_SPEC,
      name: "Roundtrip Test Brand",
      slogan: "Thử nghiệm chuyển đổi",
      colors: {
        primary: "#112233",
        accent: "#445566",
        bg: "#000000",
        text: "#ffffff",
        extra: ["#aabbcc"],
      },
    };

    const yaml = stringifyBrandYaml(spec);
    const parsed = parseBrandYaml(yaml);

    expect(parsed.name).toBe(spec.name);
    expect(parsed.slogan).toBe(spec.slogan);
    expect(parsed.colors.primary).toBe(spec.colors.primary);
    expect(parsed.colors.accent).toBe(spec.colors.accent);
    expect(parsed.colors.extra).toEqual(spec.colors.extra);
  });
});
