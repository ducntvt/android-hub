# Changelog

User-facing changes for source releases of Android Hub. Tags use `vMAJOR.MINOR.PATCH`;
see [Releasing](docs/RELEASING.md). The current source version is `0.1.0`, but no
publication date or release tag is claimed by this file.

## Unreleased

### Added

- Rust/Iced desktop workspace with independent Android devices and one/two embedded displays.
- Device creation/configuration, lifecycle management, confirmed deletion and existing archives.
- Android navigation, rotation, pointer/scroll input, explicit clipboard transfer and APK installation.
- Offline backup/restore, SDK/Java selection, managed SDK setup and guest connectivity checks.
- English/Vietnamese, system/light/dark appearance, settings and feedback panels.
- English/Vietnamese build/setup entry points and private source-release documentation.

### Changed

- App data now lives directly in Application Support, independently of the source checkout.
- Metrics source moved to `tools/metrics`; the experimental runtime/bootstrap tools were retired.
- Rust modules and shared checks were reorganized; app icon registration refreshes after packaging.

### Compatibility and limits

- The old experimental runtime was deliberately reset on the owner's machine with authorization;
  this is not an automatic data migration available to other users.
- Current target: macOS Apple Silicon with Android Emulator Google Play ARM64 API 36.
- Local builds are ad-hoc signed; no prebuilt/notarized binary release is provided.
- Clean-machine managed SDK validation, long two-game benchmarks and native accessibility remain incomplete.
