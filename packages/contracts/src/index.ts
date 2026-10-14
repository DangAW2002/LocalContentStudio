export type ActorRole = "human" | "agent" | "system";

export interface ActorContext {
  actor_id: string;
  role: ActorRole;
  active_workspace_id: string;
  grant_token?: string;
}

export type SideEffectClass = "READ" | "DRAFT" | "REVERSIBLE" | "DESTRUCTIVE" | "EXTERNAL";

export interface OperationEnvelope<T> {
  operation: string;
  api_version: string;
  request_id: string;
  idempotency_key?: string;
  expected_revision_id?: string;
  input: T;
}

export interface OperationResult<R> {
  request_id: string;
  aggregate_id?: string;
  new_revision_id?: string;
  result: R;
  warnings: string[];
  audit_id: string;
  capability_snapshot?: Record<string, unknown>;
}

export type ErrorCode =
  | "UNAUTHORIZED"
  | "FORBIDDEN_SCOPE"
  | "FORBIDDEN_HUMAN_ONLY"
  | "NOT_FOUND"
  | "INVALID_INPUT"
  | "CONFLICT_CAS"
  | "PRECONDITION_FAILED"
  | "QUOTA_EXCEEDED"
  | "UNSUPPORTED_CAPABILITY"
  | "INTERNAL_ERROR";

export interface ErrorEnvelope {
  code: ErrorCode;
  safe_message_key: string;
  safe_message: string;
  retryable: boolean;
  recovery_action: string;
  correlation_id: string;
  details?: Record<string, unknown>;
}

// Project & Revision
export type ProjectType = "poster" | "carousel" | "report" | "motion";
export type ProjectStatus = "active" | "archived" | "deleted";

export interface Project {
  id: string;
  workspace_id: string;
  title: string;
  project_type: ProjectType;
  status: ProjectStatus;
  manifest_version: string;
  brand_ref?: string;
  created_at: string;
  updated_at: string;
}

export interface ProjectRevision {
  id: string;
  project_id: string;
  parent_id?: string;
  source_hash: string;
  author_actor_id: string;
  document_id: string;
  summary: string;
  created_at: string;
}

// Creative Document
export type ElementType = "text" | "image" | "shape" | "group";
export type ElementSemanticRole =
  | "headline"
  | "productImage"
  | "cta"
  | "brandLogo"
  | "statistic"
  | "quote"
  | "socialProof"
  | "footer"
  | "body";

export interface BoundingBox {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface ElementStyle {
  font_token?: string;
  color_token?: string;
  font_size?: number;
  text_align?: string;
}

export interface Element {
  id: string;
  type: ElementType;
  semantic_role: ElementSemanticRole;
  text?: string;
  asset_ref?: string;
  box_model: BoundingBox;
  style: ElementStyle;
  locked: boolean;
  visible: boolean;
}

export interface Artboard {
  id: string;
  width: number;
  height: number;
  background: { token: string };
  elements: Element[];
}

export interface CreativeDocument {
  schema_version: string;
  id: string;
  brand_ref?: { id: string; version: number };
  artboards: Artboard[];
  template_binding?: { template_id: string; version: number };
}

// Asset & Rights
export type AssetScope = "workspace" | "global";
export type RightsState = "QUARANTINED" | "READY" | "REVOKED";
export type AssetKind = "image" | "video" | "audio" | "document";

export interface RightsAttestation {
  source: string;
  license_basis: string;
  attribution_text?: string;
  allowed_uses: string[];
  attested_by_human_actor_id: string;
  attested_at: string;
}

export interface Asset {
  id: string;
  scope: AssetScope;
  owner_workspace_id?: string;
  kind: AssetKind;
  original_name: string;
  declared_mime: string;
  verified_mime: string;
  blob_sha256: string;
  file_size_bytes: number;
  provenance: string;
  rights_state: RightsState;
  rights_attestation?: RightsAttestation;
  created_at: string;
}

// Brand
export interface VisualTokens {
  primary_color: string;
  secondary_color: string;
  surface_color: string;
  heading_font: string;
  body_font: string;
}

export interface VoiceGuidelines {
  tone: string;
  prohibited_terms: string[];
  required_disclaimers: string[];
}

export interface BrandVersion {
  id: string;
  brand_kit_id: string;
  version_number: number;
  state: "DRAFT" | "PUBLISHED" | "ARCHIVED";
  visual_tokens: VisualTokens;
  voice: VoiceGuidelines;
  logos: string[];
  provenance: string;
  created_at: string;
}

export interface BrandKit {
  id: string;
  workspace_id: string;
  name: string;
  active_version_id?: string;
  created_at: string;
}

// Handoff
export type HandoffStatus =
  | "DRAFT"
  | "PREPARED"
  | "TRANSFERRING"
  | "ACCEPTED"
  | "REJECTED"
  | "NEEDS_RECONCILIATION";

export interface DeliveryPackage {
  schema_version: string;
  delivery_id: string;
  studio_workspace_id: string;
  project_id: string;
  source_revision_id: string;
  source_hash: string;
  variant: {
    platform: string;
    caption_text: string;
    caption_hash: string;
  };
  media: Array<{
    asset_version_id: string;
    sha256: string;
    mime: string;
    bytes: number;
  }>;
  target_selectors: {
    page_id: string;
  };
  idempotency_key: string;
  status: HandoffStatus;
  created_at: string;
}
