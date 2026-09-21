# Architecture diagrams

This is a diagram-only companion to `docs/ARCHITECTURE.md`, which is the canonical, detailed,
phase-by-phase description of what's built vs. deferred and why. Read that file for the real
narrative — including caveats, verified end-to-end scenarios, and explicit "not built" sections.
These diagrams exist to make the shape of the system scannable in ten seconds; they intentionally
omit the nuance the prose version is careful about, so don't treat a box on a diagram as proof a
feature is production-ready.

## Data flow: collect → standardize → resolve → project → activate

```mermaid
flowchart TD
    subgraph Collect
        WebSDK["sdk/web<br/>(browser, queued+retry)"]
        IosSDK["sdk/ios<br/>(Swift, queued+retry)"]
        AndroidSDK["sdk/android<br/>(written, unverified)"]
        ServerCaller["Server-side callers<br/>(sdk/js, sdk/python)"]
    end

    Gateway["edge/ingest-gateway<br/>(Cloudflare Worker)<br/>CORS + shape check<br/>NOT deployed, no auth"]

    subgraph mb-api ["mb-api (Axum, no auth layer)"]
        PostEvents["POST /events"]
        QueryEP["GET /profiles, /events, /schemas, ..."]
    end

    Kafka[("Kafka / Redpanda<br/>events.raw")]

    subgraph mb-worker ["mb-worker pipeline"]
        SchemaVal["schema-registry:<br/>validate against registered schema"]
        Identity["identity:<br/>resolve claims, mint/merge profile id<br/>(SHA-256 hashed claims, unsalted)"]
        MergePolicy["merge-policy:<br/>conflict resolution"]
        MixinVal["mixins:<br/>validate profile_updates"]
        ProfileProj["profile:<br/>apply fields, write provenance"]
        AudienceEval["audiences:<br/>real-time membership evaluation"]
        JourneyTrigger["journeys:<br/>start AudienceEntered-triggered runs"]
    end

    Postgres[("Postgres<br/>events, profiles, identity_nodes,<br/>identity_audit, audiences,<br/>consent_events, policies,<br/>journey_runs, messages_sent, ...")]

    subgraph Activate
        Governance["governance:<br/>label policy + consent check<br/>per profile"]
        Webhook["connectors: webhook<br/>(one real transport)"]
        ReverseETL["PyReverseETL bulk sync<br/>(consent checked, label policy NOT re-run)"]
        JourneyWorker["mb-journeys-worker<br/>poller: advances due runs,<br/>survives restart mid-Wait"]
        Channel["channels: webhook<br/>(only real transport)"]
    end

    UI["ui/ (Next.js)<br/>Server Components only<br/>no browser calls to mb-api"]

    WebSDK -->|POST /events| Gateway
    IosSDK -->|POST /events| Gateway
    AndroidSDK -.->|not yet compiled| Gateway
    Gateway --> PostEvents
    ServerCaller -->|POST /events directly| PostEvents

    PostEvents --> Kafka
    Kafka --> SchemaVal --> Identity --> MergePolicy --> MixinVal --> ProfileProj
    ProfileProj --> Postgres
    ProfileProj --> AudienceEval --> Postgres
    AudienceEval --> JourneyTrigger --> Postgres

    Postgres --> Governance --> Webhook
    Postgres --> Governance --> ReverseETL
    Postgres --> JourneyWorker --> Channel
    Channel --> Postgres

    Postgres --> QueryEP --> UI

    classDef gap fill:#fff3cd,stroke:#b08c00,color:#3a2e00;
    classDef unverified fill:#fde2e2,stroke:#b02a2a,color:#3a0d0d;
    class Gateway gap;
    class AndroidSDK unverified;
    class ReverseETL gap;
```

**Key, since color alone shouldn't carry meaning:** boxes marked with a dashed line or explicit
label text ("NOT deployed", "unverified", "NOT re-run") are disclosed gaps, not implied failures —
cross-reference `docs/ARCHITECTURE.md`'s "What's explicitly NOT built yet" section and
`ROADMAP_HONEST.md`'s security findings for what each one means in practice. In particular: there
is no authentication anywhere on the `mb-api` box, and `tenant_id` isolation is enforced only by
application-level query filtering, not a database or gateway-level boundary — see `SECURITY.md`.

## Journey durability (Phase 5, verified by restart test)

```mermaid
sequenceDiagram
    participant W as mb-worker
    participant DB as Postgres (journey_runs)
    participant JW as mb-journeys-worker (poller)
    participant Ch as Webhook listener

    W->>DB: Audience membership Entered -> start run (Wait node, wake_at=T+5m)
    Note over JW: process killed mid-Wait (verified in Phase 5 testing)
    JW->>DB: poll for due runs (wake_at <= now)
    DB-->>JW: this run is due
    JW->>DB: advance to Action node (reservation insert, ON CONFLICT DO NOTHING)
    JW->>Ch: POST rendered template
    Ch-->>JW: 200 OK
    JW->>DB: record messages_sent, advance to Split/End
    Note over DB: event log shows waited/action_sent exactly once each,<br/>even across the restart
```

## What these diagrams do not show

- Auth/authz (there isn't any — see `SECURITY.md`)
- Multi-tenant isolation boundaries (none exist beyond a `tenant_id` column filter)
- The governance policy simulator (`POST /policy-simulate`), which reuses the same evaluation code
  as real activation but never touches `Postgres` state — see `docs/ARCHITECTURE.md` Phase 4
- Any of the "not built" systems (decisioning, AI agents, real SMS/email/push channels, a CLI) —
  they're absent from these diagrams because there is no code to diagram, not because they were
  cut for space
