# US-044 — Rust source organization and cleanup

- **Status**: implemented
- **Created**: 2026-11-18
- **Risk lane**: normal
- **Related**: [ADR 0053](../decisions/0053-rust-module-conventions.md)

## Context

The feature set is sufficient for personal use. Review the current source against
Cargo/Rust conventions, simplify large modules, align style and remove retired
engine support without changing guest behavior or persisted data.

## Contract (acceptance criteria)

- [x] Consistent named module roots and separate event, bootstrap, SDK and view responsibilities.
- [x] Public infrastructure API and data locations preserved.
- [x] No active AVO runtime dependency or sampler support; current docs describe Android Emulator.
- [x] Shared checks cover the app and standalone metrics tool; all pass.
- [x] Updated development/architecture/plan docs; task build outputs removed after packaging.

## Out of scope

Runtime migration, dependency upgrades, feature changes, guest boot and long benchmarks.
Historical ADRs, evidence, SDK license/protocol inputs and real device data are retained.

## Verify

```bash
bash scripts/check.sh
cargo metadata --offline --locked --no-deps --format-version 1
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

## Evidence

All Verify commands passed on 2026-11-18. App: 206 Rust tests passed, 22 ignored;
metrics: 2 passed; Python: 26 passed. Formatting/Clippy/shell checks passed.
Cargo metadata confirms the expected workspace/targets. Main bundle rebuilt and
`codesign --verify --deep --strict` passed. Task targets/temp logs and source
Finder metadata were removed after packaging; main app/runtime retained.
See [audit report](../reports/rust-source-audit-20261118.md) for changes and limits.
