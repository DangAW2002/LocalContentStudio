// @vitest-environment jsdom
import React from "react";
import { describe, expect, it, afterEach, beforeEach, vi } from "vitest";
import { render, screen, fireEvent, cleanup, waitFor } from "@testing-library/react";
import * as tauriBridge from "../../src/api/tauri-bridge";
import { WorkspaceSwitcherModal, slugifyName } from "../../src/components/WorkspaceSwitcherModal";
import { FolderBrowser } from "../../src/components/FolderBrowser";

const mockRootResponse = {
  root: "/home/dang/Studio",
  workspaces: [
    { slug: "kenh-a", name: "Kênh A", path: "/home/dang/Studio/kenh-a", isWorkspace: true, isCurrent: false },
    { slug: "ca-nhan", name: "Studio Cá Nhân", path: "/home/dang/Studio/ca-nhan", isWorkspace: true, isCurrent: true },
  ],
};

describe("WorkspaceSwitcherModal (Sandboxed)", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    vi.spyOn(tauriBridge, "isTauri").mockReturnValue(false);
    vi.spyOn(tauriBridge, "invoke").mockImplementation(async (cmd: string, args?: any) => {
      if (cmd === "workspaces_root_get") return mockRootResponse;
      if (cmd === "workspaces_root_set") {
        return {
          root: args.path,
          workspaces: [],
        };
      }
      return null;
    });
  });

  afterEach(() => cleanup());

  const renderModal = (overrides: Partial<React.ComponentProps<typeof WorkspaceSwitcherModal>> = {}) => {
    const props = {
      open: true,
      onClose: vi.fn(),
      recentWorkspaces: [],
      onOpenWorkspace: vi.fn().mockResolvedValue(undefined),
      onCreateWorkspace: vi.fn().mockResolvedValue(undefined),
      ...overrides,
    };
    render(<WorkspaceSwitcherModal {...props} />);
    return props;
  };

  it("slugifies Vietnamese names into folder names", () => {
    expect(slugifyName("Kênh Công Nghệ")).toBe("kenh-cong-nghe");
    expect(slugifyName("  Đời sống & AI! ")).toBe("doi-song-ai");
    expect(slugifyName("!!!")).toBe("");
  });

  it("creates a new workspace directly without exposing system paths", async () => {
    const props = renderModal();
    await waitFor(() => expect(screen.getByPlaceholderText(/Nhập tên workspace/)).toBeTruthy());

    const input = screen.getByPlaceholderText(/Nhập tên workspace/);
    fireEvent.change(input, { target: { value: "Kênh Công Nghệ" } });

    fireEvent.click(screen.getByText("Tạo Workspace"));
    await waitFor(() =>
      expect(props.onCreateWorkspace).toHaveBeenCalledWith("", "Kênh Công Nghệ"),
    );
  });

  it("lists sandboxed workspaces from Studio root and opens the chosen one", async () => {
    const props = renderModal();
    await waitFor(() => expect(screen.getByText("Kênh A")).toBeTruthy());
    expect(screen.getByText("Studio Cá Nhân")).toBeTruthy();
    expect(screen.getByText("Đang mở")).toBeTruthy();

    fireEvent.click(screen.getByText("Chuyển đến"));
    await waitFor(() => expect(props.onOpenWorkspace).toHaveBeenCalledWith("/home/dang/Studio/kenh-a"));
  });

  it("offers 1-click quick create when workspaces list is empty", async () => {
    vi.spyOn(tauriBridge, "invoke").mockImplementation(async (cmd: string) => {
      if (cmd === "workspaces_root_get") {
        return { root: "/home/dang/Studio", workspaces: [] };
      }
      return null;
    });

    const props = renderModal();
    await waitFor(() => expect(screen.getByText('Tạo nhanh "Studio Cá Nhân"')).toBeTruthy());

    fireEvent.click(screen.getByText('Tạo nhanh "Studio Cá Nhân"'));
    await waitFor(() =>
      expect(props.onCreateWorkspace).toHaveBeenCalledWith("", "Studio Cá Nhân"),
    );
  });

  it("allows customizing the Studio root storage", async () => {
    renderModal();
    await waitFor(() => expect(screen.getByText("Đổi kho...")).toBeTruthy());

    fireEvent.click(screen.getByText("Đổi kho..."));
    const rootInput = screen.getByPlaceholderText("Đường dẫn kho Studio mới...");
    fireEvent.change(rootInput, { target: { value: "/data/Studio" } });
    fireEvent.click(screen.getByText("Lưu"));

    await waitFor(() => {
      expect(tauriBridge.invoke).toHaveBeenCalledWith("workspaces_root_set", { path: "/data/Studio" });
    });
  });

  it("creates a new subfolder in FolderBrowser component", async () => {
    vi.spyOn(tauriBridge, "invoke").mockImplementation(async (cmd: string, args?: any) => {
      if (cmd === "fs_list_dirs") {
        return {
          path: "/home/dang",
          parent: "/home",
          home: "/home/dang",
          isWorkspace: false,
          entries: [{ name: "src", path: "/home/dang/src", isWorkspace: false }],
        };
      }
      if (cmd === "fs_create_dir") {
        return {
          path: args.parent,
          parent: "/home",
          home: "/home/dang",
          isWorkspace: false,
          entries: [{ name: args.name, path: `${args.parent}/${args.name}`, isWorkspace: false }],
        };
      }
      return null;
    });

    const onSelect = vi.fn();
    const onCancel = vi.fn();
    render(<FolderBrowser onSelect={onSelect} onCancel={onCancel} />);

    await waitFor(() => expect(screen.getByText("Thư mục mới")).toBeTruthy());
    fireEvent.click(screen.getByText("Thư mục mới"));

    const input = screen.getByPlaceholderText("Tên thư mục mới...");
    fireEvent.change(input, { target: { value: "thu-muc-moi" } });
    fireEvent.click(screen.getByText("Tạo"));

    await waitFor(() => {
      expect(tauriBridge.invoke).toHaveBeenCalledWith("fs_create_dir", {
        parent: "/home/dang",
        name: "thu-muc-moi",
      });
      expect(screen.getByText("thu-muc-moi")).toBeTruthy();
    });
  });
});
