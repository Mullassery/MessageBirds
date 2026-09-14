CREATE TABLE profiles (
    id          uuid PRIMARY KEY,
    tenant_id   uuid NOT NULL,
    mixins      jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX profiles_tenant_idx ON profiles (tenant_id);

-- Tracks which source currently "won" each profile field under the active
-- merge policy — the seed for full field-level lineage (Section 12).
CREATE TABLE profile_field_provenance (
    profile_id      uuid NOT NULL,
    mixin_key       text NOT NULL,
    field_path      text NOT NULL,
    value           jsonb NOT NULL,
    source          text NOT NULL,
    event_id        uuid NOT NULL,
    applied_policy  text NOT NULL,
    updated_at      timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (profile_id, mixin_key, field_path)
);
