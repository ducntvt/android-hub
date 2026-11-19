# Third-party components and provenance

Android Hub is currently a private project; no general open-source license has
been selected for its original code. Repository access and sharing are controlled
by its owner. This document does not change any third-party license.

## Checked-in protocol

`crates/infrastructure/proto/emulator_controller.proto` carries the copyright
notice **2018 The Android Open Source Project** and an **Apache License 2.0** header.
Keep that header with the file. The license text is available from
[Apache](https://www.apache.org/licenses/LICENSE-2.0).

## Cargo dependencies

The app uses Iced/wgpu, Tokio, Tonic/Prost, objc2, rfd, serialization, hashing,
compression and other transitive libraries. Resolved versions are recorded in
root `Cargo.lock`; metrics has a separate lockfile. Dependency source packages
retain their own license/copyright files. This page is a provenance overview,
not a license grant for any third-party component. Release bundles additionally
include a [dependency notice snapshot](crates/desktop/assets/THIRD-PARTY-NOTICES.txt)
for the resolved macOS ARM64 graph (conservatively including build/test packages).
It retains packaged/upstream notices, authors, repository locations and embedded
font attribution. The release packager rejects a snapshot with a different
Cargo.lock hash. Re-review it when dependencies or embedded assets change.

## Android tools and system images

Android Emulator, Android SDK components, Google Play system images and Java are
installed separately. The repository does not distribute those binaries or device
data. Their own terms apply. The bundled SDK plan records package metadata and
license text for explicit review; it does not represent accepted user consent.
Do not remove SDK/license notices or include an accepted-consent state in source.

## Project assets and helpers

- The logo's source/export/provenance are described in [assets](crates/desktop/assets/README.md).
- Java input/connectivity helper source and embedded DEX are maintained together;
  regeneration scripts are in `scripts/`.
- Historical AVO research is recorded in the docs. AVO is not an active dependency
  and its engine/source checkout is not shipped in this repository.

The snapshot is for the current private binary release; it is not a blanket audit
for future dependencies, modified assets or additional platforms. Packages without
a standalone license file are identified in the snapshot and retain their declared
license text or source notice.

## Embedded scrcpy server (physical-display feature branch)

The unmodified scrcpy Android server v5.0.1 is embedded for user-started USB
viewing. Copyright Genymobile and scrcpy contributors, Apache License 2.0.
[License](crates/infrastructure/assets/scrcpy/LICENSE) and
[download/checksum provenance](crates/infrastructure/assets/scrcpy/README.md)
are retained in source and copied into release/QA bundle Resources. The desktop
client is Android Hub's Rust adapter with an in-process VideoToolbox bridge; it
does not bundle the scrcpy desktop application, FFmpeg or SDL.
