CREATE TABLE channels (
    id          uuid PRIMARY KEY,
    tenant_id   uuid NOT NULL,
    kind        text NOT NULL,
    name        text NOT NULL,
    config      jsonb NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX channels_tenant_idx ON channels (tenant_id);
