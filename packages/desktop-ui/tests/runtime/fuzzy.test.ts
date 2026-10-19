import { describe, it, expect } from "vitest";
import { foldVietnamese, fuzzyScore } from "../../src/runtime/fuzzy";

describe("fuzzy.ts", () => {
  describe("foldVietnamese", () => {
    it("folds lowercase and uppercase diacritics including đ and Đ", () => {
      expect(foldVietnamese("Cài đặt")).toBe("cai dat");
      expect(foldVietnamese("ĐIỀU HƯỚNG")).toBe("dieu huong");
      expect(foldVietnamese("Đặng Nhật")).toBe("dang nhat");
      expect(foldVietnamese("Đĩa cứng")).toBe("dia cung");
    });
  });

  describe("fuzzyScore", () => {
    it("returns null when query cannot match target", () => {
      expect(fuzzyScore("xyz", "Cài đặt")).toBeNull();
      expect(fuzzyScore("cai dat qua dai", "Cài đặt")).toBeNull();
    });

    it("matches empty query with 0 score and empty matches", () => {
      const res = fuzzyScore("", "Cài đặt");
      expect(res).not.toBeNull();
      expect(res?.score).toBe(0);
      expect(res?.matches).toEqual([]);
    });

    it("matches Vietnamese without diacritics ('cai dat' -> 'Cài đặt')", () => {
      const res = fuzzyScore("cai dat", "Cài đặt");
      expect(res).not.toBeNull();
      expect(res?.matches).toEqual([0, 1, 2, 3, 4, 5, 6]);
    });

    it("gives higher score to exact and prefix matches", () => {
      const exact = fuzzyScore("cai dat", "Cài đặt");
      const prefix = fuzzyScore("cai", "Cài đặt hệ thống");
      const nonPrefix = fuzzyScore("dat", "Cài đặt hệ thống");

      expect(exact).not.toBeNull();
      expect(prefix).not.toBeNull();
      expect(nonPrefix).not.toBeNull();

      expect(exact!.score).toBeGreaterThan(prefix!.score);
      expect(prefix!.score).toBeGreaterThan(nonPrefix!.score);
    });

    it("prioritizes word starts and consecutive characters", () => {
      // "cd" in "Cài đặt" matches 'C' (start of word 1) and 'đ' (start of word 2)
      const res = fuzzyScore("cd", "Cài đặt");
      expect(res).not.toBeNull();
      expect(res?.matches).toEqual([0, 4]); // 'C' is 0, 'đ' is 4
    });

    it("sorts candidates properly by score", () => {
      const items = [
        "Chuyển theme",
        "Cài đặt hệ thống",
        "Mở Tools & Cài đặt hệ thống",
        "Đóng cửa sổ",
      ];
      const query = "cai dat";
      const scored = items
        .map((title) => ({ title, res: fuzzyScore(query, title) }))
        .filter((item): item is { title: string; res: NonNullable<typeof item.res> } => item.res !== null)
        .sort((a, b) => b.res.score - a.res.score);

      expect(scored.length).toBe(2);
      expect(scored[0].title).toBe("Cài đặt hệ thống");
      expect(scored[1].title).toBe("Mở Tools & Cài đặt hệ thống");
    });
  });
});
