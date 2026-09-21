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
- **Identity-matching hashes are unsalted.** `crates/identity/src/hash.rs` hashes email/phone/etc.
  with plain `SHA-256(namespace:value)`. Because these inputs are low-entropy and guessable, this
  is not a privacy-preserving hash — see `ROADMAP_HONEST.md`'s Security section for the full
  explanation and a proposed fix (keyed HMAC).
- **`sqlx 0.7.4`** (pinned in `Cargo.toml`) has a known, fixed vulnerability
  (RUSTSEC-2024-0363, binary protocol misinterpretation via truncating/overflowing casts), fixed
  in `sqlx >= 0.8.1`. Not yet upgraded — it's a real, multi-crate migration, tracked in
  `ROADMAP_HONEST.md`.
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

`cargo audit`, `npm audit`, and `pip-audit` were run against this repository on 2026-09-22 (see
`ROADMAP_HONEST.md` for exact findings). `npm audit` and `pip-audit` reported zero vulnerabilities.
`cargo audit` found one real, fixable vulnerability (`sqlx`, above) and one transitive, unmaintained
crate warning (`paste`). Dependabot (`.github/dependabot.yml`) is configured to open PRs for future
dependency updates across cargo, npm, pip, swift, and gradle.
