# MessageBirds

An open-source, event-driven Customer Data & Engagement Platform. See [`docs/OCDS.md`](docs/OCDS.md) for the canonical data model and [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the reference architecture and phased roadmap.

## Status

**Phase 4 (governance)**, in progress, on top of Phases 1–3:

```
POST /events → schema validation → mixin composition → identity resolution → merge policy → profile projection
                                                                                     ↓
                                                            real-time audience evaluation
                                                                                     ↓
                                          governed activation (label policy + consent + destination capability) → webhook
```

Phase 4 adds governance labels on mixin fields, a deny-list policy engine, an append-only consent ledger, per-profile governed activation (every send checked and explained, not silently allowed or blocked), and a policy simulator whose answer is guaranteed to match real activation since it reuses the same evaluation code. Still nothing on journeys, decisioning, AI agents, a CLI, or connectors beyond webhook. See `docs/ARCHITECTURE.md` for the full picture of what's built vs. deferred and why.

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
- `crates/connectors` — destination plugin shape (`DestinationConnector`) + one webhook implementation, activation log with `sent`/`failed`/`blocked` status
- `crates/governance` — label-based deny-list policy engine, append-only consent ledger, structured denial reasons
- `crates/api` — Axum HTTP server (ingestion + query)
- `crates/worker` — Kafka consumer pipeline (validate → resolve → merge → project → evaluate audiences)
- `sdk/js` — TypeScript client SDK, npm-workspace-linked
- `ui/` — Next.js viewer: profile timeline (+ consent section), audiences (create/members/activate), data quality dashboard, field lineage, governance (policies), policy simulator (Server Components/Actions only — no client-side calls to `mb-api`, so no CORS needed)

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
