/**
 * Activity label formatter for live agent turns.
 * Formats tool names and inputs into human-readable Vietnamese action descriptions.
 */

function getBasename(pathStr: string): string {
  const parts = pathStr.replace(/\\/g, "/").split("/").filter(Boolean);
  return parts.pop() || pathStr;
}

function extractFilePath(input: unknown): string | undefined {
  if (typeof input !== "object" || input === null) return undefined;
  const obj = input as Record<string, unknown>;
  const raw =
    obj.TargetFile ??
    obj.AbsolutePath ??
    obj.file_path ??
    obj.path ??
    obj.filePath ??
    obj.file;
  return typeof raw === "string" && raw.trim() ? getBasename(raw.trim()) : undefined;
}

function extractCommand(input: unknown): string | undefined {
  if (typeof input !== "object" || input === null) return undefined;
  const obj = input as Record<string, unknown>;
  const raw = obj.CommandLine ?? obj.command ?? obj.cmd;
  if (typeof raw !== "string" || !raw.trim()) return undefined;
  const trimmed = raw.trim();
  return trimmed.length > 50 ? trimmed.slice(0, 50) + "…" : trimmed;
}

function extractQuery(input: unknown): string | undefined {
  if (typeof input !== "object" || input === null) return undefined;
  const obj = input as Record<string, unknown>;
  const raw = obj.query ?? obj.Query ?? obj.Url ?? obj.url;
  if (typeof raw !== "string" || !raw.trim()) return undefined;
  const trimmed = raw.trim();
  return trimmed.length > 50 ? trimmed.slice(0, 50) + "…" : trimmed;
}

function extractMcpToolName(name: string, input: unknown): string | undefined {
  if (name.startsWith("mcp__studio__")) {
    const sub = name.slice("mcp__studio__".length);
    if (sub) return sub;
  }
  if (typeof input === "object" && input !== null) {
    const obj = input as Record<string, unknown>;
    const raw = obj.name ?? obj.tool_name ?? obj.ToolName;
    if (typeof raw === "string" && raw.trim()) return raw.trim();
  }
  return undefined;
}

/**
 * Maps a tool call name and input to a concise Vietnamese activity description.
 * Supports both agy (Vietnamese names) and Claude (English names).
 */
export function describeActivity(name: string, input?: unknown): string {
  const trimmedName = (name || "").trim();

  // Read file
  if (
    trimmedName === "Đọc tệp" ||
    trimmedName === "Read" ||
    trimmedName === "read_file" ||
    trimmedName === "read"
  ) {
    const file = extractFilePath(input);
    return file ? `Đang đọc ${file}` : "Đang đọc tệp";
  }

  // Create / Write file
  if (
    trimmedName === "Tạo tệp" ||
    trimmedName === "Write" ||
    trimmedName === "write_file" ||
    trimmedName === "write" ||
    trimmedName === "create_file"
  ) {
    const file = extractFilePath(input);
    return file ? `Đang tạo ${file}` : "Đang tạo tệp";
  }

  // Edit file
  if (
    trimmedName === "Sửa tệp" ||
    trimmedName === "Edit" ||
    trimmedName === "edit_file" ||
    trimmedName === "edit"
  ) {
    const file = extractFilePath(input);
    return file ? `Đang sửa ${file}` : "Đang sửa tệp";
  }

  // Shell command
  if (
    trimmedName === "Chạy lệnh shell" ||
    trimmedName === "Bash" ||
    trimmedName === "bash" ||
    trimmedName === "run_command" ||
    trimmedName === "shell"
  ) {
    const cmd = extractCommand(input);
    return cmd ? `Đang chạy lệnh: ${cmd}` : "Đang chạy lệnh shell";
  }

  // Web search / browse / fetch
  if (
    trimmedName === "Tìm kiếm & duyệt web" ||
    trimmedName === "WebSearch" ||
    trimmedName === "WebFetch" ||
    trimmedName === "web_search" ||
    trimmedName === "search_web"
  ) {
    const q = extractQuery(input);
    return q ? `Đang tìm web: "${q}"` : "Đang tìm kiếm web";
  }

  // Studio tool
  if (
    trimmedName === "Gọi công cụ Studio" ||
    trimmedName.startsWith("mcp__studio__") ||
    trimmedName === "call_mcp_tool"
  ) {
    const subTool = extractMcpToolName(trimmedName, input);
    return subTool ? `Đang gọi công cụ Studio: ${subTool}` : "Đang gọi công cụ Studio";
  }

  // Subagent delegation
  if (
    trimmedName === "Điều phối tác vụ con" ||
    trimmedName === "Task" ||
    trimmedName === "task" ||
    trimmedName === "invoke_subagent"
  ) {
    return "Đang giao việc cho agent con";
  }

  // Find file / Glob / Grep
  if (
    trimmedName === "Tìm kiếm tệp" ||
    trimmedName === "Glob" ||
    trimmedName === "Grep" ||
    trimmedName === "glob" ||
    trimmedName === "grep" ||
    trimmedName === "find_by_name"
  ) {
    return "Đang tìm tệp";
  }

  // Fallback
  if (!trimmedName || trimmedName === "Công cụ") {
    return "Đang dùng công cụ";
  }

  return `Đang dùng công cụ ${trimmedName}`;
}
