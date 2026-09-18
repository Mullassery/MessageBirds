-- PyReverseETL's generic Postgres source connector dispatches on Postgres
-- type names (`core/src/connectors/postgres.rs::pg_row_to_json` in the
-- PyReverseETL repo) and has no branch for native `uuid` columns — they
-- fall through to a `try_get::<String, _>`, which sqlx's Postgres driver
-- does not support for the wire-level UUID type, so every uuid column
-- silently decodes as `null`. Discovered by running the real integration
-- end to end, not a hypothetical.
--
-- The fix is this view, not changing `pyreverseetl_export_staging`'s real
-- column types (uuid stays uuid there for FK integrity and index
-- performance) — PyReverseETL's workflow `table` points at this view,
-- which casts every id column to text, a type its generic reader does
-- handle.
CREATE VIEW pyreverseetl_export_staging_view AS
SELECT
    id::text            AS id,
    tenant_id::text      AS tenant_id,
    destination_id::text AS destination_id,
    profile_id::text     AS profile_id,
    email,
    first_name,
    last_name,
    phone,
    created_at
FROM pyreverseetl_export_staging;
