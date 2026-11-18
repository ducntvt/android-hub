# US-027 — EN/VI, button styling and Android image version

- **Status**: implemented
- **Created**: 2026-11-16
- **Owner**: Codex

## Contract

- EN/VI controls switch interface labels immediately and persist per data root; device names, paths, license originals and raw diagnostics remain intact.
- Consistent primary/secondary/danger and hover/pressed/disabled button styles.
- Fixed square centered add button.
- Selected device and live cards show version from configured AVD target; missing metadata says unavailable, not a guessed release.
- No guest restart, migration or account change.

## Verify

cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'

Native QA: switch both languages, inspect sidebar/settings and plus alignment; verify saved preference with the load/save regression, then close QA and remove temporary artifacts. Native relaunch was not repeated due tool latency; see evidence.

## Evidence

[Verification and limits](../reports/ui-refresh/verification.md): 116 tests passed / 20 opt-in ignored; fmt, clippy, build, strict signing and native EN/VI settings smoke passed. Original external diagnostics/license text remain unchanged.
