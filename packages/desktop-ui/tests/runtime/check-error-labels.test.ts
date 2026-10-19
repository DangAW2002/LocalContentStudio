import { describe, it, expect } from "vitest";
import { getCheckErrorLabel, CHECK_ERROR_LABELS } from "../../src/runtime/check-error-labels";

describe("check-error-labels", () => {
  it("translates known error codes into clear Vietnamese labels", () => {
    expect(getCheckErrorLabel("DIMENSIONS_MISMATCH")).toBe("Sai kích thước định dạng");
    expect(getCheckErrorLabel("NO_RENDERS")).toBe("Chưa có bản render");
    expect(getCheckErrorLabel("DURATION_EXCEEDED")).toBe("Thời lượng video vượt quá giới hạn");
    expect(getCheckErrorLabel("CAPTIONS_MISSING")).toBe("Thiếu phụ đề bắt buộc");
    expect(getCheckErrorLabel("SAFE_ZONE_VIOLATION")).toBe("Vi phạm vùng an toàn (Safe Zone)");
  });

  it("returns fallback code if unknown", () => {
    expect(getCheckErrorLabel("UNKNOWN_CODE")).toBe("UNKNOWN_CODE");
  });
});
