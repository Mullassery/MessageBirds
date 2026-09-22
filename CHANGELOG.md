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

### Security
- Bumped `sqlx` from `0.7.4` to `0.8.6` (`Cargo.toml:41`), fixing RUSTSEC-2024-0363 (binary
  protocol misinterpretation via truncating/overflowing casts). This was a clean, mechanical bump
  for this codebase — no `sqlx::query!`/`query_as!` compile-time macros are used anywhere (only
  runtime `sqlx::query`/`query_as` with `.bind()`), so none of `sqlx` 0.8's breaking API changes
  (builder/macro changes) touched any call site. `cargo build --workspace`, `cargo clippy
  --workspace --all-targets -- -D warnings`, and `cargo test --workspace` (55/55, up from 52
  because of new hash tests below) all pass unchanged.
- Added `.cargo/audit.toml` to suppress RUSTSEC-2023-0071 (`rsa`, Marvin Attack timing
  side-channel) from `cargo audit` output, with the investigation documented inline: `rsa` is an
  unreachable transitive dependency of `sqlx-mysql`, which is itself gated behind `sqlx`'s
  optional `mysql` feature that this workspace never enables (only `postgres`) —
  `cargo tree -i rsa` and `cargo tree | grep mysql` both show nothing, and a clean workspace
  build produces no `sqlx-mysql`/`rsa` build artifacts. Confirmed this can't be fixed by editing
  `Cargo.lock` directly: both `cargo update -p sqlx` and a full `cargo generate-lockfile` were
  tried and neither drops the entry, because `sqlx`'s own `Cargo.toml` declares `sqlx-mysql` as an
  optional dependency that Cargo's lock resolution accounts for regardless of which features this
  workspace activates. `cargo audit` now reports 0 vulnerabilities and 0 warnings (previously 2
  vulnerabilities + 1 unmaintained warning), all real ones eliminated by the `sqlx` bump.
- Added an optional per-deployment pepper to identity-matching hashes
  (`crates/identity/src/hash.rs`, env var `MB_IDENTITY_HASH_PEPPER`) to mitigate
  precomputed-dictionary attacks against low-entropy PII (email/phone) by a reader with database
  access. `hash_value` is only ever compared against hashes this codebase itself wrote (verified:
  every call site is inside `crates/identity/src/repo.rs`, in the same crate; no external
  CDP-partner pre-hashed values are matched against it anywhere), so salting doesn't break any
  external interop. When the env var is unset, the hash is byte-for-byte identical to the old
  unsalted `SHA-256(namespace:value)` (no migration forced on unconfigured deployments); setting
  it for the first time on a deployment with existing data is a breaking change requiring a
  re-hash migration, which is not implemented here (documented in `SECURITY.md`). Added tests in
  `crates/identity/src/hash.rs`.

### Fixed
- `crates/api/src/routes/profiles.rs`: replaced an unguarded `.unwrap()` on `f64::partial_cmp`
  when sorting merge suggestions by score with `.unwrap_or(Ordering::Equal)`, so a future `NaN`
  score (e.g. a scoring-function bug) degrades to an unstable sort order instead of panicking the
  request handler on network-influenced data.

### Known issues (see `ROADMAP_HONEST.md` and `SECURITY.md` for full detail)
- No authentication or tenant isolation in `mb-api`.
- `sdk/android` has never been compiled or tested.
- `edge/ingest-gateway` has no write-key/API-key auth and has never been deployed.
- Setting `MB_IDENTITY_HASH_PEPPER` for the first time on a deployment with existing identity data
  requires a re-hash migration that doesn't exist yet — see `SECURITY.md`.
