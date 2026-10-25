// @vitest-environment jsdom
import React from "react";
import { describe, expect, it, afterEach, beforeEach, vi } from "vitest";
import { render, screen, cleanup, waitFor, act } from "@testing-library/react";
import * as tauriBridge from "../../src/api/tauri-bridge";
import { HomeView } from "../../src/views/HomeView";

describe("HomeView (Workspace Switch Reactivity)", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    vi.spyOn(tauriBridge, "isTauri").mockReturnValue(false);
  });

  afterEach(() => cleanup());

  it("reloads data and clears error when studio:workspace-changed event is fired", async () => {
    let hasWorkspace = false;

    vi.spyOn(tauriBridge, "invoke").mockImplementation(async (cmd: string) => {
      if (cmd === "engine_models_list") return [{ value: "claude-3-5-sonnet", label: "Sonnet" }];
      if (cmd === "project_list") {
        if (!hasWorkspace) {
          throw new Error("Chưa mở workspace nào");
        }
        return [
          {
            slug: "du-an-demo",
            title: "Dự án Demo",
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        ];
      }
      if (cmd === "jobs_snapshot") return [];
      if (cmd === "brand_scope_list") return [];
      return null;
    });

    render(
      <HomeView
        onOpenProject={vi.fn()}
        onNavigateSettings={vi.fn()}
      />
    );

    // Initial state: No workspace -> shows ErrorState "Chưa chọn Workspace"
    await waitFor(() => {
      expect(screen.getByText("Chưa chọn Workspace")).toBeTruthy();
    });

    // Simulate workspace created / opened
    hasWorkspace = true;
    act(() => {
      window.dispatchEvent(
        new CustomEvent("studio:workspace-changed", {
          detail: {
            name: "Workspace Mới",
            path: "/home/dang/Studio/workspace-moi",
          },
        })
      );
    });

    // After event: HomeView auto-refreshed, ErrorState disappears and projects are loaded
    await waitFor(() => {
      expect(screen.queryByText("Chưa chọn Workspace")).toBeNull();
      expect(screen.getByText("Dự án Demo")).toBeTruthy();
    });
  });
});
