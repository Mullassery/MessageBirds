# MessageBirds

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![CI](https://github.com/Mullassery/MessageBirds/actions/workflows/ci.yml/badge.svg)](https://github.com/Mullassery/MessageBirds/actions/workflows/ci.yml)
[![messagebirds on PyPI](https://img.shields.io/pypi/v/messagebirds.svg?label=messagebirds)](https://pypi.org/project/messagebirds/)

An open-source, event-driven Customer Data & Engagement Platform. See [`docs/OCDS.md`](docs/OCDS.md) for the canonical data model and [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the reference architecture and phased roadmap.

## Status

Phases 1–5 (core event platform → identity → CDP → governance → engagement) are built; since Phase 5, client-side collection (`sdk/web`/`sdk/ios`/`sdk/android`), an edge ingestion gateway, and bulk activation through PyReverseETL have been layered on top (see below). `docs/ARCHITECTURE.md` is the source of truth for what's real vs. deferred at each layer — this file is a summary, not a substitute for it.

Pipeline, as of Phase 5:

```
POST /events → schema validation → mixin composition → identity resolution → merge policy → profile projection
                                                                                     ↓
                                                            real-time audience evaluation
                                                                                     ↓
                                          governed activation (label policy + consent + destination capability) → webhook
                                                                                     ↓
                                    audience-triggered journeys (Wait/Condition/Action/Split/End,
                                    Postgres-backed durable state machine) → templated webhook sends
```

Phase 5 adds message templates, a webhook channel adapter, and journeys: a flat-node-graph workflow engine with Postgres-backed durable execution (a `journeys-worker` poller resumes runs across a restart, verified mid-`Wait`) instead of Temporal — the spec's own language ("Temporal or an equivalent") sanctions this, and it avoids a new infra dependency plus a less-mature Rust SDK this project can't fully de-risk yet. Still nothing on decisioning, AI agents, a CLI, or connectors beyond webhook. See `docs/ARCHITECTURE.md` for the full picture of what's built vs. deferred and why.

Since Phase 5: client-side event collection (`sdk/web`, `sdk/ios` verified; `sdk/android` written but not build-verified — no Android toolchain in this environment), an edge ingestion gateway that terminates CORS for browser SDKs hitting `mb-api` directly (built and verified locally, **not deployed**), and bulk audience activation through [PyReverseETL](https://github.com/Mullassery/PyReverseETL)'s real sync engine as a second destination kind alongside webhook. See `docs/ARCHITECTURE.md`'s "Collect & Activate infrastructure" section for what's real, what's disclosed-unverified, and two integration bugs discovered (and worked around) in PyReverseETL along the way.

## Architecture (this milestone)

- `crates/core` — canonical domain types (event envelope, identity refs, tenant/workspace/environment ids)
- `crates/namespaces` — identity namespace registry
- `crates/schema-registry` — event schema registry + validation, field-level governance labels
- `crates/mixins` — composable profile mixins (standard library + custom), standard library now carries real `PII`/`IDENTITY` labels
- `crates/identity` — identity graph: deterministic matching, explicit merge/split, append-only audit trail
- `crates/merge-policy` — conflict resolution strategies for profile projection
- `crates/profile` — unified customer profile projection, field-level provenance, merge-suggestion similarity scoring
- `crates/events` — Kafka/Redpanda producer/consumer wrappers
- `crates/audiences` — rule-based audience definitions, real-time streaming membership evaluation
- `crates/connectors` — destination plugin shape (`DestinationConnector`) + one webhook implementation, activation log with `sent`/`failed`/`blocked` status; a second bulk-sync destination kind (`pyreverseetl`), activated through a separate endpoint since it doesn't fit the per-record connector shape
- `crates/governance` — label-based deny-list policy engine, append-only consent ledger, structured denial reasons
- `crates/channels` — channel registry + `ChannelAdapter` trait, one real transport (webhook)
- `crates/templates` — versioned message templates, hand-rolled `{{mixin_key.field}}` rendering
- `crates/journeys` — flat-node-graph journey engine (Wait/Condition/Action/Split/End), Postgres-backed durable run state, contact-policy suppression
- `crates/api` — Axum HTTP server (ingestion + query)
- `crates/worker` — Kafka consumer pipeline (validate → resolve → merge → project → evaluate audiences → start triggered journeys)
- `crates/journeys-worker` — poller binary that advances due journey runs (the durable-execution driver)
- `sdk/js` — TypeScript client SDK, npm-workspace-linked
- `sdk/python` — Python client SDK ([`messagebirds` on PyPI](https://pypi.org/project/messagebirds/)), same scope as `sdk/js`: the core send-event/read-profile loop, plus typed dict mirrors of every response shape for building your own requests
- `sdk/web` — browser event-collection SDK (`@messagebirds/web`): a queued, retrying `POST /events` client with `localStorage` persistence and a `sendBeacon` unload flush
- `sdk/ios` — Swift package (`MessageBirdsAnalytics`) with the same queue/retry design; `swift build` and a plain-executable check (`swift run mb-verify`) pass — `swift test` can't run without full Xcode
- `sdk/android` — Kotlin module with the same design, written but not yet build-verified (no Android toolchain available where it was written — see `sdk/android/README.md`)
- `edge/ingest-gateway` — Cloudflare Worker: CORS + envelope-shape validation in front of `mb-api`'s `POST /events`, for browser SDKs that can't call `mb-api` directly. Built and verified via `wrangler dev`, **not deployed**
- `ui/` — Next.js viewer: profile timeline (+ consent, journeys sections), audiences (create/members/activate), data quality dashboard, field lineage, governance (policies), policy simulator, channels, templates, journeys (Server Components/Actions only — no client-side calls to `mb-api`, so no CORS needed)

## Development

Requires: Rust (stable), Docker, Node.js.

```bash
# 1. Start dev infra (Postgres + Redpanda)
docker compose up -d postgres redpanda

# 2. Run the services (the api binary applies migrations and seeds the
#    standard mixin library on startup — no separate migrate step needed)
DATABASE_URL=postgres://messagebirds:messagebirds@localhost:5432/messagebirds cargo run -p api
DATABASE_URL=postgres://messagebirds:messagebirds@localhost:5432/messagebirds cargo run -p worker

# 3. Send a test event and watch the profile materialize
npm install   # installs sdk/js and ui workspaces from the repo root
npm run smoke-test --workspace=sdk/js

# 4. Browse it
npm run dev --workspace=ui   # http://localhost:3000
```

## License

Apache-2.0. See [`LICENSE`](LICENSE).
