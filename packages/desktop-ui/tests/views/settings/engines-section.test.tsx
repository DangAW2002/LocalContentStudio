// @vitest-environment jsdom
import React from "react";
import { describe, expect, it, afterEach, beforeEach, vi } from "vitest";
import { render, screen, fireEvent, cleanup, waitFor } from "@testing-library/react";
import { EnginesSection } from "../../../src/views/settings/EnginesSection";
import { studioApi } from "../../../src/api/client";

describe("EnginesSection component", () => {
  beforeEach(() => {
    vi.spyOn(studioApi, "probeEngines").mockResolvedValue([
      {
        id: "claude",
        name: "Claude Code CLI",
        installed: true,
        version: "1.0.0",
        supported: true,
      },
      {
        id: "agy",
        name: "Antigravity CLI (agy)",
        installed: true,
        version: "1.4.0",
        supported: true,
      },
    ]);
    vi.spyOn(studioApi, "engineSettingsGet").mockResolvedValue({
      agyFullAccess: false,
    });
    vi.spyOn(studioApi, "engineSettingsSet").mockResolvedValue(undefined);
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it("renders agy full access switch and Vietnamese warning, and updates setting on toggle", async () => {
    render(<EnginesSection />);

    await waitFor(() => {
      expect(
        screen.getByLabelText("Cho agy toàn quyền (chạy lệnh shell không cần duyệt)")
      ).toBeDefined();
    });

    const toggle = screen.getByLabelText(
      "Cho agy toàn quyền (chạy lệnh shell không cần duyệt)"
    ) as HTMLInputElement;
    expect(toggle.checked).toBe(false);

    expect(
      screen.getByText(/agy có thể chạy mọi lệnh trên máy, chỉ bật khi tin tưởng/i)
    ).toBeDefined();

    fireEvent.click(toggle);

    await waitFor(() => {
      expect(studioApi.engineSettingsSet).toHaveBeenCalledWith({
        agyFullAccess: true,
      });
      expect(toggle.checked).toBe(true);
    });
  });
});
