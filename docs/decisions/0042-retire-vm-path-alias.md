# 0042 — Retire the VM path alias after metadata migration

- **Status**: Accepted
- **Date**: 2026-11-17
- **Supersedes**: ADR 0041 requirement to retain this machine's VM symlink; ADR 0038 LaunchServices invocation

## Decision

Migrate the stopped local runtime's INI paths and generated shell launchers to the canonical Android Hub directory. Save original metadata and a prepared/committed receipt under runtime/path-migrations before writes. Preserve file inodes and modes: removal journals identify archived registration.ini files by device/inode. Roll back completed writes on errors; retain original metadata for recovery. Refuse unreviewed JSON path references, concurrent changes, special files or active engines. Do not replace strings in guest disks, backup payloads or historical evidence.

The inspected nine QCOW2 overlays use relative backing filenames resolving beside each overlay, so no disk rebase is required. This runtime has no backup manifest requiring fingerprint migration. Rebuild the app from the canonical source root and verify the previous absolute root is absent from its executable before unlinking VM. Only the validated symlink is removed, not any directory.

A post-migration startup check exposed LaunchServices denying a new helper's Desktop log access. Start the existing generated headless engine script as a child of Hub via /bin/bash instead of opening it as an independent app. Retain authenticated headless flags and lifecycle identity checks; reap the child on a background thread. Hub's normal quit still stops registered engines. No macOS privacy settings or permissions are modified. This avoids separate helper app privacy identities and foreground activation.

## Scope and limits

This is the inspected local project's path migration, not arbitrary AVD relocation or a generic backup migration. P0/removed-device IDs, persisted registry and SDK choice remain unchanged. Keep migration originals as recovery data, not disposable test cache. Existing alias support in data-root/backup logic remains useful for other installations; it is no longer required on this machine.

A headless backend boot/stop verifies this machine's migrated AVD without VM. It does not substitute for native UI/gameplay regression or claim Android account/app state beyond a successful normal boot.
