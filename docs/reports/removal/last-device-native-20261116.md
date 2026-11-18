# Native remove-last / undo — 2026-11-16

Isolated root: `/tmp/android-hub-recovery-native-20261116/data`. Synthetic stopped profile `hub-recovery-native`, one original marker file and fixture registration; no guest boot and no personal A/B data.

- Opened Settings, asked to remove. Confirmation showed correct name/ID and explained retained data/no reclaimed space.
- Cancel closed confirmation, device remained selected.
- Reopened confirmation and removed: sidebar became empty, start disabled, settings requested selecting a device; success notice and removal history appeared.
- “Máy đã gỡ” remained usable with no active devices. Selected the archive, reviewed its path and confirmation, then returned the device.
- Device reappeared selected/stopped, with success notice and undo history. It did not automatically boot.
- Independent disk audit: one matching registry profile, original marker and registration restored, removed/restored receipts both present, archive no longer holds live data.

UI issue found: picker renders name plus ID in a single line and clips long labels at the panel edge. Full archive path and confirmation remain readable, but the selection list should expose the full identity without clipping. US-016 remains in progress pending this fix and removal recovery-history native coverage.

## Label fix verified

Replaced the single-line dropdown with selectable items showing name and ID on separate lines. Both text fields use WordOrGlyph wrapping and the existing panel scroll container. Selection still dispatches PickRemoved; undo requires the existing separate confirmation.

For visual QA, the synthetic fixture was reset to its archived state while the app was closed (moved marker/registration back to their original archive and retained the prior restored receipt outside it). This preparation is not counted as another native remove test. Rebuilt/signed the main app and copied the binary into Recovery QA. Native opening of “Máy đã gỡ” now showed the complete “Recovery original” name and “hub-recovery-native” ID. Clicking the item highlighted it and displayed the matching profile, archive path and undo action. The previously reproduced clipping is resolved.

fmt/check, workspace tests (111 passed / 20 opt-in ignored), clippy, app build and strict codesign verification passed. Logs: [tests](label-tests-20261116.log), [clippy](label-clippy-20261116.log), [build](label-build-20261116.log). No extra implementation-mirroring unit test was added for this presentation-only change. Native removal recovery-history coverage remains outstanding.
