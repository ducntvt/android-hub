# Versions and source releases

Android Hub releases are **source tags plus release notes** in the private
`ducntvt/android-hub` repository. Invited users build locally. The workflows never
upload an app, DMG, SDK/image or device data. GitHub supplies source archives.

## Version policy

Use `MAJOR.MINOR.PATCH`, optionally followed by a prerelease such as `-beta.1`.
Build metadata (`+...`) is not supported by this project's release convention.
During `0.x`, use patch increments for compatible fixes and minor increments for
feature batches or incompatible changes. Document data/SDK compatibility explicitly.
The initial source declares `0.1.0`; it has not been released simply because that
version appears in Cargo. Never move or reuse a published version tag.

| Location | Role |
| --- | --- |
| Root `Cargo.toml`, `[workspace.package].version` | Product version inherited by all three app crates |
| Root `Cargo.lock` | Resolved versions, including workspace package versions |
| App `Info.plist` | Both version fields derive automatically from Cargo's numeric `MAJOR.MINOR.PATCH` |
| `tools/metrics/Cargo.toml` and its lockfile | Separate internal tool; bump only for tool changes |
| `CHANGELOG.md` | Release notes, compatibility changes and known limitations |

Prerelease labels remain in Cargo, tags and release notes; local bundle fields use
only the numeric version. Packaging is an optimized development build with an
ad-hoc signature, not a notarized distribution or a distinct beta build number.

## CI

[CI workflow](../.github/workflows/ci.yml) runs on PRs targeting `develop`/`main`,
pushes to those branches, manual dispatch, and calls from the release workflow.
It uses `macos-15` ARM64, the pinned `rust-toolchain.toml`, Python 3.12 and locked
Cargo dependencies. CI disables debug symbols and limits compilation to two jobs
to fit the hosted runner; local builds retain their default profile. It runs the shared checks (fmt, tests, Clippy, Python,
documentation, shell syntax), then packages the app and verifies its signature,
plist and version. No GUI, SDK install, guest boot or ignored integration tests run.

Actions are pinned to commit SHAs. Cargo caches are keyed by runner, toolchain,
lockfiles and manifests; only successful pushes to `main`/`develop` save caches.
Obsolete checks on the same PR/ref are cancelled. Private macOS runners consume
Actions allowance; no billing settings are changed by these workflows.

## Prepare on develop

1. Finish the intended feature/fix PRs. Update the root Cargo version when needed.
   Refresh workspace package versions using `cargo metadata --no-deps --format-version 1`
   and inspect the lockfile diff; avoid unrelated dependency changes.
2. Move relevant Unreleased notes into exactly one nonempty section such as
   `## [0.1.0] - 2026-11-18`, using the actual release date. A future date, duplicate
   section, missing version or empty notes blocks publishing. Keep an Unreleased
   section for subsequent work.
3. Run local checks and, for app changes, a focused smoke check on disposable data:

   ```bash
   bash scripts/check.sh
   bash scripts/build-android-hub.sh
   codesign --verify --deep --strict 'Android Hub.app'
   python3 scripts/release.py verify-bundle 'Android Hub.app/Contents/Info.plist'
   ```

4. Open a release PR from `develop` to `main`. Include the intended tag, changes,
   checks, compatibility and known limits. Wait for CI and merge preserving history.
   The workflow files themselves must be merged before first use.

## Publish by tag

Pushing a `v*` tag is the explicit publish action. Do this only when ready to
release; ordinary implementation work must not create tags automatically.
Use a clean working tree and tag the intended commit already merged to `main`:

```bash
git switch main
git pull --ff-only origin main
git status --short
# Example only: replace with the exact intended Cargo version.
git tag -a v0.1.0 -m 'Android Hub v0.1.0'
python3 scripts/release.py validate v0.1.0
git push origin v0.1.0
```

[Source release workflow](../.github/workflows/release.yml) then:

1. Requires an exact Cargo/tag match, checkout at that tag, main ancestry and dated notes.
2. Runs the same macOS checks/build against the tagged source.
3. Revalidates the tag, extracts only that changelog section and creates the release
   with `--verify-tag`. A prerelease tag gets `--prerelease --latest=false`.

Only the publish job receives `contents: write`; the rest have read-only access.
The built app is discarded with the runner. No PAT, signing certificate or extra
secret is required: publishing uses the repository's `GITHUB_TOKEN`.
See [GitHub release CLI](https://cli.github.com/manual/gh_release_create).

## Failures, reruns and rollback

Check the failed job in Actions. A transient infrastructure failure can be rerun
for the same unchanged tag. An existing release is never overwritten, including a
draft created manually. If the source needs a fix, merge the fix with a new version
and changelog section, then publish a new tag; don't force-move the old tag.
A validation failure creates no release. Cancelling CI prevents its dependent
publish job from running. Tag pushes are serialized per tag.

After publication, merge `main` back into `develop`. Hotfixes start from `main`,
receive a new version/tag, and also merge back into `develop`.

A source rollback means checking out an earlier tag and rebuilding; it does not
roll back Android disks, preferences, SDK installs or schemas. Back up stopped
devices before incompatible changes and document whether downgrading is supported.
Branch/tag protection depends on the private repository's GitHub plan and must be
configured separately; workflow files alone do not enforce branch protection.
