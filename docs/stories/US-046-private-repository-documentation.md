# US-046 — Documentation for a private source repository

- **Status**: implemented
- **Created**: 2026-11-18
- **Related**: [Repository setup](../GITHUB.md), [Releasing](../RELEASING.md)

## Context

Owner selected the private repository `ducntvt/android-hub`. Collaborators build
locally; releases identify source by tag and notes without prebuilt app artifacts.
Prepare usable English/Vietnamese entry points and maintainer documentation first.

## Acceptance

- [x] English/Vietnamese READMEs describe current features, requirements, build and data location.
- [x] Setup, development, contribution and manual version/tag/release guides match implemented behavior.
- [x] Changelog starts at Unreleased; no claim of existing CI, release tags or notarized binaries.
- [x] Private repository access and third-party provenance are explicit without choosing an open-source license.
- [x] Documentation links and command/script references verified; no build or guest artifacts created.

## Out of scope

New app features, automatic release workflows, dependency upgrades, publishing a
version tag, choosing an open-source license or changing real Android data.
Remote bootstrap is a subsequent handoff once authenticated access is available.

## Verify

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/check-docs.py
```

The link check covers maintained documentation and project-owned relative links,
not historical reports, runtime artifacts or external websites.

## Evidence

Maintained-doc link/script-reference/shell-syntax checks passed. Existing 23 Python
script tests passed. No GUI, guest, app rebuild or runtime change was performed.
A preliminary filename/content-pattern scan found no files above 20 MiB or common
GitHub/AWS/Google API key/private-key patterns among candidate source files; this
is not a comprehensive secret audit. Historical local-path reports remain private.
Remote creation/push is separate and requires authenticated access plus an explicit
commit instruction. No release tag, CI workflow or binary release was created.
