// @vitest-environment jsdom
import React from "react";
import { describe, expect, it, afterEach, beforeEach, vi } from "vitest";
import { render, screen, fireEvent, cleanup, waitFor } from "@testing-library/react";
import * as tauriBridge from "../../../src/api/tauri-bridge";
import { CreateEpisodeDialog } from "../../../src/views/project/CreateEpisodeDialog";
import { AttachProfileDialog } from "../../../src/views/project/graph/AttachProfileDialog";

describe("Series Dialogs and AttachProfileDialog", () => {
  beforeEach(() => {
    window.HTMLElement.prototype.scrollIntoView = vi.fn();
    vi.restoreAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  describe("CreateEpisodeDialog", () => {
    const mockSeries = [
      {
        slug: "series-test",
        title: "Chuyện quán cà phê",
        description: "Series về quán cà phê",
        recipe: { kit: "youtube-video", modules: ["extra-broll"], playbook: "youtube-video" },
        references: ["nv-an", "quan-cafe"],
      },
    ];

    it("renders series selection, recipe summary and inputs when open", async () => {
      vi.spyOn(tauriBridge, "invoke").mockImplementation(async (cmd: string) => {
        if (cmd === "series_list") return mockSeries;
        return null;
      });

      render(
        <CreateEpisodeDialog
          open={true}
          onClose={vi.fn()}
          onCreated={vi.fn()}
          defaultSeriesSlug="series-test"
        />
      );

      expect(screen.getByText("Tạo tập mới từ series")).toBeDefined();

      // Wait for series list to load and check recipe display
      await waitFor(() => {
        expect(screen.getByText(/Chuyện quán cà phê/)).toBeDefined();
        expect(screen.getByText(/Series về quán cà phê/)).toBeDefined();
        expect(screen.getByText(/nv-an, quan-cafe/)).toBeDefined();
      });

      expect(screen.getByPlaceholderText(/Ví dụ: Tập 2: Chiến dịch Bạch Đằng/)).toBeDefined();
      expect(screen.getByPlaceholderText(/tap-2-chien-dich-bach-dang/)).toBeDefined();
    });

    it("auto-generates slug from title and validates empty slug", async () => {
      vi.spyOn(tauriBridge, "invoke").mockImplementation(async (cmd: string) => {
        if (cmd === "series_list") return mockSeries;
        return null;
      });

      render(
        <CreateEpisodeDialog
          open={true}
          onClose={vi.fn()}
          onCreated={vi.fn()}
          defaultSeriesSlug="series-test"
        />
      );

      await waitFor(() => {
        expect(screen.getByText(/Chuyện quán cà phê/)).toBeDefined();
      });

      const titleInput = screen.getByPlaceholderText(/Ví dụ: Tập 2: Chiến dịch Bạch Đằng/);
      const slugInput = screen.getByPlaceholderText(/tap-2-chien-dich-bach-dang/);
      const submitBtn = screen.getByRole("button", { name: /Tạo tập mới/i });

      // Initially title and slug are empty -> button disabled
      expect(submitBtn.hasAttribute("disabled")).toBe(true);

      // Typing title auto-slugifies
      fireEvent.change(titleInput, { target: { value: "Tập 1: Ly Cà Phê Đầu Tiên" } });
      expect((slugInput as HTMLInputElement).value).toBe("tap-1-ly-ca-phe-dau-tien");
      expect(submitBtn.hasAttribute("disabled")).toBe(false);

      // Clearing slug manually disables button
      fireEvent.change(slugInput, { target: { value: "   " } });
      expect(submitBtn.hasAttribute("disabled")).toBe(true);
    });

    it("submits correct invoke payload when form is submitted", async () => {
      const invokeSpy = vi.spyOn(tauriBridge, "invoke").mockImplementation(async (cmd: string, args?: any) => {
        if (cmd === "series_list") return mockSeries;
        if (cmd === "series_create_episode") {
          return {
            slug: args.episodeSlug,
            title: args.title,
            recipe: mockSeries[0].recipe,
            series: args.seriesSlug,
            artifacts_count: 0,
            has_uncommitted_changes: false,
            created_at: 1000,
            updated_at: 1000,
          };
        }
        return null;
      });

      const onCreated = vi.fn();
      const onClose = vi.fn();

      render(
        <CreateEpisodeDialog
          open={true}
          onClose={onClose}
          onCreated={onCreated}
          defaultSeriesSlug="series-test"
        />
      );

      await waitFor(() => {
        expect(screen.getByText(/Chuyện quán cà phê/)).toBeDefined();
      });

      const titleInput = screen.getByPlaceholderText(/Ví dụ: Tập 2: Chiến dịch Bạch Đằng/);
      const slugInput = screen.getByPlaceholderText(/tap-2-chien-dich-bach-dang/);
      const submitBtn = screen.getByRole("button", { name: /Tạo tập mới/i });

      fireEvent.change(titleInput, { target: { value: "Tập 1: Mở đầu" } });
      fireEvent.change(slugInput, { target: { value: "tap-01-mo-dau" } });

      fireEvent.click(submitBtn);

      await waitFor(() => {
        expect(invokeSpy).toHaveBeenCalledWith("series_create_episode", {
          seriesSlug: "series-test",
          episodeSlug: "tap-01-mo-dau",
          title: "Tập 1: Mở đầu",
        });
        expect(onCreated).toHaveBeenCalled();
        expect(onClose).toHaveBeenCalled();
      });
    });
  });

  describe("AttachProfileDialog", () => {
    const profiles = [
      { id: "nv-an", title: "Nhân vật An" },
      { id: "nv-ba", title: "Nhân vật Ba" },
    ];

    it("renders available profiles and target node info", () => {
      render(
        <AttachProfileDialog
          open={true}
          targetNodeId="thumb-01"
          availableProfiles={profiles}
          onClose={vi.fn()}
          onAttach={vi.fn()}
        />
      );

      expect(screen.getByText("Gắn hồ sơ tham chiếu")).toBeDefined();
      expect(screen.getByText("thumb-01")).toBeDefined();
      expect(screen.getByText(/Nhân vật An \(nv-an\)/)).toBeDefined();
    });

    it("shows empty state and disables attach button when no profiles available", () => {
      render(
        <AttachProfileDialog
          open={true}
          targetNodeId="thumb-01"
          availableProfiles={[]}
          onClose={vi.fn()}
          onAttach={vi.fn()}
        />
      );

      expect(screen.getByText(/Chưa có hồ sơ tham chiếu nào trong dự án hoặc series/)).toBeDefined();
      const attachBtn = screen.getByRole("button", { name: /Gắn hồ sơ/i });
      expect(attachBtn.hasAttribute("disabled")).toBe(true);
    });

    it("calls onAttach with selected profile id", () => {
      const onAttach = vi.fn();
      const onClose = vi.fn();

      render(
        <AttachProfileDialog
          open={true}
          targetNodeId="thumb-01"
          availableProfiles={profiles}
          onClose={onClose}
          onAttach={onAttach}
        />
      );

      const attachBtn = screen.getByRole("button", { name: /Gắn hồ sơ/i });
      expect(attachBtn.hasAttribute("disabled")).toBe(false);

      fireEvent.click(attachBtn);
      expect(onAttach).toHaveBeenCalledWith("nv-an");
      expect(onClose).toHaveBeenCalled();
    });
  });
});
