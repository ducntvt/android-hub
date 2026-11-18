# 0040 — Organize application modules and owned resources

- **Status**: Accepted
- **Date**: 2026-11-16
- **Supersedes**: ADR 0030 resource location only

## Context

The user requests scaffold improvements after reviewing iomentum/cargo-scaffold. Its small CLI delegates to library code, while the generator copies/renders files from an explicit template. It does not prescribe a desktop architecture. Android Hub already has suitable domain/infrastructure/desktop boundaries, but desktop main mixes bootstrap, state, dispatch and tests; SDK modules crowd the infrastructure root; an embedded manifest is located in reports.

## Decision

Keep the existing three-crate workspace and external API. Use a small main calling app::run, with application state/subscriptions, update handlers, view, screen and tests in separate app module files. Group SDK internals under sdk/ with concise module names and retain root re-exports. Centralize workspace package metadata and dependency versions; keep consumer-specific Tokio features in each crate. No dependency upgrades or new runtime crates.

Move the single canonical SDK manifest into infrastructure/resources. Keep its bytes and consent semantics unchanged. Add one local check entry point and document source/resource/runtime boundaries. No generator installation, scaffold hooks, new template layer or source-tree replacement is necessary for this existing app.

## Consequences

Source navigation improves; SDK internals have a local module boundary and build no longer depends on reports. SDK test module paths change, including self-spawned crash-worker filters. Private helpers needed outside sdk retain crate-only visibility. Public Backend/type APIs, package names, engine lifecycle and runtime paths stay unchanged. Artifact cleanup remains separate from checks so failed builds keep diagnostics.

A future reusable project generator should use a dedicated template containing only intended source; the live Hub workspace contains real AVD data and binary resources. No runtime migration or platform expansion is included here.

## References

Reviewed upstream commit `767d6cb879654fd29e22644ee739c5fc686c2b48`:
- [CLI bootstrap](https://github.com/iomentum/cargo-scaffold/blob/767d6cb879654fd29e22644ee739c5fc686c2b48/src/main.rs)
- [Generator, excludes, binary opt-out and hooks](https://github.com/iomentum/cargo-scaffold/blob/767d6cb879654fd29e22644ee739c5fc686c2b48/src/lib.rs)
- [Dependencies](https://github.com/iomentum/cargo-scaffold/blob/767d6cb879654fd29e22644ee739c5fc686c2b48/Cargo.toml)
