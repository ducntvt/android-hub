# Retire experiments — 2026-11-18

[US-045](../stories/US-045-retire-experiments-runtime.md),
[ADR 0054](../decisions/0054-app-owned-data-root.md).

User explicitly authorized deleting `experiments/p0/runtime` and recreating/signing
into devices again. This is a fresh start, not an AVD migration. The old tree used
about 5.8 GiB allocated space (not a guarantee of APFS space reclaimed).

## Source changes

- Moved standalone metrics package/lockfile to `tools/metrics`. Updated workspace
  exclusion, collector default binary path, checks and current instructions.
- Desktop defaults directly to `~/Library/Application Support/Android Hub`.
  `ANDROID_HUB_ROOT` now means an explicit data root without a P0 suffix.
- Backend, theme/language preferences and data-location UI share that root.
  Registry initialization starts empty; it never imports P0 IDs implicitly.
- Diagnostic examples require an explicit absolute `ANDROID_HUB_ROOT` for QA,
  rather than creating runtime files in the current source directory.
- Removed obsolete `p0-emulator-{setup,boot}.sh`, `p0-emulator-{app,probe}.py`,
  `migrate-project-path.py` and its tests. Other metrics/evidence tools remain used.
- Updated docs and marked retired path policies/commands as historical.

## Reset boundary

Completed: quit Hub normally, verified no Hub/emulator process, and held the old
runtime lock during deletion. Removed the complete experiments tree, including
old AVDs, backups, receipts and launchers/logs. Preserved external SDK/image/Java
and copied valid SDK/Java choices plus language/theme preferences to the new root.
No profiles or disks were imported.

Application Support already contained a 4 KiB fallback QA tree with only an empty
registry and lock. Verified that exact state and removed the obsolete nested tree; no unexpected
existing data was overwritten.

## Verification

All shared checks passed: 207 app Rust tests (88 desktop, 4 domain, 115
infrastructure), 22 ignored integration tests; 2 metrics tests; 23 Python tests
(three tests for the retired migration script were removed). Formatting, Clippy
with warnings denied and shell syntax passed. Main bundle build/ad-hoc signing
and `codesign --verify --deep --strict` passed. Existing transitive `block 0.1.6`
future-compatibility notice remains; dependencies were not upgraded.

Opened the rebuilt main app: process PID 72128 stayed running and created an
empty `hub-profiles.json` directly in Application Support. SDK/Java paths still
exist; no experiments directory was recreated in source or the data root. No
emulator was launched. Quit the app normally after this check.

Test changes cover
source-independent root selection, explicit QA root, empty initialization and
preservation of an existing registry. Download cancellation/rejected consent tests
now assert that the pre-existing root remains empty, instead of expecting a nested
runtime directory to be absent. No guest boot or sign-in is part of this task.

## Cleanup

Removed workspace/metrics targets (about 2.8 GiB allocated), task check/build logs,
the temporary reset script, Finder metadata and the two empty download fixtures
left by the first failing test run. Verified no `experiments`/target directories
remain, metrics source and main executable exist, and no Hub/emulator is running.
No unrelated temporary directory, external SDK cache or historical evidence was
removed. No guest was created; use **+** in the main app to create devices again.
