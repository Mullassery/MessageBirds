-- (tenant, max_messages in window_days[, channel_id]) — a send exceeding
-- this is suppressed (the run still advances), not the whole run blocked.
CREATE TABLE contact_policies (
    id              uuid PRIMARY KEY,
    tenant_id       uuid NOT NULL,
    max_messages    integer NOT NULL,
    window_days     integer NOT NULL,
    channel_id      uuid REFERENCES channels (id),
    created_at      timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX contact_policies_tenant_idx ON contact_policies (tenant_id);

-- One row per Action-node send attempt. UNIQUE(run_id, node_id) is what
-- makes a retry after a crash idempotent — see engine::execute_action.
CREATE TABLE messages_sent (
    id          uuid PRIMARY KEY,
    tenant_id   uuid NOT NULL,
    run_id      uuid NOT NULL REFERENCES journey_runs (id),
    node_id     text NOT NULL,
    channel_id  uuid NOT NULL REFERENCES channels (id),
    profile_id  uuid NOT NULL,
    subject     text,
    body        text NOT NULL,
    status      text NOT NULL CHECK (status IN ('sent', 'failed', 'suppressed')),
    detail      text,
    created_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE (run_id, node_id)
);

CREATE INDEX messages_sent_profile_idx ON messages_sent (tenant_id, profile_id, status, created_at DESC);
