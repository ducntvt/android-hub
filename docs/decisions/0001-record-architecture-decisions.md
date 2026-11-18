# 0001 — Record architecture decisions

- **Status**: Accepted
- **Date**: 2026-11-13
- **Deciders**: TODO: fill in deciders (project lead)
- **Supersedes**: none

## Context

This project is worked on by multiple humans and AI coding agents. Without a durable record of *why* a particular library, boundary, or pattern was chosen, future contributors re-derive context from `git log` and `git blame` — slow, error-prone, and often impossible when the original author is unavailable.

## Decision

We will record architectural decisions as Architecture Decision Records (ADRs) under `docs/decisions/`, following the workflow in `docs/decisions/README.md`.

Each ADR is a markdown file numbered sequentially (`NNNN-short-kebab-title.md`) and contains Context, Decision, Consequences, and Alternatives Considered.

## Consequences

- Positive:
  - New contributors (and AI agents) can read the *why* behind constraints instead of guessing.
  - Decisions become reviewable artifacts in PRs.
- Negative / cost:
  - Small overhead per significant decision (~15 min to draft).
- Follow-ups required:
  - Reference `docs/decisions/` from the project's `CLAUDE.md` / `AGENTS.md` / `README.md` so agents discover it.

## Alternatives considered

### Alternative A — Rely on commit messages and PR descriptions

Rejected: PR descriptions are not discoverable from the working tree, and commit messages rarely capture the rejected alternatives.

### Alternative B — Use a wiki or external doc tool

Rejected: drifts out of sync with the code, requires extra auth, not visible to AI agents reading the repo.

## References

- Original ADR concept: Michael Nygard, "Documenting Architecture Decisions" (2011)
