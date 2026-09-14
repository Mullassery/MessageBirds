/** Mirrors `mb_core::IdentityRef`. */
export interface IdentityRef {
  namespace: string;
  value: string;
  primary?: boolean;
  source: string;
  confidence?: number;
}

/** Mirrors `mb_core::EventSource`. */
export interface EventSource {
  type: string;
  name: string;
}

/** Mirrors `mb_core::SchemaRef`. */
export interface SchemaRef {
  name: string;
  version: string;
}

/**
 * Mirrors `mb_core::EventEnvelope`. `context.profile_updates`, if present,
 * is a map of mixin key (`"core/person@1.0"`) to the field values this
 * event contributes to that mixin — see docs/OCDS.md.
 */
export interface EventEnvelope {
  event_id: string;
  event_type: string;
  timestamp: string;
  source: EventSource;
  identity: IdentityRef[];
  context?: Record<string, unknown>;
  data?: Record<string, unknown>;
  schema: SchemaRef;
  consent?: unknown;
  governance?: unknown;
  correlation_id?: string;
  trace_id?: string;
  tenant_id: string;
}

/** Mirrors `mb_profile::FieldProvenance`. */
export interface FieldProvenance {
  profile_id: string;
  mixin_key: string;
  field_path: string;
  value: unknown;
  source: string;
  event_id: string;
  applied_policy: string;
  updated_at: string;
}

/** Response shape of `GET /profiles/{id}` and `GET /profiles/by-identity`. */
export interface ProfileView {
  id: string;
  tenant_id: string;
  mixins: Record<string, Record<string, unknown>>;
  created_at: string;
  updated_at: string;
  provenance: FieldProvenance[];
}

/** Mirrors `mb_identity::IdentityNode`. */
export interface IdentityNode {
  id: string;
  tenant_id: string;
  namespace: string;
  value_hash: string;
  profile_id: string;
  confidence: number;
  source: string;
  first_seen_at: string;
  last_seen_at: string;
}

export type IdentityAuditKind = "linked" | "merged" | "split";

/**
 * Mirrors `mb_identity::IdentityAuditEntry`. `namespace`/`value_hash` are
 * `null` for `"merged"` entries, which record a whole-profile merge rather
 * than one identity claim.
 */
export interface IdentityAuditEntry {
  id: string;
  tenant_id: string;
  kind: IdentityAuditKind;
  namespace: string | null;
  value_hash: string | null;
  profile_id: string;
  previous_profile_id: string | null;
  source: string;
  created_at: string;
}

/** Response shape of `GET /profiles/{id}/identity`. */
export interface IdentityView {
  nodes: IdentityNode[];
  audit: IdentityAuditEntry[];
}

/**
 * One entry in `GET /profiles/{id}/merge-suggestions` — a confidence-scored
 * candidate for a manual merge. Never applied automatically.
 */
export interface MergeSuggestion {
  profile_id: string;
  score: number;
}

/** One row of `GET /profiles/{id}/events`. */
export interface EventSummary {
  id: string;
  event_type: string;
  schema_name: string;
  schema_version: string;
  /** Raw claims this event carried — the one place to recover a claim's
   * plaintext value for `POST /profiles/{id}/split`, since the identity
   * graph only ever stores/returns hashes. */
  identity: IdentityRef[];
  data: Record<string, unknown>;
  received_at: string;
  occurred_at: string;
}
