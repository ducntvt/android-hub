# Scaffold optimization — 2026-11-16

## Reference and scope

Reviewed iomentum/cargo-scaffold at commit `767d6cb879654fd29e22644ee739c5fc686c2b48`: main.rs, lib.rs, git.rs, helpers.rs, Cargo.toml, rust-toolchain.toml and .scaffold.toml. [Upstream source](https://github.com/iomentum/cargo-scaffold/tree/767d6cb879654fd29e22644ee739c5fc686c2b48).

The repository implements a configurable project generator, not a desktop framework. Its small main delegates to the library; template configuration controls parameters, file exclusion, binary templating opt-out and hooks. Hub adopts a small entry point and explicit configuration/resource boundaries. No generator installation, template generation, upstream code copy or extra dependencies were needed.

## Applied changes

- `desktop/src/main.rs`: 2,093 → 8 lines. Application state/initialization/subscriptions live in `app/mod.rs`; transitions in `app/update.rs`; existing tests in `app/tests.rs`. Existing ui.rs and screen.rs moved under app/ without behavior changes.
- `infrastructure/src/sdk_*.rs` and setup.rs grouped under `src/sdk/`: cache, cache_lock, catalog, download, extract, install, package, plan, probe, registration, setup, workflow. Public crate re-exports and Backend methods retained. Updated crash-worker test filter strings for the module paths.
- Canonical install-plan.json moved byte-for-byte from docs/reports/sdk-installer to infrastructure/resources/sdk-install-plan.json. Build no longer includes docs/reports. Updated current link and recorded the location amendment to ADR 0030.
- Root Cargo.toml owns shared version/edition/publish setting and dependency versions. Members inherit them, retaining their existing feature sets. Internal crates use publish=false; app bundling is unaffected. No Cargo.lock changes.
- `scripts/check.sh` runs format, Rust tests, all-target clippy, Python tests and shell syntax checks. No GUI/VM start, SDK install or opt-in integration execution.
- `docs/DEVELOPMENT.md` documents the current source map, naming, resources, verification and cleanup.

## Verification

- `bash scripts/check.sh`: passed. 121 Rust tests passed, 20 existing opt-in tests ignored; 22 Python tests passed. fmt and all-target clippy -D warnings passed; shell scripts parsed successfully.
- `cargo build --locked -p android-hub`: passed. This build was not copied over the installed main app because behavior is unchanged.
- `cargo package --list --allow-dirty -p hub-infrastructure`: resource manifest and embedded DEX included, docs/runtime excluded. Cargo reports missing publication metadata; these private crates are not configured for crates.io publication.
- Test inventory contains both renamed crash workers. Ignored real guest/SDK integration tests were not run; none are claimed as passed.
- Desktop update and test bodies compared with the original, ignoring formatting and the update method's module visibility: unchanged.
- SHA-256 checks below confirmed no dependency resolution, embedded plan contents, main app or registry changes.

```
Unchanged SHA-256: Android Hub.app/Contents/MacOS/android-hub
Unchanged SHA-256: experiments/p0/runtime/hub-profiles.json
Unchanged SHA-256: Cargo.lock
Unchanged SHA-256: crates/infrastructure/resources/sdk-install-plan.json
Desktop update handlers unchanged except module visibility/formatting
All desktop state test bodies preserved
```

## Cleanup and limits

Removed the task-specific source backup/check logs under /tmp/android-hub-scaffold-20261116 and the 2.3 GiB build target after checks. No QA app or Android guest was opened; the main app, SDKs and all personal runtime remain intact. No Git repository was initialized and no commit was made.

This improves source organization and repeatable development checks, not game FPS. The large update/UI files can be split by feature when those features change; this refactor avoids creating a speculative layer hierarchy. Runtime location and p0 naming remain a separate migration. Earlier scaffold/cleanup reports describe their original snapshots; this report and DEVELOPMENT.md describe the current layout.
