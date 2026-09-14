CREATE TABLE mixins (
    id          uuid PRIMARY KEY,
    namespace   text NOT NULL,
    name        text NOT NULL,
    version     text NOT NULL,
    fields      jsonb NOT NULL,
    status      text NOT NULL CHECK (status IN ('active', 'deprecated')),
    created_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE (namespace, name, version)
);

CREATE INDEX mixins_namespace_name_idx ON mixins (namespace, name, created_at DESC);

-- The standard library (core/identity, core/person, core/contact,
-- core/device @1.0) is seeded at service startup from standard-mixins/*.yaml
-- (see mb-mixins::PgMixinRepo::seed_standard_library), not here, so the
-- YAML files stay the single source of truth for their shape.
