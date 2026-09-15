CREATE TABLE destinations (
    id          uuid PRIMARY KEY,
    tenant_id   uuid NOT NULL,
    kind        text NOT NULL,
    name        text NOT NULL,
    config      jsonb NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX destinations_tenant_idx ON destinations (tenant_id);

-- Every activation attempt, one row per (audience, destination, profile)
-- send. The audit record Section 60 requires.
CREATE TABLE activation_log (
    id              uuid PRIMARY KEY,
    tenant_id       uuid NOT NULL,
    audience_id     uuid NOT NULL,
    destination_id  uuid NOT NULL,
    profile_id      uuid NOT NULL,
    status          text NOT NULL CHECK (status IN ('sent', 'failed')),
    detail          text,
    created_at      timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX activation_log_audience_idx ON activation_log (audience_id, created_at DESC);
