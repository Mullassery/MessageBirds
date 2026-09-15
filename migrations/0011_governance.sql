CREATE TABLE policies (
    id          uuid PRIMARY KEY,
    tenant_id   uuid NOT NULL,
    label       text NOT NULL,
    action      text NOT NULL,
    effect      text NOT NULL CHECK (effect IN ('allow', 'deny')),
    priority    integer NOT NULL DEFAULT 0,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX policies_tenant_label_action_idx ON policies (tenant_id, label, action);

-- Append-only. Never UPDATEd or DELETEd — consent history must stay
-- reconstructable (Section 17).
CREATE TABLE consent_events (
    id              uuid PRIMARY KEY,
    tenant_id       uuid NOT NULL,
    profile_id      uuid NOT NULL,
    purpose         text NOT NULL,
    granted         boolean NOT NULL,
    source          text NOT NULL,
    jurisdiction    text,
    consent_version text,
    expires_at      timestamptz,
    created_at      timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX consent_events_profile_purpose_idx ON consent_events (tenant_id, profile_id, purpose, created_at DESC);
