# US-029 — Stop Android devices when closing Hub

- **Status**: implemented
- **Owner**: Codex
- **Created**: 2026-11-16
- **Related**: [ADR 0039](../decisions/0039-stop-engines-on-quit.md)

## Contract

Close/Quit stops registered running engines before exiting. Wait for existing mutations, block new work, continue stopping other engines after one fails, and retain Hub with Retry/Return if shutdown fails. EN/VI shutdown feedback. No force-kill or global adb kill-server.

## Out of scope

Crash/Force Quit watchdog; restarting or stopping personal guests during verification.

## Affected areas

Desktop native quit hook, update/view/i18n; infrastructure stop orchestration.

## Verify

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

## Evidence

All Verify commands passed: 121 tests passed, 20 existing opt-in integration tests ignored; workspace/all-target clippy with -D warnings, format check, build and strict codesign verification succeeded. Logs: [test](../reports/stop-on-quit/test.log), [clippy](../reports/stop-on-quit/clippy.log), [build](../reports/stop-on-quit/build.log).

State regressions cover in-flight initialization/backup/SDK changes, duplicate close, blocked new work, failed shutdown retry/return. Backend orchestration regression confirms only registered running IDs are attempted and a failed first stop does not skip the second.

Native QA used `/tmp/android-hub-quit-qa-20261116` with an empty separate registry. Cmd+Q produced the UI tool's `App quit` result; reopening created a new QA process. On reopening, the UI tool repeatedly returned `cgWindowNotFound`, so native close-button verification remains unconfirmed. The second QA process was identified by exact path and terminated during cleanup. No real Android guest was started/stopped, so end-to-end shutdown of running guests is not claimed in this verification.

Disposable QA app/data, diagnostic sample, temporary logs and target were removed. Main app and personal runtime retained. Force Quit/crash bypass the normal cleanup path.
