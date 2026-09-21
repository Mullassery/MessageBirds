## What does this PR do?

<!-- One or two sentences. Link an issue if there is one. -->

## Component(s) touched

<!-- crates/<name>, sdk/js, sdk/web, sdk/python, sdk/ios, sdk/android, edge/ingest-gateway, ui/, docs -->

## How was this tested?

<!--
Be specific and honest — this project's docs (docs/ARCHITECTURE.md, ROADMAP_HONEST.md) are
deliberately blunt about what's verified vs. not, and PRs are expected to match that tone.
State plainly which of these you did, and don't describe untested code as working:
-->

- [ ] Ran the relevant test suite (`cargo test -p <crate>`, `npm run test --workspace=<pkg>`,
      `pytest sdk/python`, `swift test`/`swift run mb-verify`) — paste the actual output below
- [ ] Added/updated tests covering the change
- [ ] Manually verified end to end (describe exactly what you ran and observed)
- [ ] Not tested (explain why, and what risk that leaves)

```
<paste test output here>
```

## Docs updated?

- [ ] `docs/ARCHITECTURE.md` and/or `docs/OCDS.md` updated to match the new behavior
- [ ] `ROADMAP_HONEST.md` updated if this closes or introduces a disclosed gap
- [ ] N/A — no user-visible or architectural change

## Checklist

- [ ] `cargo fmt --all -- --check` / `tsc --noEmit` / equivalent passes for the language(s) touched
- [ ] No `TODO`/`FIXME` placeholders that pretend to work — finished, not merged, or explicitly
      disclosed as a gap
- [ ] No secrets, credentials, or real customer data in the diff
