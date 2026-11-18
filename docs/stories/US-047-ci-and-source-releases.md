# US-047 — CI and source releases

- **Status**: implemented
- **Owner**: Android Hub maintainer
- **Created**: 2026-11-18
- **Risk lane**: normal
- **Related**: [ADR 0055](../decisions/0055-source-release-automation.md)

## Context

The private repository needs repeatable checks and versioned source releases.
The owner wants tag-based publishing without distributing an app or SDK.

## Contract (acceptance criteria)

- [x] PR/branch/manual CI checks Rust, Python, docs and shell scripts on macOS ARM64.
- [x] Packaging verifies ad-hoc signature and Cargo-derived bundle versions.
- [x] Tag releases reuse CI and reject mismatched versions, unmerged commits and missing notes.
- [x] Prereleases are marked correctly; existing releases cannot be overwritten.
- [x] Toolchain and actions are pinned; only publishing has write permissions.
- [x] Updated docs explain release preparation, failure recovery and remaining manual checks.

## Out of scope

Publishing an actual tag/release, binary distribution, signing accounts, guest boot,
billing changes, remote branch protection and runtime data changes.

## Affected areas

`.github/workflows`, packaging/release/check scripts, toolchain pin and maintained docs.

## Verify

```bash
bash scripts/check.sh
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
python3 scripts/release.py verify-bundle 'Android Hub.app/Contents/Info.plist'
git diff --check
```

Also validate both workflows with actionlint 1.7.12 before handoff.

## Evidence

- `scripts/check.sh`: passed; 207 app Rust tests, 2 metrics tests and 31 Python tests.
  The 22 opt-in Android integration tests remain ignored. Both Clippy checks and
  both rustfmt checks passed; documentation checks passed across 27 maintained files.
- Packaging, strict ad-hoc signature verification, plist lint and Cargo/bundle
  version comparison passed (`0.1.0`).
- actionlint 1.7.12 passed for both workflow files (syntax/expressions; ShellCheck
  was unavailable). `git diff --check` passed.
- Release gate tests use temporary Git repositories, including mismatched tags,
  non-main commits, checkout mismatch, annotated tags, invalid/duplicate/missing
  notes and prerelease versions. Fixtures are automatically removed.
- Existing dependency `block 0.1.6` emits a future-Rust compatibility notice;
  current pinned-toolchain checks pass. No dependency upgrades were included.
- Local build targets and task tools/logs were removed after validation. Main app
  retained; no QA app or guest was launched.
- Remote workflow execution still requires committing/pushing these changes.
  No tag or release was published; release publication itself is not exercised.
