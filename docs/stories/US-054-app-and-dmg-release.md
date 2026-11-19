# US-054 — Publish app ZIP and DMG releases

- **Status**: in_progress
- **Owner**: Android Hub maintainer
- **Created**: 2026-11-19
- **Risk lane**: normal
- **Related**: [ADR 0057](../decisions/0057-ad-hoc-app-distribution.md), [Releasing](../RELEASING.md)

## Context

The owner wants a downloadable app and DMG in addition to source. Release assets
must correspond to the version tag and contain no personal data or Android SDKs.

## Contract (acceptance criteria)

- [x] Build a release-profile ARM64 app with matching version and explicit macOS target.
- [x] Produce verified app ZIP, DMG with Applications shortcut, install notes and SHA-256 checksums.
- [ ] Preserve the main app/data; close any QA and remove disposable packaging/build outputs.
- [ ] Tag workflow checks source first, packages the tag, uploads assets to a draft and publishes only after upload success.
- [x] Document ad-hoc/non-notarized status, external SDK requirements and dependency notices.
- [ ] Publish v0.1.0 from main and verify its tag, assets and release visibility.

## Out of scope

Developer ID, notarization, Intel builds, auto-update, bundling Android/Java,
fresh-machine SDK qualification and additional game testing.

## Affected areas

Build/package scripts, release workflow, changelog, notices and EN/VI documentation.

## Verify

```bash
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts -p 'test_*.py'
PYTHONDONTWRITEBYTECODE=1 python3 scripts/check-docs.py
bash -n scripts/build-android-hub.sh
bash -n scripts/package-release.sh
bash scripts/package-release.sh
git diff --check
```

The packaging script mounts/detaches its own image and verifies the contained app.
PR/tag CI supplies the full Rust checks/build. Confirm GitHub release assets and
checksums after publication. No guest matrix is required for a packaging change.

## Evidence

Local ARM64 release build passed on macOS 27 after disabling build-time macro
stripping for Rust issue #157750. Verified bundle version/signature and system-only
dylibs; ZIP extraction, DMG integrity/mount/Applications link and SHA-256 checks
passed. No Hub window or guest was launched. 32 Python tests, shell syntax,
documentation and whitespace checks passed. Final assets remain under `dist/`.
PR/tag CI and publication confirmation are pending.
