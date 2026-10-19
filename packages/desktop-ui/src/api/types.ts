/**
 * Local Content Studio - IPC Types Contract
 * 
 * Defines all TypeScript interfaces for communication between the React desktop UI
 * and the Rust desktop core via Tauri IPC.
 * Snake_case command names in Tauri, camelCase TS fields in frontend.
 */

// ==========================================
// File and Media Types
// ==========================================

export type FileKind =
  | "markdown"
  | "text"
  | "image"
  | "video"
  | "audio"
  | "html"
  | "code"
  | "json"
  | "unknown";

export interface ProjectFileEntry {
  /** Relative path within the project root */
  path: string;
  /** File or directory name */
  name: string;
  /** Whether this entry is a directory */
  isDir: boolean;
  /** File size in bytes */
  size: number;
  /** Classified file kind for viewer selection */
  kind: FileKind;
  /** Last updated ISO timestamp */
  updatedAt?: string;
}

export interface FileReadRequest {
  /** Project slug (optional if reading absolute or workspace-relative path) */
  project?: string;
  /** Path to read */
  path: string;
}

export interface FileReadResult {
  /** Classified file kind */
  kind: FileKind;
  /** MIME type string e.g. text/markdown, image/png */
  mime: string;
  /** UTF-8 text content if text-based file */
  text?: string;
  /** Safe file or blob asset URL for streaming/rendering binary media */
  assetUrl?: string;
  /** Byte size of file */
  size: number;
}

export interface FileWriteRequest {
  /** Project slug (optional if writing absolute or workspace-relative path) */
  project?: string;
  /** Target file path */
  path: string;
  /** UTF-8 string text content to write */
  text: string;
}

export interface FileWriteResult {
  /** Workspace-relative path written */
  path: string;
  /** Byte size written */
  size: number;
}

// ==========================================
// Workspace Types
// ==========================================

export interface WorkspaceSummary {
  /** Absolute local filesystem path of the workspace */
  path: string;
  /** Friendly display name of the workspace */
  name: string;
  /** Last opened ISO timestamp */
  lastOpenedAt?: string;
  /** Whether this workspace is currently open */
  isCurrent?: boolean;
}

export interface WorkspaceCreateRequest {
  /** Target filesystem directory */
  path: string;
  /** Friendly name */
  name: string;
}

export interface WorkspaceOpenRequest {
  /** Target filesystem path */
  path: string;
}

export interface OrgSummary {
  id: string;
  name: string;
  path: string;
}

export interface BudgetSummary {
  currentSpendUsd: number;
  maxSpendUsd: number;
  warningThreshold: number;
}

export interface BudgetStatus {
  todaySpendUsd: number;
  dailyBudgetUsd?: number | null;
  monthSpendUsd: number;
  monthlyBudgetUsd?: number | null;
  warningThreshold: number;
  isDailyOver: boolean;
  isMonthlyOver: boolean;
}

export interface WorkspaceCurrentResponse {
  workspace: WorkspaceSummary;
  org?: OrgSummary;
  budget?: BudgetSummary;
}

// ==========================================
// Project Types
// ==========================================

export interface ProjectSummary {
  /** Project slug (unique within workspace) */
  slug: string;
  /** Project title */
  title: string;
  /** Associated preset kit name if any */
  kit?: string;
  /** Active module IDs enabled in this project */
  modules?: string[];
  /** Project status */
  status: "active" | "archived" | "completed";
  /** ISO timestamp when created */
  createdAt: string;
  /** ISO timestamp when updated */
  updatedAt: string;
  /** Number of generated artifacts */
  artifactCount?: number;
}

export interface ProjectCreateRequest {
  /** Unique slug for filesystem directory */
  slug: string;
  /** User-facing title */
  title: string;
  /** Optional kit preset */
  kit?: string;
  /** Optional list of initial modules */
  modules?: string[];
}

// ==========================================
// Thread, Engine and Turn Types
// ==========================================

export type EngineType = "claude" | "agy";

/** One entry of `engine_models`. */
export interface EngineModelOption {
  value: string;
  label: string;
}

export interface ThreadSummary {
  /** Thread ID */
  id: string;
  /** Owning project slug */
  project: string;
  /** Engine adapter used for this thread */
  engine: EngineType;
  /** Optional model identifier override */
  model?: string;
  /** Optional friendly thread title */
  title?: string;
  /** Number of turns executed */
  turnCount: number;
  /** ISO timestamp */
  createdAt: string;
  /** ISO timestamp */
  updatedAt: string;
}

export interface ThreadCreateRequest {
  project: string;
  engine: EngineType;
  model?: string;
}

export interface ThreadEventsRequest {
  threadId: string;
  cursor?: string;
  limit?: number;
}

export interface ThreadEventsResponse {
  events: EngineTurnEvent[];
  nextCursor?: string;
  hasMore: boolean;
}

export interface TurnStartRequest {
  threadId: string;
  prompt: string;
}

export interface TurnStartResponse {
  turnId: string;
}

export interface TurnCancelRequest {
  turnId: string;
}

export interface TurnCancelResponse {
  success: boolean;
}

export interface TurnUndoRequest {
  turnId: string;
}

export interface TurnUndoResponse {
  success: boolean;
  restoredTurnId?: string;
}

export interface PermissionRespondRequest {
  requestId: string;
  allow: boolean;
}

export interface PermissionRespondResponse {
  acknowledged: boolean;
}

// ==========================================
// Engine Normalized Events
// ==========================================

export type EngineEvent =
  | { type: "UserPrompt"; text: string }
  | { type: "Session"; id?: string }
  | { type: "TextDelta"; text: string }
  | { type: "Text"; text: string }
  | { type: "Thinking"; text: string }
  | { type: "ToolCall"; id: string; name: string; input: unknown }
  | { type: "ToolResult"; id: string; outputPreview: string; isError: boolean }
  | { type: "FileChange"; path: string; op: "create" | "modify" | "delete" }
  | { type: "PermissionRequest"; requestId: string; tool: string; input: unknown }
  | { type: "PermissionDenied"; tool: string; input: unknown }
  | { type: "Usage"; inputTokens: number; outputTokens: number; cacheReadTokens: number; cacheWriteTokens?: number; reasoningTokens?: number; costUsd?: number | null; estimatedCostUsd?: number | null }
  | { type: "Done"; status: string; text?: string }
  | { type: "Error"; message: string };

export interface EngineTurnEvent {
  threadId: string;
  turnId: string;
  event: EngineEvent;
}

// ==========================================
// Checkpoint Types
// ==========================================

export interface CheckpointSummary {
  /** Checkpoint SHA-256 hash or ID */
  id: string;
  /** ISO timestamp */
  timestamp: string;
  /** Human or automated checkpoint summary */
  message: string;
  /** Number of files changed in CAS */
  filesChanged: number;
  /** Who triggered checkpoint: user or turn:turnId */
  author?: string;
}

export interface CheckpointRestoreRequest {
  id: string;
}

export interface CheckpointRestoreResponse {
  success: boolean;
  restoredCheckpointId: string;
}

export interface CheckpointCreateRequest {
  /** Optional human-readable checkpoint label */
  label?: string;
}

export interface CheckpointCreateResult {
  /** Checkpoint SHA-256 hash or ID */
  id: string;
}

// ==========================================
// Engines & Catalog Types
// ==========================================

export interface EngineProbeResult {
  /** Engine/tool ID: claude, agy, ffmpeg, chromium, node */
  id: string;
  /** Display name */
  name: string;
  /** Whether the CLI executable was detected */
  installed: boolean;
  /** Version string detected */
  version?: string;
  /** Absolute binary path detected */
  path?: string;
  /** Whether this version is officially supported */
  supported: boolean;
  /** Additional notes or installation guidelines */
  notes?: string;
}

export interface CatalogKit {
  /** Kit identifier e.g. youtube-video, shorts, carousel */
  id: string;
  /** Friendly Vietnamese name */
  name: string;
  /** Description of what this kit produces */
  description: string;
  /** Module IDs included */
  modules: string[];
  /** Expected token consumption footprint */
  tokenBudget: number;
  /** Search tags */
  tags: string[];
}

export interface CatalogModule {
  id: string;
  name: string;
  description: string;
  capabilities: string[];
  skillCount: number;
  tokenCount: number;
}

export interface CatalogSuggestRequest {
  text: string;
}

export interface CatalogSuggestResult {
  suggestedKits: CatalogKit[];
  suggestedModules: CatalogModule[];
}

export interface ProjectModulesSetRequest {
  project: string;
  modules: string[];
}

export interface ProjectModulesSetResponse {
  project: string;
  modules: string[];
  totalTokenBudget: number;
}

// ==========================================
// Scope & Brands Types
// ==========================================

export type ScopeKind = "brands" | "channels" | "playbooks" | "skills" | "tools" | "capabilities";

export interface ScopeItem {
  id: string;
  name: string;
  kind: ScopeKind;
  /** Hierarchy level: App, Organization, or Workspace */
  layer: "app" | "org" | "workspace";
  path?: string;
  summary?: string;
  metadata?: Record<string, unknown>;
  /** Channels only: output formats with their safe zones. */
  formats?: ChannelFormat[];
  /** Whether this workspace item overrides an App/Org inherited item */
  isOverride?: boolean;
  /** Configuration error message if file could not be parsed */
  error?: string;
}

export interface ScopeValidationError {
  line: number;
  column: number;
  message: string;
}

export interface ScopeValidationResult {
  valid: boolean;
  errors: ScopeValidationError[];
}

export interface ScopeActionResponse {
  success: boolean;
  id: string;
  kind: string;
  path?: string;
}

export interface ChannelFormat {
  name: string;
  width: number;
  height: number;
  aspect: string;
  fps?: number;
  max_duration_s?: number;
  min_duration_s?: number;
  /** Fractional margins (0..1) from each edge that UI overlays may cover. */
  safe_zone: { top: number; right: number; bottom: number; left: number };
}

// ==========================================
// Library Assets Types
// ==========================================

export interface AssetListFilter {
  kind?: string;
  search?: string;
  limit?: number;
}

export interface AssetItem {
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

// ==========================================
// Jobs & Resource Pool Types
// ==========================================

export type JobState = "PENDING" | "RUNNING" | "VERIFYING" | "DONE" | "FAILED" | "CANCELLED";

export interface JobSnapshot {
  id: string;
  title: string;
  kind: string;
  state: JobState;
  progress?: number;
  stageName?: string;
  createdAt: string;
  error?: string;
  peakMemoryBytes?: number;
}

export interface DiskUsageReport {
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

// ==========================================
// Artifact Types
// ==========================================

export interface ArtifactSummary {
  /** Unique artifact ID e.g. thumb-01, video-main */
  id: string;
  /** Artifact kind e.g. design, motion, timeline, text, audio */
  kind: string;
  /** User-facing title */
  title: string;
  /** Brand slug reference */
  brand?: string;
  /** Applicable platform channel IDs */
  channels?: string[];
  /** Workspace-relative directory path */
  dir: string;
  /** Workspace-relative main source path */
  source: string;
}

export interface ArtifactRenderRequest {
  project: string;
  id: string;
}

export interface ArtifactProbe {
  name: string;
  available: boolean;
  note?: string;
}

export interface ArtifactRenderResult {
  cached: boolean;
  outputs: string[];
  jobId?: string;
  probes: ArtifactProbe[];
}

export interface CheckFinding {
  severity: "info" | "warn" | "error";
  code: string;
  message_vi: string;
  location?: string;
  fix_hint_vi?: string;
}

export interface ChannelCheckResult {
  channel_id: string;
  passed: boolean;
  findings: CheckFinding[];
}

export interface ArtifactCheckRequest {
  project: string;
  id: string;
  channels?: string[];
}

export interface ArtifactCheckResult {
  passed: boolean;
  channels: Record<string, ChannelCheckResult>;
}

// ==========================================
// Global Event Payloads
// ==========================================

export interface TurnEventPayload {
  threadId: string;
  turnId: string;
  event: EngineEvent;
}

export type JobLifecycleState =
  | "queued"
  | "admitted"
  | "running"
  | "succeeded"
  | "failed"
  | "cancelled";

export interface JobEventPayload {
  jobId: string;
  kind: string;
  pool: string;
  state: JobLifecycleState;
  progress?: number;
  message?: string;
  peakMemoryBytes?: number;
  /** Snapshot-shaped view of the same job, used by the Jobs panel. */
  event: JobSnapshot;
}

export interface FsEventPayload {
  paths: string[];
}

export interface AgyConnectResult {
  connected: boolean;
  shim?: string;
}

export interface AgyDisconnectResult {
  connected: boolean;
}

// ==========================================
// Secret & Provider Endpoint Types
// ==========================================

export interface SecretStatusItem {
  name: string;
  present: boolean;
}

export interface SecretSetRequest {
  name: string;
  value: string;
}

export interface SecretDeleteRequest {
  name: string;
}

export interface ProviderEndpointSetRequest {
  toolId: string;
  baseUrl?: string | null;
}

// ==========================================
// Provider Pricing & Env Probe Types
// ==========================================

export interface ProviderPricing {
  image_usd?: number | null;
  video_usd_per_s?: number | null;
  models?: Record<string, ProviderPricing>;
}

export interface ProviderPricingSetRequest {
  providerId: string;
  pricing?: ProviderPricing | null;
}

export interface EnvProbeItem {
  id: string;
  name: string;
  found: boolean;
  version?: string | null;
  path?: string | null;
  hint: string;
}

// ==========================================
// Tool Installation & Management Types
// ==========================================

export type ToolInstallScope = "app" | "workspace";

export interface ToolInstallPreviewRequest {
  source: string;
  scope: ToolInstallScope;
}

export interface ToolInstallConfirmRequest {
  previewId: string;
}

export interface ToolUninstallRequest {
  toolId: string;
  version?: string | null;
  scope?: ToolInstallScope | null;
}

export interface ToolInstallPreview {
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
  scope: ToolInstallScope;
  targetDir: string;
  stagedDir: string;
  isReinstall: boolean;
}

export interface InstalledToolInfo {
  id: string;
  version?: string | null;
  scope: string;
  path: string;
  manifest: any;
  probe: {
    available: boolean;
    missing: string[];
    noteVi?: string;
    note_vi?: string;
  };
  canUninstall: boolean;
}

// Publishing Types
// ==========================================

export type PublishTarget = "youtube" | "facebook" | "tiktok" | "instagram" | "export";

export type PublishPrivacy = "private" | "unlisted" | "public";

export type PublishStatus = "draft" | "awaiting_confirmation" | "publishing" | "published" | "failed";

export interface PublishMetadata {
  title: string;
  description: string;
  tags: string[];
  privacy: PublishPrivacy;
  schedule?: string;
  thumbnail?: string;
}

export interface PublishRequest {
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

export interface PublishListRequest {
  project?: string;
}

export interface PublishUpdateRequest {
  id: string;
  metadata: PublishMetadata;
}

export interface PublishFinding {
  severity: "info" | "warn" | "error";
  code: string;
  message_vi: string;
  location?: string;
  fix_hint_vi?: string;
}

export interface PublishPrepareRequest {
  project?: string;
  artifact: string;
  channel: string;
  target: PublishTarget;
  metadata: PublishMetadata;
}

export interface PublishPrepareOutcome {
  id: string;
  status: PublishStatus;
  findings: PublishFinding[];
  files: string[];
  request: PublishRequest;
}

export interface PublishConfirmRequest {
  id: string;
  channelId?: string;
}

export interface PublishCancelRequest {
  id: string;
}

export interface YouTubeErrorDetail {
  code: string;
  messageVi: string;
  fixVi: string;
}

export interface YouTubeChannelAccount {
  id: string;
  title: string;
  handle?: string;
  avatarUrl?: string;
  connectedAt: string;
}

export interface YoutubeClientImportRequest {
  json?: string;
  clientId?: string;
  clientSecret?: string;
}

export interface YoutubeClientImportResult {
  clientIdLast6: string;
  message: string;
}

export interface YoutubeClientStatus {
  configured: boolean;
  clientIdLast6?: string;
}
export interface YoutubeDisconnectResult {
  success: boolean;
  revokeUrl: string;
}

// ==========================================
// Text-to-Speech (TTS) Types
// ==========================================

export interface TtsVoiceItem {
  id: string;
  name: string;
  provider: string;
  quality?: string;
  downloaded: boolean;
  sizeBytes?: number;
  sizeHuman?: string;
  licenseNote?: string;
}

export interface TtsProviderInfo {
  id: string;
  name: string;
  available: boolean;
  noteVi: string;
  voices: TtsVoiceItem[];
  binaryInstalled: boolean;
  binaryPath?: string;
  binaryVersion?: string;
}

export interface TtsStatusResponse {
  providers: TtsProviderInfo[];
  defaultProvider?: string;
  defaultVoice?: string;
}

export interface TtsModelPreview {
  provider: string;
  voice: string;
  name: string;
  sizeBytes: number;
  sizeHuman: string;
  license: string;
  licenseNote: string;
  downloadUrls: string[];
}

export interface TtsModelInstallResult {
  success: boolean;
  message: string;
  provider: string;
  voice: string;
}

export interface TtsPreviewRequest {
  text: string;
  provider?: string;
  voice?: string;
  speed?: number;
}

export interface TtsPreviewResponse {
  audioUrl: string;
  audioPath: string;
  provider: string;
  voice: string;
  durationEstimateSecs?: number;
  sizeBytes: number;
  note?: string;
}

export interface TtsConfigResponse {
  defaultProvider?: string;
  defaultVoice?: string;
}

export interface TtsSetConfigRequest {
  defaultProvider?: string;
  defaultVoice?: string;
}


// Web Mode (Chế độ web cá nhân) Types
// ==========================================

export interface WebStatus {
  enabled: boolean;
  lan_allowed: boolean;
  port: number;
  has_password: boolean;
  local_url?: string;
  lan_url?: string;
  lan_ip?: string;
  qr_code_svg?: string;
  active_sessions: number;
}

export interface WebConfigureRequest {
  enabled?: boolean;
  lan_allowed?: boolean;
  port?: number;
  password?: string;
}
