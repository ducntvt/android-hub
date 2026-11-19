# 0059 — Uninstall only confirmed Hub-owned paths after exit

- **Status**: Accepted
- **Date**: 2026-11-19
- **Deciders**: Android Hub maintainer; user approved implementation
- **Supersedes**: none

## Context

The app and its data live separately. A self-uninstaller must not interpret an
arbitrary environment override, shared SDK path or source checkout as owned data.

## Decision

Expose uninstall in Hub settings, with allocated-size/path preview, optional
retention of `backups/` in place and exact `DELETE` confirmation. Require idle
mutation state before preview/confirmation. Reuse normal engine shutdown before
handing off to a copied Rust executable in a private temporary directory.

Production accepts only the running `Android Hub.app` in `/Applications` or the
current user's `Applications`, and the default Application Support data root.
The existing isolated QA bundle layout is accepted only below the system temp
directory with sibling `data/`; all other overrides/dev bundles are refused.
Validate bundle identifier, path identities and mount boundaries. Descendant
symlinks are unlinked rather than traversed. No administrator authorization is
requested; unwritable trees fail preflight.

The helper locks the data root, checks engines/processes, marks uninstall pending,
acknowledges readiness, then waits for its actual parent to exit. It revalidates
before deleting data and removes the app last. A pending marker blocks normal
backend mutations; it never authorizes automatic retry. Failure clears the marker and opens a text report. App removal starts only after
data cleanup; errors can leave partial data or a partial bundle. Partial deletion cannot be rolled back.
The helper removes its own temporary executable on success. Backup retention
keeps the existing path; compatible SDK/images are still needed to restore later.

## Consequences

External SDKs, Java, Android Studio, repository files, downloaded DMGs and other
app copies are outside scope. Multiple Hub processes or active engine processes
block uninstall rather than risking another session. Other local users' data is
not touched. Finder deletion does not invoke this workflow.

## Alternatives considered

Deleting the source/data directory based only on environment variables is unsafe.
A shell script with recursive deletion is unnecessary: the Rust helper can reuse
validated backend state. Deleting from a still-running GUI risks late writes, so
cleanup starts only after parent exit. Moving user backups would invalidate paths;
retain them in place instead.
