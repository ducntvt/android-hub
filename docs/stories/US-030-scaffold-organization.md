# US-030 — Organize the existing Rust scaffold

- **Status**: implemented
- **Created**: 2026-11-16
- **Risk lane**: normal
- **Related**: [ADR 0040](../decisions/0040-scaffold-organization.md)

## Contract

- Small desktop entry point; state/update/view/tests have explicit homes.
- SDK modules grouped without changing public API or behavior.
- One canonical SDK manifest owned by infrastructure; build independent of reports.
- Shared Cargo metadata/dependencies and one local check command.
- No dependency upgrades, new runtime libraries, runtime migration, app launch or personal guest changes.

## Verify

```bash
bash scripts/check.sh
cargo build --locked -p android-hub
cargo package --list --allow-dirty -p android-hub-infrastructure
```

Additionally compare Cargo.lock, canonical SDK manifest bytes, main app and personal registry hashes with the pre-change baseline. Inspect default and ignored test inventory after module moves. Keep the installed app unchanged: this is a source organization refactor.

## Evidence

All Verify commands passed. 121 Rust and 22 Python tests passed; 20 opt-in Rust tests remain ignored. Module filter inventory and protected hashes verified. See [report](../reports/scaffold-optimization-20261116.md). Temporary source backup/check logs and target removed after verification; main app and runtime preserved.
