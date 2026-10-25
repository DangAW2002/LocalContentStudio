// @vitest-environment jsdom
import React from "react";
import { describe, expect, it, afterEach, beforeEach, vi } from "vitest";
import { render, screen, fireEvent, cleanup, waitFor } from "@testing-library/react";
import * as tauriBridge from "../../src/api/tauri-bridge";
import { WorkspaceSwitcherModal, slugifyName } from "../../src/components/WorkspaceSwitcherModal";

const listings: Record<string, unknown> = {
  "": {
    path: "/home/dang",
    parent: "/home",
    home: "/home/dang",
    isWorkspace: false,
    entries: [
      { name: "kenh-a", path: "/home/dang/kenh-a", isWorkspace: true },
      { name: "src", path: "/home/dang/src", isWorkspace: false },
    ],
  },
  "/home/dang/src": {
    path: "/home/dang/src",
    parent: "/home/dang",
    home: "/home/dang",
    isWorkspace: false,
    entries: [],
  },
};

describe("WorkspaceSwitcherModal (web)", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    vi.spyOn(tauriBridge, "isTauri").mockReturnValue(false);
    vi.spyOn(tauriBridge, "invoke").mockImplementation(async (cmd: string, args?: any) => {
      if (cmd === "fs_list_dirs") return listings[args?.path ?? ""];
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

  it("suggests a path under the server home when typing a name", async () => {
    const props = renderModal();
    fireEvent.click(screen.getByText("Tạo mới..."));
    await waitFor(() => expect(tauriBridge.invoke).toHaveBeenCalledWith("fs_list_dirs", { path: null }));
    await waitFor(() => expect(screen.getByPlaceholderText(/\/home\/dang\/Studio/)).toBeTruthy());

    fireEvent.change(screen.getByLabelText("Tên Workspace"), { target: { value: "Kênh Công Nghệ" } });
    const pathInput = screen.getByLabelText("Đường dẫn thư mục") as HTMLInputElement;
    expect(pathInput.value).toBe("/home/dang/Studio/kenh-cong-nghe");

    fireEvent.click(screen.getByText("Tạo và mở"));
    await waitFor(() =>
      expect(props.onCreateWorkspace).toHaveBeenCalledWith("/home/dang/Studio/kenh-cong-nghe", "Kênh Công Nghệ"),
    );
  });

  it("keeps a manually edited path when the name changes", async () => {
    renderModal();
    fireEvent.click(screen.getByText("Tạo mới..."));
    await waitFor(() => expect(screen.getByPlaceholderText(/\/home\/dang\/Studio/)).toBeTruthy());
    const pathInput = screen.getByLabelText("Đường dẫn thư mục") as HTMLInputElement;
    fireEvent.change(pathInput, { target: { value: "/srv/ws" } });
    fireEvent.change(screen.getByLabelText("Tên Workspace"), { target: { value: "Khác" } });
    expect(pathInput.value).toBe("/srv/ws");
  });

  it("browses server folders and opens the chosen one", async () => {
    const props = renderModal();
    fireEvent.click(screen.getByText("Mở thư mục..."));
    fireEvent.click(screen.getByText("Chọn thư mục..."));

    await waitFor(() => expect(screen.getByText("kenh-a")).toBeTruthy());
    expect(screen.getByText("Workspace")).toBeTruthy();

    fireEvent.click(screen.getByText("src"));
    await waitFor(() => expect(screen.getByText("Không có thư mục con.")).toBeTruthy());
    fireEvent.click(screen.getByText("Mở thư mục này"));

    await waitFor(() => expect(props.onOpenWorkspace).toHaveBeenCalledWith("/home/dang/src"));
  });

  it("offers quick 1-click default workspace creation when recents is empty", async () => {
    const props = renderModal();
    fireEvent.click(screen.getByText("Tạo mới..."));
    await waitFor(() => expect(screen.getByText("Tạo nhanh Cá Nhân")).toBeTruthy());
    fireEvent.click(screen.getByText("Tạo nhanh Cá Nhân"));
    await waitFor(() =>
      expect(props.onCreateWorkspace).toHaveBeenCalledWith("/home/dang/Studio/ca-nhan", "Studio Cá Nhân"),
    );
  });

  it("creates a new subfolder in FolderBrowser", async () => {
    vi.spyOn(tauriBridge, "invoke").mockImplementation(async (cmd: string, args?: any) => {
      if (cmd === "fs_list_dirs") return listings[args?.path ?? ""];
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

    renderModal();
    fireEvent.click(screen.getByText("Mở thư mục..."));
    fireEvent.click(screen.getByText("Chọn thư mục..."));
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
