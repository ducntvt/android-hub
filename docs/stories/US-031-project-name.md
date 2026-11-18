# US-031 — Rename the project to Android Hub

- **Status**: implemented
- **Created**: 2026-11-16
- **Risk lane**: normal
- **Related**: [ADR 0041](../decisions/0041-android-hub-project-name.md)

## Contract

Use Android Hub/android-hub for project, crate and app identifiers and product commands. Preserve Android guest identities/data and legacy Java overrides. Directory rename retains the VM compatibility alias. Backup identity accepts only aliases to the same canonical SDK/AVD. No guest/UI launch for verification.

## Verify

```bash
bash scripts/check.sh
cargo test --locked --manifest-path tools/metrics/Cargo.toml
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Compare third-party lock entries before/after; verify protected runtime metadata hashes and new bundle identifier. After rename, verify the old alias and new directory resolve to the same inode and that registered absolute AVD paths still resolve. Remove task temp/build outputs at handoff.

## Evidence

All Verify commands passed: 123 workspace Rust tests, 2 metrics tests and 22 Python tests. Project renamed with compatibility alias; runtime metadata hashes/inode and old AVD paths verified, main app rebuilt/signed, outputs cleaned. See [report](../reports/project-rename-20261116.md) and [path evidence](../reports/project-rename-20261116.json).
