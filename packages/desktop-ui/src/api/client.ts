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
}

export const studioApi = new StudioApiClient();
