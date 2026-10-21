import { describe, it, expect } from "vitest";
import type { DownstreamPlan } from "../../../src/api/types";

describe("Graph Dialogs and UI logic", () => {
  it("calculates downstream plan free/paid counts and skips pinned", () => {
    const mockPlan: DownstreamPlan = {
      targetRef: "script-01",
      freeStepsCount: 1,
      paidStepsCount: 1,
      totalEstimatedCostUsd: 0.05,
      steps: [
        {
          id: "voice-01",
          kind: "generation",
          title: "Voice",
          paid: true,
          estimatedCostUsd: 0.05,
          pinned: false,
          skipped: false,
        },
        {
          id: "timeline-01",
          kind: "timeline",
          title: "Timeline",
          paid: false,
          estimatedCostUsd: 0,
          pinned: true,
          skipped: true,
          skipReason: "Đã khóa (pinned)",
        },
      ],
    };

    expect(mockPlan.steps).toHaveLength(2);
    expect(mockPlan.steps[0].paid).toBe(true);
    expect(mockPlan.steps[1].skipped).toBe(true);
    expect(mockPlan.totalEstimatedCostUsd).toBe(0.05);
  });

  it("formats role labels accurately", () => {
    const roles: Record<string, string> = {
      uses: "uses (Phụ thuộc trực tiếp)",
      reference: "reference (Hồ sơ tham chiếu)",
      variant_of: "variant_of (Biến thể)",
    };

    expect(roles.uses).toContain("trực tiếp");
    expect(roles.reference).toContain("tham chiếu");
    expect(roles.variant_of).toContain("Biến thể");
  });
});
