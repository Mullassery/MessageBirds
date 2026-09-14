CREATE TABLE schemas (
    id          uuid PRIMARY KEY,
    name        text NOT NULL,
    version     text NOT NULL,
    fields      jsonb NOT NULL,
    status      text NOT NULL CHECK (status IN ('active', 'deprecated')),
    created_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE (name, version)
);

CREATE INDEX schemas_name_idx ON schemas (name, created_at DESC);

INSERT INTO schemas (id, name, version, fields, status, created_at) VALUES (
    gen_random_uuid(),
    'commerce.product_view',
    '1.0',
    '{
        "product_id": {"type": "string", "required": true},
        "category": {"type": "string", "required": false},
        "price": {"type": "number", "required": false}
    }'::jsonb,
    'active',
    now()
);
