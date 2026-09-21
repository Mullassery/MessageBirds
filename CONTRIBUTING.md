# Contributing to MessageBirds

Thanks for considering a contribution. This is a solo-maintained, pre-1.0 project — expect the
architecture to still move, and expect review to take a few days on a best-effort basis rather
than a guaranteed SLA.

## Before you start

Read `docs/ARCHITECTURE.md` and `ROADMAP_HONEST.md` first. Both are written in a deliberately
blunt "what's real vs. deferred" style — they'll tell you faster than the code will whether the
thing you want to build already half-exists, was tried and abandoned, or is a disclosed gap
waiting for exactly this kind of PR.

For anything beyond a small fix, please open an issue first to discuss the approach before writing
code — this project has a strong existing pattern (small, honestly-scoped, verified-end-to-end
increments) and it's easier to agree on scope before the diff exists than after.

## Repository layout

- `crates/` — Rust workspace (event platform, identity, CDP, governance, engagement engines)
- `edge/ingest-gateway` — Cloudflare Worker (TypeScript)
- `sdk/js`, `sdk/web`, `sdk/python`, `sdk/ios`, `sdk/android` — client SDKs
- `ui/` — Next.js internal viewer/admin app
- `migrations/` — SQL migrations, applied automatically by the `api` binary on startup
- `docs/` — architecture and data-model documentation

## Development setup

```bash
docker compose up -d postgres redpanda
DATABASE_URL=postgres://messagebirds:messagebirds@localhost:5432/messagebirds cargo run -p api
DATABASE_URL=postgres://messagebirds:messagebirds@localhost:5432/messagebirds cargo run -p worker
npm install
```

See `README.md` for the full quick-start including sending a test event.

## Running tests and lint before opening a PR

Match what CI (`.github/workflows/ci.yml`) actually runs:

```bash
# Rust
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# JS/TS
npm run build --workspace=sdk/js
npm exec --workspace=sdk/web -- tsc --noEmit && npm run test --workspace=sdk/web
npm run typecheck --workspace=edge/ingest-gateway && npm run test --workspace=edge/ingest-gateway
npm exec --workspace=ui -- tsc --noEmit && npm run build --workspace=ui

# Python
pip install -e "./sdk/python[dev]"
pytest sdk/python

# iOS (needs a Mac with full Xcode.app, not just Command Line Tools)
cd sdk/ios && swift build && swift test && swift run mb-verify
```

`sdk/android` has no CI job yet — there's no Gradle/Android SDK available to run one against.
If you can get it building, that's a genuinely useful contribution; please include how you
verified it (what you ran, on what) since the existing docs are explicit that it's never been
compiled.

## PR expectations

- Keep PRs scoped to one crate/SDK/concern where possible — this matches how the project has been
  built phase-by-phase.
- If you touch behavior described in `docs/ARCHITECTURE.md` or `docs/OCDS.md`, update those docs
  in the same PR. Stale docs are treated as a bug here.
- State plainly in the PR description what you tested and how (unit tests, manual end-to-end,
  neither). Don't describe untested code as working — that's the one house rule this project is
  strict about; see `docs/ARCHITECTURE.md` for the tone to match.
- No `TODO`/`FIXME` placeholders that pretend to work — if something isn't finished, either finish
  it, don't merge it, or disclose the gap explicitly in the code comment and in `ROADMAP_HONEST.md`.

## Reporting bugs and requesting features

Use the GitHub issue templates (`.github/ISSUE_TEMPLATE/`). For security issues, do **not** open
a public issue — see `SECURITY.md`.

## License

By contributing, you agree your contributions are licensed under this project's Apache-2.0
license (see `LICENSE`).
