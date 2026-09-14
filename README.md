# MessageBirds

An open-source, event-driven Customer Data & Engagement Platform. See [`docs/OCDS.md`](docs/OCDS.md) for the canonical data model and [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the reference architecture and phased roadmap.

## Status

**Phase 1 (foundation)**, in progress. The current milestone proves the canonical data path end-to-end:

```
POST /events → schema validation → mixin composition → identity resolution → merge policy → profile projection → GET /profiles/{id}
```

Nothing beyond this — no audiences, governance, consent, activation, journeys, decisioning, AI agents, CLI, or UI yet. See `docs/ARCHITECTURE.md` for what's next.

## Architecture (this milestone)

- `crates/core` — canonical domain types (event envelope, identity refs, tenant/workspace/environment ids)
- `crates/namespaces` — identity namespace registry
- `crates/schema-registry` — event schema registry + validation
- `crates/mixins` — composable profile mixins (standard library + custom)
- `crates/identity` — identity graph, deterministic matching, audit trail
- `crates/merge-policy` — conflict resolution strategies for profile projection
- `crates/profile` — unified customer profile projection + field-level provenance
- `crates/events` — Kafka/Redpanda producer/consumer wrappers
- `crates/api` — Axum HTTP server (ingestion + query)
- `crates/worker` — Kafka consumer pipeline (validate → resolve → merge → project)
- `sdk/js` — TypeScript client SDK

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
cd sdk/js && npm install && npm run smoke-test
```

## License

Apache-2.0. See [`LICENSE`](LICENSE).
