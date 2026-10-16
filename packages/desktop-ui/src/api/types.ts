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
  | { type: "Usage"; inputTokens: number; outputTokens: number; cacheReadTokens: number; costUsd?: number }
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
// Global Event Payloads
// ==========================================

export interface TurnEventPayload {
  threadId: string;
  turnId: string;
  event: EngineEvent;
}

export interface JobEventPayload {
  jobId: string;
  event: JobSnapshot;
}

export interface FsEventPayload {
  paths: string[];
}
