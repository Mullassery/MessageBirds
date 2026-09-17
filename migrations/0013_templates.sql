CREATE TABLE message_templates (
    id              uuid PRIMARY KEY,
    tenant_id       uuid NOT NULL,
    channel_kind    text NOT NULL,
    name            text NOT NULL,
    version         integer NOT NULL,
    subject         text,
    body            text NOT NULL,
    created_at      timestamptz NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, name, version)
);

CREATE INDEX message_templates_tenant_idx ON message_templates (tenant_id, name, version DESC);
