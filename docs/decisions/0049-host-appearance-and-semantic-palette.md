# 0049 — Read host appearance and centralize semantic UI colors

- **Status**: Accepted
- **Date**: 2026-11-17
- **Deciders**: Android Hub desktop
- **Supersedes**: fixed light appearance in US-027

## Context

US-040 requires persisted System/Light/Dark presentation without guest restarts. Iced 0.13's exposed window events do not supply a system-theme event. Existing custom white backgrounds and fixed foreground colors bypass Iced's theme.

## Decision

Store a separate `ui-appearance` scalar beside `ui-language`, using write/rename and System fallback for missing/unknown values. Do not migrate or modify guest profiles. Use AppKit `NSApplication.effectiveAppearance` matched against Aqua/DarkAqua, via typed objc2 bindings already in the lockfile. MainThreadMarker guards every AppKit access. Read once at initialization and on the existing 3-second Tick (no new helper, polling timer, subprocess, or network request). Never override NSApplication.appearance: system detection must continue inheriting macOS.

Resolve the chosen preference into cached Iced themes. Shell, panels, dialogs, feedback and controls share semantic surface/text/status colors; the streamed Android image is unchanged. Use existing canvas support for small vector icons instead of a font glyph or a new asset dependency. Hub preferences groups language, appearance and links to the existing SDK/storage workflows; quick language buttons remain in the sidebar. Unsaved device-settings navigation keeps its existing confirmation.

## Consequences

- Appearance follows real macOS state within one normal refresh. A transient unavailable native query retains the last observed value; initial non-macOS fallback is light. This release targets macOS.
- AppKit window chrome and native picker still follow macOS when an explicit Hub palette is selected. Only Hub-rendered controls use the explicit palette.
- Theme changes do not touch display sessions, command epoch, backend configuration, credentials or guest input generation.
- New preferences and semantic colors require round-trip/fallback/contrast checks and native small-window VI/EN smoke. Keyboard focus and VoiceOver remain separate evidence within US-040; vector icons do not establish accessibility support.

## Alternatives considered

- Infer dark mode from time: rejected because it does not reflect user preference.
- New background watcher or shell `defaults`: rejected because an existing low-frequency Tick is sufficient and typed AppKit avoids parsing external command output.
- Read appearance on every frame: rejected to avoid repeated native work during video streaming.
- Globally change macOS appearance: out of scope; application presentation must not alter system settings.

## References

- [Apple effectiveAppearance](https://developer.apple.com/documentation/appkit/nsapplication/effectiveappearance)
- [Apple bestMatch(from:)](https://developer.apple.com/documentation/appkit/nsappearance/bestmatch(from:))
- [US-040](../stories/US-040-desktop-visual-language-accessibility.md)
