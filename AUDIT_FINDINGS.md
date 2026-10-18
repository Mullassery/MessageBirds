# Audit findings — 2026-10-19

This file records the repository-specific findings from the portfolio audit
conducted on 2026-10-19. It is documentation only; no code or release artifacts
were changed in this pass unless explicitly noted under "Implementation status".

Sources: the repository's own README/source/workflows, GitHub metadata and
Actions runs, PyPI metadata, and official competitor documentation. Claims are
marked as verified, inferred, or open questions. "Absent" means not found in the
inspected repository/docs, not proof that it cannot exist.

## What it is

Self-hosted, event-driven Customer Data & Engagement Platform (Rust + TypeScript): ingestion, identity resolution, real-time audiences, governance, journeys, and SDKs.

## Verified strengths

- 16 Rust crates; identity resolution with field provenance and merge/split; real-time audiences; consent ledger and label deny-list policies; durable Postgres-backed journeys; web/iOS/Python SDKs.
- Honest README about scale/security gaps; integration-test-backed.
- Governance/provenance is a credible differentiator for regulated teams.

## Verified gaps and user-facing risks

- Destination breadth is still thin: webhook, HubSpot, and Klaviyo only; no Salesforce/Braze/ad-platform catalog, so most activation workflows remain blocked.
- Android SDK written but not build-verified; edge gateway not deployed; no production/scale evidence; 156 events/sec sequential reference measurement.
- No GitHub release; PyPI 0.1.0 sdist only; quick start needs Rust + Postgres + Redpanda rather than one compose command.
- 28 open PRs, mostly dependency updates.

## Direct competitors

- RudderStack (large event-stream/warehouse/reverse-ETL destination catalog; ELv2), Jitsu (MIT, warehouse-first), PostHog (analytics/replay/flags/experiments), Segment (commercial).

## Ranked next actions

- P1: ship 5–10 verified destinations (Braze, Klaviyo, HubSpot, Salesforce, ad-platform CAPI) with retries/rate limits/DLQ (L).
- P1: one-command `docker compose` with all services + seeded demo (S).
- P1: align GitHub release/tag with the PyPI version (S).
- P2: verify Android SDK and publish SDKs consistently (S).

## Do not build

- AI decisioning/agents; audience sequence conditions until the connector layer is real.

## Evidence gaps / validation needed

- No external deployment; no load test beyond the reference number; no independent security review; destination demand untested.

## Recommended disposition

Invest only if connector breadth is funded; otherwise position as a governance/reference implementation.

## Implementation status

Implemented the first destination-breadth step in this pass:

- Added `hubspot` and `klaviyo` destination kinds (webhook + PyReverseETL
  remain).
- HubSpot: contacts batch upsert by email; Klaviyo: profile import with
  configurable revision.
- Both require an email, fail closed when missing, and use the shared SSRF
  guard. No live vendor credentials were available; tests cover config/payload
  validation and profile-field extraction.
- API accepts the new kinds and dispatches per destination kind.
- `cargo test --workspace`: 64 passed, 0 failed; fmt/clippy clean.

Added per-tenant API keys (`MESSAGEBIRDS_API_KEYS` JSON map), plus the original
single-key `MESSAGEBIRDS_API_KEY`/`_TENANT` path. Each key's tenant is enforced
against the `tenant_id` query/body value. Roles/SSO remain open.
Also still open: Salesforce/Braze/ad-platform connectors and live vendor
verification.
