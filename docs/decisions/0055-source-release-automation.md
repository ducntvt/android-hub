# 0055 — Reuse macOS checks for source tag releases

- **Status**: Accepted
- **Date**: 2026-11-18
- **Deciders**: Android Hub maintainer
- **Supersedes**: none

## Context

The private project is distributed as source. Tags must identify tested main
commits with consistent Cargo metadata and reviewed release notes.

## Decision

Use one reusable macOS ARM64 CI workflow for PRs, branches and release tags.
Validate tags on Linux before allocating macOS build time. Publish source-only
GitHub Releases after checks, with write permission limited to the publish job.
Read product metadata using Python's standard-library TOML parser (Python 3.11+).
Use the numeric Cargo version for both local bundle version fields; keep beta
labels in Git/Cargo/release notes. Pin Rust and third-party actions.

## Consequences

- Local and hosted checks share the same script; release checks cannot silently drift.
- No Apple signing account, PAT or binary artifact retention is needed.
- Private macOS CI consumes Actions allowance; cache only trusted branch builds.
- Owner prepares a dated changelog section and deliberately pushes each release tag.
- Bundle metadata does not distinguish prerelease candidates of the same numeric version.
- Workflow configuration does not replace branch/tag protection or guest smoke testing.

## Alternatives considered

Manual release publishing is simpler but permits accidental tag/version drift.
Prebuilt app releases introduce signing/notarization and distribution work beyond
this personal source-only project. Separate release checks duplicate maintenance.

## References

- [Release process](../RELEASING.md)
- [Reusable workflows](https://docs.github.com/en/actions/how-tos/reuse-automations/reuse-workflows)
- [Hosted runners](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
