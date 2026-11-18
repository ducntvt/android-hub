# Contributing

Android Hub is a private, source-distributed project. Access is granted by the
owner. Read [development](docs/DEVELOPMENT.md) and [setup](docs/SETUP.md) before
running device operations. Repository access does not authorize public redistribution.

## Branches and pull requests

- `main`: reviewed baseline for source releases.
- `develop`: integration branch for the next release.
- `feature/<short-name>` and `fix/<short-name>`: short-lived branches from `develop`.
- `hotfix/<short-name>`: urgent fix from `main`; bring it back into `develop` too.

```bash
git switch develop
git pull --ff-only origin develop
git switch -c fix/display-reconnect
```

Open a focused PR into `develop`. Explain the problem, resulting behavior,
verification and material limits. Use a merge commit for `develop → main` and
`main → develop` to preserve shared history; do not repeatedly squash long-lived
branch promotion. Feature PRs may use squash merge. Remove merged short-lived
branches when no longer needed. Do not force-push shared branches or release tags.

For a solo maintainer, PRs still record intent/checks, but another reviewer's
approval is not a requirement. CI/rulesets are not yet configured in this initial
source baseline; run the documented checks manually until automation is installed.

## Code conventions

Use default rustfmt and resolve Clippy warnings. Rust modules/functions use
snake_case, types use UpperCamelCase, Cargo packages/examples use kebab-case.
Prefer `module.rs` with a `module/` child directory. Keep pure rules in `domain`,
process/storage work in `infrastructure`, and Iced state/update/view in `desktop`.
Never invoke shell commands from widget rendering.

Reuse existing UI styles/icons/localization. User-facing Hub text needs both
English and Vietnamese; do not translate user names, paths or external licenses.
Do not upgrade dependencies incidentally in a UI/docs change. Keep both Cargo.lock
files. Required DEX, SDK manifest, logo and protocol files are source/build inputs.

## Verification

```bash
bash scripts/check.sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/check-docs.py
```

For app changes, also package and verify:

```bash
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Do not run ignored tests as a group: some download data, use SDK tools, create
AVDs or boot guests. Read each test's prerequisites and use disposable data.
Report skipped coverage accurately. A passing build does not establish gameplay
compatibility or frame-rate performance.

Normal multi-file changes should have a bounded [story](docs/stories/README.md).
Record meaningful architectural/data-contract decisions in an [ADR](docs/decisions/README.md).
Small fixes and documentation edits can stay concise.

## Data, cleanup and commits

Use an isolated QA root for tests. Never delete personal Application Support data,
SDKs, backups or receipts as cleanup without specific authorization. Close test
apps/guests and remove task-created target/cache/temporary bundles after handoff.
Keep the user's main app and their active sessions unless stopping them is part
of the agreed task.

Use a concise commit message, for example `fix: preserve display focus` or
`docs: explain first device setup`. Do not add AI `Co-Authored-By` trailers.
Coding agents must leave changes uncommitted unless the user explicitly requests
a commit; a completed edit is not permission to commit automatically.

## Release and reporting

Follow [Releasing](docs/RELEASING.md) for version/tag promotion. Record visible
changes under [Unreleased](CHANGELOG.md); do not invent publication dates.
For sensitive reports, use [Security](SECURITY.md). Regular issues should include
a reproduction and redacted diagnostics, never account credentials or device disks.
