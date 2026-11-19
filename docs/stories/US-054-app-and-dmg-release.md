# US-054 — Publish app ZIP and DMG releases

- **Status**: implemented
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
- [x] Preserve the main app/data; close any QA and remove disposable packaging/build outputs.
- [x] Tag workflow checks source first, packages the tag, uploads assets to a draft and publishes only after upload success.
- [x] Document ad-hoc/non-notarized status, external SDK requirements and dependency notices.
- [x] Publish v0.1.0 from main and verify its tag, assets and release visibility.

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
passed. A QA wrapper using the release executable opened the empty workspace
and exited normally; process/app inventory confirmed shutdown. No guest was
created or started, and the main app stayed open. 32 Python tests, shell syntax,
documentation and whitespace checks passed. Final assets remain under `dist/`.

[PR #3](https://github.com/ducntvt/android-hub/pull/3) passed shared CI and merged.
Annotated tag `v0.1.0` points to main commit
`1746eb59df15f8bb37b0b88e058ea43f60d61fda`.
[Release workflow](https://github.com/ducntvt/android-hub/actions/runs/44455312356)
passed validation, macOS checks/build, Android helper reproducibility and release
packaging/publishing. [v0.1.0](https://github.com/ducntvt/android-hub/releases/tag/v0.1.0)
was published on 2026-11-19 (not a draft or prerelease) with all four assets uploaded.

Downloaded assets were checked against SHA256SUMS. Extracted app signature, ARM64
architecture, version 0.1.0, macOS 15.0 target and dependency notices matched;
`hdiutil verify` also passed on the downloaded DMG. Published asset SHA-256:

- ZIP: `3a4adfb7bb1f0536fb3bd48d6449a020a497346ebfbcea20405b0b24b66e5bf7`.
- DMG: `da38059be4ea3af5c78fcec6df89f425a664c835431dda6e99f9737cc52c6a31`.

Local `dist/v0.1.0-macos-arm64/` now holds these published assets and the extracted
app. Disposable target, QA wrapper/data, license-fetch cache and task files were
removed; the main app and personal SDK/device data were preserved. The release
runner cleaned its own staging/build outputs. Main was synchronized into develop.
The post-publication evidence update lives on develop; the published tag is unchanged.
