# Versions and source releases

Android Hub releases are **source tags plus release notes** in the private
`ducntvt/android-hub` repository. Invited users build locally. No `.app`, DMG,
SDK/image or device data is uploaded as a release asset.

This is a manual workflow. There is currently no `.github/workflows` release or
CI automation. No tag is created merely by changing documentation or Cargo version.

## Version policy

Use `MAJOR.MINOR.PATCH`, with prereleases such as `0.2.0-beta.1` when appropriate.
During `0.x`, use patch increments for compatible fixes and minor increments for
feature batches or incompatible changes; document data/SDK compatibility explicitly.
Reserve `1.0.0` for a declared stable compatibility baseline. See [SemVer](https://semver.org/).

The initial source declares `0.1.0`; that does not mean it has been released yet.
The tag is `v` plus the exact Cargo version, for example `v0.1.0`.
Never move/reuse a published version tag to replace a release. Publish a new version.

| Location | Role today |
| --- | --- |
| Root `Cargo.toml`, `[workspace.package].version` | Product version inherited by the three app crates |
| Root `Cargo.lock` | Resolved versions, including workspace package versions |
| `scripts/build-android-hub.sh` | Currently hardcodes macOS bundle version/build number; update them in the same PR |
| `tools/metrics/Cargo.toml` and its lockfile | Separate internal tool version; bump only for changes to that tool |
| `CHANGELOG.md` | User-facing changes and compatibility notes |

The bundle's `CFBundleShortVersionString` is the numeric product version (for
example `0.2.0`); `CFBundleVersion` is a positive build number increased when
packaging a new release candidate. Keep prerelease labels in Cargo/tag/release
notes. Current local packaging does not distinguish a beta label in its bundle
short-version field. Deriving bundle metadata automatically is future build work.

## Prepare on develop

1. Start from current `develop`; finish the intended feature/fix PRs.
2. Open a small version/changelog PR into `develop` if a bump is needed. Update
   Cargo version, bundle fields and changelog together. Refresh the lockfile with
   `cargo metadata --no-deps --format-version 1`; inspect the diff and reject
   unrelated dependency changes.
3. Move the relevant Unreleased notes to a version section when preparing the
   release. Record the actual publication date at release time.
4. Run:

   ```bash
   bash scripts/check.sh
   PYTHONDONTWRITEBYTECODE=1 python3 scripts/check-docs.py
   bash scripts/build-android-hub.sh
   codesign --verify --deep --strict 'Android Hub.app'
   plutil -p 'Android Hub.app/Contents/Info.plist'
   ```

5. For app changes, do a focused smoke check on disposable data. Record any skipped
   integration/gameplay/SDK checks. The local packaging script currently builds the
   optimized development profile, not `cargo build --release`.
6. Open the release PR from `develop` to `main`. Include the intended tag, summary,
   checks, data compatibility and known limitations. Merge preserving branch history.

## Tag the verified main commit

Use a clean working tree. These are maintainer commands to run when explicitly
publishing a release, not part of ordinary development:

```bash
git switch main
git pull --ff-only origin main
git status --short
# Confirm the intended version and absence of unrelated changes before continuing.
cargo metadata --offline --locked --no-deps --format-version 1

# Example only: substitute the exact intended Cargo version.
git tag -a v0.1.0 -m 'Android Hub v0.1.0'
git push origin v0.1.0
```

Tag the commit actually merged to `main`, not an unmerged feature commit. Annotated
tags are the default convention. Never delete an existing remote tag just to make
a failed command succeed.

## GitHub release notes

Create a draft release from the existing tag, review its notes, then publish:

```bash
gh release create v0.1.0 --repo ducntvt/android-hub --verify-tag --draft \
  --title 'Android Hub v0.1.0' --notes-file /path/to/release-notes.md
```

For a beta/RC, use the exact prerelease tag and add `--prerelease`. The notes file
should contain the relevant changelog section, build instructions, compatibility
changes and known limits. Do not attach the app or personal logs. GitHub provides
source archives from tags; private repository access still applies.
See [GitHub releases](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases).

After publication, merge `main` back into `develop` and keep an empty Unreleased
section for the next work. Hotfixes start from `main`, receive a new version/tag,
and must also be merged back into `develop`.

## Rollback and data compatibility

A source rollback means checking out an earlier tag and rebuilding. It does not
roll back Android disks, preferences, SDK installs or database/schema changes.
Back up stopped devices before an incompatible change. Release notes must describe
any migration and whether downgrading is supported; never promise downgrade safety
solely because an old binary builds.

## Later automation

A future CI workflow can run checks/build on PRs and `main`/`develop`; a tag workflow
can verify tag/version/main ancestry and draft source-only release notes. It need
not upload binaries. Protect shared branches and published tags where the repo's
GitHub plan supports rulesets. Solo maintenance need not require another reviewer.
None of those remote settings is implied to be enabled by this document.
