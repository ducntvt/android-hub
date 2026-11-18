# US-045 — Retire experiments and start with app-owned data

- **Status**: implemented
- **Created**: 2026-11-18
- **Related**: [ADR 0054](../decisions/0054-app-owned-data-root.md)

## Context and authorization

User explicitly allows deletion of `experiments/p0/runtime`, accepting login again,
and asks to move still-used components to appropriate locations. This is an
intentional fresh start, not a migration of existing Android devices. Old AVDs,
accounts, backups, logs and recovery receipts are deleted after stopping Hub/engines.
Preserve installed external SDK/image/Java and reusable source.

## Acceptance

- [x] Default app data is directly under Application Support/Android Hub; QA override remains isolated.
- [x] Metrics lives under tools/metrics; retired P0 bootstrap/path migration scripts removed.
- [x] Settings and backend agree on one data root; no active code recreates experiments.
- [x] App and tests build/pass; fresh-root initialization verified without booting a guest.
- [x] Old runtime removed after process/lock checks; SDK/Java/preferences retained where valid.
- [x] Docs and main app updated; task targets/temp files cleaned.

## Verify

```bash
bash scripts/check.sh
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Also verify stopped process inventory before removal, old folder absent and new
registry empty after fresh-root initialization. Do not start a game or log in.

## Evidence

All Verify commands passed. 207 app Rust tests + 2 metrics tests; 23 Python tests.
22 integration tests remain ignored. Rebuilt/signed main app initialized an empty
registry in Application Support; quit completed and process inventory is clear.
Old experiments and the verified empty fallback tree were removed under the
authorized reset. External SDK/image/Java and preferences retained. Build targets,
task logs/script and two empty failed-test fixtures removed after verification.
See [report](../reports/experiments-retirement-20261118.md).
