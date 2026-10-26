import { describe, expect, it } from "vitest";
import { describeActivity } from "../../../src/runtime/chat/activity-label";

describe("describeActivity", () => {
  describe("agy Vietnamese tool names", () => {
    it("formats 'Đọc tệp' with various input path formats", () => {
      expect(describeActivity("Đọc tệp", { path: "script.md" })).toBe("Đang đọc script.md");
      expect(describeActivity("Đọc tệp", { file_path: "src/nested/script.md" })).toBe("Đang đọc script.md");
      expect(describeActivity("Đọc tệp", { TargetFile: "C:\\Windows\\Path\\file.txt" })).toBe("Đang đọc file.txt");
      expect(describeActivity("Đọc tệp", { AbsolutePath: "/unix/path/note.md" })).toBe("Đang đọc note.md");
      expect(describeActivity("Đọc tệp", {})).toBe("Đang đọc tệp");
      expect(describeActivity("Đọc tệp")).toBe("Đang đọc tệp");
    });

    it("formats 'Tạo tệp' with various input path formats", () => {
      expect(describeActivity("Tạo tệp", { path: "outline.md" })).toBe("Đang tạo outline.md");
      expect(describeActivity("Tạo tệp", { AbsolutePath: "C:\\project\\outline.md" })).toBe("Đang tạo outline.md");
      expect(describeActivity("Tạo tệp", {})).toBe("Đang tạo tệp");
    });

    it("formats 'Sửa tệp' with various input path formats", () => {
      expect(describeActivity("Sửa tệp", { path: "script.md" })).toBe("Đang sửa script.md");
      expect(describeActivity("Sửa tệp", { file_path: "src/script.md" })).toBe("Đang sửa script.md");
      expect(describeActivity("Sửa tệp", {})).toBe("Đang sửa tệp");
    });

    it("formats 'Chạy lệnh shell' with command extraction and truncation", () => {
      expect(describeActivity("Chạy lệnh shell", { CommandLine: "npm run build" })).toBe(
        "Đang chạy lệnh: npm run build"
      );
      expect(describeActivity("Chạy lệnh shell", { command: "git status" })).toBe(
        "Đang chạy lệnh: git status"
      );

      // Long command > 50 characters gets truncated to 50 chars + "…"
      const longCmd = "ffmpeg -i input.mp4 -vf scale=1280:720 -c:v libx264 -crf 23 -c:a aac -b:a 192k output.mp4";
      const result = describeActivity("Chạy lệnh shell", { CommandLine: longCmd });
      expect(result.startsWith("Đang chạy lệnh: ")).toBe(true);
      expect(result.endsWith("…")).toBe(true);
      expect(result).toBe(`Đang chạy lệnh: ${longCmd.slice(0, 50)}…`);

      expect(describeActivity("Chạy lệnh shell", {})).toBe("Đang chạy lệnh shell");
    });

    it("formats 'Tìm kiếm & duyệt web' with query extraction", () => {
      expect(describeActivity("Tìm kiếm & duyệt web", { query: "react hooks" })).toBe(
        'Đang tìm web: "react hooks"'
      );
      expect(describeActivity("Tìm kiếm & duyệt web", { Query: "vite config" })).toBe(
        'Đang tìm web: "vite config"'
      );
      expect(describeActivity("Tìm kiếm & duyệt web", { Url: "https://google.com" })).toBe(
        'Đang tìm web: "https://google.com"'
      );
      expect(describeActivity("Tìm kiếm & duyệt web", {})).toBe("Đang tìm kiếm web");
    });

    it("formats 'Gọi công cụ Studio' with subtool name", () => {
      expect(describeActivity("Gọi công cụ Studio", { name: "render_video" })).toBe(
        "Đang gọi công cụ Studio: render_video"
      );
      expect(describeActivity("Gọi công cụ Studio", { tool_name: "generate_image" })).toBe(
        "Đang gọi công cụ Studio: generate_image"
      );
      expect(describeActivity("Gọi công cụ Studio", { ToolName: "export_pdf" })).toBe(
        "Đang gọi công cụ Studio: export_pdf"
      );
      expect(describeActivity("Gọi công cụ Studio", {})).toBe("Đang gọi công cụ Studio");
    });

    it("formats 'Điều phối tác vụ con'", () => {
      expect(describeActivity("Điều phối tác vụ con")).toBe("Đang giao việc cho agent con");
      expect(describeActivity("Điều phối tác vụ con", { task: "research" })).toBe("Đang giao việc cho agent con");
    });

    it("formats 'Tìm kiếm tệp'", () => {
      expect(describeActivity("Tìm kiếm tệp")).toBe("Đang tìm tệp");
    });

    it("formats generic 'Công cụ'", () => {
      expect(describeActivity("Công cụ")).toBe("Đang dùng công cụ");
    });
  });

  describe("Claude English tool names", () => {
    it("formats Read, Write, Edit", () => {
      expect(describeActivity("Read", { TargetFile: "foo.txt" })).toBe("Đang đọc foo.txt");
      expect(describeActivity("read_file", { path: "foo.txt" })).toBe("Đang đọc foo.txt");
      expect(describeActivity("Write", { path: "bar.ts" })).toBe("Đang tạo bar.ts");
      expect(describeActivity("write_file", { path: "bar.ts" })).toBe("Đang tạo bar.ts");
      expect(describeActivity("Edit", { path: "baz.ts" })).toBe("Đang sửa baz.ts");
      expect(describeActivity("edit_file", { path: "baz.ts" })).toBe("Đang sửa baz.ts");
    });

    it("formats Bash", () => {
      expect(describeActivity("Bash", { command: "cargo test" })).toBe("Đang chạy lệnh: cargo test");
      expect(describeActivity("bash", { command: "pnpm dev" })).toBe("Đang chạy lệnh: pnpm dev");
    });

    it("formats WebSearch and WebFetch", () => {
      expect(describeActivity("WebSearch", { query: "tauri v2" })).toBe('Đang tìm web: "tauri v2"');
      expect(describeActivity("WebFetch", { url: "https://tauri.app" })).toBe('Đang tìm web: "https://tauri.app"');
    });

    it("formats Glob and Grep", () => {
      expect(describeActivity("Glob", { pattern: "**/*.rs" })).toBe("Đang tìm tệp");
      expect(describeActivity("Grep", { pattern: "fn main" })).toBe("Đang tìm tệp");
    });

    it("formats Task", () => {
      expect(describeActivity("Task", { description: "do something" })).toBe("Đang giao việc cho agent con");
    });

    it("formats mcp__studio__* tools", () => {
      expect(describeActivity("mcp__studio__render_video")).toBe("Đang gọi công cụ Studio: render_video");
      expect(describeActivity("mcp__studio__get_artifact")).toBe("Đang gọi công cụ Studio: get_artifact");
    });
  });

  describe("fallback behavior", () => {
    it("handles unknown or custom tools", () => {
      expect(describeActivity("custom_pipeline")).toBe("Đang dùng công cụ custom_pipeline");
      expect(describeActivity("blender_exec", { script: "cube.py" })).toBe("Đang dùng công cụ blender_exec");
    });

    it("handles empty or missing name", () => {
      expect(describeActivity("")).toBe("Đang dùng công cụ");
      expect(describeActivity(null as any)).toBe("Đang dùng công cụ");
    });
  });
});
