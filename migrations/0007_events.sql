-- The immutable event store: written by the worker once an event passes
-- schema validation and identity resolution. This, not the profile table,
-- is the source of truth (Section 3, Section 9).
CREATE TABLE events (
    id              uuid PRIMARY KEY,
    tenant_id       uuid NOT NULL,
    event_type      text NOT NULL,
    schema_name     text NOT NULL,
    schema_version  text NOT NULL,
    identity        jsonb NOT NULL,
    context         jsonb NOT NULL,
    data            jsonb NOT NULL,
    profile_id      uuid NOT NULL,
    received_at     timestamptz NOT NULL DEFAULT now(),
    occurred_at     timestamptz NOT NULL
);

CREATE INDEX events_profile_idx ON events (profile_id, occurred_at DESC);
CREATE INDEX events_tenant_idx ON events (tenant_id, occurred_at DESC);

-- Events that failed schema validation (Section 11). Queryable for a data
-- quality dashboard; a dedicated Kafka DLQ topic is a later phase.
CREATE TABLE dead_letter_events (
    id              uuid PRIMARY KEY,
    tenant_id       uuid NOT NULL,
    event_id        uuid NOT NULL UNIQUE,
    schema_name     text NOT NULL,
    schema_version  text NOT NULL,
    source          text NOT NULL,
    reason          text NOT NULL,
    raw_payload     jsonb NOT NULL,
    created_at      timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX dead_letter_events_tenant_idx ON dead_letter_events (tenant_id, created_at DESC);
