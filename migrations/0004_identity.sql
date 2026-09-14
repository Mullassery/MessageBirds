CREATE TABLE identity_nodes (
    id              uuid PRIMARY KEY,
    tenant_id       uuid NOT NULL,
    namespace       text NOT NULL,
    value_hash      text NOT NULL,
    -- Deliberately no FK to profiles(id): identity resolution can mint a
    -- profile id before the profile projection row exists; the profile
    -- crate upserts on that id. Loose coupling by design.
    profile_id      uuid NOT NULL,
    confidence      double precision NOT NULL DEFAULT 1.0,
    source          text NOT NULL,
    first_seen_at   timestamptz NOT NULL DEFAULT now(),
    last_seen_at    timestamptz NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, namespace, value_hash)
);

CREATE INDEX identity_nodes_profile_idx ON identity_nodes (tenant_id, profile_id);

-- Append-only. Never UPDATEd or DELETEd by application code — identity
-- history must always be reconstructable (Section 8).
CREATE TABLE identity_audit (
    id                      uuid PRIMARY KEY,
    tenant_id               uuid NOT NULL,
    kind                    text NOT NULL CHECK (kind IN ('linked', 'merged')),
    namespace               text,
    value_hash              text,
    profile_id              uuid NOT NULL,
    previous_profile_id     uuid,
    source                  text NOT NULL,
    created_at              timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX identity_audit_profile_idx ON identity_audit (tenant_id, profile_id, created_at);
