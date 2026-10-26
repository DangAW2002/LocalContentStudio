import { describe, expect, it } from "vitest";
import {
  stripPromptPrefixes,
  cleanProjectTitle,
  buildInitialPrompt,
} from "../../src/views/HomeView";

describe("stripPromptPrefixes", () => {
  it("strips a single playbook prefix", () => {
    expect(
      stripPromptPrefixes("/YouTube Video Chuẩn Làm video về iPhone 16", "YouTube Video Chuẩn")
    ).toBe("Làm video về iPhone 16");
  });

  it("strips repeated playbook prefixes", () => {
    expect(
      stripPromptPrefixes(
        "/YouTube Video Chuẩn /YouTube Video Chuẩn Làm video về iPhone 16",
        "YouTube Video Chuẩn"
      )
    ).toBe("Làm video về iPhone 16");
  });

  it("strips brand context prefix", () => {
    expect(
      stripPromptPrefixes(
        "[Áp dụng Brand: Công nghệ (id: b1)] Kịch bản chi tiết"
      )
    ).toBe("Kịch bản chi tiết");
  });

  it("strips both brand and playbook prefixes regardless of order", () => {
    expect(
      stripPromptPrefixes(
        "[Áp dụng Brand: Tech (id: b1)] /TikTok Shorts Video 30s",
        "TikTok Shorts"
      )
    ).toBe("Video 30s");

    expect(
      stripPromptPrefixes(
        "/TikTok Shorts [Áp dụng Brand: Tech (id: b1)] Video 30s",
        "TikTok Shorts"
      )
    ).toBe("Video 30s");
  });

  it("strips generic slash commands when no matching playbook list provided", () => {
    expect(stripPromptPrefixes("/custom-action Tạo outline")).toBe("Tạo outline");
    expect(stripPromptPrefixes("/review_code Đánh giá PR")).toBe("Đánh giá PR");
  });

  it("handles exact command with no remaining text", () => {
    expect(stripPromptPrefixes("/YouTube Video Chuẩn", "YouTube Video Chuẩn")).toBe("");
    expect(stripPromptPrefixes("/shorts")).toBe("");
  });

  it("leaves prompt untouched if there are no prefixes", () => {
    expect(stripPromptPrefixes("Làm video giới thiệu")).toBe("Làm video giới thiệu");
  });
});

describe("cleanProjectTitle", () => {
  it("returns stripped title truncated up to 60 characters", () => {
    const title = cleanProjectTitle(
      "/YouTube Video Chuẩn Làm video giới thiệu sản phẩm mới chi tiết nhất",
      "YouTube Video Chuẩn"
    );
    expect(title).toBe("Làm video giới thiệu sản phẩm mới chi tiết nhất");
    expect(title.length).toBeLessThanOrEqual(60);
  });

  it("falls back to playbook name when prompt only contains playbook prefix", () => {
    expect(
      cleanProjectTitle("/YouTube Video Chuẩn", "YouTube Video Chuẩn")
    ).toBe("YouTube Video Chuẩn");
  });

  it("falls back to default title when prompt is empty or only brand prefix", () => {
    expect(cleanProjectTitle("   ")).toBe("Dự án mới");
    expect(cleanProjectTitle("[Áp dụng Brand: Tech (id: b1)]")).toBe("Dự án mới");
  });

  it("uses custom fallback title when provided", () => {
    expect(
      cleanProjectTitle("/YouTube Video Chuẩn", "YouTube Video Chuẩn", "Tiêu đề dự phòng")
    ).toBe("Tiêu đề dự phòng");
  });
});

describe("buildInitialPrompt", () => {
  it("prepends playbook prefix when not already present in prompt", () => {
    const res = buildInitialPrompt("Làm video hay", "YouTube Video Chuẩn");
    expect(res).toBe("/YouTube Video Chuẩn Làm video hay");
  });

  it("does not duplicate playbook prefix if prompt already starts with it", () => {
    const res = buildInitialPrompt(
      "/YouTube Video Chuẩn Làm video hay",
      "YouTube Video Chuẩn"
    );
    expect(res).toBe("/YouTube Video Chuẩn Làm video hay");
  });

  it("does not duplicate playbook prefix if prompt contains it", () => {
    const res = buildInitialPrompt(
      "Yêu cầu theo /YouTube Video Chuẩn cho kênh",
      "YouTube Video Chuẩn"
    );
    expect(res).toBe("Yêu cầu theo /YouTube Video Chuẩn cho kênh");
  });

  it("prepends brand context when brand is selected and not already in prompt", () => {
    const res = buildInitialPrompt("Làm video", undefined, "b-1", "Kênh Tech");
    expect(res).toBe("[Áp dụng Brand: Kênh Tech (id: b-1)] Làm video");
  });

  it("does not duplicate brand context if already present in prompt", () => {
    const res = buildInitialPrompt(
      "[Áp dụng Brand: Kênh Tech (id: b-1)] Làm video",
      undefined,
      "b-1",
      "Kênh Tech"
    );
    expect(res).toBe("[Áp dụng Brand: Kênh Tech (id: b-1)] Làm video");
  });

  it("combines both playbook and brand without duplicating either", () => {
    const res = buildInitialPrompt(
      "/Shorts Kịch bản 15s",
      "Shorts",
      "b-1",
      "Kênh Tech"
    );
    expect(res).toBe("[Áp dụng Brand: Kênh Tech (id: b-1)] /Shorts Kịch bản 15s");
  });
});
