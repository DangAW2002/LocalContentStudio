// @vitest-environment jsdom
import React from "react";
import { describe, expect, it, afterEach, beforeEach, vi } from "vitest";
import { render, screen, fireEvent, cleanup } from "@testing-library/react";
import { ChannelDetail } from "../../../src/views/brand/ChannelDetail";
import type { ScopeItem } from "../../../src/api/types";

describe("ChannelDetail component", () => {
  beforeEach(() => {
    window.HTMLElement.prototype.scrollIntoView = vi.fn();
  });

  afterEach(() => {
    cleanup();
  });

  const sampleYaml = `
id: test-channel
name: "Kênh Test"
description: "Mô tả kênh test"
formats:
  - id: format-1
    name: "Video Dọc"
    width: 1080
    height: 1920
    ratio: "9:16"
    fps: 30
    duration_max_s: 60
    safe_zone:
      top: 0.15
      bottom: 0.25
      left: 0.05
      right: 0.05
limits:
  title_max_chars: 100
  desc_max_chars: 2000
  captions_required: true
files:
  max_size_mb: 250
`;

  const baseItem: ScopeItem = {
    id: "test-channel",
    name: "Kênh Test",
    layer: "workspace",
    isOverride: true,
  };

  it("renders format details and calculates ratio and safe zone preview", () => {
    render(
      <ChannelDetail
        selectedItem={baseItem}
        rawText={sampleYaml}
      />
    );

    // Form inputs rendered
    expect(screen.getByDisplayValue("Kênh Test")).toBeDefined();
    expect(screen.getByDisplayValue("1080")).toBeDefined();
    expect(screen.getByDisplayValue("1920")).toBeDefined();
    expect(screen.getByDisplayValue("9:16")).toBeDefined();

    // Safe zone preview inputs
    expect(screen.getByDisplayValue("0.15")).toBeDefined();
    expect(screen.getByDisplayValue("0.25")).toBeDefined();

    // Revert override button visible when isOverride is true
    expect(screen.getByText("Bỏ bản ghi đè")).toBeDefined();
  });

  it("updates aspect ratio automatically when width or height changes", () => {
    render(
      <ChannelDetail
        selectedItem={baseItem}
        rawText={sampleYaml}
      />
    );

    const widthInput = screen.getByDisplayValue("1080");
    const heightInput = screen.getByDisplayValue("1920");

    // Change to 1920x1080 (16:9)
    fireEvent.change(widthInput, { target: { value: "1920" } });
    fireEvent.change(heightInput, { target: { value: "1080" } });

    expect(screen.getByDisplayValue("16:9")).toBeDefined();

    // Change to 1080x1080 (1:1)
    fireEvent.change(widthInput, { target: { value: "1080" } });
    expect(screen.getByDisplayValue("1:1")).toBeDefined();
  });
});
