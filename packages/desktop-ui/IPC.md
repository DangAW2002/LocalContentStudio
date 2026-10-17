# Local Content Studio — Tauri IPC Contract

Phiên bản: v2 (2026-10-16)  
Tài liệu này định nghĩa contract giao tiếp giữa Desktop UI (React 18 + TS) và Desktop Core (Rust Tauri 2) qua Tauri IPC.
Tất cả tên command Tauri ở dạng `snake_case`, các tham số và trường payload TypeScript ở dạng `camelCase`.

---

## 1. Commands

### 1.1 Quản lý Workspace

#### `app_recent_workspaces`
- **Mục đích**: Lấy danh sách các workspace đã mở gần đây.
- **Input**: None
- **Output**: `WorkspaceSummary[]`
```ts
interface WorkspaceSummary {
  path: string;
  name: string;
  lastOpenedAt?: string;
  isCurrent?: boolean;
}
```

#### `workspace_create`
- **Mục đích**: Khởi tạo workspace mới tại thư mục được chỉ định.
- **Input**: `{ path: string, name: string }`
- **Output**: `WorkspaceSummary`

#### `workspace_open`
- **Mục đích**: Mở workspace có sẵn từ đường dẫn trên đĩa.
- **Input**: `{ path: string }`
- **Output**: `WorkspaceSummary`

#### `workspace_current`
- **Mục đích**: Lấy thông tin workspace hiện tại cùng thông tin tổ chức và ngân sách.
- **Input**: None
- **Output**: `WorkspaceCurrentResponse`
```ts
interface WorkspaceCurrentResponse {
  workspace: WorkspaceSummary;
  org?: { id: string; name: string; path: string };
  budget?: { currentSpendUsd: number; maxSpendUsd: number; warningThreshold: number };
}
```

---

### 1.2 Quản lý Dự án & Tệp

#### `project_list`
- **Mục đích**: Liệt kê các dự án trong workspace hiện tại.
- **Input**: None
- **Output**: `ProjectSummary[]`
```ts
interface ProjectSummary {
  slug: string;
  title: string;
  kit?: string;
  modules?: string[];
  status: "active" | "archived" | "completed";
  createdAt: string;
  updatedAt: string;
  artifactCount?: number;
}
```

#### `project_create`
- **Mục đích**: Tạo dự án mới kèm slug và kit/module ban đầu.
- **Input**: `{ slug: string, title: string, kit?: string, modules?: string[] }`
- **Output**: `ProjectSummary`

#### `project_files`
- **Mục đích**: Liệt kê cấu trúc tệp thuộc dự án.
- **Input**: `{ project: string }`
- **Output**: `ProjectFileEntry[]`
```ts
interface ProjectFileEntry {
  path: string;
  name: string;
  isDir: boolean;
  size: number;
  kind: "markdown" | "text" | "image" | "video" | "audio" | "html" | "code" | "json" | "unknown";
  updatedAt?: string;
}
```

#### `file_read`
- **Mục đích**: Đọc nội dung tệp hoặc lấy asset URL an toàn cho media (hỗ trợ đường dẫn relative theo project hoặc workspace-relative).
- **Input**: `{ project?: string, path: string }`
- **Output**: `FileReadResult`
```ts
interface FileReadResult {
  kind: "markdown" | "text" | "image" | "video" | "audio" | "html" | "code" | "json" | "unknown";
  mime: string;
  text?: string;
  assetUrl?: string;
  size: number;
}
```

#### `file_write`
- **Mục đích**: Ghi nội dung văn bản vào tệp nguồn (sửa trực tiếp trên Stage; hỗ trợ relative theo project hoặc workspace).
- **Input**: `{ project?: string, path: string, text: string }`
- **Output**: `{ path: string, size: number }`
```ts
interface FileWriteResult {
  path: string;
  size: number;
}
```

---

### 1.3 Thread, Agent Engine & Lượt thực thi (Turn)

#### `thread_list`
- **Mục đích**: Lấy danh sách hội thoại của một dự án.
- **Input**: `{ project: string }`
- **Output**: `ThreadSummary[]`
```ts
interface ThreadSummary {
  id: string;
  project: string;
  engine: "claude" | "agy";
  model?: string;
  title?: string;
  turnCount: number;
  createdAt: string;
  updatedAt: string;
}
```

#### `thread_create`
- **Mục đích**: Tạo thread mới với engine đã chọn (Claude hoặc agy).
- **Input**: `{ project: string, engine: "claude" | "agy", model?: string }`
- **Output**: `ThreadSummary`

#### `thread_events`
- **Mục đích**: Lấy lịch sử sự kiện của một thread.
- **Input**: `{ threadId: string, cursor?: string, limit?: number }`
- **Output**: `{ events: EngineTurnEvent[], nextCursor?: string, hasMore: boolean }`

#### `turn_start`
- **Mục đích**: Gửi prompt bắt đầu một lượt xử lý mới của agent.
- **Input**: `{ threadId: string, prompt: string }`
- **Output**: `{ turnId: string }`

#### `turn_cancel`
- **Mục đích**: Hủy lượt đang chạy (kill process tree của engine adapter).
- **Input**: `{ turnId: string }`
- **Output**: `{ success: boolean }`

#### `turn_undo`
- **Mục đích**: Hoàn tác các sửa đổi file của một turn về checkpoint trước đó.
- **Input**: `{ turnId: string }`
- **Output**: `{ success: boolean, restoredTurnId?: string }`

#### `permission_respond`
- **Mục đích**: Phản hồi yêu cầu cấp quyền tool (allow / deny).
- **Input**: `{ requestId: string, allow: boolean }`
- **Output**: `{ acknowledged: boolean }`

---

### 1.4 Checkpoint (CAS)

#### `checkpoint_list`
- **Mục đích**: Lấy danh sách các điểm khôi phục snapshot.
- **Input**: None
- **Output**: `CheckpointSummary[]`
```ts
interface CheckpointSummary {
  id: string;
  timestamp: string;
  message: string;
  filesChanged: number;
  author?: string;
}
```

#### `checkpoint_restore`
- **Mục đích**: Phục hồi trạng thái tệp về một checkpoint.
- **Input**: `{ id: string }`
- **Output**: `{ success: boolean, restoredCheckpointId: string }`

#### `checkpoint_create`
- **Mục đích**: Tạo điểm kiểm soát CAS thủ công (ví dụ trước khi sửa trực tiếp trên Stage).
- **Input**: `{ label?: string }`
- **Output**: `{ id: string }`

---

### 1.5 Engines Probe & Catalog Skill

#### `engine_models`
- **Mục đích**: Danh sách model engine chấp nhận cho `--model`. claude: alias họ model (`opus`, `sonnet`, `haiku`, `fable`, luôn trỏ bản mới nhất); agy: lấy trực tiếp từ `agy models` (cache 10 phút). Phần tử đầu là mặc định.
- **Input**: `{ engine: "claude" | "agy" }`
- **Output**: `{ value: string; label: string }[]`

#### `engines_probe`
- **Mục đích**: Kiểm tra tình trạng cài đặt và khả năng tương thích của các công cụ bên ngoài (claude, agy, ffmpeg, chromium, node).
- **Input**: None
- **Output**: `EngineProbeResult[]`
```ts
interface EngineProbeResult {
  id: string;
  name: string;
  installed: boolean;
  version?: string;
  path?: string;
  supported: boolean;
  notes?: string;
}
```

#### `catalog_kits`
- **Mục đích**: Lấy danh mục các kit sản phẩm có sẵn.
- **Input**: None
- **Output**: `CatalogKit[]`
```ts
interface CatalogKit {
  id: string;
  name: string;
  description: string;
  modules: string[];
  tokenBudget: number;
  tags: string[];
}
```

#### `catalog_modules`
- **Mục đích**: Lấy danh mục các module kỹ năng đã được tuyển chọn.
- **Input**: None
- **Output**: `CatalogModule[]`
```ts
interface CatalogModule {
  id: string;
  name: string;
  description: string;
  capabilities: string[];
  skillCount: number;
  tokenCount: number;
}
```

#### `catalog_suggest`
- **Mục đích**: Tự động đề xuất kit và module dựa trên văn bản yêu cầu người dùng gõ.
- **Input**: `{ text: string }`
- **Output**: `{ suggestedKits: CatalogKit[], suggestedModules: CatalogModule[] }`

#### `project_modules_set`
- **Mục đích**: Cập nhật danh sách module kích hoạt cho một dự án.
- **Input**: `{ project: string, modules: string[] }`
- **Output**: `{ project: string, modules: string[], totalTokenBudget: number }`

---

### 1.6 Phạm vi (Scope) & Thư viện (Assets)

#### `scope_list`
- **Mục đích**: Lấy danh sách các thực thể từ chuỗi kế thừa App -> Tổ chức -> Workspace.
- **Input**: `{ kind: "brands" | "channels" | "playbooks" | "skills" | "tools" | "capabilities" }`
- **Output**: `ScopeItem[]`
```ts
interface ScopeItem {
  id: string;
  name: string;
  kind: string;
  layer: "app" | "org" | "workspace";
  path?: string;
  summary?: string;
  metadata?: Record<string, unknown>;
}
```

#### `asset_list`
- **Mục đích**: Danh sách tài nguyên trong thư viện workspace.
- **Input**: `{ filter?: { kind?: string, search?: string, limit?: number } }`
- **Output**: `AssetItem[]`
```ts
interface AssetItem {
  id: string;
  name: string;
  kind: "image" | "video" | "audio" | "document" | "font" | "other";
  mime: string;
  size: number;
  path: string;
  previewUrl?: string;
  createdAt: string;
  metadata?: Record<string, unknown>;
}
```

---

### 1.7 Tác vụ (Jobs) & Đĩa (Disk)

#### `jobs_snapshot`
- **Mục đích**: Ảnh chụp trạng thái các tác vụ đang chạy, hoàn thành hoặc lỗi trong scheduler.
- **Input**: None
- **Output**: `JobSnapshot[]`
```ts
interface JobSnapshot {
  id: string;
  title: string;
  kind: string;
  state: "PENDING" | "RUNNING" | "VERIFYING" | "DONE" | "FAILED" | "CANCELLED";
  progress?: number;
  stageName?: string;
  createdAt: string;
  error?: string;
}
```

#### `disk_usage`
- **Mục đích**: Thống kê dung lượng đĩa sử dụng theo từng khu vực.
- **Input**: None
- **Output**: `DiskUsageReport`
```ts
interface DiskUsageReport {
  blobsBytes: number;
  cacheBytes: number;
  projectsBytes: number;
  libraryBytes: number;
  totalBytes: number;
  formatted: {
    blobs: string;
    cache: string;
    projects: string;
    library: string;
    total: string;
  };
}
```

---

### 1.8 Quản lý Thành phẩm (Artifacts)

#### `artifact_list`
- **Mục đích**: Liệt kê các thành phẩm (artifact) của một dự án.
- **Input**: `{ project: string }`
- **Output**: `ArtifactSummary[]`
```ts
interface ArtifactSummary {
  id: string;
  kind: string;
  title: string;
  brand?: string;
  channels?: string[];
  dir: string;    // Workspace-relative path
  source: string; // Workspace-relative path
}
```

#### `artifact_render`
- **Mục đích**: Kích hoạt pipeline render cho một artifact (HTML->PNG/PDF, HyperFrames->video, FFmpeg timeline).
- **Input**: `{ project: string, id: string }`
- **Output**: `ArtifactRenderResult`
```ts
interface ArtifactRenderResult {
  cached: boolean;
  outputs: string[]; // Workspace-relative paths
  jobId?: string;
  probes: Array<{ name: string; available: boolean; note?: string }>;
}
```

#### `artifact_check`
- **Mục đích**: Chạy checker kiểm tra tiêu chuẩn Brand và Channel cho một artifact.
- **Input**: `{ project: string, id: string, channels?: string[] }`
- **Output**: `ArtifactCheckResult`
```ts
interface CheckFinding {
  severity: "info" | "warn" | "error";
  code: string;
  message_vi: string;
  location?: string;
  fix_hint_vi?: string;
}

interface ChannelCheckResult {
  channel_id: string;
  passed: boolean;
  findings: CheckFinding[];
}

interface ArtifactCheckResult {
  passed: boolean;
  channels: Record<string, ChannelCheckResult>;
}
```

---

### 1.9 Quản lý Khoá bí mật & Endpoint

#### `secret_status`
- **Mục đích**: Lấy danh sách trạng thái tồn tại của tất cả các khoá API được khai báo trong tool manifest. Không bao giờ trả về giá trị khoá.
- **Input**: None
- **Output**: `SecretStatusItem[]`
```ts
interface SecretStatusItem {
  name: string;
  present: boolean;
}
```

#### `secret_set`
- **Mục đích**: Lưu giá trị khoá API vào Windows Credential Manager. Tên khoá phải thuộc danh sách được khai báo trong tool manifest.
- **Input**: `{ name: string, value: string }`
- **Output**: `void`

#### `secret_delete`
- **Mục đích**: Xoá khoá API khỏi Windows Credential Manager.
- **Input**: `{ name: string }`
- **Output**: `void`

#### `provider_endpoint_get`
- **Mục đích**: Lấy cấu hình URL endpoint tuỳ chỉnh theo từng tool provider từ `app.db`.
- **Input**: None
- **Output**: `Record<string, string>` (map `<tool_id>: base_url`)

#### `provider_endpoint_set`
- **Mục đích**: Cập nhật hoặc xoá URL endpoint tuỳ chỉnh của tool provider trong `app.db`. URL phải là https (cho phép http đối với 127.0.0.1/localhost) và không chứa userinfo/query/fragment.
- **Input**: `{ toolId: string, baseUrl?: string | null }`
- **Output**: `void`

#### `provider_pricing_get`
- **Mục đích**: Lấy cấu hình giá ước tính tuỳ chỉnh theo từng provider từ `app.db`.
- **Input**: None
- **Output**: `Record<string, ProviderPricing>`
```ts
interface ProviderPricing {
  image_usd?: number | null;
  video_usd_per_s?: number | null;
  models?: Record<string, ProviderPricing>;
}
```

#### `provider_pricing_set`
- **Mục đích**: Cập nhật hoặc đặt lại giá ước tính theo provider trong `app.db`. Giá phải là số hữu hạn không âm (`>= 0`).
- **Input**: `{ providerId: string, pricing?: ProviderPricing | null }`
- **Output**: `void`

#### `env_probe`
- **Mục đích**: Kiểm tra sự hiện diện và phiên bản của các công cụ phụ thuộc ngoài (Node, npx, FFmpeg, ffprobe, Chrome/Edge, yt-dlp, claude, agy) bằng cách thực thi trực tiếp binary thật (`--version`).
- **Input**: None
- **Output**: `EnvProbeItem[]`
```ts
interface EnvProbeItem {
  id: string;
  name: string;
  found: boolean;
  version?: string | null;
  path?: string | null;
  hint: string;
---

### 1.10 Xuất bản & Đăng tải (Publishing)

#### `publish_prepare`
- **Mục đích**: Chuẩn bị bản nháp yêu cầu xuất bản, xác thực siêu dữ liệu theo giới hạn kênh (`channel@1` text limits) và kiểm tra sự tồn tại của tệp render mới nhất. Trả về mã yêu cầu với trạng thái `awaiting_confirmation` cùng danh sách cảnh báo/lỗi kiểm tra (findings).
- **Input**: `{ project?: string, artifact: string, channel: string, target: PublishTarget, metadata: PublishMetadata }`
- **Output**: `PublishPrepareOutcome`

#### `publish_list`
- **Mục đích**: Liệt kê các yêu cầu xuất bản trong workspace, tùy chọn lọc theo project.
- **Input**: `{ project?: string }`
- **Output**: `PublishRequest[]`

#### `publish_get`
- **Mục đích**: Lấy thông tin chi tiết một yêu cầu xuất bản theo id.
- **Input**: `{ id: string }`
- **Output**: `PublishRequest`

#### `publish_update_metadata`
- **Mục đích**: Cập nhật siêu dữ liệu (tiêu đề, mô tả, thẻ, chế độ hiển thị, lịch đăng, ảnh thu nhỏ) cho bản nháp hoặc bản đăng chờ xác nhận.
- **Input**: `{ id: string, metadata: PublishMetadata }`
- **Output**: `PublishRequest`

#### `publish_confirm`
- **Mục đích**: Xác nhận và tiến hành xuất bản/đăng tải (chạy dưới dạng tác vụ trong scheduler với sự kiện `job-event`). Hỗ trợ chọn kênh đăng cụ thể qua `channelId` (mặc định kênh dùng gần nhất). Đảm bảo tính bất biến (idempotent): bản đăng đã `published` không thể gửi lại; bản đăng `failed` cần người dùng xác nhận thử lại rõ ràng. Tuyệt đối không tự động thử lại (NEVER auto-retry).
- **Input**: `{ id: string, channelId?: string }`
- **Output**: `PublishRequest`
- **Lỗi**: Với YouTube, trả về cấu trúc lỗi `YouTubeErrorDetail` (`{ code, messageVi, fixVi }`).

#### `publish_cancel`
- **Mục đích**: Hủy yêu cầu xuất bản đang chờ xác nhận (chuyển về nháp) hoặc hủy tiến trình đang đăng.
- **Input**: `{ id: string }`
- **Output**: `PublishRequest`

#### `youtube_client_import`
- **Mục đích**: Nạp cấu hình Google OAuth Client từ nội dung tệp JSON (`installed` Desktop app) hoặc nhập trực tiếp Client ID & Client Secret. Phân tích `installed` và từ chối tệp loại `web` với thông báo tiếng Việt: "Hãy tạo client loại Desktop app". Lưu khóa an toàn vào Windows Credential Manager (`YOUTUBE_CLIENT_ID`, `YOUTUBE_CLIENT_SECRET`), không bao giờ echo giá trị hoặc ghi log.
- **Input**: `{ json?: string, clientId?: string, clientSecret?: string }`
- **Output**: `YoutubeClientImportResult`
```ts
interface YoutubeClientImportResult {
  ok: boolean;
  clientIdLast6: string;
}
```

#### `youtube_client_status`
- **Mục đích**: Kiểm tra xem Google OAuth Client đã được nạp hay chưa và trả về 6 ký tự cuối của Client ID nếu có.
- **Input**: None
- **Output**: `YoutubeClientStatus`
```ts
interface YoutubeClientStatus {
  configured: boolean;
  clientIdLast6?: string;
}
```

#### `youtube_connect`
- **Mục đích**: Khởi chạy luồng ủy quyền OAuth 2.0 PKCE cho YouTube qua loopback server cục bộ (thời gian chờ 5 phút). Khi hoàn tất, gọi YouTube Data API `channels.list?part=snippet&mine=true` lấy thông tin kênh, lưu refresh token theo kênh (`YOUTUBE_REFRESH_TOKEN:<channelId>`) trong Credential Manager và lưu thông tin kênh không bí mật (id, title, handle, avatarUrl) vào `app.db`.
- **Input**: None
- **Output**: `YouTubeChannelAccount`
```ts
interface YouTubeChannelAccount {
  id: string;
  title: string;
  handle?: string;
  avatarUrl?: string;
  connectedAt: string;
}
```
- **Lỗi**: Trả về `YouTubeErrorDetail` ánh xạ tiếng Việt:
```ts
interface YouTubeErrorDetail {
  code: string;
  messageVi: string;
  fixVi: string;
}
```
Bao gồm các mã lỗi chuẩn: `youtubeSignupRequired`, `access_denied`, `redirect_uri_mismatch`, `invalid_grant`, `quotaExceeded`, `uploadLimitExceeded`, `forbidden`, `timeout`, `network`.

#### `youtube_channels_list`
- **Mục đích**: Liệt kê các kênh YouTube đã liên kết từ `app.db`. Hỗ trợ tính tương thích ngược bằng cách tự động migrate refresh token đơn cũ `YOUTUBE_REFRESH_TOKEN` sang `YOUTUBE_REFRESH_TOKEN:<channelId>`.
- **Input**: None
- **Output**: `YouTubeChannelAccount[]`

#### `youtube_channel_disconnect`
- **Mục đích**: Ngắt kết nối kênh YouTube: xóa refresh token của kênh khỏi Credential Manager và xóa bản ghi kênh khỏi `app.db`. Trả về đường link để người dùng thu hồi hoàn toàn quyền trên tài khoản Google nếu muốn.
- **Input**: `{ channelId: string }`
- **Output**: `YoutubeDisconnectResult`
```ts
interface YoutubeDisconnectResult {
  ok: boolean;
  channelId: string;
  revokeUrl: string;
}
```

```ts
type PublishTarget = "youtube" | "facebook" | "tiktok" | "instagram" | "export";
type PublishPrivacy = "private" | "unlisted" | "public";
type PublishStatus = "draft" | "awaiting_confirmation" | "publishing" | "published" | "failed";

interface PublishMetadata {
  title: string;
  description: string;
  tags: string[];
  privacy: PublishPrivacy;
  schedule?: string;
  thumbnail?: string;
}

interface PublishRequest {
  id: string;
  artifact: string;
  project: string;
  channel: string;
  target: PublishTarget;
  files: string[];
  metadata: PublishMetadata;
  status: PublishStatus;
  resultUrl?: string;
  error?: string;
  createdAt: string;
  updatedAt: string;
}
```

---

### 1.8 Cài đặt và Quản lý Công cụ (Tools)

#### `tool_install_preview`
- **Mục đích**: Phân tích cú pháp và xác thực manifest công cụ từ thư mục cục bộ hoặc URL HTTPS tải zip (tối đa 100MB, kiểm tra an toàn zip-slip/symlink). Trả về thông tin chi tiết (lệnh, tham số, khả năng, phụ thuộc, rủi ro) trước khi người dùng xác nhận.
- **Input**: `{ source: string, scope: "app" | "workspace" }`
- **Output**: `ToolInstallPreview`
```ts
interface ToolInstallPreview {
  previewId: string;
  toolId: string;
  version: string;
  kind: string;
  description: string;
  command: string;
  args: string[];
  provides: string[];
  needs: string[];
  network?: string | null;
  sideEffect: string;
  scope: "app" | "workspace";
  targetDir: string;
  stagedDir: string;
  isReinstall: boolean;
}
```

#### `tool_install_confirm`
- **Mục đích**: Xác nhận cài đặt công cụ đã xem trước. Chuyển thư mục tạm vào thư mục đích (`<app>/tools/<id>@<version>` hoặc `<workspace>/.studio/tools/<id>@<version>`). Không chạy bất kỳ lệnh nào ngoại trừ probe phụ thuộc được khai báo trong manifest.
- **Input**: `{ previewId: string }`
- **Output**: `InstalledToolInfo`

#### `tool_list_installed`
- **Mục đích**: Liệt kê toàn bộ công cụ đã cài đặt và các công cụ có sẵn (builtin), kèm kết quả probe kiểm tra phụ thuộc.
- **Input**: None
- **Output**: `InstalledToolInfo[]`
```ts
interface InstalledToolInfo {
  id: string;
  version: string;
  scope: string;
  path: string;
  manifest: any;
  probe: {
    available: boolean;
    missing: string[];
    noteVi: string;
  };
  canUninstall: boolean;
}
```

#### `tool_uninstall`
- **Mục đích**: Gỡ bỏ cài đặt công cụ khỏi ứng dụng hoặc workspace.
- **Input**: `{ toolId: string, version?: string | null, scope?: "app" | "workspace" | null }`
- **Output**: `boolean`

---

## 2. Events

### `turn-event`
- **Payload**: `{ threadId: string, turnId: string, event: EngineEvent }`
- **Các biến thể của `EngineEvent`**:
  - `UserPrompt`: `{ type: "UserPrompt", text: string }` — sự kiện đầu tiên của mỗi lượt (prompt của người dùng), lưu trong lịch sử
  - `Session`: `{ type: "Session", id?: string }`
  - `TextDelta`: `{ type: "TextDelta", text: string }`
  - `Text`: `{ type: "Text", text: string }`
  - `Thinking`: `{ type: "Thinking", text: string }`
  - `ToolCall`: `{ type: "ToolCall", id: string, name: string, input: unknown }`
  - `ToolResult`: `{ type: "ToolResult", id: string, outputPreview: string, isError: boolean }`
  - `FileChange`: `{ type: "FileChange", path: string, op: "create" | "modify" | "delete" }`
  - `PermissionRequest`: `{ type: "PermissionRequest", requestId: string, tool: string, input: unknown }`
  - `PermissionDenied`: `{ type: "PermissionDenied", tool: string, input: unknown }`
  - `Usage`: `{ type: "Usage", inputTokens: number, outputTokens: number, cacheReadTokens: number, costUsd?: number | null, estimatedCostUsd?: number | null }`
  - `Done`: `{ type: "Done", status: string, text?: string }`
  - `Error`: `{ type: "Error", message: string }`

### `job-event`
- **Mục đích**: Phát sóng các bước chuyển trạng thái và cập nhật tiến độ của tác vụ từ job scheduler.
- **Payload**: `JobEventPayload`
```ts
interface JobEventPayload {
  jobId: string;
  kind: string;
  pool: string;
  state: "queued" | "admitted" | "running" | "succeeded" | "failed" | "cancelled";
  progress?: number;
  message?: string;
  peakMemoryBytes?: number;
}
```

### `fs-event`
- **Payload**: `{ paths: string[] }`
