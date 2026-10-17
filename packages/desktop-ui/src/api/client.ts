/**
 * Local Content Studio - IPC API Client
 * 
 * Typed wrapper around Tauri IPC commands and events.
 * Uses invoke() and listen() from @tauri-apps/api.
 * Fails safely with "Chưa kết nối Studio core" when core is absent.
 */

import { invoke, listen } from "@tauri-apps/api";
import type {
  WorkspaceSummary,
  WorkspaceCreateRequest,
  WorkspaceOpenRequest,
  WorkspaceCurrentResponse,
  ProjectSummary,
  ProjectCreateRequest,
  ProjectFileEntry,
  FileReadRequest,
  FileReadResult,
  ThreadSummary,
  ThreadCreateRequest,
  ThreadEventsRequest,
  ThreadEventsResponse,
  TurnStartRequest,
  TurnStartResponse,
  TurnCancelRequest,
  TurnCancelResponse,
  TurnUndoRequest,
  TurnUndoResponse,
  PermissionRespondRequest,
  PermissionRespondResponse,
  CheckpointSummary,
  CheckpointRestoreRequest,
  CheckpointRestoreResponse,
  EngineProbeResult,
  EngineModelOption,
  EngineType,
  CatalogKit,
  CatalogModule,
  CatalogSuggestRequest,
  CatalogSuggestResult,
  ProjectModulesSetRequest,
  ProjectModulesSetResponse,
  ScopeKind,
  ScopeItem,
  AssetListFilter,
  AssetItem,
  JobSnapshot,
  DiskUsageReport,
  TurnEventPayload,
  JobEventPayload,
  FsEventPayload,
  FileWriteRequest,
  FileWriteResult,
  CheckpointCreateRequest,
  CheckpointCreateResult,
  ArtifactSummary,
  ArtifactRenderRequest,
  ArtifactRenderResult,
  ArtifactCheckRequest,
  ArtifactCheckResult,
  SecretStatusItem,
  SecretSetRequest,
  SecretDeleteRequest,
  ProviderEndpointSetRequest,
  ProviderPricing,
  ProviderPricingSetRequest,
  EnvProbeItem,
  ToolInstallPreviewRequest,
  ToolInstallConfirmRequest,
  ToolUninstallRequest,
  ToolInstallPreview,
  InstalledToolInfo,
  PublishRequest,
  PublishMetadata,
  PublishPrepareRequest,
  PublishPrepareOutcome,
  YouTubeErrorDetail,
  YouTubeChannelAccount,
  YoutubeClientImportRequest,
  YoutubeClientImportResult,
  YoutubeClientStatus,
  YoutubeDisconnectResult,
} from "./types";

export class StudioApiClient {
  // ==========================================
  // Workspace Commands
  // ==========================================

  /**
   * Fetch recent workspaces list.
   * Command: `app_recent_workspaces`
   */
  async getRecentWorkspaces(): Promise<WorkspaceSummary[]> {
    return invoke<WorkspaceSummary[]>("app_recent_workspaces");
  }

  /**
   * Create a new workspace directory and database.
   * Command: `workspace_create`
   */
  async createWorkspace(req: WorkspaceCreateRequest): Promise<WorkspaceSummary> {
    return invoke<WorkspaceSummary>("workspace_create", { ...req });
  }

  /**
   * Open an existing workspace directory.
   * Command: `workspace_open`
   */
  async openWorkspace(req: WorkspaceOpenRequest): Promise<WorkspaceSummary> {
    return invoke<WorkspaceSummary>("workspace_open", { ...req });
  }

  /**
   * Get the currently active workspace info and metadata.
   * Command: `workspace_current`
   */
  async getCurrentWorkspace(): Promise<WorkspaceCurrentResponse> {
    return invoke<WorkspaceCurrentResponse>("workspace_current");
  }

  // ==========================================
  // Project & File Commands
  // ==========================================

  /**
   * List all projects in the current workspace.
   * Command: `project_list`
   */
  async getProjects(): Promise<ProjectSummary[]> {
    return invoke<ProjectSummary[]>("project_list");
  }

  /**
   * Create a new project folder and manifest.
   * Command: `project_create`
   */
  async createProject(req: ProjectCreateRequest): Promise<ProjectSummary> {
    return invoke<ProjectSummary>("project_create", { ...req });
  }

  /**
   * List files belonging to a specific project.
   * Command: `project_files`
   */
  async getProjectFiles(project: string): Promise<ProjectFileEntry[]> {
    return invoke<ProjectFileEntry[]>("project_files", { project });
  }

  /**
   * Read file content and classified media kind.
   * Command: `file_read`
   */
  async readFile(req: FileReadRequest): Promise<FileReadResult> {
    return invoke<FileReadResult>("file_read", { ...req });
  }

  // ==========================================
  // Thread & Turn Commands
  // ==========================================

  /**
   * List conversation threads for a project.
   * Command: `thread_list`
   */
  async getThreads(project: string): Promise<ThreadSummary[]> {
    return invoke<ThreadSummary[]>("thread_list", { project });
  }

  /**
   * Create a new engine thread.
   * Command: `thread_create`
   */
  async createThread(req: ThreadCreateRequest): Promise<ThreadSummary> {
    return invoke<ThreadSummary>("thread_create", { ...req });
  }

  /**
   * Get historical events for a thread.
   * Command: `thread_events`
   */
  async getThreadEvents(req: ThreadEventsRequest): Promise<ThreadEventsResponse> {
    return invoke<ThreadEventsResponse>("thread_events", { ...req });
  }

  /**
   * Start a new agent turn.
   * Command: `turn_start`
   */
  async startTurn(req: TurnStartRequest): Promise<TurnStartResponse> {
    return invoke<TurnStartResponse>("turn_start", { ...req });
  }

  /**
   * Cancel an ongoing agent turn by killing the process tree.
   * Command: `turn_cancel`
   */
  async cancelTurn(req: TurnCancelRequest): Promise<TurnCancelResponse> {
    return invoke<TurnCancelResponse>("turn_cancel", { ...req });
  }

  /**
   * Roll back changes from a specific turn using CAS checkpoint.
   * Command: `turn_undo`
   */
  async undoTurn(req: TurnUndoRequest): Promise<TurnUndoResponse> {
    return invoke<TurnUndoResponse>("turn_undo", { ...req });
  }

  /**
   * Respond to a tool permission prompt (allow or deny).
   * Command: `permission_respond`
   */
  async respondPermission(req: PermissionRespondRequest): Promise<PermissionRespondResponse> {
    return invoke<PermissionRespondResponse>("permission_respond", { ...req });
  }

  // ==========================================
  // Checkpoint Commands
  // ==========================================

  /**
   * List all CAS checkpoints in the workspace.
   * Command: `checkpoint_list`
   */
  async getCheckpoints(): Promise<CheckpointSummary[]> {
    return invoke<CheckpointSummary[]>("checkpoint_list");
  }

  /**
   * Restore files to a specific checkpoint.
   * Command: `checkpoint_restore`
   */
  async restoreCheckpoint(req: CheckpointRestoreRequest): Promise<CheckpointRestoreResponse> {
    return invoke<CheckpointRestoreResponse>("checkpoint_restore", { ...req });
  }

  // ==========================================
  // Engines & Catalog Commands
  // ==========================================

  /**
   * Probe external tools and CLI engines (claude, agy, ffmpeg, etc.)
   * Command: `engines_probe`
   */
  async probeEngines(): Promise<EngineProbeResult[]> {
    return invoke<EngineProbeResult[]>("engines_probe");
  }

  /**
   * Models an engine accepts (claude: family aliases; agy: live `agy models`). First entry is the default.
   * Command: `engine_models`
   */
  async getEngineModels(engine: EngineType): Promise<EngineModelOption[]> {
    return invoke<EngineModelOption[]>("engine_models", { engine });
  }

  /**
   * Get all registered product kits.
   * Command: `catalog_kits`
   */
  async getCatalogKits(): Promise<CatalogKit[]> {
    return invoke<CatalogKit[]>("catalog_kits");
  }

  /**
   * Get all registered catalog modules.
   * Command: `catalog_modules`
   */
  async getCatalogModules(): Promise<CatalogModule[]> {
    return invoke<CatalogModule[]>("catalog_modules");
  }

  /**
   * Suggest kits and modules based on user request text.
   * Command: `catalog_suggest`
   */
  async suggestCatalog(req: CatalogSuggestRequest): Promise<CatalogSuggestResult> {
    return invoke<CatalogSuggestResult>("catalog_suggest", { ...req });
  }

  /**
   * Update active modules for a project.
   * Command: `project_modules_set`
   */
  async setProjectModules(req: ProjectModulesSetRequest): Promise<ProjectModulesSetResponse> {
    return invoke<ProjectModulesSetResponse>("project_modules_set", { ...req });
  }

  // ==========================================
  // Scope, Brand & Assets Commands
  // ==========================================

  /**
   * List items from the App -> Org -> Workspace scope chain.
   * Command: `scope_list`
   */
  async getScopeItems(kind: ScopeKind): Promise<ScopeItem[]> {
    return invoke<ScopeItem[]>("scope_list", { kind });
  }

  /**
   * List assets in the workspace library.
   * Command: `asset_list`
   */
  async getAssets(filter?: AssetListFilter): Promise<AssetItem[]> {
    return invoke<AssetItem[]>("asset_list", { filter });
  }

  // ==========================================
  // Jobs & Storage Commands
  // ==========================================

  /**
   * Snapshot of all background jobs in the scheduler.
   * Command: `jobs_snapshot`
   */
  async getJobsSnapshot(): Promise<JobSnapshot[]> {
    return invoke<JobSnapshot[]>("jobs_snapshot");
  }

  /**
   * Disk space breakdown by area.
   * Command: `disk_usage`
   */
  async getDiskUsage(): Promise<DiskUsageReport> {
    return invoke<DiskUsageReport>("disk_usage");
  }

  // ==========================================
  // Event Listeners
  // ==========================================

  /**
   * Listen to streaming agent turn events (TextDelta, ToolCall, FileChange, etc.)
   */
  async onTurnEvent(callback: (payload: TurnEventPayload) => void): Promise<() => void> {
    return listen<TurnEventPayload>("turn-event", (e) => callback(e.payload));
  }

  /**
   * Listen to job scheduler progress and state changes.
   */
  async onJobEvent(callback: (payload: JobEventPayload) => void): Promise<() => void> {
    return listen<JobEventPayload>("job-event", (e) => callback(e.payload));
  }

  /**
   * Listen to workspace filesystem change events.
   */
  async onFsEvent(callback: (payload: FsEventPayload) => void): Promise<() => void> {
    return listen<FsEventPayload>("fs-event", (e) => callback(e.payload));
  }

  // ==========================================
  // Editing & Artifacts
  // ==========================================

  /** Command: `file_write` (direct edits on the Stage). */
  async writeFile(req: FileWriteRequest): Promise<FileWriteResult> {
    return invoke<FileWriteResult>("file_write", { ...req });
  }

  /** Command: `checkpoint_create`. */
  async createCheckpoint(req: CheckpointCreateRequest = {}): Promise<CheckpointCreateResult> {
    return invoke<CheckpointCreateResult>("checkpoint_create", { ...req });
  }

  /** Command: `artifact_list`. */
  async listArtifacts(project: string): Promise<ArtifactSummary[]> {
    return invoke<ArtifactSummary[]>("artifact_list", { project });
  }

  /** Command: `artifact_render`. */
  async renderArtifact(req: ArtifactRenderRequest): Promise<ArtifactRenderResult> {
    return invoke<ArtifactRenderResult>("artifact_render", { ...req });
  }

  /** Command: `artifact_check`. */
  async checkArtifact(req: ArtifactCheckRequest): Promise<ArtifactCheckResult> {
    return invoke<ArtifactCheckResult>("artifact_check", { ...req });
  }

  // ==========================================
  // agy Integration
  // ==========================================

  /** Command: `agy_connect`. */
  async agyConnect(): Promise<{ connected: boolean; shim?: string }> {
    return invoke<{ connected: boolean; shim?: string }>("agy_connect");
  }

  /** Command: `agy_disconnect`. */
  async agyDisconnect(): Promise<{ connected: boolean }> {
    return invoke<{ connected: boolean }>("agy_disconnect");
  }

  // ==========================================
  // Secrets & Provider Endpoints
  // ==========================================

  /** Command: `secret_status`. */
  async getSecretStatus(): Promise<SecretStatusItem[]> {
    return invoke<SecretStatusItem[]>("secret_status");
  }

  /** Command: `secret_set`. */
  async setSecret(req: SecretSetRequest): Promise<void> {
    return invoke<void>("secret_set", { ...req });
  }

  /** Command: `secret_delete`. */
  async deleteSecret(req: SecretDeleteRequest): Promise<void> {
    return invoke<void>("secret_delete", { ...req });
  }

  /** Command: `provider_endpoint_get`. */
  async getProviderEndpoints(): Promise<Record<string, string>> {
    return invoke<Record<string, string>>("provider_endpoint_get");
  }

  /** Command: `provider_endpoint_set`. */
  async setProviderEndpoint(req: ProviderEndpointSetRequest): Promise<void> {
    return invoke<void>("provider_endpoint_set", { ...req });
  }

  /** Command: `provider_pricing_get`. */
  async getProviderPricing(): Promise<Record<string, ProviderPricing>> {
    return invoke<Record<string, ProviderPricing>>("provider_pricing_get");
  }

  /** Command: `provider_pricing_set`. */
  async setProviderPricing(req: ProviderPricingSetRequest): Promise<void> {
    return invoke<void>("provider_pricing_set", { ...req });
  }

  /** Command: `env_probe`. */
  async envProbe(): Promise<EnvProbeItem[]> {
    return invoke<EnvProbeItem[]>("env_probe");
  }

  // ==========================================
  // Tool Management Commands
  // ==========================================

  /**
   * Preview a tool installation from folder or URL.
   * Command: `tool_install_preview`
   */
  async toolInstallPreview(req: ToolInstallPreviewRequest): Promise<ToolInstallPreview> {
    return invoke<ToolInstallPreview>("tool_install_preview", { req });
  }

  /**
   * Confirm and finalize tool installation.
   * Command: `tool_install_confirm`
   */
  async toolInstallConfirm(req: ToolInstallConfirmRequest): Promise<InstalledToolInfo> {
    return invoke<InstalledToolInfo>("tool_install_confirm", { req });
  }

  /**
   * List all installed and builtin tools with probe results.
   * Command: `tool_list_installed`
   */
  async toolListInstalled(): Promise<InstalledToolInfo[]> {
    return invoke<InstalledToolInfo[]>("tool_list_installed");
  }

  /**
   * Uninstall a tool.
   * Command: `tool_uninstall`
   */
  async toolUninstall(req: ToolUninstallRequest): Promise<boolean> {
    return invoke<boolean>("tool_uninstall", { req });
  }

  // ==========================================
  // Publishing Commands
  // ==========================================

  /**
   * Prepare a draft publish request with validation against channel limits.
   * Command: `publish_prepare`
   */
  async publishPrepare(req: PublishPrepareRequest): Promise<PublishPrepareOutcome> {
    return invoke<PublishPrepareOutcome>("publish_prepare", { ...req });
  }

  /**
   * List publish requests in the workspace, optionally filtered by project slug.
   * Command: `publish_list`
   */
  async publishList(project?: string): Promise<PublishRequest[]> {
    return invoke<PublishRequest[]>("publish_list", { project });
  }

  /**
   * Get details of a single publish request.
   * Command: `publish_get`
   */
  async publishGet(id: string): Promise<PublishRequest> {
    return invoke<PublishRequest>("publish_get", { id });
  }

  /**
   * Update metadata for a draft or awaiting-confirmation publish request.
   * Command: `publish_update_metadata`
   */
  async publishUpdateMetadata(id: string, metadata: PublishMetadata): Promise<PublishRequest> {
    return invoke<PublishRequest>("publish_update_metadata", { id, metadata });
  }

  /**
   * Confirm and run a publish request.
   * Command: `publish_confirm`
   */
  async publishConfirm(id: string, channelId?: string): Promise<PublishRequest> {
    return invoke<PublishRequest>("publish_confirm", { id, channelId });
  }

  /**
   * Cancel an in-progress or awaiting-confirmation publish request.
   * Command: `publish_cancel`
   */
  async publishCancel(id: string): Promise<PublishRequest> {
    return invoke<PublishRequest>("publish_cancel", { id });
  }

  /**
   * Import Google OAuth Client JSON or raw credentials.
   * Command: `youtube_client_import`
   */
  async youtubeClientImport(req: YoutubeClientImportRequest): Promise<YoutubeClientImportResult> {
    return invoke<YoutubeClientImportResult>("youtube_client_import", { ...req });
  }

  /**
   * Get YouTube OAuth client status.
   * Command: `youtube_client_status`
   */
  async youtubeClientStatus(): Promise<YoutubeClientStatus> {
    return invoke<YoutubeClientStatus>("youtube_client_status");
  }

  /**
   * Connect YouTube using OAuth 2.0 PKCE and loopback listener.
   * Command: `youtube_connect`
   */
  async youtubeConnect(): Promise<YouTubeChannelAccount> {
    return invoke<YouTubeChannelAccount>("youtube_connect");
  }

  /**
   * List connected YouTube channel accounts.
   * Command: `youtube_channels_list`
   */
  async youtubeChannelsList(): Promise<YouTubeChannelAccount[]> {
    return invoke<YouTubeChannelAccount[]>("youtube_channels_list");
  }

  /**
   * Disconnect a YouTube channel account.
   * Command: `youtube_channel_disconnect`
   */
  async youtubeChannelDisconnect(channelId: string): Promise<YoutubeDisconnectResult> {
    return invoke<YoutubeDisconnectResult>("youtube_channel_disconnect", { channelId });
  }
}

/**
 * Parse structured or serialized YouTube error into YouTubeErrorDetail.
 */
export function parseYouTubeError(err: any): YouTubeErrorDetail | null {
  if (!err) return null;
  if (typeof err === "object" && err.code && (err.messageVi || err.fixVi)) {
    return {
      code: err.code,
      messageVi: err.messageVi || err.message_vi || "",
      fixVi: err.fixVi || err.fix_vi || "",
    };
  }
  const str = typeof err === "string" ? err : err.message || "";
  try {
    const parsed = JSON.parse(str);
    if (
      parsed &&
      typeof parsed === "object" &&
      parsed.code &&
      (parsed.messageVi || parsed.fixVi || parsed.message_vi || parsed.fix_vi)
    ) {
      return {
        code: parsed.code,
        messageVi: parsed.messageVi || parsed.message_vi || "",
        fixVi: parsed.fixVi || parsed.fix_vi || "",
      };
    }
  } catch {}

  // Fallback pattern matching for string errors
  const lower = str.toLowerCase();
  if (lower.includes("youtubesignuprequired") || (lower.includes("unauthorized") && lower.includes("kênh"))) {
    return {
      code: "youtubeSignupRequired",
      messageVi: "Tài khoản này chưa có kênh YouTube",
      fixVi: "Vào youtube.com → Tạo kênh, rồi bấm Thử lại",
    };
  }
  if (lower.includes("access_denied")) {
    return {
      code: "access_denied",
      messageVi: "Email chưa có trong Test users của OAuth consent screen",
      fixVi: "Vào Google Cloud Console → APIs & Services → OAuth consent screen → Audience → Test users → thêm đúng email sở hữu kênh, rồi thử lại.",
    };
  }
  if (lower.includes("redirect_uri_mismatch") || lower.includes("desktop app")) {
    return {
      code: "redirect_uri_mismatch",
      messageVi: "Client phải là loại Desktop app",
      fixVi: "Tạo lại OAuth client trên Google Cloud Console với Application type là 'Desktop app' (không phải Web application), rồi nạp lại tệp JSON.",
    };
  }
  if (lower.includes("invalid_grant") || lower.includes("token revoked") || lower.includes("expired or revoked")) {
    return {
      code: "invalid_grant",
      messageVi: "Phiên đăng nhập hết hạn",
      fixVi: "Bấm Kết nối YouTube lại để cấp quyền mới.",
    };
  }
  if (lower.includes("quotaexceeded")) {
    return {
      code: "quotaExceeded",
      messageVi: "Hết hạn mức API YouTube hôm nay (mặc định ~6 lần tải/ngày)",
      fixVi: "Thử lại vào ngày mai (hạn mức đặt lại vào nửa đêm giờ Thái Bình Dương PT) hoặc yêu cầu tăng hạn mức trên Google Cloud.",
    };
  }
  if (lower.includes("uploadlimitexceeded")) {
    return {
      code: "uploadLimitExceeded",
      messageVi: "Kênh đã vượt quá giới hạn số lượng video được tải lên trong ngày của YouTube",
      fixVi: "Chờ 24 giờ rồi thử tải lại video.",
    };
  }
  if (lower.includes("forbidden") || lower.includes("403")) {
    return {
      code: "forbidden",
      messageVi: "Không có quyền truy cập YouTube Data API (HTTP 403)",
      fixVi: "Kiểm tra xem YouTube Data API v3 đã được Enable trong Google Cloud project chưa, và tài khoản đã được cấp quyền chưa.",
    };
  }
  if (lower.includes("timeout") || lower.includes("hết thời gian chờ")) {
    return {
      code: "timeout",
      messageVi: "Hết thời gian chờ xác thực YouTube từ trình duyệt",
      fixVi: "Bấm Kết nối YouTube lại và hoàn tất đăng nhập trong vòng 5 phút. Đừng tắt Studio khi đang đăng nhập.",
    };
  }
  if (lower.includes("connection refused") || lower.includes("error sending request") || lower.includes("kết nối mạng")) {
    return {
      code: "network",
      messageVi: "Lỗi kết nối mạng tới máy chủ Google/YouTube",
      fixVi: "Kiểm tra kết nối mạng internet hoặc thiết lập proxy/tường lửa của máy tính, rồi thử lại.",
    };
  }

  return null;
}

export const studioApi = new StudioApiClient();
