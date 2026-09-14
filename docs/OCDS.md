# Open Customer Data Schema (OCDS)

OCDS is the canonical, implementation-independent data model MessageBirds is built around. It exists so that SDKs, connectors, the profile store, the merge policy engine, and (eventually) governance and activation all speak the same language — no connector gets to invent its own customer JSON.

This document describes the model as implemented in Phase 1. It will grow as later phases (governance labels, consent, marketing actions — see `ARCHITECTURE.md`) add their own primitives.

## Event envelope

Every event crossing the platform — `POST /events`, the `events.raw` Kafka topic, the immutable `events` table — is this shape:

```json
{
  "event_id": "3fa85f64-5717-4562-b3fc-2c963f66afa6",
  "event_type": "commerce.product_view",
  "timestamp": "2026-09-14T10:00:00Z",
  "source": { "type": "web", "name": "website" },
  "identity": [
    { "namespace": "anonymous_id", "value": "abc123", "primary": true, "source": "web-sdk", "confidence": 1.0 }
  ],
  "context": {
    "profile_updates": {
      "core/person@1.0": { "first_name": "Jane" },
      "core/contact@1.0": { "email": "jane@example.com" }
    }
  },
  "data": { "product_id": "p1", "category": "shoes", "price": 79.99 },
  "schema": { "name": "commerce.product_view", "version": "1.0" },
  "tenant_id": "b3b3b3b3-...",
  "consent": null,
  "governance": null,
  "correlation_id": null,
  "trace_id": null
}
```

Events are immutable and never mutated in place. Profiles are projections *derived from* streams of these events, not the source of truth (see `ARCHITECTURE.md`).

`data` is validated against the event's registered schema. `context.profile_updates`, if present, is a map of **mixin key** to the field values this event contributes to that mixin on the resolved profile — that's the mechanism by which an event updates the customer profile (see "Mixins" below).

## Schemas

A schema defines the shape of one event type's `data` payload: field names, types, which are required, and (optionally) enumerations. Schemas are versioned; a new version of an existing schema name must be backward compatible with the previous one (no dropped required fields, no changed field types, no newly-required fields) or registration is rejected with the specific incompatibility — schema semantics never change silently.

```json
{
  "name": "commerce.product_view",
  "version": "1.0",
  "fields": {
    "product_id": { "type": "string", "required": true },
    "category": { "type": "string", "required": false },
    "price": { "type": "number", "required": false }
  }
}
```

Manage schemas via `GET/POST /schemas` and `GET /schemas/{name}/{version}`.

## Mixins

A mixin is a reusable, composable, versioned semantic block that a **profile** — not an event — is built out of. Where a schema shapes one event type, a mixin shapes one facet of a customer (`person`, `contact`, `device`, ...), and a profile is the union of every mixin that's been populated for it.

Mixin definitions live under a namespace: `core` is reserved for the standard library (`core/identity@1.0`, `core/person@1.0`, ...); anything else is a tenant-owned custom mixin (`acme/fitness_membership@1.0`), registered via `POST /mixins`. Custom mixins cannot use the `core` namespace — that's what keeps them from corrupting the standard model.

### Standard mixin library (this phase)

| Mixin | Fields |
|---|---|
| `core/identity@1.0` | `anonymous_id`, `customer_id`, `crm_id`, `primary_namespace` |
| `core/person@1.0` | `first_name`, `last_name`, `birth_date`, `gender` |
| `core/contact@1.0` | `email`, `phone`, `address` |
| `core/device@1.0` | `device_id`, `platform`, `app_version`, `push_token` |

Definitions live as YAML under `standard-mixins/` — that's the single source of truth, embedded into the `mb-mixins` crate at compile time and seeded into the registry at service startup. The rest of the library described in the product spec (`commerce`, `loyalty`, `subscription`, `consent`, `engagement`, ...) arrives in later phases alongside the primitives that consume them (audiences, governance, engagement).

`core/identity@1.0` is special: its fields are derived automatically from an event's `identity` claims by the worker, not supplied via `context.profile_updates` — the identity graph is the source of truth for "who is this."

## Namespaces

An identity namespace (`email`, `phone`, `anonymous_id`, `customer_id`, `crm_id`, `device_id`, ...) is registered data (`GET /namespaces`), not a hardcoded enum — new namespaces can be introduced without a code change. Each has a priority used when identity claims disagree about which profile they belong to.

## Identity graph

Every identity claim on an event (`{namespace, value, primary, source, confidence}`) is resolved against a graph of `(tenant, namespace, value) -> profile_id` links. Values are stored hashed, never in the clear. Resolution is deterministic (exact match) in this phase — probabilistic matching is a later phase. When claims on one event point at two different existing profiles, the profile linked to the `primary` claim wins and the other is merged into it; every link and merge is recorded in an append-only audit trail, never overwritten.

A client that only knows its own claim (e.g. the `anonymous_id` it generated) can look its profile up via `GET /profiles/by-identity?tenant_id=...&namespace=...&value=...`.

## Merge policies

When more than one source has reported a value for the same profile field, the tenant's merge policy decides which one wins. Implemented this phase: `SOURCE_PRIORITY` (an ordered list of trusted sources) and `LATEST_TIMESTAMP` (most recent observation wins — also the fallback when no source priority is configured). The rest of the strategy list (`EARLIEST_TIMESTAMP`, `MOST_TRUSTED_SOURCE`, `CONFIDENCE_WEIGHTED`, `FIELD_LEVEL`, `CUSTOM`) is modeled but returns an explicit "not implemented in this phase" error rather than silently behaving like a different strategy.

## Profiles

A profile is the resolved, mixin-composed view of a customer: `{ id, tenant_id, mixins, ... }` where `mixins` is `{"core/person@1.0": {"first_name": "Jane"}, ...}`. Every winning field value also has a provenance record (`source`, `event_id`, `applied_policy`) — the seed for full field-level lineage. Fetch via `GET /profiles/{id}` (includes provenance) or `GET /profiles/{id}/events` (the event timeline).
