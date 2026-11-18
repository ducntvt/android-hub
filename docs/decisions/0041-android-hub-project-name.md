# 0041 — Standardize the project name as Android Hub

- **Status**: Accepted
- **Date**: 2026-11-16
- **Supersedes**: Old project/package naming; ADR 0009 path comparison accepts resolving aliases

## Context

The user requests Android Hub as the project name and related identifiers. The visible app already has this name, but the directory is VM, the main bundle identifier contains personalvm, and internal packages use abbreviated hub names. Existing AVD registrations and launcher scripts contain absolute paths through VM.

## Decision

Use Android Hub as the display/directory name and android-hub as its machine-readable stem. Crates become android-hub, android-hub-domain and android-hub-infrastructure. The isolated metrics package becomes android-hub-metrics; its diagnostic engine-metrics binary retains its task name. Main bundle identity becomes local.androidhub.desktop; QA/engine/Java identities already use local.androidhub. Rename product build/QA/metrics commands and their Python tests, and update source imports and active documentation.

ANDROID_HUB_JAVA_HOME takes priority over the legacy P0_JAVA_HOME alias. Both override saved settings and disable in-app Java changes. QA clears both variables. Research-specific P0 names and existing device IDs/data filenames remain stable identifiers, not product branding.

Rename the physical project directory to Android Hub and leave VM as a relative compatibility symlink to it. Perform after checks/build, with no Hub/guest processes and under the runtime lock; refuse an occupied destination. Retain inode and metadata hashes and verify both paths resolve to the same data. Roll back the directory rename if alias creation or immediate verification fails.

A backup's recorded SDK/AVD absolute paths may resolve through that alias; accept them only if their canonical paths match the current SDK/AVD. Keep identity, port, image and payload hash checks. Never rewrite backup manifests or guest disks. Missing/relative/retargeted paths remain rejected. This supports a directory rename retaining aliases, not copying backups to unrelated machines.

## Consequences

Finder/project users see the new name. Old absolute paths remain usable; VM is an intentional compatibility link, not disposable cache. Moving the project again or removing the link still requires a separate runtime migration. The main bundle's changed identity may appear as a new app to macOS; this app's data resides outside bundle-ID-specific storage. Do not rewrite historical report evidence or old generated launchers just to remove old names.

## Alias retirement

ADR 0042 migrates the local runtime metadata and rebuilt app, then removes the VM alias after verification. This document records the earlier compatibility phase.
