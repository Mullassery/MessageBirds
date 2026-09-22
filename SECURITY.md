# Security Policy

MessageBirds is a Customer Data Platform: by design it ingests, resolves, and stores customer
identity claims (email, phone, device ids) and profile data. Read this before deploying it
against real customer data — as of 2026-09-22, **it is not ready for that**, and the gaps below
are the reason why. This is not hedging; see `ROADMAP_HONEST.md` for the full technical detail.

## Current security posture (read this first)

This is pre-production software. The following are known, disclosed, unfixed gaps — not
hypothetical risks:

- **No authentication or authorization anywhere in `mb-api`.** There is no login, no API key, no
  session, no RBAC. Any caller who can reach the HTTP port and knows (or guesses) a `tenant_id`
  UUID can read or write that tenant's data, including PII and consent records. `tenant_id` is
  threaded through every table as a plain column with no enforcement layer above it.
- **`edge/ingest-gateway` has no write-key/API-key check.** It validates CORS and request-body
  shape only, then forwards to `mb-api`. It has also never been deployed (no `wrangler deploy`
  has been run against a real Cloudflare account), which limits real-world exposure to zero today,
  but the code itself has no credential check to rely on once it is deployed.
- **Identity-matching hashes support an optional per-deployment pepper, but it defaults to off.**
  `crates/identity/src/hash.rs` hashes email/phone/etc. with
  `SHA-256(pepper || ":" || namespace || ":" || value)`, where `pepper` comes from the
  `MB_IDENTITY_HASH_PEPPER` environment variable (fixed 2026-09-22). **You must set this variable**
  in any environment that touches real PII, or the hash is still plain, unkeyed
  `SHA-256(namespace:value)` — low-entropy inputs remain crackable by a reader with database
  access via a precomputed dictionary. Setting the pepper for the first time on a deployment that
  already has identity data written is a breaking change (old hashes stop matching); there is no
  re-hash migration tool, so plan that separately. See `ROADMAP_HONEST.md`'s Security section for
  the full threat model.
- **`sqlx` was upgraded from `0.7.4` to `0.8.6`** (fixed 2026-09-22), resolving RUSTSEC-2024-0363
  (binary protocol misinterpretation via truncating/overflowing casts). No breaking API changes
  in `sqlx` 0.8 affected this codebase (no compile-time `query!`/`query_as!` macros are used).
- **No rate limiting anywhere** — not in `mb-api`, not in `edge/ingest-gateway`.
- **No multi-tenancy isolation** beyond the `tenant_id` column convention described above.

If you deploy this today, deploy it behind a network boundary you control (not the open internet),
with no real customer PII, until authentication and tenant isolation exist.

## Reporting a vulnerability

This is a solo-maintained open-source project. Please report suspected vulnerabilities privately
rather than opening a public GitHub issue:

- Use **[GitHub Security Advisories](https://github.com/Mullassery/MessageBirds/security/advisories/new)**
  for this repository ("Report a vulnerability" under the Security tab), or
- Email **mullassery@gmail.com** with a description, reproduction steps, and impact assessment.

There is no dedicated security team and no SLA. As a solo maintainer, expect an initial
acknowledgment within a few days on a best-effort basis, not a guaranteed response time. Please
don't test against any real deployment other than one you control — there is no hosted/public
instance of MessageBirds to test against.

## Secrets and PII handling

- No secrets, API keys, or credentials are committed to this repository (checked this pass: no
  `.env` files, no hardcoded tokens/keys/passwords in source, no credential-shaped files tracked
  by git). The only credential-looking string anywhere is the local dev Postgres password
  (`messagebirds`/`messagebirds`) in `docker-compose.yml`, which matches the `DATABASE_URL` the
  `README.md` quick-start already prints in plain text — a conventional local-dev-only value, not
  a real secret.
- Raw event payloads (`events` table) legitimately store PII in cleartext where the source system
  sent it (e.g. an email address in a `context.profile_updates` payload) — this is required for
  profile projection to work and is documented in `docs/OCDS.md`. Only the *identity-matching*
  layer (`identity_nodes`, `identity_audit`) hashes values, and only for equality-matching
  purposes, not as an at-rest encryption guarantee for the raw event log.
- Consent state (`consent_events`) is append-only by convention (never `UPDATE`d/`DELETE`d by
  application code) so consent history is always reconstructable — see `docs/ARCHITECTURE.md`'s
  Phase 4 section. This is enforced by discipline in the repository code, not by a database-level
  `REVOKE UPDATE` grant — a determined or buggy caller with DB access could still mutate it.
- There is no data retention or delete-and-forget (right-to-be-forgotten) implementation anywhere
  in this codebase. If you need one for compliance, you must build it before storing real customer
  data.

## Supported versions

This project is pre-1.0 (`version = "0.1.0"` across the Rust workspace and `sdk/python`). There is
one supported line: `main`. No backported security fixes to older tags — there aren't any tags to
backport to yet.

## Dependency scanning

`cargo audit`, `npm audit`, and `pip-audit` were run against this repository on 2026-09-22. `npm
audit` and `pip-audit` reported zero vulnerabilities. `cargo audit` originally found two real
findings (`sqlx`, above, and an unreachable `rsa` transitive dependency pulled in only via
`sqlx-mysql`'s unactivated `mysql` feature — see `ROADMAP_HONEST.md`) plus one transitive,
unmaintained crate warning (`paste`). As of the same day: the `sqlx` bump above fixes the real
vulnerability (and incidentally dropped the `paste` dependency entirely), and `.cargo/audit.toml`
documents and suppresses the unreachable `rsa` finding (it cannot be removed from `Cargo.lock`
itself — both `cargo update` and a full lockfile regeneration were tried and neither drops it,
since `sqlx` declares it as an optional dependency regardless of this workspace's enabled
features). `cargo audit` now reports 0 vulnerabilities and 0 warnings. Dependabot
(`.github/dependabot.yml`) is configured to open PRs for future dependency updates across cargo,
npm, pip, swift, and gradle.
