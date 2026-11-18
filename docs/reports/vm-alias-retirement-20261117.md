# Retire VM alias — 2026-11-17

## Completed

The `/Users/ducnt/Desktop/Projects/VM` symlink is removed. The canonical project and app remain under `/Users/ducnt/Desktop/Projects/Android Hub`.

Migrated 12 INI/launcher files: active AVD registration and generated hardware config; registrations/configs in removed-device archives; five legacy generated launchers. Paths containing the new directory's space are quoted in shell launchers. File inode/mode preserved during metadata edits, so removed-device journal identities remain valid. No guest disk content was edited by the migration tool.

Original metadata and the committed receipt remain at `experiments/p0/runtime/path-migrations/1794880582977064000/`. These are recovery data, not test cache. Historical reports and metadata originals intentionally retain the old path as evidence; they are not active app inputs.

Nine QCOW2 headers use relative backing filenames, each resolving to its neighboring base image. No disk rebase was needed. This runtime has no backup JSON referring to the old root. Current INI/launcher metadata and the rebuilt main executable contain no old absolute project root.

## Launcher correction and lifecycle evidence

LaunchServices initially returned success without starting the engine. Captured stderr identified `Operation not permitted` while the helper tried to write its log on Desktop. Backend now spawns the existing headless shell launcher as a child of Hub, with null standard streams and a waiter thread to reap it; no system privacy settings were changed.

An initial short-lived probe exited immediately after spawn, and its child was subsequently gone. The final probe kept the parent alive like Hub does throughout the lifecycle. With VM already absent, it started Android 1 (`hub-1794829194704`, PID 709), observed `sys.boot_completed=1`, then stopped that exact PID using Backend. Final process inspection confirmed no Hub, emulator or probe left running. No game/account input was performed. Native Hub UI was not reopened during this bounded check.

The emulator regenerated the active hardware-qemu.ini on normal boot; its inode consequently changed after the pre-boot identity check. Archived registration inodes remained intact. Removed catalog after the drill: zero skipped entries; Android A and B still available for undo. The older third archive retains its pre-existing port conflict with Android 1.

## Verification

- Migration tests: shell path quoting, read-only planning, inode/mode preservation and rollback after an injected failure.
- Full check script: 123 Rust tests passed, 20 existing opt-in tests ignored; 25 Python tests passed; fmt, all-target clippy -D warnings and shell syntax passed.
- Main app rebuilt from canonical directory and strict codesign verification passed.
- Old alias absent; no old absolute root in current runtime INIs/launchers or main executable.
- [Machine-readable lifecycle/path checks](vm-alias-retirement-20261117.json).

Temporary probe source was removed. Build target and task scratch directory were removed after checks. Main app, SDK, Android runtime and small migration metadata originals retained.
