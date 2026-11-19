# US-055 — Uninstall Android Hub and its owned data

- **Status**: implemented
- **Owner**: Android Hub maintainer
- **Created**: 2026-11-19
- **Risk lane**: high-risk
- **Related**: [ADR 0059](../decisions/0059-confirmed-app-uninstall.md)

## Context

Deleting the app in Finder leaves Android devices and managed SDKs behind.
The owner requested an explicit uninstall action with preview, DELETE confirmation
and an optional backup exemption.

## Contract (acceptance criteria)

- [x] Settings offers a localized preview of paths and allocated sizes, with backup retention and exact DELETE confirmation.
- [x] Busy work must finish/cancel first. Confirmation stops registered engines; failure leaves data intact.
- [x] Only the installed running app and validated Hub data root are eligible. External SDK/Java, source and other apps are preserved.
- [x] A copied helper validates identities, holds the Hub lock and waits for the parent to exit before deleting. Data-stage errors retain the app; errors expose a report.
- [x] No automatic deletion on restart; symlinks are not followed and mounted volumes are rejected.
- [x] Destructive tests use disposable fixtures only; main app and guests stay untouched and test artifacts are removed.

## Out of scope

Finder Trash hooks, removing external SDK/Java, administrator privileges, wiping
free space, removing other users' data, publication or version bump.

## Verify

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts -p 'test_*.py'
PYTHONDONTWRITEBYTECODE=1 python3 scripts/check-docs.py
git diff --check
```

## Evidence

Verified on 2026-11-19:

- Workspace: 249 tests passed, 25 environment-dependent tests ignored. Formatting,
  Clippy and diff whitespace checks passed; 32 script tests and documentation
  checks passed. Clippy retains the existing upstream `block v0.1.6` future
  compatibility notice.
- Disposable fixture tests exercise both complete removal and backup retention,
  external SDK/symlink preservation, changed paths, locks, interrupted handoff,
  shutdown failure and an engine appearing during handoff.
- Native isolated QA: English/Vietnamese previews, DELETE gating, clearing
  confirmation on backup changes, Escape and Cancel verified. QA app closed.
- A real copied helper was started through its stdin/stdout protocol and correctly
  refused the concurrent main Hub. QA data stayed intact with no pending marker.
- No production uninstall or complete destructive native UI flow was executed.
  Main Hub and user data remained untouched. QA bundle, build output and task logs
  were removed at handoff; release bundles were retained.

Target release: v0.2.0. This feature is not part of v0.1.1.
