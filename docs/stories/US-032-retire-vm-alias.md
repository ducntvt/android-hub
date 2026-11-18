# US-032 — Migrate runtime paths and remove VM alias

> Historical: the one-time migration tool and its tests were retired in US-045 after the user authorized resetting the old runtime. Original verification commands below describe that earlier revision.

- **Status**: implemented
- **Created**: 2026-11-17
- **Related**: [ADR 0042](../decisions/0042-retire-vm-path-alias.md)

## Contract

Update current and removed AVD metadata and historical generated launchers, preserving registration inode identities. Rebuild from Android Hub, remove only the VM symlink, and verify one existing Android device can boot/stop without it. Retain migration rollback metadata. No app/game input or unrelated guest starts.

## Verify

```bash
bash scripts/check.sh
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Additional migration checks: old absolute root absent from app/current metadata, relative QCOW backing paths resolve, removed catalog has no newly unavailable devices, exact-PID boot/stop drill with VM absent, no remaining QA/test engine, disposable build/temp cleanup.

## Evidence

Verify commands passed: 123 Rust tests, 25 Python tests, clippy, format and signed build. Headless Android 1 boot/stop passed with VM absent; no engine/probe remains. Metadata migration receipts retained, temporary probe/build outputs cleaned. See [report](../reports/vm-alias-retirement-20261117.md).
