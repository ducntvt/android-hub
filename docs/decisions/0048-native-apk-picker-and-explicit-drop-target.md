# 0048 — Native APK picker and explicit drop target

- **Status**: Accepted
- **Date**: 2026-11-17
- **Deciders**: Android Hub implementation
- **Supersedes**: none; extends ADR 0007 and 0045

## Context

US-039 requires file selection without dragging, and a clear destination in a two-device workspace. Iced 0.13 window FileDropped/FileHovered events carry a path but no coordinates. An earlier pointer position cannot identify the drop destination reliably.

## Decision

Use rfd 0.17.2 AsyncFileDialog on macOS, within the existing windowed Iced application. Use its single-file APK filter; validate the result and dropped files through the same asynchronous file inspection. Do not write custom AppKit FFI or invoke shell dialogs.

A per-device Cài APK… button captures its ID/name/PID. Drop always asks the user to choose a running device, including single-device mode. Hover describes this next step rather than claiming a destination. Picker/inspection/confirmation retain a request token, command epoch and original selection; cancellation, changed selection/session or stale observation cannot launch installation. Backend retains its own lock/PID/AVD/boot/path checks. Confirmation shows filename, size and path; no APK bytes are loaded in the UI.

Clipboard ACK continues to come from the existing generation-gated RPC response. UI supplies the device name as feedback subject, never clipboard text. Back/Home/Recent remain per-device controls.

## Consequences

- Native picker adds one direct dependency with defaults disabled, scoped to macOS. No download manager, background clipboard sync or install retry is introduced.
- File metadata may become stale after preview; backend validates the path again before installation, as before. Preview does not guarantee package identity or package contents.
- Drop requires one extra explicit destination click. This is intentional because the event does not prove a card destination.
- Native picker cancel/success and guest operations require isolated smoke verification. Non-macOS picker returns a clear unsupported error; macOS Apple Silicon remains the supported platform.

## Alternatives considered

- Last pointer location: rejected because drag coordinates are absent and stale hover can target the wrong guest.
- Handwritten NSOpenPanel bridge: rejected due to avoidable unsafe lifecycle/threading work.
- Global selected-device default for drop: rejected because two-card layout implies card targeting that the API cannot deliver.

## References

- [rfd threading and async dialogs](https://docs.rs/rfd/0.17.2/rfd/)
- Local Iced 0.13 window event source: FileHovered(PathBuf), FileDropped(PathBuf), FilesHoveredLeft.
- [US-039](../stories/US-039-apk-and-clipboard-actions.md)
