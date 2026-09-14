CREATE TABLE merge_policies (
    id          uuid PRIMARY KEY,
    tenant_id   uuid NOT NULL,
    name        text NOT NULL,
    strategy    text NOT NULL CHECK (strategy IN (
                    'SOURCE_PRIORITY', 'LATEST_TIMESTAMP', 'EARLIEST_TIMESTAMP',
                    'MOST_TRUSTED_SOURCE', 'CONFIDENCE_WEIGHTED', 'FIELD_LEVEL', 'CUSTOM'
                )),
    config      jsonb NOT NULL DEFAULT '{}'::jsonb,
    version     integer NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, name, version)
);

CREATE INDEX merge_policies_tenant_idx ON merge_policies (tenant_id, created_at DESC);
