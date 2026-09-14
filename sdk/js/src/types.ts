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
