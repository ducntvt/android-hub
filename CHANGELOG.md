# Changelog

User-facing changes for Android Hub. Tags use `vMAJOR.MINOR.PATCH`;
see [Releasing](docs/RELEASING.md).

## Unreleased

### In development — physical device feature branch

- USB phone discovery and embedded VideoToolbox video with explicit start/stop.
- Phone touch, swipe, two-axis wheel, navigation and ASCII keyboard control,
  with separate focus beside an emulator and cancellation on capture/session changes. IME/Unicode, audio and game key mapping are not supported.

- USB phone APK picker/confirmation and explicit clipboard transfer.
  Installation refuses to replace existing apps; clipboard success requires read-back.
  Reconnect, interrupted installation and emulator/phone destination checks passed (US-061).

## [0.2.0] - 2026-11-19

### Added

- Explicit uninstall from Hub settings with path/size review, DELETE confirmation,
  optional backup retention and after-exit cleanup of the installed app and owned
  data. External SDKs/Java and source checkouts are preserved.

### Compatibility and limits

- Existing devices, SDK settings and backups are retained when updating; no data
  migration is required. Uninstall only deletes data after explicit confirmation.
- Uninstall supports the app in `/Applications` or `~/Applications` with the
  default Hub data directory. Other Hub/Android processes must be closed first.
- Apple Silicon builds remain ad-hoc signed, not Developer ID signed or notarized.
  Android SDKs, Java and system images are downloaded or selected separately.

## [0.1.1] - 2026-11-19

### Fixed

- Accept AVD configurations that omit the redundant `target` field when the
  Android image identity is verified; conflicting values remain rejected.

### Updating from 0.1.0

- Quit Android Hub, replace the app in Applications, then reopen it and create
  a new Android device. Existing SDKs, images and registered devices are retained.
- Files from failed creation attempts remain preserved for inspection; this patch
  does not automatically register or delete them.

## [0.1.0] - 2026-11-19

### Added

- Apple Silicon release app ZIP and drag-to-Applications DMG, with SHA-256 checksums.

- Choose Android 12–16 at device creation, with immutable image identity and optional image downloads.
- Installed-image inventory and guarded removal of unused Hub-managed images.
- API 31-compatible Android helpers and a separate CI helper-reproducibility job.
- macOS ARM64 CI and guarded source releases from version tags.

- Rust/Iced desktop workspace with independent Android devices and one/two embedded displays.
- Device creation/configuration, lifecycle management, confirmed deletion and existing archives.
- Android navigation, rotation, pointer/scroll input, explicit clipboard transfer and APK installation.
- Offline backup/restore, SDK/Java selection, managed SDK setup and guest connectivity checks.
- English/Vietnamese, system/light/dark appearance, settings and feedback panels.
- English/Vietnamese build/setup entry points and private source-release documentation.

### Changed

- Pinned Rust toolchain; app bundle versions now derive from Cargo automatically.
- Build scripts require Python 3.11+ for standard-library TOML metadata parsing.

- App data now lives directly in Application Support, independently of the source checkout.
- Metrics source moved to `tools/metrics`; the experimental runtime/bootstrap tools were retired.
- Rust modules and shared checks were reorganized; app icon registration refreshes after packaging.

### Compatibility and limits

- The old experimental runtime was deliberately reset on the owner's machine with authorization;
  this is not an automatic data migration available to other users.
- Current target: macOS Apple Silicon with Android Emulator Google Play ARM64 Android 12–16.
  Qualification and game limitations are recorded in the E01 reports.
- Prebuilt app ZIP/DMG target macOS 15+ on Apple Silicon. They are ad-hoc signed,
  not Developer ID signed or notarized; Gatekeeper may block downloaded copies.
- Packages contain only Hub; SDK, Java, Android images and device data are separate.
- Clean-machine managed SDK validation, long two-game benchmarks and native accessibility remain incomplete.
