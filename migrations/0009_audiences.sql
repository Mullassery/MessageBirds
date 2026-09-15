CREATE TABLE audiences (
    id          uuid PRIMARY KEY,
    tenant_id   uuid NOT NULL,
    name        text NOT NULL,
    version     integer NOT NULL,
    conditions  jsonb NOT NULL,
    status      text NOT NULL CHECK (status IN ('active', 'archived')),
    created_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, name, version)
);

CREATE INDEX audiences_tenant_active_idx ON audiences (tenant_id, status);

-- Current membership state. Re-entering after exiting updates this row in
-- place; full history lives in audience_membership_events below.
CREATE TABLE audience_memberships (
    audience_id     uuid NOT NULL REFERENCES audiences (id),
    profile_id      uuid NOT NULL,
    entered_at      timestamptz NOT NULL,
    exited_at       timestamptz,
    PRIMARY KEY (audience_id, profile_id)
);

CREATE INDEX audience_memberships_profile_idx ON audience_memberships (profile_id);
CREATE INDEX audience_memberships_active_idx ON audience_memberships (audience_id) WHERE exited_at IS NULL;

-- Append-only. Every entered/exited transition, even ones the current-state
-- table above has since overwritten.
CREATE TABLE audience_membership_events (
    id          uuid PRIMARY KEY,
    tenant_id   uuid NOT NULL,
    audience_id uuid NOT NULL,
    profile_id  uuid NOT NULL,
    kind        text NOT NULL CHECK (kind IN ('entered', 'exited')),
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX audience_membership_events_profile_idx ON audience_membership_events (tenant_id, profile_id, created_at);
