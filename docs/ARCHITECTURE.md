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

## What's built (Phase 2: identity)

Confidence-scored merge suggestions, explicit merge/split, and a profile viewer, layered on the Phase 1 foundation without changing it:

- **Merge suggestions** (`GET /profiles/{id}/merge-suggestions`) — never automatic. Candidates are anchored on a shared `core/device@1.0.device_id` (the only non-hashed, comparable signal available — see `docs/OCDS.md`), scored by `mb_profile::similarity::score` (shared device + Jaro-Winkler name similarity). A profile with no device id on record gets no suggestions rather than noisy ones.
- **Explicit merge** (`POST /profiles/{id}/merge`) and **split** (`POST /profiles/{id}/split`) on `mb-identity`'s `IdentityRepo`, both writing to the append-only `identity_audit` trail (now `linked | merged | split`). Split only unlinks one identity claim going forward — it does **not** migrate `profiles.mixins`/`profile_field_provenance`, which are keyed by profile id, not by claim, and can't be reliably split apart once merged. The API response says this explicitly.
- **`GET /profiles/{id}/identity`** — the identity graph (hashed) plus the audit trail for a profile, from either side of a merge/split.
- **`ui/`** — a Next.js (App Router) profile timeline viewer: look up a profile by an identity claim, view composed mixins/provenance/identity graph/audit trail/event timeline, act on merge suggestions, and split off a claim (using the plaintext value visible in that profile's event timeline — the identity graph itself only ever shows hashes). All data fetching is server-side (Server Components + Server Actions); nothing calls `mb-api` from the browser, so `mb-api` needs no CORS layer.

## What's built (Phase 3: CDP)

Audiences, one destination connector, a data quality dashboard, and field lineage — layered on Phases 1–2 without changing them:

- **Audiences** (`mb-audiences`) — `Condition` trees of attribute comparisons (on mixin fields) and event-occurrence checks (`event X ≥N times in the last D days`), composed with AND/OR/NOT. Covers attributes, events, behavior (count thresholds), and temporal (`NOT(event ... within N days)`) from Section 20. **No sequence conditions** (`A then B then not C`) — that's ordered, time-aware matching, a genuinely different evaluation model, not implemented.
- **Streaming membership** — evaluated synchronously in the worker (`crates/worker/src/pipeline.rs`) right after a profile's mixins are updated, for that tenant's active audiences only. Synced to `audience_memberships` (current state) + `audience_membership_events` (append-only history). **Not** republished as synthetic `customer.entered_audience` Kafka events (Section 21) — that needs its own schema registration and loop-safety work and is deferred, not faked. A consequence: an audience only re-evaluates a profile when *that profile* receives another event — a condition that becomes false for reasons unrelated to the profile's own activity (e.g. a pure time-window lapsing with no new event) won't exit it until the next event arrives.
- **One connector** (`mb-connectors`) — a `DestinationConnector` trait (Section 18's plugin shape) with one implementation, `webhook`. `POST /audiences/{id}/activate` runs synchronously over an audience's current members, sends each via the destination's connector, and writes one `activation_log` row per attempt (Section 60's audit requirement) — without the governance/consent checks that would normally precede it, since those aren't built yet.
- **Data quality dashboard** (`ui/app/data-quality`) — surfaces `dead_letter_events`, which has existed and been populated since Phase 1 but was never visible anywhere until now.
- **Field lineage** (`ui/app/profiles/[id]/lineage`) — traces a profile field's *current winning value* back through `profile_field_provenance` → the source event (new `GET /events/{id}`) → its schema → the mixin definition (new `GET /mixins/{namespace}/{name}/{version}`) → the merge policy applied. Built entirely from data already captured — **not** a ledger of every value ever observed for that field (a bigger, different feature, still deferred).

Two latent bugs fixed in the course of this phase, both pre-existing and only now exercised by a read path that renders them: `SchemaStatus`/`MixinStatus` had `sqlx` casing (`"active"`) but no matching `serde` casing, so their JSON was `"Active"` until the lineage view was about to display it; `Condition`'s natural `#[serde(tag = "type")]` representation doesn't compile for a recursive enum (`Not(Box<Condition>)`) — serde's tagged-enum `Content` buffering blows the compiler's type-instantiation limit — so `Condition` is externally tagged (`{"attribute": {...}}`, not `{"type": "attribute", ...}`) instead.

## What's built (Phase 4: governance)

Labels, marketing actions, a deny-list policy engine, an append-only consent ledger, and a policy simulator — layered on Phases 1–3 without changing their event/profile/audience behavior:

- **Governance labels** (`mb-schema-registry::FieldDef.labels`) — free-form strings (`"PII"`, `"HEALTH"`, ...), not a closed enum, matching the extensibility already established for mixin namespaces. The standard vocabulary (Section 13) is documented, not enforced. Only **mixin field** labels are wired into evaluation this phase; schema field labels are captured but not yet acted on. The standard mixins now carry real labels (`contact@1.0.email`/`.phone` → `PII, DIRECT_IDENTIFIER`, `person@1.0`'s name/birth-date fields → `PII`, `identity@1.0.customer_id`/`.crm_id` → `IDENTITY`).
- **Marketing actions & destination capability** — `Destination` (`mb-connectors`) gained `supported_actions` (Section 14); activating for an action a destination doesn't declare is a `DestinationCapability` denial.
- **Policy engine** (`mb-governance`) — `Policy(label, action, effect, priority)`, default **allow**, an explicit `Deny` blocks (Section 15). No exceptions/inheritance/versioning. `evaluate_labels` returns every matching `Deny` as a structured `Reason`, not a bare "policy violation" (Section 16).
- **Consent** — `consent_events` is append-only (revoking never erases the earlier grant); current state per purpose is the latest non-expired event. A fixed mapping (`consent_purpose_for_action`) decides which of the Section 14 actions need which Section 17 purpose; `ANALYTICS`/`DATA_ENRICHMENT`/`AI_PROCESSING` don't gate on consent.
- **Governed activation** — `POST /audiences/{id}/activate` now takes a `marketing_action` and checks destination capability, label policy (against the mixin fields *actually populated* on that specific profile — `profile_field_labels`), and consent **per member**, skipping and explaining (not silently sending or silently dropping) anything that fails. `activation_log.status` gained `blocked` (never attempted) alongside `sent`/`failed` (attempted).
- **Policy simulator** (`POST /policy-simulate`, `ui/app/policy-simulator`) — reuses the exact same evaluation primitives as real activation, so its answer is guaranteed consistent with what activation would actually do. Label/policy/capability is checked against every field populated across the audience's *current members* (a data-shape question); consent is reported as a `granted`/`missing`/`total` count, since it's genuinely per-profile and a single yes/no would misrepresent that.

Verified end to end: a `Deny(PII, ADVERTISING)` policy blocked a profile carrying `email`/`first_name` from an `ADVERTISING` activation with an explicit reason, while the same audience activated fine for `ANALYTICS` (no matching policy); granting `advertising` consent removed the `ConsentMissing` reason while the `PII` policy still blocked independently; the simulator's `blocked_fields`/`consent_summary` matched the real activation outcome exactly; consent history survived a revoke (both events present, current state correctly showed the latest).

## What's built (Phase 5: engagement)

Message templates, one real channel transport, and durable journey orchestration — layered on Phases 1–4 without changing their event/profile/audience/governance behavior:

- **Durable execution = a Postgres-backed state machine, not Temporal.** The spec's own language ("Temporal or an equivalent") sanctions this. `journey_runs.(current_node, status, wake_at)` is persisted after every node transition; a dedicated poller binary (`mb-journeys-worker`, parallel to how `api`/`worker` are already split by concern) advances every run whose `wake_at` has passed. This is real, verified durability — killing and restarting `journeys-worker` mid-`Wait` resumes the run from persisted state rather than restarting or double-firing the eventual send (verified: the event log shows `waited`/`action_sent` exactly once each across the restart). It is **not** distributed fault-tolerance across many worker replicas — a real gap, not hidden.
- **Journey graphs are a flat node list** (`Node { id, kind }`, referenced by string id), not a nested tree — by construction, this sidesteps the exact serde `Content`-buffering compile failure `Condition` hit in Phase 3 (recursive internally-tagged enums don't compile), and matches how real workflow engines represent graphs anyway.
- **`Condition` nodes reuse `mb_audiences::evaluate_condition` directly** — a journey branches on the same attribute/event logic an audience can be built from, guaranteed identical semantics rather than a second copy. (Promoted from a private method on the audience repo to a public function this phase.)
- **One real channel transport: webhook** (`mb-channels`), same honest pattern as Phase 3's one destination connector — genuinely POSTs the rendered message and is verified against a real local listener, not a fake "looks like it sends" shim. Other `kind`s can be registered but sending against them fails rather than silently succeeding.
- **Templates** (`mb-templates`) — hand-rolled `{{mixin_key.field}}` substitution against a profile's composed mixins (no regex dependency), versioned per `(tenant, name)`; a journey's Action node pins to a specific template *id*, so past runs keep using the version they were authored against even after a newer version is registered.
- **Split nodes are the only experimentation primitive** — deterministic weighted branching (FNV-1a hash of `profile_id + node_id`, not `std`'s unspecified-seed `DefaultHasher`, so the same profile always lands in the same branch across restarts). No statistical-significance tracking, bandits, or holdout reporting.
- **No visual canvas** — journeys are authored as JSON (a flat node list), matching the spec's own principle that the canvas must not be the source of truth. The UI shows/edits the JSON and renders a read-only node list.
- **Contact policy** (`mb_journeys::ContactPolicy`) — `(tenant, max_messages, window_days, channel_id?)`; a send that would exceed the trailing-window count is suppressed (`messages_sent.status = 'suppressed'`), not the whole run blocked. Verified: a second run's Action node was suppressed while the run still reached `End`.
- **Idempotent sends under retry** — `messages_sent` has `UNIQUE(run_id, node_id)`; the Action node does a reservation insert (`ON CONFLICT DO NOTHING RETURNING id`) before calling the channel adapter, so a crash between "sent" and "advanced" can't double-send on resume — the same pattern already proven for `persist_event`.
- **Trigger wiring** — `crates/worker/src/pipeline.rs` starts a run automatically for every `Entered` membership change matching a journey's `AudienceEntered` trigger, reusing data the pipeline already computes; `POST /journeys/{id}/start` is a legitimate manual-enrollment escape hatch, not just a test hook.

Verified end to end (`scripts`-style Python check against a live stack, not curl one-liners): registering a channel/template/audience-triggered journey (`Wait → Action → Split → End`), sending an event that entered the trigger audience, confirming the run started automatically and genuinely parked at `Wait` with a future `wake_at`, killing and restarting `journeys-worker` mid-wait and confirming the run resumed (not restarted, not double-fired), the webhook listener receiving the exact rendered body (`"Hi Jane!"` from `core/person@1.0.first_name`), and a tight contact policy suppressing a second run's send while that run still completed.

## What's built (Collect & Activate infrastructure: client SDKs, edge ingestion, PyReverseETL)

Not the product spec's numbered Phase 6 (Decisioning) — this deepens the COLLECT and ACT ends of the pipeline instead: client-side event collection (previously only reachable via server-side `POST /events`) and a second, bulk activation path alongside Phase 3's per-profile webhook connector.

- **Client SDKs** (`sdk/web`, `sdk/ios`, `sdk/android`) — all three share one design: a persisted queue (localStorage / a JSON file / `SharedPreferences`-adjacent file, per platform), one real `POST /events` call per queued event (there is no batch endpoint to batch into), exponential backoff, drop-and-log after 5 attempts. `sdk/web` and `sdk/ios` are genuinely verified this phase (vitest; `swift build` + a plain-executable check — see below). `sdk/android` is **written but not compiled or tested** — this environment has no `gradle`/`kotlinc`/Android SDK. See `sdk/android/README.md`.
  - `sdk/ios` verification note: `swift build` compiles the library for real. `swift test` (both XCTest and the swift-testing framework) **cannot run in this environment** — it has only the Xcode Command Line Tools, not full Xcode.app, and neither `XCTest.framework` nor the `Testing` module resolves (`xcrun --find xctest` fails; `import Testing` doesn't compile even with `swift build --build-tests`). The XCTest/swift-testing suite in `Tests/MessageBirdsAnalyticsTests` is written and intended for CI/a machine with Xcode; the `mb-verify` executable target (`swift run mb-verify`) exercises the same public-API scenarios with plain `assert`s and is what actually ran here.
- **Edge ingestion gateway** (`edge/ingest-gateway`, Cloudflare Worker) — exists because `sdk/web` is the first caller to hit `mb-api` directly from a browser origin, and `mb-api` deliberately has no CORS layer (every prior caller ran server-side). The gateway terminates CORS and rejects malformed envelope shapes before forwarding to `mb-api`'s real `POST /events`. No API-key/write-key auth and no rate limiting — there's no real write-key issuance anywhere in this codebase to check against yet, so a fake check would be worse than the disclosed gap. **Not deployed** (no `wrangler deploy` run, per instruction) — verified locally via `wrangler dev` (CORS preflight, malformed-body 400, and `sdk/web`'s smoke test routed all the way through into a real materialized profile).
- **PyReverseETL bulk activation** (`mb-connectors`'s new `pyreverseetl` destination kind, `POST /audiences/{id}/activate-via-reverse-etl`) — [PyReverseETL](https://github.com/Mullassery/PyReverseETL) is a real bulk source-table→destination sync tool (CLI: `create-workflow` + `create-activation` + `execute-activation`, backed by its own real Rust sync engine), not a per-record API, so it doesn't fit `DestinationConnector::send`'s per-profile shape and gets its own endpoint rather than a branch inside `activate_audience`. Consent is checked per profile before export (same as `activate_audience`) — a profile without the required consent purpose is excluded and logged `Blocked`. **Field-level label policy is not re-run on this path** — a disclosed gap, not a silent skip. Passing profiles' flattened standard-mixin fields (`core/contact@1.0.email/.phone`, `core/person@1.0.first_name/.last_name`) are staged in `pyreverseetl_export_staging` (truncate-and-refill per destination — this MVP supports one active `pyreverseetl` destination sync at a time, not concurrent ones sharing the table), then the API shells out to the real `pyreverseetl` CLI (`tokio::process::Command`) and parses its real JSON result.
  - **Discovered, not hypothetical**: `docker-compose.server.yml` in the PyReverseETL repo advertises a REST API server on :8080 and `examples/basic_activation.py` imports a `pyre` module — neither exists in the real code (no axum/actix anywhere in `PyReverseETL/core`; `import pyre` fails). The real integration surface is the CLI + the state file it persists to (`~/.pyreverseetl/state.json` by default), confirmed by reading `python/pyreverseetl/cli.py` directly.
  - **Also discovered, not hypothetical**: PyReverseETL's generic Postgres source connector (`core/src/connectors/postgres.rs::pg_row_to_json`) dispatches on Postgres type names and has no branch for native `uuid` columns — they fall through to a `try_get::<String, _>` that fails for the wire-level UUID type, silently decoding as `null`. Worked around with `pyreverseetl_export_staging_view` (migration `0017`), which casts every id column to `text` before PyReverseETL's workflow reads it — the real `pyreverseetl_export_staging` table keeps native `uuid` columns for FK integrity and index performance; only the view PyReverseETL points at casts them.
  - Verified end to end against PyReverseETL's real webhook destination (not a mock of PyReverseETL): two profiles matching an audience, only one with the required consent granted; the endpoint exported exactly the consenting profile, logged the other `Blocked` with a `consent_missing` reason, and the real PyReverseETL sync engine delivered the exact profile fields to a real local listener with `rows_synced: 1`.

## Deployment (this phase)

Minimal dev mode only: Postgres + Redpanda via `docker-compose.yml`, `api` and `worker` run directly with `cargo run`. The heavier "enterprise" deployment described in the product spec (Kafka cluster, Flink, Temporal, ClickHouse, object storage, Kubernetes) is not needed to prove the foundation and isn't set up yet.

## What's explicitly NOT built yet

This is deliberate, not an oversight — the product spec's own guidance is to get the foundation right before layering anything on top of it:

- **Sequence audience conditions** (`A THEN B THEN NOT C`) — attribute/event/AND/OR/NOT are done (Phase 3); ordered time-aware chains are a different evaluation model
- **Streaming membership as Kafka events** — membership changes land in Postgres only, not republished as `customer.entered_audience`/`customer.exited_audience` events
- **Connectors beyond one webhook destination** — no Salesforce, Braze, Shopify, GA4, Snowflake, BigQuery, ad platforms, or any source connector beyond the HTTP API
- **Schema field labels aren't enforced** — only mixin field labels feed policy evaluation; a schema's own `labels` are stored, not acted on
- **Policy engine has no exceptions, inheritance, or versioning** — a flat `(label, action, effect, priority)` deny-list only
- **Consent purpose mapping is fixed code**, not tenant-configurable, and there's no jurisdiction-aware consent logic beyond storing the field
- **Real SMS/email/push channel adapters** — only webhook has a real transport; other channel `kind`s can be registered but sending against them fails rather than silently succeeding
- **Statistical experimentation** — Split nodes do deterministic weighted branching only; no significance tracking, bandits, or holdout reporting
- **Visual journey canvas** — journeys are authored/viewed as a flat JSON node list
- **Distributed journey-worker fault tolerance** — the Postgres-backed state machine survives one worker's restart/crash; it is not yet safe to run many `journeys-worker` replicas concurrently against the same runs
- **`Event` journey triggers** — the `Trigger` enum has an `Event` variant, but only `AudienceEntered` is wired into the worker pipeline
- **`sdk/android` is unverified** — written to real Android/Kotlin APIs, but this environment has no `gradle`/`kotlinc`/Android SDK to compile or test it
- **Edge gateway has no auth or rate limiting, and isn't deployed** — no real write-key issuance exists to check against yet; `wrangler deploy` was never run
- **Reverse-ETL activation skips field-level label policy** and supports one active `pyreverseetl` destination sync at a time (the staging table is shared, truncated-and-refilled per run, not partitioned for concurrent destinations)
- **Decisioning** (rules, scoring, next-best-action)
- **AI agents, AI assistant, model provider abstraction**
- **CLI, admin UI beyond the profile/audience/data-quality/governance/simulator viewers**
- **Multi-tenancy enforcement (RBAC, SSO, tenant isolation)** — `tenant_id` is threaded through every table, but nothing yet stops one tenant from querying another's data; there's no auth layer at all, and `ui/` has no login
- **Automatic identity resolution stays exact-match only** — probabilistic signals (Phase 2) only ever produce *suggestions* a human confirms; nothing auto-merges based on a fuzzy match
- **Full lineage** — `profile_field_provenance` records only the *winning* source per field, not every observation; the Phase 3 lineage view traces that winner, not history
- **Data retention / delete-and-forget workflows**
- **Full standard mixin library** (`commerce`, `loyalty`, `subscription`, `consent`, `engagement`, ...) — only `identity`, `person`, `contact`, `device` are seeded

## Roadmap

Phases as described in the product spec, in order:

1. **Core event platform** — done
2. **Identity** — done: deterministic matching, confidence-scored merge suggestions, explicit merge/split, profile timeline UI. Not done: automatic probabilistic resolution (by design — see above), full lineage (every observation, not just the winner)
3. **CDP** — done: rule-based audiences with real-time streaming membership, a data quality dashboard, field lineage, one webhook destination connector. Not done: sequence conditions, membership-as-Kafka-events, any connector beyond webhook, batch/scheduled segmentation
4. **Governance** — done: labels, marketing actions, a deny-list policy engine, append-only consent, governed per-profile activation, a policy simulator matching real activation behavior. Not done: exceptions/inheritance/policy versioning, schema-field-label enforcement, tenant-configurable consent purposes
5. **Engagement** — done: message templates (versioned, journey-pinned), one real channel transport (webhook), journeys as a flat node graph (Wait/Condition/Action/Split/End), Postgres-backed durable orchestration (verified across a worker restart mid-wait), contact-policy suppression. Not done: real SMS/email/push adapters, statistical experimentation, visual canvas, multi-replica journey-worker fault tolerance, `Event` triggers
   - Also since Phase 5: client SDKs (`sdk/web`, `sdk/ios` verified; `sdk/android` written, unverified), an edge ingestion gateway (built, not deployed), and bulk activation through PyReverseETL — deepening COLLECT and ACT, not the spec's own numbered Phase 6 below
6. **Decisioning** — rules, scoring, next-best-action, frequency/contact policy
7. **AI** — AI assistant, tool-using agents, agent governance, model provider abstraction
8. **Enterprise** — multi-tenancy enforcement, RBAC/SSO, Terraform provider, GitOps, Kubernetes, enterprise connectors (Salesforce, Snowflake, BigQuery, ...)
