-- Backs the "pyreverseetl" destination kind (Phase 6, Track E):
-- PyReverseETL is a bulk source-table -> destination sync tool (its CLI is
-- `create-workflow` + `create-activation` + `execute-activation`, driven
-- by its own real Rust sync engine), not a per-record API like the
-- webhook connector's DestinationConnector::send. Its Postgres source
-- connector is pointed at this table permanently (one workflow, created
-- once outside MessageBirds via the pyreverseetl CLI); MessageBirds
-- truncates and refills the rows scoped to one destination immediately
-- before shelling out to `pyreverseetl execute-activation`, so each run
-- only ever sees the members that just passed the consent check.
--
-- Only standard-mixin fields are exported this way (email/first_name/
-- last_name/phone) — a disclosed scope cut, see docs/ARCHITECTURE.md.
CREATE TABLE pyreverseetl_export_staging (
    id              uuid PRIMARY KEY,
    tenant_id       uuid NOT NULL,
    destination_id  uuid NOT NULL REFERENCES destinations (id),
    profile_id      uuid NOT NULL,
    email           text,
    first_name      text,
    last_name       text,
    phone           text,
    created_at      timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX pyreverseetl_export_staging_destination_idx
    ON pyreverseetl_export_staging (destination_id);
