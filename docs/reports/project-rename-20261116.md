# Android Hub project rename — 2026-11-16

## Result

Physical project directory: `/Users/ducnt/Desktop/Projects/Android Hub`.
Compatibility symlink: `/Users/ducnt/Desktop/Projects/VM` → `Android Hub`.
Open future editor/workspace sessions using the canonical Android Hub directory. The old writable workspace path is now a symlink, so this existing execution session needs explicit access to the new path for final cleanup; no security configuration was changed.

Product name remains Android Hub; main bundle identifier is now `local.androidhub.desktop`, with both CFBundleName and CFBundleDisplayName set. The main app was rebuilt and signed.

## Names updated

- Cargo: android-hub, android-hub-domain, android-hub-infrastructure; isolated metrics package android-hub-metrics. Updated Rust imports and workspace/local dependency references. Third-party dependency versions/checksums are unchanged.
- Commands: build-android-hub.sh, build-android-hub-qa.py, collect-android-hub-metrics.py; corresponding Python tests renamed. Updated live source/verification documentation references.
- Java override: ANDROID_HUB_JAVA_HOME takes precedence, with P0_JAVA_HOME retained as a compatibility fallback. UI/backend guards honor both; QA launcher clears both.
- Removed personalvm from newly generated bundle IDs. Existing runtime-generated launchers and persisted device IDs/files remain unchanged.

## Data compatibility

Rename executed under the existing runtime lock after confirming no main Hub or registered emulator was running. Occupied destinations are refused. Directory inode retained and three protected metadata files (registry, AVD registration and config.ini) retained their SHA-256. One registered absolute AVD path was checked and still resolves. See [machine-readable evidence](project-rename-20261116.json).

No guest was started/stopped, no disks copied, and no backup manifest rewritten. VM is a compatibility link, not a second data copy or cache. Keep it until a separate runtime migration removes all persisted path dependencies.

Data-root resolution follows an existing project symlink to display/use the canonical new root. Backup verification resolves recorded absolute SDK/AVD aliases and still requires them to match the exact current locations, plus existing profile/port/image/payload checks. Missing or retargeted aliases are rejected. This does not authorize copying backups between unrelated machines.

## Verification

- bash scripts/check.sh: 123 Rust tests passed, 20 pre-existing opt-in tests ignored; 22 Python tests passed; fmt, all-target clippy -D warnings and shell syntax passed.
- Isolated android-hub-metrics crate: 2 tests passed.
- New tests cover root selection through the renamed directory alias and backup identity through SDK/AVD aliases, including missing/retargeted links. QA isolation test covers both Java variable names.
- bash scripts/build-android-hub.sh and strict codesign verification passed. Bundle name/identifier verified from plist.
- cargo metadata --locked --offline --no-deps after rename identifies the canonical Android Hub workspace and all three new package names.
- Protected metadata hashes were rechecked after rename and before cleanup.

Removed the workspace target, metrics target, and task scratch under /tmp/android-hub-rename-20261116 after verification. No QA app was launched. Preserved main app, SDK and all runtime data. Historical report paths/names remain evidence from their original revision.
