import { describe, expect, it } from "vitest";
import { describeActivity, describeStep } from "../../../src/runtime/chat/activity-label";

describe("activity-label", () => {
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

    describe("Task C tool mappings (Vietnamese and raw engine names)", () => {
      it("formats 'Tạo ảnh' and 'generate_image'", () => {
        expect(describeActivity("Tạo ảnh")).toBe("Đang tạo ảnh");
        expect(describeActivity("generate_image")).toBe("Đang tạo ảnh");
      });

      it("formats 'Đọc trang web' and 'read_url_content'", () => {
        expect(describeActivity("Đọc trang web")).toBe("Đang đọc trang web");
        expect(describeActivity("read_url_content")).toBe("Đang đọc trang web");
      });

      it("formats 'Tìm kiếm web' and 'search_web'", () => {
        expect(describeActivity("Tìm kiếm web")).toBe("Đang tìm kiếm web");
        expect(describeActivity("Tìm kiếm web", { query: "vitest" })).toBe('Đang tìm web: "vitest"');
        expect(describeActivity("search_web")).toBe("Đang tìm kiếm web");
        expect(describeActivity("search_web", { query: "tauri" })).toBe('Đang tìm web: "tauri"');
      });

      it("formats 'Duyệt web' and browser tools", () => {
        expect(describeActivity("Duyệt web")).toBe("Đang duyệt web");
        expect(describeActivity("browser_navigate")).toBe("Đang duyệt web");
        expect(describeActivity("browser_click")).toBe("Đang duyệt web");
        expect(describeActivity("open_browser_url")).toBe("Đang duyệt web");
        expect(describeActivity("read_browser_page")).toBe("Đang duyệt web");
      });

      it("formats 'Chờ lệnh' and wait/command_status tools", () => {
        expect(describeActivity("Chờ lệnh")).toBe("Đang chờ lệnh");
        expect(describeActivity("command_status")).toBe("Đang chờ lệnh");
        expect(describeActivity("wait")).toBe("Đang chờ lệnh");
        expect(describeActivity("wait_5_seconds")).toBe("Đang chờ lệnh");
      });

      it("formats 'Hỏi người dùng' and ask_question/ask_permission", () => {
        expect(describeActivity("Hỏi người dùng")).toBe("Đang hỏi người dùng");
        expect(describeActivity("ask_question")).toBe("Đang hỏi người dùng");
        expect(describeActivity("ask_permission")).toBe("Đang hỏi người dùng");
      });

      it("formats 'Chạy quy trình' and run_workflow", () => {
        expect(describeActivity("Chạy quy trình")).toBe("Đang chạy quy trình");
        expect(describeActivity("run_workflow")).toBe("Đang chạy quy trình");
      });

      it("formats 'Đọc tài nguyên' and list_resources/read_resource", () => {
        expect(describeActivity("Đọc tài nguyên")).toBe("Đang đọc tài nguyên");
        expect(describeActivity("list_resources")).toBe("Đang đọc tài nguyên");
        expect(describeActivity("read_resource")).toBe("Đang đọc tài nguyên");
      });

      it("formats 'view_file' as 'Đọc tệp'", () => {
        expect(describeActivity("view_file", { path: "dan-y.md" })).toBe("Đang đọc dan-y.md");
        expect(describeActivity("view_file")).toBe("Đang đọc tệp");
      });

      it("formats subagent management tools as 'Điều phối tác vụ con'", () => {
        expect(describeActivity("manage_task")).toBe("Đang giao việc cho agent con");
        expect(describeActivity("define_subagent")).toBe("Đang giao việc cho agent con");
        expect(describeActivity("manage_subagents")).toBe("Đang giao việc cho agent con");
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

  describe("describeStep", () => {
    it("converts progressive 'Đang ...' to capitalized neutral action", () => {
      expect(describeStep("Đọc tệp", { path: "script.md" })).toBe("Đọc script.md");
      expect(describeStep("Tạo tệp", { path: "outline.md" })).toBe("Tạo outline.md");
      expect(describeStep("Sửa tệp", { path: "script.md" })).toBe("Sửa script.md");
      expect(describeStep("Chạy lệnh shell", { CommandLine: "npm run build" })).toBe(
        "Chạy lệnh: npm run build"
      );
      expect(describeStep("Tạo ảnh")).toBe("Tạo ảnh");
      expect(describeStep("generate_image")).toBe("Tạo ảnh");
    });

    it("formats Task C actions in neutral form", () => {
      expect(describeStep("Đọc trang web")).toBe("Đọc trang web");
      expect(describeStep("read_url_content")).toBe("Đọc trang web");
      expect(describeStep("Tìm kiếm web")).toBe("Tìm kiếm web");
      expect(describeStep("Tìm kiếm web", { query: "react" })).toBe('Tìm web: "react"');
      expect(describeStep("Duyệt web")).toBe("Duyệt web");
      expect(describeStep("browser_navigate")).toBe("Duyệt web");
      expect(describeStep("Chờ lệnh")).toBe("Chờ lệnh");
      expect(describeStep("Hỏi người dùng")).toBe("Hỏi người dùng");
      expect(describeStep("Chạy quy trình")).toBe("Chạy quy trình");
      expect(describeStep("Đọc tài nguyên")).toBe("Đọc tài nguyên");
      expect(describeStep("Điều phối tác vụ con")).toBe("Giao việc cho agent con");
      expect(describeStep("Tìm kiếm tệp")).toBe("Tìm tệp");
    });

    it("formats fallback tool descriptions correctly", () => {
      expect(describeStep("Công cụ")).toBe("Dùng công cụ");
      expect(describeStep("my_custom_tool")).toBe("Dùng công cụ my_custom_tool");
      expect(describeStep("")).toBe("Dùng công cụ");
    });
  });
});
