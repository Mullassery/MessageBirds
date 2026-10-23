-- Registered event schemas for Squid webhook envelope v1 ingestion.
-- The adapter maps each operation to one of these object schemas. They are
-- intentionally permissive because Squid owns the record field set; the
-- canonical envelope shape is validated separately by the adapter.

INSERT INTO schemas (id, name, version, fields, status, created_at) VALUES
    (gen_random_uuid(), 'squid.person',      '1.0', '{}'::jsonb, 'active', now()),
    (gen_random_uuid(), 'squid.company',     '1.0', '{}'::jsonb, 'active', now()),
    (gen_random_uuid(), 'squid.opportunity', '1.0', '{}'::jsonb, 'active', now()),
    (gen_random_uuid(), 'squid.task',        '1.0', '{}'::jsonb, 'active', now()),
    (gen_random_uuid(), 'squid.note',        '1.0', '{}'::jsonb, 'active', now())
ON CONFLICT (name, version) DO NOTHING;