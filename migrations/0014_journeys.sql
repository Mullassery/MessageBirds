CREATE TABLE journeys (
    id          uuid PRIMARY KEY,
    tenant_id   uuid NOT NULL,
    name        text NOT NULL,
    version     integer NOT NULL,
    trigger     jsonb NOT NULL,
    nodes       jsonb NOT NULL,
    entry_node  text NOT NULL,
    status      text NOT NULL CHECK (status IN ('active', 'archived')),
    created_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, name, version)
);

CREATE INDEX journeys_tenant_active_idx ON journeys (tenant_id, status);

-- Durable execution state. Persisted after every node transition — see
-- crates/journeys/src/engine.rs.
CREATE TABLE journey_runs (
    id              uuid PRIMARY KEY,
    tenant_id       uuid NOT NULL,
    journey_id      uuid NOT NULL REFERENCES journeys (id),
    profile_id      uuid NOT NULL,
    current_node    text NOT NULL,
    status          text NOT NULL CHECK (status IN ('running', 'waiting', 'completed', 'failed')),
    wake_at         timestamptz,
    started_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX journey_runs_due_idx ON journey_runs (status, wake_at) WHERE status = 'waiting';
CREATE INDEX journey_runs_journey_idx ON journey_runs (journey_id, started_at DESC);
CREATE INDEX journey_runs_profile_idx ON journey_runs (tenant_id, profile_id, started_at DESC);

-- Append-only step log — the "why did this happen" trail for a run.
CREATE TABLE journey_run_events (
    id          uuid PRIMARY KEY,
    run_id      uuid NOT NULL REFERENCES journey_runs (id),
    node_id     text NOT NULL,
    kind        text NOT NULL CHECK (kind IN (
                    'entered', 'waited', 'branched', 'action_sent',
                    'action_suppressed', 'action_failed', 'completed'
                )),
    detail      text,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX journey_run_events_run_idx ON journey_run_events (run_id, created_at);
