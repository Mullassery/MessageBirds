# PyCustomerHub

An open-source, event-driven Customer Data & Engagement Platform (CDP) built in Rust and TypeScript.

[![CI](https://github.com/Mullassery/PyCustomerHub/actions/workflows/ci.yml/badge.svg)](https://github.com/Mullassery/PyCustomerHub/actions/workflows/ci.yml)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![pycustomerhub on PyPI](https://img.shields.io/pypi/v/pycustomerhub.svg?label=pycustomerhub)](https://pypi.org/project/pycustomerhub/)

## What it is

PyCustomerHub is a self-hosted CDP and engagement platform that implements the full collection-to-activation pipeline: ingestion → schema validation → mixin composition → identity resolution → merge policy → profile projection → real-time audience evaluation → governed activation → durable journeys with templated sends. It accepts both native OCDS envelopes (`POST /events`) and a common tracking HTTP contract (`/v1/track`, `/v1/identify`, `/v1/page`, `/v1/screen`, `/v1/group`, `/v1/alias`, `/v1/batch`), runs on Postgres + Kafka (Redpanda) with no external accounts, ships client SDKs for web, iOS, Android, JS, and Python, and provides a Next.js viewer UI. Activation uses a shared retry/backoff/rate-limit policy. The pipeline is real, wired, and integration-tested end to end — see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for precisely what is built vs. deferred at each layer.

## Features / Highlights

- **Identity resolution with provenance** — deterministic matching, explicit merge/split, confidence-scored merge suggestions, and an append-only audit trail; every profile field records which event set it, its source, and the conflict policy applied. Verified live on real event sequences (correct merges, no false merges).
- **Real-time audiences** — rule-based definitions with attribute, event-occurrence, and temporal conditions, evaluated synchronously against streaming events; append-only membership history.
- **Governance-first activation** — label-based deny-list policies (e.g. `Deny(PII, ADVERTISING)`), destination capability checks, an append-only consent ledger, and a policy simulator that reuses the real evaluation primitives.
- **Durable journeys** — a flat-node-graph engine (Wait/Condition/Action/Split/End) with Postgres-backed durable run state and a poller that resumes runs across a restart; templated webhook sends.
- **Measured single-node ingestion** — live ingestion measured at ~156 events/sec (sequential, single connection) on the reference machine; zero external accounts to stand up.
- **Client SDKs** — `@pycustomerhub/web` (queued, retrying, `sendBeacon` flush), `sdk/ios` (Swift, build-verified), `sdk/js` and `sdk/python` (same core scope), `sdk/android` (written, not yet build-verified), plus a Cloudflare Workers edge ingestion gateway for CORS.
- **HTTP tracking-source compatibility** — `/v1/track`, `/v1/identify`, `/v1/page`, `/v1/screen`, `/v1/group`, `/v1/alias`, and `/v1/batch` accept the common analytics HTTP shape, so existing client instrumentation can point at PyCustomerHub without being rewritten. Write keys map to tenants; messages are translated into OCDS and run through the same identity/audience/journey pipeline.
- **Signed CRM webhooks** — `/v1/squid` verifies Squid envelope v1 HMAC signatures, maps workspace IDs to tenants, registers object-specific schemas, and preserves record context. Person names/emails/phones update the standard mixins; other objects stay event data.
- **Destination connectors** — webhook, HubSpot contacts (batch upsert by email), and Klaviyo profile import, all behind the same governance/consent gate.
- **Bulk activation** — a fourth destination kind alongside webhook/HubSpot/Klaviyo via [PyReverseETL](https://github.com/Mullassery/PyReverseETL)'s real sync engine.
- **Delivery policy** — every destination shares retry with capped exponential backoff for transient failures, plus optional per-destination rate limiting. Permanent config, payload, and SSRF failures fail immediately.
- **Clear extension points** — inbound sources translate into one canonical event envelope; outbound connectors implement one small trait and inherit the shared delivery policy.

## Who it's for / Use cases

- Engineering and data teams building a **self-hosted, local-first CDP** who want real identity resolution, audiences, consent-aware activation, and journeys without a proprietary control plane.
- Teams with **existing tracking instrumentation** that want to keep their current `track` / `identify` calls while moving profile, audience, and journey logic into their own infrastructure.
- Teams evaluating a transparent, single-layer event pipeline (one Kafka topic, one worker) over a larger multi-service stack.
- Privacy/ops-minded builders who want label-based governance, retry-aware delivery, and full activation audit logs (`sent`/`failed`/`blocked`).

## What it is not

- **Not a large connector library.** Three real per-profile destination connectors ship today (webhook, HubSpot contacts, Klaviyo profiles) plus the bulk-sync path via PyReverseETL. There is no pre-built catalog of dozens of destinations.
- **Not a transformation runtime yet.** Field-level mapping is governed by schemas/mixins and connector adapters; user-defined transformation execution is planned, not shipped.
- **Not a decisioning/AI stack.** Decisioning, AI agents, and a CLI are not implemented.
- **No sequence conditions in audiences** (e.g. `A then B then not C`) — that's a genuinely different, ordered evaluation model and is not built.
- **Not hardened at scale.** It has not run production at scale or been independently security-reviewed — the operational hardening of established commercial platforms is a real, acknowledged gap. You must set `PYCUSTOMERHUB_IDENTITY_HASH_PEPPER` before pointing it at real PII (see SECURITY.md).
- **Young.** Single-maintainer, phases 1–5 built; the Android SDK is written but not build-verified, and the edge ingest gateway is built/tested locally but **not deployed**.

## Project status

`v0.8.0` (Rust workspace) / `pycustomerhub` 0.8.0 is the latest published release. The current tree also contains unreleased integration work: HTTP tracking-source compatibility, write-key tenant mapping, seeded `http.*` schemas, signed Squid envelope-v1 webhook ingestion with workspace→tenant mapping, seeded `squid.*` schemas, and shared connector retry/backoff/rate limiting.

Phases 1–5 of the reference architecture are built (core event platform → identity → CDP → governance → engagement), with client-side collection, the edge gateway, HTTP tracking ingestion, and PyReverseETL activation layered on top. [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) is the source of truth for what's real vs. deferred at each layer, and [ROADMAP_HONEST.md](ROADMAP_HONEST.md) tracks technical debt and security findings with file:line specifics.

## Installation

Requires Rust (stable), Docker, and Node.js. Start the dev infrastructure and services:

```bash
# 1. Start dev infra (Postgres + Redpanda)
docker compose up -d postgres redpanda

# 2. Run the services (pch-api applies migrations and seeds the standard mixin
#    library on startup — no separate migrate step needed)
DATABASE_URL=postgres://pycustomerhub:pycustomerhub@localhost:5432/pycustomerhub cargo run -p pch-api
DATABASE_URL=postgres://pycustomerhub:pycustomerhub@localhost:5432/pycustomerhub cargo run -p pch-worker

# 3. Send a test event and watch a profile materialize
npm install                                  # installs sdk/js and ui workspaces
npm run smoke-test --workspace=sdk/js

# 4. Browse it
npm run dev --workspace=ui                   # http://localhost:3000
```

Set `PYCUSTOMERHUB_API_KEYS` to a JSON object mapping keys to tenant UUIDs
(`{"key-a":"<uuid>"}`) or to objects with a role
(`{"viewer":{"tenant_id":"<uuid>","role":"read_only"}}`); `writer` can write
profiles/events/audiences but cannot modify destinations, policies, channels,
templates, journeys, schemas, or mixins. Keys require
`Authorization: Bearer <key>` on every route except `/healthz`; tenant
mismatches are rejected with `403`, and `read_only` keys can only use
GET/HEAD/OPTIONS. The single-key `PYCUSTOMERHUB_API_KEY` (+ `_TENANT`)
still works as an admin key. For SSO, set `PYCUSTOMERHUB_TRUST_PROXY_AUTH=1`
behind an authenticating proxy that strips client headers: the API reads
`X-Auth-Request-User`, `X-Auth-Request-Groups` (`pycustomerhub-admin`/`-writer`),
and `X-Auth-Request-Tenant`. Leave all unset for local development.

Before pointing this at real PII, set `PYCUSTOMERHUB_IDENTITY_HASH_PEPPER` (a random per-deployment secret) on the `api`, `worker`, and `journeys-worker` processes — it is mixed into the identity-matching hash in `crates/identity/src/hash.rs`. Leaving it unset reproduces the old unsalted behavior; the legacy `MB_IDENTITY_HASH_PEPPER` name is also read. See `SECURITY.md`.

## HTTP tracking source

Existing analytics clients can send their usual `track` / `identify` /
`page` / `screen` / `group` / `alias` calls to PyCustomerHub. Map one or
more write keys to tenants on the API process:

```bash
export PYCUSTOMERHUB_HTTP_WRITE_KEYS='{"wk_live_123":"<tenant-uuid>"}'
```

Send a write key with `X-Write-Key: wk_live_123`, HTTP Basic auth
(username = write key), or a `"writeKey"` field in the JSON body. The
source fails closed when no keys are configured or no key matches; it does
not fall back to an unauthenticated tenant.

```bash
curl -X POST http://localhost:8080/v1/identify \
  -H 'Content-Type: application/json' \
  -H 'X-Write-Key: wk_live_123' \
  -d '{"userId":"user-9","traits":{"email":"jane@example.com","firstName":"Jane"}}'

curl -X POST http://localhost:8080/v1/track \
  -H 'Content-Type: application/json' \
  -H 'X-Write-Key: wk_live_123' \
  -d '{"anonymousId":"anon-1","event":"Order Completed","properties":{"revenue":42}}'
```

`/v1/batch` accepts `{"batch":[{"type":"track", ...}, ...]}`. The
translation is deliberate: `identify` traits update `core/contact@1.0` and
`core/person@1.0`; `track` properties stay event data; group ids are kept
in context rather than merged into person profiles; unknown client context
is preserved. All events use seeded `http.*` schemas, so they flow through
normal schema validation instead of being rejected as unregistered.

## Squid webhook source

Squid's signed record webhooks can be delivered directly to
`POST /v1/squid`. Map each Squid workspace to a PyCustomerHub tenant and
its webhook secrets:

```bash
export PYCUSTOMERHUB_SQUID_WORKSPACES='{
  "<squid-workspace-uuid>": {
    "tenant_id": "<pycustomerhub-tenant-uuid>",
    "secrets": ["<webhook-secret>", "<rotated-secret>"]
  }
}'
```

Squid creates a separate secret per webhook configuration, so list every
active secret for the workspace. A single `"secret"` string is also
accepted. The endpoint verifies `x-squid-signature: sha256=<hex>` over the
exact raw body, cross-checks the event/workspace/schema headers, and fails
closed when the workspace or secrets are unknown. It accepts
`person|company|opportunity|task|note` `created|updated|deleted|soft_deleted`
operations.

Mapping is explicit:

- Envelope `id` → `event_id`; `type` → `event_type`; `occurredAt` → timestamp.
- `data.id` → `crm_id`; person primary email → `email`.
- Person name/email/phone populate the standard `person` and `contact` mixins.
- Company, opportunity, task, and note events use their own `crm_id` as the
  profile key and keep their record shape in event data/context; they do not
  fake person fields.
- `deleted` and `soft_deleted` events are recorded as events; they do not
  delete the PyCustomerHub profile. Profile deletion has its own governance
  path.
- Each object uses a registered `squid.<object>@1.0` schema, so events pass
  normal validation instead of landing in the dead-letter queue.
- Squid does not emit consent metadata; consent remains a downstream policy
  decision rather than an invented field.

## Edge ingestion gateway

The optional Cloudflare Workers gateway for browser traffic now performs
write-key authentication before forwarding to `POST /events`:

```bash
export PYCUSTOMERHUB_EDGE_WRITE_KEYS='{"wk_live_123":"<tenant-uuid>"}'
```

It accepts `X-Write-Key`, `Authorization: Bearer <key>`, or the legacy
`tenant_id` field as the credential, rewrites `tenant_id` to the mapped
tenant, and fails closed when no mapping is configured. Rate limiting and
production deployment remain future work.

## Integration model

PyCustomerHub has one canonical event model and two integration directions:

- **Inbound:** `POST /events` for typed OCDS envelopes, `/v1/*` for clients already sending the common analytics HTTP shape, or `/v1/squid` for signed CRM record webhooks. All paths converge on the same `EventEnvelope` before touching Kafka.
- **Outbound:** destination connectors implement one trait (`DestinationConnector`). Webhook, HubSpot, and Klaviyo use it directly; PyReverseETL handles bulk sync separately because it is a table-to-destination workflow, not a per-profile API call.
- **Shared delivery:** retries, backoff, and rate limiting live outside individual adapters, so new connectors inherit the same behavior automatically.
- **Extension path:** a new source only has to translate its payload into `EventEnvelope`; a new destination only has to build one request and report one result.

## Migration from MessageBirds

The project was renamed from MessageBirds to PyCustomerHub. Existing
deployments can migrate incrementally:

- **Rust crates/imports:** `pch-*` crates are the new names; rebuild against
  `pch_*` imports.
- **Environment variables:** use `PYCUSTOMERHUB_*`. The old `MESSAGEBIRDS_*`
  names (and `MB_IDENTITY_HASH_PEPPER`) are still accepted as deprecated
  fallbacks.
- **SSO groups:** use `pycustomerhub-admin` / `pycustomerhub-writer`. The old
  `messagebirds-admin` / `messagebirds-writer` groups are still accepted.
- **Kafka consumer group:** the worker default is now
  `pycustomerhub-worker`. During a rolling migration, set
  `KAFKA_GROUP_ID=messagebirds-worker` to keep the existing committed offsets.
- **Postgres:** local compose defaults are now
  `pycustomerhub`/`pycustomerhub`/`pycustomerhub`. Existing volumes and
  deployment credentials are unchanged until you update them.
- **SDKs:** npm packages are scoped `@pycustomerhub/*`; Python code imports
  `pycustomerhub` (`PyCustomerHubClient`). A deprecated `messagebirds` Python
  import shim and old web/iOS/Android storage-key migrations are included.
- **URLs:** use `https://github.com/Mullassery/PyCustomerHub`. GitHub redirects
  old repository links, but update local remotes.

## Event-triggered journeys

Active journeys can now start from an ingested event type, not only audience
entry:

```json
{ "trigger": { "type": "event", "event_type": "order.completed" } }
```

The worker matches `Trigger::Event` against the event's `event_type` after
persisting the event and syncing audience membership, then starts a durable run
for the resolved profile. Audience-entry triggers continue to work unchanged.

## Quick start

```bash
docker compose up -d postgres redpanda

# Terminal 1:
DATABASE_URL=postgres://pycustomerhub:pycustomerhub@localhost:5432/pycustomerhub cargo run -p pch-api
# Terminal 2:
DATABASE_URL=postgres://pycustomerhub:pycustomerhub@localhost:5432/pycustomerhub cargo run -p pch-worker

# Send a real event from Node:
npm install
npm run smoke-test --workspace=sdk/js
```

Then open the viewer at `http://localhost:3000`, look up the profile the smoke test created, and inspect its identity graph, field provenance, and event timeline.

## Usage

The core loop, from the SDK through activation:

```
POST /events or /v1/* → normalize → schema validation → mixin composition → identity resolution
                                                                                     ↓
                                                              merge policy → profile projection
                                                                                     ↓
                                                          real-time audience evaluation
                                                                                     ↓
                            governed activation (label + consent + capability) → retry/rate-limit → destination
                                                                                     ↓
                                 audience-triggered journeys (durable, Postgres-backed) → templated sends
```

- **Ingestion:** `POST /events` accepts event envelopes (schema-validated; failures go to `dead_letter_events`). `/v1/{track,identify,page,screen,group,alias,batch}` accepts the common HTTP tracking shape using write-key authentication, and `/v1/squid` accepts HMAC-signed CRM record webhooks; all paths converge on the same canonical event.
- **Queries:** `GET /profiles/{id}`, `GET /profiles/by-identity`, `GET /profiles/{id}/events`, `GET /profiles/{id}/identity`.
- **Manage:** `GET/POST /schemas`, `mixins`, `namespaces`, merge policies, audiences, templates, channels, journeys, and governed `POST /audiences/{id}/activate`.
- **SDKs:** `sdk/js` and `sdk/python` expose the send-event/read-profile loop plus typed response shapes; `sdk/web` is the browser collection SDK.
- The `ui/` Next.js viewer uses Server Components/Actions only — no client-side calls to `pch-api`, so no CORS layer is needed.

## Destination kinds

`POST /destinations` accepts `kind` = `webhook`, `hubspot`, `klaviyo`, or
`pyreverseetl` (bulk). HubSpot and Klaviyo require an email on the profile and
fail closed when it is missing.

```json
{"kind": "hubspot", "config": {"access_token": "pat-..."}}
{"kind": "klaviyo", "config": {"api_key": "pk_...", "revision": "2024-10-15"}}
```

`base_url` is optional for both (defaults: `https://api.hubapi.com`,
`https://a.klaviyo.com`). All outbound URLs go through the same SSRF guard as
webhooks.

Delivery policy is configured per destination and applies to every connector:

```json
{
  "retry": {
    "max_attempts": 3,
    "initial_backoff_ms": 250,
    "max_backoff_ms": 5000
  },
  "rate_limit_per_second": 10
}
```

Transient transport failures, timeouts, HTTP 408/425/429, and 5xx responses
retry with exponential backoff; validation/config/auth failures fail
immediately. `rate_limit_per_second` paces sends within an activation run.

## Roadmap

No dates or guarantees — this is the integration-first order of work, not a promise list.

**Near term (integration leverage):**
- **Verified connector set:** add 5–10 production-tested destinations across CRM, lifecycle messaging, ad conversions, and warehouse/data-lake export, with sandbox tests rather than config-only adapters.
- **Durable activation queue:** replace request-scoped sends with an outbox/worker model that survives restarts, supports scheduling, idempotency keys, and cross-process rate limits.
- **Batch-aware first-party SDKs:** use `/v1/batch` for queued flushes instead of one request per event.
- **Connector test harness:** recorded request/response fixtures and contract tests so new adapters are verified consistently.
- **Production deployment:** Helm/Kubernetes manifests, readiness/liveness separation, Prometheus/OpenTelemetry metrics, and alerting.
- **Webhook hardening:** signing secrets, rotation, and replay protection for outbound webhook destinations.

**Next (platform depth):**
- **User-defined transformations:** versioned, previewable transforms for field mapping, filtering, enrichment, and PII masking before delivery.
- **Broader client coverage:** official server SDKs and additional cross-platform mobile SDKs, plus HTTP-source presets for common client libraries.
- **Warehouse integrations:** first-class warehouse export, scheduled syncs, and concurrent bulk activation beyond the current single-staging-table path.
- **Live pipeline visibility:** event tracing from ingestion through delivery, with per-destination failure explanations.
- **Better identity operations:** review queues for merges, re-hash migration tooling, and fuller field-level lineage.

**Later (CDP depth):**
- **Warehouse-native profiles:** identity graphs, customer 360 tables, and incremental feature computation in the warehouse.
- **Probabilistic identity:** opt-in, reviewable fuzzy matching with explicit confidence thresholds.
- **Sequence audiences and experimentation:** ordered behavioral conditions, holdouts, and statistical reporting.
- **Governance expansion:** tenant-configurable consent purposes, schema-label enforcement, policy versioning, and exceptions/inheritance.
- **Enterprise operations:** full RBAC, SSO integration boundaries, tenant isolation hardening, CLI, and richer admin UI.
- **Decisioning and AI:** only after the connector and delivery layers are stable.

## Documentation

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — reference architecture, phase-by-phase, what's real vs. deferred
- [docs/OCDS.md](docs/OCDS.md) — the canonical data model
- [docs/architecture/README.md](docs/architecture/README.md) — data-flow and journey-durability diagrams
- [ROADMAP_HONEST.md](ROADMAP_HONEST.md) — status buckets, technical debt, security findings
- [SECURITY.md](SECURITY.md) — current security posture and how to report a vulnerability
- [CONTRIBUTING.md](CONTRIBUTING.md) — dev setup, test commands, PR expectations

## Known issues and deferred work

**Blocked / unverified:**
- Live HubSpot/Klaviyo sandbox verification (no vendor credentials in this environment).
- Live Squid end-to-end delivery (no running Squid/Postgres stack in this environment); envelope mapping and HMAC verification are unit-tested.
- Android SDK build verification (no Android toolchain).

**Deliberately deferred:**
- Broader connector coverage, durable activation workers, user-defined transformations, production deployment tooling, and warehouse-native profiles. See [Roadmap](#roadmap).
- Sequence audiences, experimentation, and AI decisioning until the integration and delivery layers are stronger.
- Full RBAC/SSO productization and multi-tenant isolation hardening.

Full shipped/blocked/rejected detail: [`QUEUE_STATUS.md`](QUEUE_STATUS.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Bug/security reports: see [SECURITY.md](SECURITY.md) — please don't file security issues as public GitHub issues. Behavior changes are tracked in [CHANGELOG.md](CHANGELOG.md).

## Related projects

- [PyReverseETL](https://github.com/Mullassery/PyReverseETL) — quality-validated data activation with audit trail, lineage, and compliance records; PyCustomerHub's bulk-sync destination
- [ClusterAudienceKit](https://github.com/Mullassery/ClusterAudienceKit) — RFM analysis, clustering, CLV, churn detection, and lookalikes; complements PyCustomerHub's rule-based audiences
- [PrismNote](https://github.com/Mullassery/prismnote) — Jupyter-compatible data-science notebook with SQL execution across 9+ database backends
- [MudFish](https://github.com/Mullassery/MudFish) — Rust-native web crawling and intelligence engine for collecting raw web data
- [PyAPICheck](https://github.com/Mullassery/PyAPICheck) — transparent API security discovery with a Rust core and Python CLI/SDK

[Browse all projects →](https://github.com/Mullassery?tab=repositories)

## License

Apache-2.0. See [LICENSE](LICENSE).