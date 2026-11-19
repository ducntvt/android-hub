# Versions and releases

Android Hub releases in private `ducntvt/android-hub` contain source, release notes,
an Apple Silicon app ZIP, a DMG, `INSTALL.txt` and `SHA256SUMS`. Repository access is
required. SDKs, Java, Android images, devices and accounts are never bundled.

## Version policy

Use `MAJOR.MINOR.PATCH`, optionally a prerelease such as `-beta.1`; build metadata
(`+...`) is not supported. During `0.x`, increment patch for compatible fixes and
minor for feature batches or incompatible changes. Never move or reuse a published
tag. Describe data/image compatibility explicitly.

`Cargo.toml` workspace version is canonical; `Cargo.lock` records the workspace
package versions. Both app plist version fields use the numeric Cargo version.
Prerelease labels remain in Cargo, tag and asset names. The independent metrics
tool is versioned separately. `CHANGELOG.md` supplies release notes.

## Packaging

```bash
bash scripts/package-release.sh
```

Requires an Apple Silicon Mac, Rust pinned by `rust-toolchain.toml`, Xcode command
line tools and Python 3.11+. It builds `cargo build --release --locked`, targets
macOS 15.0 and places verified outputs in `dist/v<version>-macos-arm64/`:

- `Android Hub.app`: local app bundle.
- `Android-Hub-v<version>-macos-arm64.app.zip`: the app bundle for downloading.
- `Android-Hub-v<version>-macos-arm64.dmg`: same app, Applications shortcut and notes.
- `INSTALL.txt` and `SHA256SUMS`.

Existing output directories are refused; move a previous package aside deliberately
before rebuilding the same version. Temporary staging is removed on exit. Packaging
checks architecture, bundle version/signature, system-only dynamic dependencies,
DMG integrity, mounted app and extracted ZIP contents, then verifies checksums.
It does not launch Hub or boot guests. Keep final `dist/` deliverables; remove build
cache after verification. The development build command remains
`bash scripts/build-android-hub.sh`; add `--release` for a release-profile local app.

These personal builds are **ad-hoc signed, not Developer ID signed or notarized**.
Gatekeeper may block downloaded copies. Do not represent them as Apple-approved
or ask users to disable system-wide protection. Users may build from source.
The minimum deployment target is 15.0, not certification of every macOS release.
Fresh-machine SDK installation and game performance remain separately qualified.

## Prepare on develop

1. Synchronize with `main`, finish the changes and set the intended Cargo version.
   If changing version, refresh workspace entries with
   `cargo metadata --no-deps --format-version 1` and inspect the lockfile diff.
2. Move release notes from Unreleased into exactly one nonempty dated section,
   e.g. `## [0.1.0] - 2026-11-19`. Keep Unreleased for later changes. Future dates,
   duplicate sections, empty notes and a Cargo/tag mismatch block publishing.
3. Run applicable local checks and package validation; review architecture,
   dependencies, notices and compatibility. Use disposable data for any UI smoke.
4. Open a PR to `main`, wait for CI and merge preserving history. The workflow
   includes macOS checks/build and Android helper reproducibility.

## Publish by tag

Tag the intended clean commit already merged to `main`. Publishing requires an
explicit user request; routine implementation does not create a tag automatically.

```bash
git switch main
git pull --ff-only origin main
git status --short
# Substitute the intended, unused Cargo version.
git tag -a v0.1.0 -m 'Android Hub v0.1.0'
python3 scripts/release.py validate v0.1.0
git push origin v0.1.0
```

The [release workflow](../.github/workflows/release.yml) validates the tag/Cargo
match, main ancestry and dated notes, then runs the shared CI workflow. Only after
checks pass does its macOS publish job build and verify the release assets. It
creates a draft with ZIP/DMG/checksums/install notes, then publishes that draft.
Prerelease tags are marked prerelease and not latest. No app store submission,
Apple signing credentials or PAT is needed; publishing uses `GITHUB_TOKEN` with
`contents: write` only in that job. Actions are pinned to commit SHAs.

The uploaded files are built from the tagged source on CI; a local build validates
the packaging path but can differ byte-for-byte due to SDK/toolchain host details,
signing and container timestamps. Use the checksums attached to that release.

## Failure, retry and cleanup

Validation/check/build failure does not publish a release. Failed uploads may leave
a draft; inspect/delete that incomplete draft explicitly before rerunning the same
unchanged tag. The workflow refuses to overwrite an existing release, including a
draft. Never force-move a published tag: source fixes need a new version and tag.
The publish job removes its disposable app/build directories even after failure.

After publication, merge `main` back into `develop`. Hotfixes also merge back.
Checking out an old source tag does not roll back Android disks, registry, SDKs or
images. Back up stopped devices before incompatible changes. GitHub branch/tag
protection depends on the account plan; workflows alone do not enforce it.
