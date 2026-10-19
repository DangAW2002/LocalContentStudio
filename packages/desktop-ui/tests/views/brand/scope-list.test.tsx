// @vitest-environment jsdom
import React from "react";
import { describe, expect, it, vi, afterEach } from "vitest";
import { render, screen, fireEvent, cleanup } from "@testing-library/react";
import { ScopeList } from "../../../src/views/brand/ScopeList";
import type { ScopeItem } from "../../../src/api/types";

describe("ScopeList component", () => {
  afterEach(() => {
    cleanup();
  });

  const mockItems: ScopeItem[] = [
    {
      id: "brand-app",
      name: "Thương hiệu Gốc",
      layer: "app",
      path: "app/brands/brand-app",
      summary: "Mẫu gốc từ App",
    },
    {
      id: "brand-org",
      name: "Thương hiệu Tổ Chức",
      layer: "org",
      path: "org/brands/brand-org",
      summary: "Mẫu từ tổ chức",
    },
    {
      id: "brand-override",
      name: "Thương hiệu Ghi Đè",
      layer: "workspace",
      path: "workspace/brands/brand-override",
      summary: "Bản sửa đổi trong workspace",
      isOverride: true,
    },
    {
      id: "broken-item",
      name: "Mục Lỗi",
      layer: "workspace",
      path: "workspace/channels/broken.yaml",
      error: "Dòng 5: Cú pháp YAML không hợp lệ",
    },
  ];

  it("renders list items and shows correct layer badges", () => {
    const handleSelect = vi.fn();
    render(
      <ScopeList
        items={mockItems}
        selectedId="brand-app"
        onSelectId={handleSelect}
      />
    );

    expect(screen.getByText("Thương hiệu Gốc")).toBeDefined();
    expect(screen.getByText("Thương hiệu Tổ Chức")).toBeDefined();
    expect(screen.getByText("Thương hiệu Ghi Đè")).toBeDefined();
    expect(screen.getByText("Mặc định ứng dụng")).toBeDefined(); // App layer
    expect(screen.getByText("Tổ chức")).toBeDefined(); // Org layer
  });

  it("shows 'Đang ghi đè' badge when isOverride is true", () => {
    const handleSelect = vi.fn();
    render(
      <ScopeList
        items={mockItems}
        selectedId="brand-app"
        onSelectId={handleSelect}
      />
    );

    expect(screen.getByText("Đang ghi đè")).toBeDefined();
  });

  it("shows 'Lỗi cấu hình' badge when error is present", () => {
    const handleSelect = vi.fn();
    render(
      <ScopeList
        items={mockItems}
        selectedId="brand-app"
        onSelectId={handleSelect}
      />
    );

    expect(screen.getByText("Lỗi cấu hình")).toBeDefined();
    expect(screen.getByText("Dòng 5: Cú pháp YAML không hợp lệ")).toBeDefined();
  });

  it("calls onSelectId when an item is clicked", () => {
    const handleSelect = vi.fn();
    render(
      <ScopeList
        items={mockItems}
        selectedId="brand-app"
        onSelectId={handleSelect}
      />
    );

    fireEvent.click(screen.getByText("Thương hiệu Ghi Đè"));
    expect(handleSelect).toHaveBeenCalledWith("brand-override");
  });

  it("renders 'Mới' button and fires onNewItem callback", () => {
    const handleNew = vi.fn();
    render(
      <ScopeList
        items={mockItems}
        selectedId="brand-app"
        onSelectId={vi.fn()}
        onNewItem={handleNew}
      />
    );

    const newBtn = screen.getByText("Mới");
    expect(newBtn).toBeDefined();
    fireEvent.click(newBtn);
    expect(handleNew).toHaveBeenCalledTimes(1);
  });
});
