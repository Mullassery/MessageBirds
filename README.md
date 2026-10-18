# MessageBirds

An open-source, event-driven Customer Data & Engagement Platform (CDP) built in Rust and TypeScript.

[![CI](https://github.com/Mullassery/MessageBirds/actions/workflows/ci.yml/badge.svg)](https://github.com/Mullassery/MessageBirds/actions/workflows/ci.yml)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![messagebirds on PyPI](https://img.shields.io/pypi/v/messagebirds.svg?label=messagebirds)](https://pypi.org/project/messagebirds/)

## What it is

MessageBirds is a self-hosted CDP and engagement platform that implements the full collection-to-activation pipeline: `POST /events` → schema validation → mixin composition → identity resolution → merge policy → profile projection → real-time audience evaluation → governed activation (webhook, HubSpot, Klaviyo, and bulk sync) → durable journeys with templated sends. It runs on Postgres + Kafka (Redpanda) with no external accounts, ships client SDKs for web, iOS, Android, JS, and Python, and provides a Next.js viewer UI. The pipeline is real, wired, and integration-tested end to end — see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for precisely what is built vs. deferred at each layer.

## Features / Highlights

- **Identity resolution with provenance** — deterministic matching, explicit merge/split, confidence-scored merge suggestions, and an append-only audit trail; every profile field records which event set it, its source, and the conflict policy applied. Verified live on real event sequences (correct merges, no false merges).
- **Real-time audiences** — rule-based definitions with attribute, event-occurrence, and temporal conditions, evaluated synchronously against streaming events; append-only membership history.
- **Governance-first activation** — label-based deny-list policies (e.g. `Deny(PII, ADVERTISING)`), destination capability checks, an append-only consent ledger, and a policy simulator that reuses the real evaluation primitives.
- **Durable journeys** — a flat-node-graph engine (Wait/Condition/Action/Split/End) with Postgres-backed durable run state and a poller that resumes runs across a restart; templated webhook sends.
- **Parity-with-OSS speed** — live ingestion measured at ~156 events/sec (sequential, single connection) on the reference machine; zero external accounts to stand up.
- **Client SDKs** — `@messagebirds/web` (queued, retrying, `sendBeacon` flush), `sdk/ios` (Swift, build-verified), `sdk/js` and `sdk/python` (same core scope), `sdk/android` (written, not yet build-verified), plus a Cloudflare Workers edge ingestion gateway for CORS.
- **Destination connectors** — webhook, HubSpot contacts (batch upsert by email), and Klaviyo profile import, all behind the same governance/consent gate.
- **Bulk activation** — a fourth destination kind alongside webhook/HubSpot/Klaviyo via [PyReverseETL](https://github.com/Mullassery/PyReverseETL)'s real sync engine.

## Who it's for / Use cases

- Engineering and data teams building a **self-hosted, local-first CDP** who want real identity resolution, audiences, consent-aware activation, and journeys without a proprietary control plane.
- Teams evaluating a transparent, single-layer event pipeline (one Kafka topic, one worker) over a many-microservice vendor stack.
- Privacy/ops-minded builders who want label-based governance and full activation audit logs (`sent`/`failed`/`blocked`).

## What it is not

- **Not a large connector library.** Three real per-profile destination connectors ship today (webhook, HubSpot contacts, Klaviyo profiles) plus the bulk-sync path via PyReverseETL. There is no pre-built catalog of dozens of destinations.
- **Not a decisioning/AI stack.** Decisioning, AI agents, and a CLI are not implemented.
- **No sequence conditions in audiences** (e.g. `A then B then not C`) — that's a genuinely different, ordered evaluation model and is not built.
- **Not hardened at scale.** It has not run production at scale or been independently security-reviewed — RudderStack's years of production hardening are a real, acknowledged gap. You must set `MB_IDENTITY_HASH_PEPPER` before pointing it at real PII (see SECURITY.md).
- **Young.** Single-maintainer, phases 1–5 built; the Android SDK is written but not build-verified, and the edge ingest gateway is built/tested locally but **not deployed**.

## Project status

`v0.3.0` (Rust workspace) / `messagebirds` 0.3.0 on PyPI. Phases 1–5 of the reference architecture are built (core event platform → identity → CDP → governance → engagement), with client-side collection, the edge gateway, and PyReverseETL activation layered on top. Active and integration-test-backed; [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) is the source of truth for what's real vs. deferred at each layer, and [ROADMAP_HONEST.md](ROADMAP_HONEST.md) tracks technical debt and security findings with file:line specifics.

## Installation

Requires Rust (stable), Docker, and Node.js. Start the dev infrastructure and services:

```bash
# 1. Start dev infra (Postgres + Redpanda)
docker compose up -d postgres redpanda

# 2. Run the services (mb-api applies migrations and seeds the standard mixin
#    library on startup — no separate migrate step needed)
DATABASE_URL=postgres://messagebirds:messagebirds@localhost:5432/messagebirds cargo run -p api
DATABASE_URL=postgres://messagebirds:messagebirds@localhost:5432/messagebirds cargo run -p worker

# 3. Send a test event and watch a profile materialize
npm install                                  # installs sdk/js and ui workspaces
npm run smoke-test --workspace=sdk/js

# 4. Browse it
npm run dev --workspace=ui                   # http://localhost:3000
```

Set `MESSAGEBIRDS_API_KEY` to require `Authorization: Bearer <key>` on every
route except `/healthz`. This is a single-key gate, not tenant isolation; leave
it unset for local development.

Before pointing this at real PII, set `MB_IDENTITY_HASH_PEPPER` (a random per-deployment secret) on the `api`, `worker`, and `journeys-worker` processes — it is mixed into the identity-matching hash in `crates/identity/src/hash.rs`. Leaving it unset reproduces the old unsalted behavior; see `SECURITY.md`.

## Quick start

```bash
docker compose up -d postgres redpanda

# Terminal 1:
DATABASE_URL=postgres://messagebirds:messagebirds@localhost:5432/messagebirds cargo run -p api
# Terminal 2:
DATABASE_URL=postgres://messagebirds:messagebirds@localhost:5432/messagebirds cargo run -p worker

# Send a real event from Node:
npm install
npm run smoke-test --workspace=sdk/js
```

Then open the viewer at `http://localhost:3000`, look up the profile the smoke test created, and inspect its identity graph, field provenance, and event timeline.

## Usage

The core loop, from the SDK through activation:

```
POST /events → schema validation → mixin composition → identity resolution → merge policy → profile projection
                                                                                     ↓
                                                          real-time audience evaluation
                                                                                     ↓
                                    governed activation (label policy + consent + destination capability) → webhook
                                                                                     ↓
                                         audience-triggered journeys (durable, Postgres-backed) → templated sends
```

- **Ingestion:** `POST /events` accepts event envelopes (schema-validated; failures go to `dead_letter_events`).
- **Queries:** `GET /profiles/{id}`, `GET /profiles/by-identity`, `GET /profiles/{id}/events`, `GET /profiles/{id}/identity`.
- **Manage:** `GET/POST /schemas`, `mixins`, `namespaces`, merge policies, audiences, templates, channels, journeys, and governed `POST /audiences/{id}/activate`.
- **SDKs:** `sdk/js` and `sdk/python` expose the send-event/read-profile loop plus typed response shapes; `sdk/web` is the browser collection SDK.
- The `ui/` Next.js viewer uses Server Components/Actions only — no client-side calls to `mb-api`, so no CORS layer is needed.

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

## Documentation

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — reference architecture, phase-by-phase, what's real vs. deferred
- [docs/OCDS.md](docs/OCDS.md) — the canonical data model
- [docs/architecture/README.md](docs/architecture/README.md) — data-flow and journey-durability diagrams
- [ROADMAP_HONEST.md](ROADMAP_HONEST.md) — status buckets, technical debt, security findings
- [SECURITY.md](SECURITY.md) — current security posture and how to report a vulnerability
- [CONTRIBUTING.md](CONTRIBUTING.md) — dev setup, test commands, PR expectations

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Bug/security reports: see [SECURITY.md](SECURITY.md) — please don't file security issues as public GitHub issues. Behavior changes are tracked in [CHANGELOG.md](CHANGELOG.md).

## Related projects

- [PyReverseETL](https://github.com/Mullassery/PyReverseETL) — quality-validated data activation with audit trail, lineage, and compliance records; MessageBirds' bulk-sync destination
- [ClusterAudienceKit](https://github.com/Mullassery/ClusterAudienceKit) — RFM analysis, clustering, CLV, churn detection, and lookalikes; complements MessageBirds' rule-based audiences
- [PrismNote](https://github.com/Mullassery/prismnote) — Jupyter-compatible data-science notebook with SQL execution across 9+ database backends
- [MudFish](https://github.com/Mullassery/MudFish) — Rust-native web crawling and intelligence engine for collecting raw web data
- [PyAPICheck](https://github.com/Mullassery/PyAPICheck) — transparent API security discovery with a Rust core and Python CLI/SDK

[Browse all projects →](https://github.com/Mullassery?tab=repositories)

## License

Apache-2.0. See [LICENSE](LICENSE).