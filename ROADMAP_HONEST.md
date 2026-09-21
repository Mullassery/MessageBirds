# Honest Roadmap & Technical Debt

This file exists so status claims in this repo are checkable, not marketing. For the detailed
phase-by-phase build history and what each layer does, `docs/ARCHITECTURE.md` is the source of
truth — it is unusually thorough and already written in this same disclosure-first style. This
file adds two things `docs/ARCHITECTURE.md` doesn't: a flat status bucket list, and a concrete,
file:line technical-debt/security backlog for a dedicated follow-up session.

Last verified: 2026-09-22, by actually running each component's test suite and reading the
relevant source (not by re-reading old docs). Commands and results are in the section below.

## Status, in four buckets

### 1. Shipped and verified by me, this pass
- `crates/*` (Rust workspace, 17 crates) — `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`: all clean, 52 tests passed, 0 failed. No integration tests requiring a live Postgres/Kafka exist — everything that ran is a unit test.
- `sdk/js` — `tsc` build clean.
- `sdk/web` — `tsc --noEmit` clean, `vitest run`: 5/5 tests passed.
- `edge/ingest-gateway` — `tsc --noEmit` clean, `vitest run`: 8/8 tests passed.
- `sdk/python` — `pytest sdk/python`: 6/6 tests passed (Python 3.11 venv, editable install).
- `ui/` (Next.js) — `tsc --noEmit` clean, `next build` succeeds, all 13 routes compile.
- `sdk/ios` — `swift build` and `swift run mb-verify` both pass (4/4 assertions in the verify CLI). `swift test` fails on this machine with `error: no such module 'Testing'` — confirmed this is a real environment limitation (only Xcode Command Line Tools installed, not full Xcode.app), not a code problem; the CI workflow (`.github/workflows/ci.yml`, `ios-sdk` job) runs on `macos-15` with `maxim-lobanov/setup-xcode@v1` pinned to `latest-stable`, which is the only place `Tests/MessageBirdsAnalyticsTests` has ever actually executed.

### 2. Shipped, honestly disclosed as unverified or not deployed (docs/ARCHITECTURE.md already says this — confirmed still true)
- `sdk/android` — written against real Kotlin/Android APIs, never compiled. Confirmed again this pass: `gradle`, `kotlinc`, `adb` are all absent from this machine too, so this is still a real, standing gap, not a stale claim.
- `edge/ingest-gateway` — built and passes tests, but has never been `wrangler deploy`'d; no API-key/write-key auth, no rate limiting.
- PyReverseETL bulk activation path — works against the real PyReverseETL CLI, but skips field-level label policy (only consent is checked on that path).

### 3. Not built — no code exists (from `docs/ARCHITECTURE.md`'s "explicitly NOT built" list, spot-checked, still accurate)
Decisioning, AI agents/model-provider abstraction, a CLI, multi-tenancy enforcement (RBAC/SSO/tenant isolation), real SMS/email/push channels, connectors beyond one webhook, sequence audience conditions, statistical experimentation, visual journey canvas, data retention/delete-and-forget workflows, most of the standard mixin library (only identity/person/contact/device are seeded).

### 4. Explicitly out of scope for this project's current milestone
Enterprise deployment topology (Kafka cluster, Flink, Temporal, ClickHouse, object storage, Kubernetes) — `docker-compose.yml` (Postgres + Redpanda) is the only deployment target and that's a deliberate, stated choice, not a gap.

## Validation commands run this pass (with real output)

```
cargo fmt --all -- --check                                    # clean
cargo clippy --workspace --all-targets -- -D warnings         # clean (1 unrelated future-incompat warning, see below)
cargo test --workspace                                        # 52 passed, 0 failed
cargo audit                                                    # 2 vulnerabilities, 1 unmaintained warning — see Security
npm install && npm run build --workspace=sdk/js                # clean
npm exec --workspace=sdk/web -- tsc --noEmit && npm run test --workspace=sdk/web   # clean, 5/5
npm run typecheck --workspace=edge/ingest-gateway && npm run test --workspace=edge/ingest-gateway  # clean, 8/8
npm exec --workspace=ui -- tsc --noEmit && npm run build --workspace=ui            # clean, builds
pip install -e "./sdk/python[dev]" && pytest sdk/python        # 6/6 passed
cd sdk/ios && swift build && swift run mb-verify               # both pass
cd sdk/ios && swift test                                       # fails locally: no `Testing` module (CLT-only machine, expected)
npm audit                                                       # 0 vulnerabilities
pip-audit (against sdk/python's declared deps)                 # 0 known vulnerabilities
actionlint .github/workflows/ci.yml                             # clean
```

## Security findings

1. **Real, fixable dependency vulnerability: `sqlx 0.7.4`.** `cargo audit` reports `RUSTSEC-2024-0363` ("Binary Protocol Misinterpretation caused by Truncating or Overflowing Casts"), fixed in `sqlx >=0.8.1`. The workspace pins `sqlx = { version = "0.7", ... }` in `Cargo.toml:41`. `sqlx` is used for all Postgres access across every crate with a `repo.rs`, so bumping to 0.8 is a real migration (API changes in `sqlx` 0.8, `sqlx::query_as` builder changes, `PgPool` construction changes) — not a drop-in version bump. **Not fixed in this pass** (non-trivial, needs its own session + full re-test of every crate's repo layer).
2. **`rsa 0.9.10`** — `RUSTSEC-2023-0071` (Marvin Attack timing side-channel), no fix available upstream. Traced this: it's pulled in transitively by `sqlx-mysql` (`cargo tree -i rsa` and `cargo tree | grep mysql` both show **nothing** — it's not actually in the compiled dependency graph for this workspace's enabled features, only `postgres` is enabled in `Cargo.toml:41`). This means `Cargo.lock` has a stale/orphaned entry for a MySQL-only dependency tree that isn't actually reachable from this workspace's feature set. Low real risk (not compiled into any binary here) but poor hygiene — a `cargo update` (after auditing what else moves) should prune it. Not fixed in this pass — touching `Cargo.lock` broadly wasn't in scope for a doc-first pass and deserves its own verification.
3. **`paste 1.0.15`** — flagged unmaintained (`RUSTSEC-2024-0436`), not a vulnerability. Transitive, low priority.
4. **Unsalted, deterministic PII hashing for identity matching.** `crates/identity/src/hash.rs` hashes identity claim values (email, phone, etc.) with plain `SHA-256(namespace || ":" || value)` (`hash_value`, `crates/identity/src/hash.rs:5-10`) before storing them in `identity_nodes.value_hash` / `identity_audit.value_hash`. This is by design for exact-match dedup (documented in `docs/OCDS.md` and `docs/ARCHITECTURE.md`), but it is **not** a privacy-preserving hash for genuinely low-entropy inputs like emails and phone numbers: anyone with read access to `identity_nodes` can precompute `SHA256("email:" + guess)` for a candidate list (a leaked breach corpus, a phone number range) and confirm exact matches with no brute-force cost, i.e., de-anonymize the hash without ever touching the plaintext events table. This is a real weakness the existing docs don't call out — they only note that raw events (which legitimately need cleartext PII to be useful) carry the plaintext. **Recommendation for follow-up**: a per-tenant (or per-installation) HMAC key (`HMAC-SHA256(key, value)`) instead of unkeyed SHA-256 would close this without changing the matching semantics (still deterministic, still equality-comparable) — a real fix, not a big one, but it touches a migration-sensitive column (existing hashes would need re-hashing or a versioned hash column), so it's flagged here rather than done live.
5. **No authentication or tenant isolation anywhere in `mb-api`.** Confirmed by grep: zero references to `auth`, `Authorization`, `cors`, or `middleware` in `crates/api/src/`. `tenant_id` is a plain field threaded through every table and query parameter — nothing stops a caller who knows a `tenant_id` UUID from reading or writing another tenant's data, including PII and consent state. `docs/ARCHITECTURE.md` already discloses this ("no auth layer at all"); repeating it here because for a customer-data platform this is the single highest-impact gap, disclosed or not, and should be the top priority before anyone points a real browser/device SDK at a real deployment.
6. **`edge/ingest-gateway` has no write-key/API-key check.** Confirmed by reading `edge/ingest-gateway/src/`: it validates CORS and envelope shape only, then forwards straight to `mb-api`. Combined with finding 5, there is currently no credential anywhere in the request path from a browser to Postgres. Already disclosed in `docs/ARCHITECTURE.md`; also not deployed (`wrangler deploy` has never been run), which limits current exposure to zero in practice.
7. **No SQL injection found.** I audited every `sqlx::query_as(&format!(...))` call (33 call sites across `crates/audiences`, `crates/channels`, `crates/governance`, `crates/journeys`, `crates/templates` — e.g. `crates/audiences/src/repo.rs:91-97`) because dynamic query-string construction is the classic injection smell. In every case the `format!` interpolates only a compile-time `const` column list (e.g. `AUDIENCE_COLUMNS` at `crates/audiences/src/repo.rs:71`); every actual value is passed through `.bind()` with numbered placeholders. This pattern is safe as used, just worth someone re-checking if a new call site copies the pattern carelessly.
8. **No secrets found committed.** Searched for API keys, passwords, tokens, private keys, `.env` files, and credential-shaped files across the tree (excluding `node_modules`/`target`/`.build`). The only credential-shaped string is the local dev Postgres password (`messagebirds`/`messagebirds`) in `docker-compose.yml`, which matches the `DATABASE_URL` already printed in `README.md`'s own quick-start — a conventional, low-stakes local-dev-only credential, not a leaked production secret.

## Technical debt (concrete, file:line)

**Needs a dedicated follow-up session** (all four below are real, non-trivial, deliberately not touched in this pass):
- `Cargo.toml:41` — `sqlx = "0.7"` is behind a version with a known, fixed CVE (see Security #1). Bumping is a multi-crate migration.
- `crates/identity/src/hash.rs:5-10` — unsalted SHA-256 for PII identity matching (see Security #4). Fixing means a keyed hash plus a migration story for existing hashed values.
- No authentication/authorization layer at all in `mb-api` (see Security #5). This is the biggest single piece of real work left before this could touch real customer data.
- `sdk/android` is unverified (no build environment available in two separate sessions now, including this one). Needs a machine with Gradle/Android SDK to even discover whether it compiles.

**Minor, noted but not urgent:**
- `crates/api/src/routes/profiles.rs:209` — `suggestions.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap())`. `partial_cmp` on `f64` returns `None` for `NaN`; if a similarity score is ever `NaN` (e.g. a future scoring change divides by zero), this panics the request handler instead of erroring gracefully. Low likelihood today (`mb_profile::similarity::score` doesn't currently produce `NaN`), but it's an unguarded `unwrap()` on network-influenced data in a live HTTP handler.
- `crates/connectors/src/repo.rs:18,35`, `crates/profile/src/repo.rs:31,81`, `crates/governance/src/repo.rs:41`, `crates/journeys/src/repo.rs:16` — six `#[allow(clippy::too_many_arguments)]` suppressions. All are on repo constructor/insert functions with 8+ positional args; legitimate for now but a natural refactor target (builder structs) if any of these functions grow another parameter.
- `rsa`/`sqlx-mysql` orphaned `Cargo.lock` entries (see Security #2) — cosmetic/hygiene, worth cleaning up whenever `sqlx` gets bumped anyway.
- No integration tests anywhere in the Rust workspace exercise a live Postgres/Kafka — every one of the 52 passing `cargo test` cases is a pure unit test (verified by reading the test output: no `#[sqlx::test]` or testcontainers usage found). The extensive manual "verified end to end" narratives in `docs/ARCHITECTURE.md` (killing/restarting `journeys-worker` mid-`Wait`, the governance policy simulator matching real activation, etc.) are real but were done by hand against a live stack, not captured as automated tests. This is the single biggest gap between "the code works" and "we'd know if it stopped working" — a good candidate for the next dedicated session, especially for `crates/journeys` and `crates/worker`'s pipeline, which are the most stateful/timing-sensitive code in the repo.
- No `docs/architecture/README.md`-level diagram existed before this pass (see below) — not debt exactly, but a real gap for anyone trying to onboard without reading five paragraphs of prose first.

## What I deliberately did not do this pass

- Did not bump `sqlx`, touch `Cargo.lock` beyond what `cargo test`/`cargo audit` did read-only, or attempt a keyed-hash migration for identity values — all three are real fixes that need their own review/testing cycle, not a doc pass.
- Did not attempt to install Gradle/Android SDK to verify `sdk/android` — out of scope for a documentation pass and the repo's own docs already disclose this honestly.
- Did not write new integration tests against a live Postgres/Kafka stack — flagged above as the top testing gap, but writing them is implementation work, not disclosure.
