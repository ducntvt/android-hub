# 0053 — Consistent named Rust modules

- **Status**: Accepted
- **Date**: 2026-11-18
- **Related**: [ADR 0040](0040-scaffold-organization.md)

> Update: [ADR 0054](0054-app-owned-data-root.md) replaces the metrics/runtime path retention below. Module and style decisions still apply.

## Decision

Keep the three Cargo crates and their existing dependency boundaries. Rust does
not mandate a desktop architecture. Prefer named module roots (`app.rs`, `sdk.rs`)
with same-named directories for children; both this form and `mod.rs` are legal.
Split large event catalogs, application startup, SDK handlers and device-settings
views by responsibility. Infrastructure snapshot parsing and shutdown aggregation
have dedicated private modules with the same root public exports. Unit tests stay
with their module or in a child `tests.rs`; examples use kebab-case target names.

Use default rustfmt and Clippy, including the excluded standalone metrics tool.
Keep its existing collector path; relocating tools or real runtime is unnecessary.
Remove the retired AVO sampler allowlist entry. Retain historical evidence and
license/protocol provenance, but remove retired-engine plans from current docs.

## References

- [Cargo package layout](https://doc.rust-lang.org/cargo/guide/project-layout.html)
- [Rust modules and alternate file paths](https://doc.rust-lang.org/book/ch07-05-separating-modules-into-different-files.html)
- [Rust Style Guide](https://doc.rust-lang.org/style-guide/)

## Consequences

No new dependency, persisted schema, engine lifecycle behavior or data-path change.
Named files are easier to identify in editors. SDK message dispatch still passes
through the existing navigation, modal and shutdown guards before its handler.

## Alternatives considered

Keeping mixed `mod.rs` and named roots is valid Rust but makes file discovery less
consistent. Creating application/CLI crates now would add boundaries without a
second consumer or engine. Moving the metrics package/runtime solely for naming
would force collector changes or data migration unrelated to this cleanup.
