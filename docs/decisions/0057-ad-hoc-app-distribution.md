# 0057 — Distribute an ad-hoc signed app ZIP and DMG

- **Status**: Accepted
- **Date**: 2026-11-19
- **Deciders**: Android Hub maintainer, at the owner's request
- **Supersedes**: [ADR 0055](0055-source-release-automation.md) source-only distribution decision

## Context

The owner now requests prebuilt app and DMG release assets. No Apple Developer ID
or notarization credentials have been configured. The application still requires
external SDK/Java/images and must never carry personal guest data.

## Decision

Build the locked release profile for ARM64 with a macOS 15.0 deployment target,
ad-hoc sign the bundle and distribute it as ZIP and DMG with install notes and
SHA-256 checksums. Keep source archives. Publish from a checked main tag only after
shared CI and packaging verification. Upload all assets to a draft before publication.

## Consequences

- Users with repository access can download an app without installing Rust.
- This does not solve Gatekeeper trust: downloads are explicitly non-notarized.
- Signing/notarization needs a separate future decision and credentials.
- CI performs a release build in addition to the shared development-profile checks.
- SDKs, Android images, Java and personal data remain outside the package.
- Dependency/license notices must accompany the bundle and stay aligned with Cargo.lock.
- Existing output directories and published releases are not silently overwritten.
- Build-time proc-macro stripping is disabled for the pinned Rust/macOS 27 issue;
  these host libraries are not included in the app.

## Alternatives considered

Source-only releases do not satisfy the new request. Developer ID/notarization is
preferable for wider distribution but requires credentials not configured here.
Uploading local binaries manually would not provide repeatable tag automation.

## References

- [Release process](../RELEASING.md)
- [Rust LINKEDIT issue](https://github.com/rust-lang/rust/issues/157750)
- [Developer ID distribution](https://developer.apple.com/developer-id/)
