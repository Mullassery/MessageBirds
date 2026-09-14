# MessageBirds

An open-source, event-driven Customer Data & Engagement Platform. See [`docs/OCDS.md`](docs/OCDS.md) for the canonical data model and [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the reference architecture and phased roadmap.

## Status

**Phase 2 (identity)**, in progress, on top of the Phase 1 foundation:

```
POST /events → schema validation → mixin composition → identity resolution → merge policy → profile projection → GET /profiles/{id}
```

Phase 2 adds confidence-scored merge suggestions (never automatic), explicit merge/split with an append-only audit trail, and a profile timeline UI (`ui/`). Still nothing on audiences, governance, consent, activation, journeys, decisioning, AI agents, or a CLI. See `docs/ARCHITECTURE.md` for what's next.

## Architecture (this milestone)

- `crates/core` — canonical domain types (event envelope, identity refs, tenant/workspace/environment ids)
- `crates/namespaces` — identity namespace registry
- `crates/schema-registry` — event schema registry + validation
- `crates/mixins` — composable profile mixins (standard library + custom)
- `crates/identity` — identity graph: deterministic matching, explicit merge/split, append-only audit trail
- `crates/merge-policy` — conflict resolution strategies for profile projection
- `crates/profile` — unified customer profile projection, field-level provenance, merge-suggestion similarity scoring
- `crates/events` — Kafka/Redpanda producer/consumer wrappers
- `crates/api` — Axum HTTP server (ingestion + query)
- `crates/worker` — Kafka consumer pipeline (validate → resolve → merge → project)
- `sdk/js` — TypeScript client SDK, npm-workspace-linked
- `ui/` — Next.js profile timeline viewer (Server Components/Actions only — no client-side calls to `mb-api`, so no CORS needed)

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
