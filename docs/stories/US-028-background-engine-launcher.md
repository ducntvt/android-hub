# US-028 — Hide per-device engine launcher from Dock

- **Status**: implemented
- **Created**: 2026-11-16
- **Owner**: Codex

## Contract

Generate per-device agent bundles, launch without foreground activation, and remove Hub-hub naming from newly generated helpers. Preserve running guests, main Hub foreground behavior and authenticated headless transport. Existing running foreground helpers need a future explicit stop/start to pick up the change.

## Verify

cargo fmt --all -- --check
cargo test --locked -p android-hub-infrastructure engine_launcher_is_a_background_agent_with_isolated_identity
cargo clippy --workspace --all-targets --locked -- -D warnings
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'

Do not restart personal guests for testing. Validate generated plist with Apple's plutil and launcher syntax with bash -n; actual Dock behavior on the installed emulator remains a next-start observation.

## Evidence

Targeted regression passed: generated plist parsed with macOS plutil, LSUIElement true, separate agent identifier/path per port, shell syntax valid, authenticated headless flags unchanged and legacy launcher marker untouched. fmt check, workspace/all-target clippy -D warnings, app packaging and strict signing passed. Logs: ../reports/background-launcher/{test,clippy,build}.log.

No real guest was started/stopped and no Dock observation is claimed for the installed emulator. This change uses Apple's documented agent-app behavior; a currently running legacy helper needs explicit stop/start from the reopened updated Hub. Old bundles are left intact rather than deleting files beneath running processes. Build target cleaned after verification; main app retained.
