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
- `crates/*` (Rust workspace, 17 crates) — `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`: all clean, 55 tests passed (52 baseline + 3 added with the identity-hash pepper fix, see Security #4), 0 failed. `cargo audit`: 0 vulnerabilities, 0 warnings (see Security #1/#2 — both real findings fixed 2026-09-22). No integration tests requiring a live Postgres/Kafka exist — everything that ran is a unit test.
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
cargo audit                                                    # was 2 vulnerabilities + 1 unmaintained warning; 0/0 after the 2026-09-22 fixes — see Security
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

1. ~~**Real, fixable dependency vulnerability: `sqlx 0.7.4`.**~~ **Fixed 2026-09-22.** Bumped to
   `sqlx = "0.8.1"` (resolved to `0.8.6`, `Cargo.toml:41`), fixing `RUSTSEC-2024-0363`. This turned
   out to be a clean, mechanical bump for this codebase: it uses only runtime `sqlx::query`/
   `sqlx::query_as` with `.bind()` everywhere (verified — no `sqlx::query!`/`query_as!`
   compile-time macros anywhere in the workspace), so none of `sqlx` 0.8's breaking changes
   (which mostly affect the macro/offline-mode path) touched any call site.
   `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and
   `cargo test --workspace` (55/55, up from 52 — see item 4) all pass with zero source changes
   needed beyond the version bump itself. This also incidentally dropped the transitive `paste`
   dependency entirely, so finding 3 below is now moot.
2. ~~**`rsa 0.9.10`**~~ **Documented and suppressed 2026-09-22**, not fixed at the `Cargo.lock`
   level because it turns out that's not possible from this side. Confirmed the earlier diagnosis
   (unreachable, pulled in only via `sqlx-mysql`'s unactivated `mysql` feature) and then actually
   tried to prune it: both `cargo update -p sqlx` and a full `rm Cargo.lock && cargo
   generate-lockfile` were run, and neither removes the `sqlx-mysql`/`rsa` entries — `sqlx`'s own
   `Cargo.toml` declares `sqlx-mysql` as an optional dependency, and Cargo's lockfile resolution
   accounts for every optional dependency a crate declares regardless of which features any
   workspace member actually activates. There is no clean way to remove this from `Cargo.lock`
   short of patching `sqlx` upstream. Added `.cargo/audit.toml` instead, which suppresses
   `RUSTSEC-2023-0071` from `cargo audit` output with the full investigation inline as a comment,
   so the tool's output stays meaningful for advisories that are actually reachable.
   `cargo audit` now reports **0 vulnerabilities and 0 warnings**.
3. ~~**`paste 1.0.15`**~~ **Gone as of the `sqlx` 0.8 bump** (item 1) — `sqlx` 0.8.6 no longer
   pulls in `paste` at all. No action needed.
4. **Unsalted, deterministic PII hashing for identity matching — partially fixed 2026-09-22.**
   `crates/identity/src/hash.rs`'s `hash_value` now mixes in an optional per-deployment pepper
   read from the `MB_IDENTITY_HASH_PEPPER` environment variable:
   `SHA-256(pepper || ":" || namespace || ":" || value)` when the pepper is set, falling back to
   the original byte-for-byte `SHA-256(namespace || ":" || value)` when it's unset (so existing
   unconfigured deployments keep matching every hash they've already written — no forced
   migration). Confirmed this is safe to add: every call site of `hash_value` lives inside
   `crates/identity/src/repo.rs`, in the same crate — the hash is never compared against an
   externally-supplied pre-hashed value from a CDP integration partner or anything else outside
   this codebase, so keying it doesn't break any interop. **This is only a partial fix**: the
   pepper is opt-in (an env var, not enforced), so a deployment that doesn't set it is exactly as
   exposed as before — `SECURITY.md` now says this explicitly rather than implying the gap is
   closed. It's also still a single global pepper rather than genuinely per-tenant, and setting it
   for the first time on a deployment with existing identity data is a breaking change (a
   re-hash migration, not implemented here, would be needed). Tests added in
   `crates/identity/src/hash.rs`.
5. **No authentication or tenant isolation anywhere in `mb-api`.** Confirmed by grep: zero references to `auth`, `Authorization`, `cors`, or `middleware` in `crates/api/src/`. `tenant_id` is a plain field threaded through every table and query parameter — nothing stops a caller who knows a `tenant_id` UUID from reading or writing another tenant's data, including PII and consent state. `docs/ARCHITECTURE.md` already discloses this ("no auth layer at all"); repeating it here because for a customer-data platform this is the single highest-impact gap, disclosed or not, and should be the top priority before anyone points a real browser/device SDK at a real deployment.
6. **`edge/ingest-gateway` has no write-key/API-key check.** Confirmed by reading `edge/ingest-gateway/src/`: it validates CORS and envelope shape only, then forwards straight to `mb-api`. Combined with finding 5, there is currently no credential anywhere in the request path from a browser to Postgres. Already disclosed in `docs/ARCHITECTURE.md`; also not deployed (`wrangler deploy` has never been run), which limits current exposure to zero in practice.
7. **No SQL injection found.** I audited every `sqlx::query_as(&format!(...))` call (33 call sites across `crates/audiences`, `crates/channels`, `crates/governance`, `crates/journeys`, `crates/templates` — e.g. `crates/audiences/src/repo.rs:91-97`) because dynamic query-string construction is the classic injection smell. In every case the `format!` interpolates only a compile-time `const` column list (e.g. `AUDIENCE_COLUMNS` at `crates/audiences/src/repo.rs:71`); every actual value is passed through `.bind()` with numbered placeholders. This pattern is safe as used, just worth someone re-checking if a new call site copies the pattern carelessly.
8. **No secrets found committed.** Searched for API keys, passwords, tokens, private keys, `.env` files, and credential-shaped files across the tree (excluding `node_modules`/`target`/`.build`). The only credential-shaped string is the local dev Postgres password (`messagebirds`/`messagebirds`) in `docker-compose.yml`, which matches the `DATABASE_URL` already printed in `README.md`'s own quick-start — a conventional, low-stakes local-dev-only credential, not a leaked production secret.

## Technical debt (concrete, file:line)

**Needs a dedicated follow-up session:**
- No authentication/authorization layer at all in `mb-api` (see Security #5). This is the biggest single piece of real work left before this could touch real customer data.
- `sdk/android` is unverified (no build environment available in two separate sessions now, including this one). Needs a machine with Gradle/Android SDK to even discover whether it compiles.
- A re-hash migration for `crates/identity/src/hash.rs`'s pepper (Security #4): the pepper support added 2026-09-22 is opt-in via env var and doesn't retroactively re-hash existing data — a deployment that wants to turn it on after already having identity data needs a migration tool that doesn't exist yet.
- Making the identity-hash pepper mandatory/enforced (rather than silently falling back to unsalted) is a real product decision (breaks any deployment that hasn't set it) and deliberately wasn't done unilaterally in this pass.

**Resolved 2026-09-22** (previously listed here, kept for history):
- ~~`Cargo.toml:41` — `sqlx = "0.7"` behind a known, fixed CVE~~ — bumped to `0.8.6`, clean mechanical change, see Security #1.
- ~~`rsa`/`sqlx-mysql` orphaned `Cargo.lock` entries~~ — confirmed unremovable from `Cargo.lock` itself; suppressed via `.cargo/audit.toml` instead, see Security #2.

**Minor, noted but not urgent:**
- ~~`crates/api/src/routes/profiles.rs:209` — unguarded `.unwrap()` on `f64::partial_cmp`~~ **Fixed
  2026-09-22**: now `.unwrap_or(std::cmp::Ordering::Equal)`, so a future `NaN` score degrades to
  an unstable sort order instead of panicking the request handler.
- `crates/connectors/src/repo.rs:18,35`, `crates/profile/src/repo.rs:31,81`, `crates/governance/src/repo.rs:41`, `crates/journeys/src/repo.rs:16` — six `#[allow(clippy::too_many_arguments)]` suppressions. All are on repo constructor/insert functions with 8+ positional args; legitimate for now but a natural refactor target (builder structs) if any of these functions grow another parameter.
- No integration tests anywhere in the Rust workspace exercise a live Postgres/Kafka — every one of the 55 passing `cargo test` cases is a pure unit test (verified by reading the test output: no `#[sqlx::test]` or testcontainers usage found). The extensive manual "verified end to end" narratives in `docs/ARCHITECTURE.md` (killing/restarting `journeys-worker` mid-`Wait`, the governance policy simulator matching real activation, etc.) are real but were done by hand against a live stack, not captured as automated tests. This is the single biggest gap between "the code works" and "we'd know if it stopped working" — a good candidate for the next dedicated session, especially for `crates/journeys` and `crates/worker`'s pipeline, which are the most stateful/timing-sensitive code in the repo.
- No `docs/architecture/README.md`-level diagram existed before this pass (see below) — not debt exactly, but a real gap for anyone trying to onboard without reading five paragraphs of prose first.

## What I deliberately did not do this pass

- Did not build an authentication/tenant-isolation layer for `mb-api`, add write-key auth to
  `edge/ingest-gateway`, or enforce the identity-hash pepper (make it mandatory rather than
  opt-in) — all real feature work, out of scope for a quick-fix pass.
- Did not write a re-hash migration tool for existing identity data when the pepper is turned on
  for the first time — real implementation work, flagged in Technical debt above.
- Did not attempt to install Gradle/Android SDK to verify `sdk/android` — no toolchain available,
  same as every prior pass.
- Did not write new integration tests against a live Postgres/Kafka stack — flagged above as the
  top testing gap, but writing them is implementation work, not a quick fix.
