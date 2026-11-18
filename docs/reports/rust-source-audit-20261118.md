# Rust source audit — 2026-11-18

Scope: source organization, naming/style, retired engine support, documentation
and disposable outputs. [US-044](../stories/US-044-rust-source-consistency.md),
[ADR 0053](../decisions/0053-rust-module-conventions.md).

## Findings and changes

The existing three-crate Cargo workspace already has useful dependency boundaries.
Rust does not prescribe an application architecture; adding application/CLI crates
would not improve the current single-engine app. Keep `domain`, `infrastructure`
and `desktop`, root Cargo metadata/lockfile, resource ownership and data paths.

| Area | Change |
| --- | --- |
| Module roots | `desktop/src/app/mod.rs` → `app.rs`; `infrastructure/src/sdk/mod.rs` → `sdk.rs`; child directories retained |
| Desktop state | Events moved to `app/message.rs`; bootstrap/subscriptions to `app/runtime.rs`; root shrank from 799 to 549 lines |
| Update handling | SDK events moved to `app/sdk.rs`, retaining guards and outer modal/shutdown handling; `update.rs` reduced from 1392 to 1091 lines |
| View composition | Device settings moved to `app/ui/settings.rs`; shared presentation helpers remain in `ui.rs`, reduced from 655 to 441 lines |
| Infrastructure | Snapshot parsing/version metadata, shutdown aggregation and facade tests moved to private modules; public exports unchanged; `lib.rs` reduced from 845 to 495 lines |
| Diagnostic example | `display_probe.rs` → `display-probe.rs`, matching Cargo example naming convention |
| Retired AVO support | Removed `avo-vmm` from metrics executable allowlist; current source/scripts contain no AVO reference or dependency |
| Style/checks | Default rustfmt and warning-denying Clippy; shared check now includes standalone metrics fmt/tests/clippy |
| Docs | Updated development map, architecture, setup, README, plan and indexes; marked stale task/MVP entries as historical and retired the AVO-first proposal |

File length reductions reflect responsibility moves, not deleted features. Shared
SDK setup constraints, data locking, async result guards and public Backend methods
remain. No dependency version, persisted schema, runtime path or guest behavior was
intentionally changed. No global lint suppression or custom formatter introduced.

AVO checkout/build/boot tools were already absent. Historical P0 reports and ADRs
remain as evidence; the current plan no longer instructs developers to build AVO.
SDK protocol/license provenance and required manifest/DEX inputs remain intact.
The `flate2` dependency enables ZIP Deflate via feature unification (ADR 0025);
absence of a direct import is not evidence that it is unused.

## Verification

- `cargo check --workspace --all-targets --locked`: passed, including renamed example.
- `bash scripts/check.sh`: passed. App: 206 tests (88 desktop, 4 domain, 114 infrastructure),
  22 explicitly ignored; standalone metrics: 2 tests; Python: 26 tests. Workspace and
  metrics formatting/Clippy, plus shell syntax checks, passed.
- `cargo metadata --offline --locked --no-deps --format-version 1`: passed; three app
  members, two infrastructure examples, expected build script, no AVO dependency.
- `bash scripts/build-android-hub.sh`: passed; main bundle rebuilt and signed ad-hoc.
- `codesign --verify --deep --strict 'Android Hub.app'`: passed.
- Local links in updated documentation: valid.

No GUI/guest boot, SDK download or benchmark was needed for the source moves.
Ignored integration tests were not run. This audit does not claim new gameplay,
performance, accessibility or clean-machine installation evidence. Cargo still
reports the existing transitive `block 0.1.6` future-compatibility notice; current
compile/tests/Clippy pass. Dependency upgrades are outside this refactor.

## Cleanup

Removed after successful packaging (about 2.7 GiB of local build outputs): workspace `target/`, metrics
`experiments/p0/control/target/`, `/tmp/android-hub-scaffold-{check,verify,build}-20261118.log`
and `/tmp/android-hub-scaffold-metadata-20261118.json`. Removed Finder `.DS_Store` metadata from the project root, `crates/`, `docs/`
and `docs/reports/`. Cleanup verified both target directories absent and the main
executable/runtime registry present. No QA app was opened by this task. Preserve the main
bundle, real runtime/SDK/AVDs, backups, receipts and global Cargo caches.
