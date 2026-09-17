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

/** Response of `GET /events/{id}` — a single event, in full. */
export interface EventDetail {
  id: string;
  tenant_id: string;
  event_type: string;
  schema_name: string;
  schema_version: string;
  identity: IdentityRef[];
  context: Record<string, unknown>;
  data: Record<string, unknown>;
  profile_id: string;
  received_at: string;
  occurred_at: string;
}

/** Mirrors `mb_schema_registry::FieldType`. */
export type FieldType = "string" | "number" | "boolean" | "object" | "array";

/** Mirrors `mb_schema_registry::FieldDef`. */
export interface FieldDef {
  type: FieldType;
  required: boolean;
  enum?: unknown[] | null;
  /** Governance labels (Section 13), e.g. `["PII", "DIRECT_IDENTIFIER"]`. */
  labels: string[];
}

export type SchemaStatus = "active" | "deprecated";

/** Mirrors `mb_schema_registry::Schema` — response of `GET /schemas/{name}/{version}`. */
export interface SchemaDefinition {
  id: string;
  name: string;
  version: string;
  fields: Record<string, FieldDef>;
  status: SchemaStatus;
  created_at: string;
}

export type MixinStatus = "active" | "deprecated";

/** Mirrors `mb_mixins::MixinDef` — response of `GET /mixins/{namespace}/{name}/{version}`. */
export interface MixinDef {
  id: string;
  namespace: string;
  name: string;
  version: string;
  fields: Record<string, FieldDef>;
  status: MixinStatus;
  created_at: string;
}

/** Mirrors `mb_audiences::AttributeOp`. */
export type AttributeOp =
  | "equals"
  | "not_equals"
  | "exists"
  | "not_exists"
  | "greater_than"
  | "less_than"
  | "contains";

/**
 * Mirrors `mb_audiences::Condition`. Externally tagged on the wire
 * (`{"attribute": {...}}`, not `{"type": "attribute", ...}`) — see the
 * comment on the Rust type for why.
 */
export type Condition =
  | { attribute: { mixin: string; field: string; op: AttributeOp; value?: unknown } }
  | { event: { event_type: string; within_days: number; min_count: number } }
  | { and: Condition[] }
  | { or: Condition[] }
  | { not: Condition };

export type AudienceStatus = "active" | "archived";

/** Mirrors `mb_audiences::AudienceDefinition`. */
export interface AudienceDefinition {
  id: string;
  tenant_id: string;
  name: string;
  version: number;
  conditions: Condition;
  status: AudienceStatus;
  created_at: string;
}

export type MembershipKind = "entered" | "exited";

/** Mirrors `mb_audiences::Membership` — current membership state. */
export interface Membership {
  audience_id: string;
  profile_id: string;
  entered_at: string;
  exited_at: string | null;
}

/** Mirrors `mb_connectors::Destination`. */
export interface Destination {
  id: string;
  tenant_id: string;
  kind: string;
  name: string;
  config: Record<string, unknown>;
  /** Section 14 marketing-action vocabulary this destination is declared for. */
  supported_actions: string[];
  created_at: string;
}

/**
 * Mirrors `mb_governance::Reason` — why an activation was denied.
 * Externally tagged (`{"kind": "label_policy_denied", ...}`) via serde's
 * `#[serde(tag = "kind")]`, which is fine here since `Reason` isn't
 * recursive (unlike `Condition`).
 */
export type Reason =
  | { kind: "label_policy_denied"; label: string; action: string; policy_id: string }
  | { kind: "consent_missing"; purpose: string }
  | { kind: "destination_capability"; action: string };

/** One profile skipped during activation, and why. */
export interface BlockedProfile {
  profile_id: string;
  reasons: Reason[];
}

/** Response of `POST /audiences/{id}/activate`. */
export interface ActivationSummary {
  sent: number;
  failed: number;
  blocked: BlockedProfile[];
}

/** One row of `GET /dead-letter-events`. */
export interface DeadLetterEventView {
  id: string;
  event_id: string;
  schema_name: string;
  schema_version: string;
  source: string;
  reason: string;
  raw_payload: unknown;
  created_at: string;
}

export type Effect = "allow" | "deny";

/** Mirrors `mb_governance::Policy`. */
export interface Policy {
  id: string;
  tenant_id: string;
  label: string;
  action: string;
  effect: Effect;
  priority: number;
  created_at: string;
}

/** Mirrors `mb_governance::ConsentEvent` — one append-only consent record. */
export interface ConsentEvent {
  id: string;
  tenant_id: string;
  profile_id: string;
  purpose: string;
  granted: boolean;
  source: string;
  jurisdiction: string | null;
  consent_version: string | null;
  expires_at: string | null;
  created_at: string;
}

/** Mirrors `mb_governance::ConsentState` — derived current state for one purpose. */
export interface ConsentState {
  purpose: string;
  granted: boolean;
  source: string;
  updated_at: string;
}

/** Response of `GET /profiles/{id}/consent`. */
export interface ConsentView {
  current: ConsentState[];
  history: ConsentEvent[];
}

/** Response of `POST /policy-simulate` (Section 49). */
export interface PolicySimulateResponse {
  allowed_fields: string[];
  blocked_fields: string[];
  applicable_policies: Reason[];
  consent_summary: { granted: number; missing: number; total: number } | null;
  final_decision: "allow" | "deny" | "partial";
}

/** Mirrors `mb_channels::Channel`. Only `kind: "webhook"` has a real transport. */
export interface Channel {
  id: string;
  tenant_id: string;
  kind: string;
  name: string;
  config: unknown;
  created_at: string;
}

/** Mirrors `mb_templates::MessageTemplate`. */
export interface MessageTemplate {
  id: string;
  tenant_id: string;
  channel_kind: string;
  name: string;
  version: number;
  subject: string | null;
  body: string;
  created_at: string;
}

/** Mirrors `mb_templates::RenderedTemplate`. */
export interface RenderedTemplate {
  subject: string | null;
  body: string;
  missing_variables: string[];
}

/** Mirrors `mb_journeys::Trigger`. Internally tagged on `kind`, like `NodeKind`. */
export type Trigger =
  | { kind: "audience_entered"; audience_id: string }
  | { kind: "event"; event_type: string };

/** Mirrors `mb_journeys::SplitBranch`. */
export interface SplitBranch {
  next: string;
  weight: number;
}

/**
 * Mirrors `mb_journeys::NodeKind` — internally tagged on `kind`
 * (not recursive, unlike `Condition`, so this is safe to tag).
 */
export type NodeKind =
  | { kind: "wait"; duration_seconds: number; next: string }
  | { kind: "condition"; condition: Condition; if_true: string; if_false: string }
  | { kind: "action"; channel_id: string; template_id: string; next: string }
  | { kind: "split"; branches: SplitBranch[] }
  | { kind: "end" };

/** Mirrors `mb_journeys::Node` — `NodeKind` flattened alongside `id`. */
export type Node = { id: string } & NodeKind;

export type JourneyStatus = "active" | "archived";

/** Mirrors `mb_journeys::JourneyDefinition`. Graph is a flat node list, not nested. */
export interface JourneyDefinition {
  id: string;
  tenant_id: string;
  name: string;
  version: number;
  trigger: Trigger;
  nodes: Node[];
  entry_node: string;
  status: JourneyStatus;
  created_at: string;
}

export type RunStatus = "running" | "waiting" | "completed" | "failed";

/** Mirrors `mb_journeys::JourneyRun`. */
export interface JourneyRun {
  id: string;
  tenant_id: string;
  journey_id: string;
  profile_id: string;
  current_node: string;
  status: RunStatus;
  wake_at: string | null;
  started_at: string;
  updated_at: string;
}

export type RunEventKind =
  | "entered"
  | "waited"
  | "branched"
  | "action_sent"
  | "action_suppressed"
  | "action_failed"
  | "completed";

/** Mirrors `mb_journeys::JourneyRunEvent` — one append-only step log entry. */
export interface JourneyRunEvent {
  id: string;
  run_id: string;
  node_id: string;
  kind: RunEventKind;
  detail: string | null;
  created_at: string;
}

/** Mirrors `mb_journeys::ContactPolicy`. `channel_id: null` applies across all channels. */
export interface ContactPolicy {
  id: string;
  tenant_id: string;
  max_messages: number;
  window_days: number;
  channel_id: string | null;
  created_at: string;
}
