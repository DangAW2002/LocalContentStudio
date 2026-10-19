// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { formatCheckpointTime } from "../../../src/views/project/OutputsPane";

describe("formatCheckpointTime", () => {
  it("formats ISO timestamp in Vietnamese day/month hour:minute format", () => {
    // 2026-10-18T14:27:00Z in local time or formatted with local getters
    const d = new Date("2026-10-18T14:27:00");
    const expectedDay = String(d.getDate()).padStart(2, "0");
    const expectedMonth = String(d.getMonth() + 1).padStart(2, "0");
    const expectedHour = String(d.getHours()).padStart(2, "0");
    const expectedMin = String(d.getMinutes()).padStart(2, "0");
    const expected = `${expectedDay}/${expectedMonth} ${expectedHour}:${expectedMin}`;

    expect(formatCheckpointTime("2026-10-18T14:27:00")).toBe(expected);
  });

  it("returns original string if timestamp is invalid", () => {
    expect(formatCheckpointTime("invalid-date-string")).toBe("invalid-date-string");
    expect(formatCheckpointTime("")).toBe("");
  });
});
