# Changelog

All notable changes to this project are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); this project has not yet cut a versioned
release (still `0.1.0` everywhere), so everything so far lives under `[Unreleased]`. No history is
backfilled here beyond what's independently verifiable from `git log` and `docs/ARCHITECTURE.md` —
see those for the detailed phase-by-phase narrative; this file is intentionally a summary, not a
duplicate.

## [Unreleased]

### Added
- Core event platform: schema registry, mixin composition, identity resolution, merge policies,
  profile projection (`crates/core`, `crates/namespaces`, `crates/schema-registry`,
  `crates/mixins`, `crates/identity`, `crates/merge-policy`, `crates/profile`, `crates/events`).
- Confidence-scored merge suggestions, explicit profile merge/split, profile timeline UI.
- Audiences (rule-based, real-time streaming membership), one webhook destination connector, a
  data quality dashboard, field lineage view.
- Governance: field labels, a deny-list policy engine, an append-only consent ledger, governed
  per-profile activation, a policy simulator.
- Engagement: versioned message templates, a real webhook channel transport, Postgres-backed
  durable journey orchestration (Wait/Condition/Action/Split/End nodes), contact-policy
  suppression.
- Client SDKs: `sdk/js` (TypeScript), `sdk/web` (browser, queued/retrying), `sdk/python`
  (published on PyPI as `messagebirds`), `sdk/ios` (Swift package, `swift build` +
  `swift run mb-verify` verified), `sdk/android` (Kotlin, written but not build-verified — no
  Android toolchain available in any session so far).
- `edge/ingest-gateway`: a Cloudflare Worker terminating CORS in front of `mb-api`, built and
  tested locally, never deployed.
- Bulk audience activation via [PyReverseETL](https://github.com/Mullassery/PyReverseETL) as a
  second destination kind alongside webhook.
- CI (`.github/workflows/ci.yml`): Rust (fmt/clippy/test), JS (sdk/js, sdk/web, edge/ingest-gateway,
  ui), Python (pytest), iOS (swift build/test on macos-15).
- `SECURITY.md`, `CONTRIBUTING.md`, `ROADMAP_HONEST.md`, `docs/architecture/README.md` (diagrams),
  `.github/dependabot.yml`, issue/PR templates — this documentation pass.

### Known issues (see `ROADMAP_HONEST.md` and `SECURITY.md` for full detail)
- No authentication or tenant isolation in `mb-api`.
- `sqlx 0.7.4` has a known, fixed CVE (RUSTSEC-2024-0363); not yet upgraded to `>=0.8.1`.
- Identity-matching hashes (`crates/identity/src/hash.rs`) are unsalted SHA-256 over low-entropy
  PII — crackable by a determined reader with database access.
- `sdk/android` has never been compiled or tested.
- `edge/ingest-gateway` has no write-key/API-key auth and has never been deployed.
