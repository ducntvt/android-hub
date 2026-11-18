## Project documentation for agents

When making non-trivial changes, consult:

- @docs/ARCHITECTURE.md — boundary rules, layers, patterns to follow / avoid
- @docs/decisions/ — Architecture Decision Records (ADRs); see `README.md` for index
- @docs/stories/ — bounded work packets with verify-all commands; see `README.md` for workflow

For normal-sized features, write a story using `docs/stories/template.md` and follow it through `planned → in_progress → implemented`. Run all commands under `## Verify` before marking implemented.

If a change alters a layer boundary, picks between viable alternatives, or affects auth/data/contracts, write a new ADR using `docs/decisions/template.md`.

## QA session cleanup

User instruction (2026-11-16): close test Android Hub apps after each test session. Verify their processes have exited. Keep the user's main Hub and existing Android guests running unless stopping them is explicitly part of the authorized test. Do not leave multiple QA windows open after handoff.

User instruction (2026-11-16): remove disposable build/test artifacts at handoff. Record temporary paths created for a task, close their QA apps and remove their temporary checkouts/bundles when finished. Use `PYTHONDONTWRITEBYTECODE=1` for Python checks. Do not classify an AVD, registry, backup, SDK install, embedded DEX or bundled SDK manifest as cache. Keep the main app; do not rebuild after cleaning `target/` solely to verify cleanup. Runtime relocation requires a separate migration, not a folder rename.

## Current data layout

As of US-045 (2026-11-18), default app data lives directly in
`~/Library/Application Support/Android Hub`; `ANDROID_HUB_ROOT` is an explicit
QA data root with no legacy suffix. The user authorized deleting the old
`experiments/p0/runtime` and recreating devices, not migrating AVDs. Do not
recreate that experimental tree. Metrics source is `tools/metrics`. Future
cleanup must preserve populated app data unless explicitly authorized again.
