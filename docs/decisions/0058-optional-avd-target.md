# 0058 — Accept an omitted AVD target with verified image identity

- **Status**: Accepted
- **Date**: 2026-11-19
- **Deciders**: Android Hub maintainer
- **Supersedes**: none; refines [ADR 0056](0056-immutable-android-image-per-device.md)

## Context

A fresh-machine v0.1.0 report contains a Google Play ARM64 API 36 `config.ini`
with `image.sysdir.1`, `abi.type` and `tag.id`, but no `target`. Hub currently
rejects the absent redundant field as a conflicting Android version. The exact
upstream tool behavior that omitted it has not been reproduced locally.

## Decision

Require the supported SDK-relative `image.sysdir.1` path to identify the version.
Treat `target` as optional; when present, it must exactly match that version's
`android-API` value. Empty, malformed or conflicting values remain errors.
Keep existing ABI/flavor, SDK metadata, revision, dependency and payload identity
checks. Do not synthesize or rewrite `target` in existing AVDs.

## Consequences

Creation and subsequent start/backup/restore checks accept a verified image even
when the redundant target field is absent. No registry schema or image identity
changes. Previously incomplete creation journals remain incomplete; the fix does
not adopt or delete their files. Users can create a new device after updating.

Regression coverage exercises all five supported versions without `target`,
backup/restore validation, contradictory metadata and modified image payloads.
This does not qualify the entire fresh-machine setup or guest boot flow.

## Alternatives considered

- Force `target` into every new AVD: unnecessary mutation and does not address
  reading existing metadata or backups.
- Skip image validation: could permit the wrong version/ABI or changed image;
  the immutable image contract must remain enforced.

## Verification

The new regression test failed with the reported conflict before the fix, then
passed for Android 12–16. Infrastructure tests: 140 passed, 25 ignored (host/guest
integration tests); Clippy with warnings denied, formatting and documentation
checks passed. Backup tests require host process access and were rerun outside
the sandbox after its access restriction. No Hub or Android guest was started.
The reporter's machine and full SDK installation flow have not been retested.
