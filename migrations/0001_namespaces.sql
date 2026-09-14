CREATE TABLE namespaces (
    id          uuid PRIMARY KEY,
    key         text NOT NULL UNIQUE,
    kind        text NOT NULL CHECK (kind IN ('deterministic', 'probabilistic')),
    priority    integer NOT NULL DEFAULT 0,
    created_at  timestamptz NOT NULL DEFAULT now()
);

INSERT INTO namespaces (id, key, kind, priority, created_at) VALUES
    (gen_random_uuid(), 'customer_id',   'deterministic', 100, now()),
    (gen_random_uuid(), 'crm_id',        'deterministic', 90,  now()),
    (gen_random_uuid(), 'email',         'deterministic', 80,  now()),
    (gen_random_uuid(), 'phone',         'deterministic', 70,  now()),
    (gen_random_uuid(), 'loyalty_id',    'deterministic', 60,  now()),
    (gen_random_uuid(), 'device_id',     'deterministic', 40,  now()),
    (gen_random_uuid(), 'app_instance_id','deterministic', 40, now()),
    (gen_random_uuid(), 'ga4_id',        'deterministic', 30,  now()),
    (gen_random_uuid(), 'anonymous_id',  'deterministic', 10,  now());
