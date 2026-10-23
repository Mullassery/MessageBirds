-- Permissive schemas for the HTTP tracking-source compatibility endpoints.
-- The canonical event still passes through the registry; these schemas only
-- declare the envelope shape so arbitrary client properties and traits stay
-- available as event data without being rejected as unregistered.

INSERT INTO schemas (id, name, version, fields, status, created_at) VALUES
    (gen_random_uuid(), 'http.track',    '1.0', '{}'::jsonb, 'active', now()),
    (gen_random_uuid(), 'http.identify', '1.0', '{}'::jsonb, 'active', now()),
    (gen_random_uuid(), 'http.page',     '1.0', '{}'::jsonb, 'active', now()),
    (gen_random_uuid(), 'http.screen',   '1.0', '{}'::jsonb, 'active', now()),
    (gen_random_uuid(), 'http.group',    '1.0', '{}'::jsonb, 'active', now()),
    (gen_random_uuid(), 'http.alias',    '1.0', '{}'::jsonb, 'active', now())
ON CONFLICT (name, version) DO NOTHING;