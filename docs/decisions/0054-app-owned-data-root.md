# 0054 — App-owned data root after authorized reset

- **Status**: Accepted
- **Date**: 2026-11-18
- **Supersedes**: legacy root policy in ADR 0017 and runtime retention in ADR 0053

## Context

User permits deleting the current experimental runtime and recreating devices.
Existing SDK/image and Java are outside the project. Keeping the source checkout
as default storage causes data to be mistaken for experiments/cache.

## Decision

Default to `~/Library/Application Support/Android Hub` regardless of the build
checkout. `ANDROID_HUB_ROOT` continues to override the data root for QA, but now
means that directory directly, with no `experiments/p0/runtime` suffix. Backend,
preferences, diagnostics and QA use the same root. New initialization writes an
empty registry and never imports P0 guest IDs. Do not copy/relocate old AVDs or
backup fingerprints. Preserve valid external SDK/Java settings and UI preferences.

Move the dependency-free metrics package to `tools/metrics`, retaining its own
manifest/lockfile and explicit shared checks. Remove the retired P0 emulator
setup/boot/probe/launcher scripts and one-time VM path migration script/tests.
Keep metrics collectors, historical evidence/verifiers and required build inputs.

## Alternatives and consequences

A full AVD migration is unnecessary for this user's requested fresh start and
would need separate compatibility/rollback work. Keeping a compatibility symlink
would leave the old source data contract active. This reset is deliberate and
irreversible; future data moves still need a proper migration. Older binaries/QA
roots are not automatically imported. No SDK download, guest boot or sign-in is
part of this change. The main bundle is rebuilt before the old runtime is removed.
