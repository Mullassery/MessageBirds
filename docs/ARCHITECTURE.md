# Architecture

MessageBirds is built around one principle: **COLLECT → STANDARDIZE → RESOLVE → UNDERSTAND → DECIDE → ORCHESTRATE → ACT → MEASURE**. This phase implements the first three steps — collect, standardize, resolve — plus enough of a profile projection to prove the canonical model holds together end to end. See `OCDS.md` for the data model itself.

## What's built (Phase 1)

```
 client / SDK
      |
      v
 POST /events  (mb-api) ── validates envelope shape only
      |
      v
 Kafka/Redpanda: events.raw   (mb-events)
      |
      v
 worker (mb-worker)
      |
      +-- schema-registry: validate `data` against the registered schema
      |     (fail -> dead_letter_events, Section 11)
      |
      +-- identity: resolve claims against the identity graph,
      |     mint/merge profile ids, append-only audit trail (Section 8)
      |
      +-- merge-policy: look up the tenant's active policy (Section 10)
      |
      +-- mixins: validate context.profile_updates against registered
      |     mixin definitions
      |
      +-- profile: apply each mixin's fields through the merge policy,
      |     write field provenance (Section 12 seed)
      |
      +-- persist the raw event (immutable, source of truth)
      |
      v
 Postgres: events, profiles, profile_field_provenance,
           identity_nodes, identity_audit, schemas, mixins,
           namespaces, merge_policies, dead_letter_events
      |
      v
 GET /profiles/{id}, GET /profiles/by-identity, GET /profiles/{id}/events
 GET/POST /schemas, GET/POST /mixins, GET /namespaces   (mb-api)
```

Crates map directly onto this: `mb-core` (shared types, no I/O) → `mb-namespaces` / `mb-schema-registry` / `mb-mixins` / `mb-identity` / `mb-merge-policy` / `mb-profile` (one per canonical primitive) → `mb-events` (Kafka wrappers) → `mb-api` / `mb-worker` (the two binaries that wire everything together). `sdk/js` is a thin TypeScript client used by `sdk/js/examples/smoke-test.ts` to prove the loop end to end.

## Deployment (this phase)

Minimal dev mode only: Postgres + Redpanda via `docker-compose.yml`, `api` and `worker` run directly with `cargo run`. The heavier "enterprise" deployment described in the product spec (Kafka cluster, Flink, Temporal, ClickHouse, object storage, Kubernetes) is not needed to prove the foundation and isn't set up yet.

## What's explicitly NOT built yet

This is deliberate, not an oversight — the product spec's own guidance is to get the foundation right before layering anything on top of it:

- **Audiences** (attribute/event/sequence/temporal segmentation, streaming membership)
- **Governance labels, consent engine, marketing actions, data usage policy engine, policy simulator**
- **Activation / destinations** (webhook, Braze, email, ads, warehouses, ...)
- **Journeys, durable orchestration (Temporal), visual canvas**
- **Decisioning** (rules, scoring, next-best-action)
- **AI agents, AI assistant, model provider abstraction**
- **CLI, admin UI, identity graph UI, policy simulator UI**
- **Multi-tenancy enforcement (RBAC, SSO, tenant isolation)** — `tenant_id` is threaded through every table, but nothing yet stops one tenant from querying another's data; there's no auth layer at all
- **Probabilistic identity matching** — resolution is exact-match only
- **Full lineage** — `profile_field_provenance` records only the *winning* source per field, not every observation
- **Data retention / delete-and-forget workflows**
- **Full standard mixin library** (`commerce`, `loyalty`, `subscription`, `consent`, `engagement`, ...) — only `identity`, `person`, `contact`, `device` are seeded

## Roadmap

Phases as described in the product spec, in order:

1. **Core event platform** — this milestone
2. **Identity** — deterministic matching is done; probabilistic matching, richer merge/split tooling, profile timeline UI are next
3. **CDP** — audiences (batch + streaming), connectors beyond the HTTP API, data quality dashboard, full lineage UI
4. **Governance** — labels, marketing actions, consent engine, data usage policy engine, policy simulator
5. **Engagement** — message templates, channel adapters (email/push/SMS/WhatsApp), journeys, Temporal-backed durable orchestration
6. **Decisioning** — rules, scoring, next-best-action, frequency/contact policy
7. **AI** — AI assistant, tool-using agents, agent governance, model provider abstraction
8. **Enterprise** — multi-tenancy enforcement, RBAC/SSO, Terraform provider, GitOps, Kubernetes, enterprise connectors (Salesforce, Snowflake, BigQuery, ...)
