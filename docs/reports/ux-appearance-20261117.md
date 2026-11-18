# US-040 — Appearance implementation and partial native audit

Status: implemented within the US-040 contract, including its explicit framework-gap exception. Native accessibility is not implemented; follow-up US-043 records that limitation.

## Implemented

- Persisted System/Light/Dark Hub preference, separate from guest profiles. Unknown/missing preference falls back to System. AppKit effectiveAppearance is read on the main thread, using the existing 3-second Tick; no helper process or new timer.
- Cached light/dark themes; semantic surfaces and foreground colors for shell, dialogs, feedback and controls. Shared body/caption/section/gap/radius tokens. Vector +/−/sidebar/device/Back/Home/Recent icons replace font glyphs.
- Hub settings groups language, appearance, SDK and storage entry points. Sidebar quick VI/EN remains. Unsaved device settings still require resolving the draft before entering Hub preferences.
- Command shortcuts: ⌘, preferences, ⌘F search, ⌘N create, ⌘1/2 layout, ⌘0 fit, ⌘B sidebar. Routing respects captured text events and modal guards; the Android canvas excludes Command-modified keys.

## Automated verification

Latest `bash scripts/check.sh` passes 176 Rust tests (75 desktop, 4 domain, 97 infrastructure), 26 Python tests, fmt, clippy and shell syntax. The existing 20 ignored integration cases remain unexecuted. Build and strict codesign pass, including the final autofocus/checkbox-padding/placeholder-contrast changes.

New tests cover preference round-trip/corruption fallback, unchanged guest profile/command/input epoch during theme changes, dirty-settings navigation, shortcut text-capture guards and contrast of semantic text/active controls in both palettes. The selected/hovered sidebar row was added to contrast checks after native inspection exposed insufficient contrast in the generated primary tint; explicit selected/raised semantic surfaces now pass 4.5:1 for both main and secondary text.

## Native observations

One app at `/tmp/android-hub-appearance-qa-20261117/Android Hub QA.app`, with isolated QA root and two stopped registry fixtures, no AVD or guest boot. Initial fixture IDs lacked the required Hub prefix and correctly produced a validation error; fixed the fixture IDs, and the normal poll recovered. This was not a product regression.

- ⌘, opened Hub settings. System initially displayed the light palette. Selecting Dark switched shell, cards, panel and controls immediately.
- English switched all new panel/help text, sidebar and controls; raw device names stayed unchanged. Files confirmed `ui-appearance=dark` and `ui-language=en`.
- Long names (48 and 43 characters) wrapped in the sidebar at the initial 1180-point window width; panel controls were visible in both languages.
- ⌘F visibly focused search; subsequent typing `second` filtered the sidebar to the matching fixture.
- An existing error-details modal showed a visible default-cancel focus outline and Esc dismissed it.
- Native observation found the generated selected-row tint too bright in dark mode. Fixed afterward, with contrast regression passing; the follow-up minimum-window run confirmed the corrected tint.
- Cmd+Q exited; app inventory and exact process-path check confirmed no QA app remained. No engine was started by this layout test.

## Keyboard and minimum-window follow-up

Isolated `/tmp/android-hub-focus-qa-20261117/Android Hub QA.app`, two stopped synthetic profiles, no AVD or guest launch. The QA launcher uses the bounded 1000×640 content-size option; native capture is 2000×1344 pixels including the title bar.

- Final dark selected-row tint and long VI/EN names were readable. Toolbar, sidebar and Hub preferences remained reachable at minimum size. English/light and Vietnamese/dark were observed; scroll areas correctly kept overflowing list/help content available.
- From search, Tab traversed VI/EN and both device rows. Focusing the partially hidden second row scrolled it fully into view. Enter selected that device; Shift-Tab returned to the previous row and scrolled it into view without changing selection.
- Command-N opened the creation dialog. Tab traversed name, environment actions, Advanced, RAM/CPU/FPS/orientation/resolution and checkbox within the modal. Enter expanded Advanced; Down changed RAM from 3072 to 4096; Space checked Start after creation. Escape dismissed the form without creating an AVD.
- Native inspection found two polish issues: an old shell focus outline remained behind the form, and the checkbox outline crossed its label. Fixed by focusing the name input on open and adding checkbox padding. Light-mode input placeholder was also too faint; switched enabled placeholders to the semantic secondary foreground and extended the contrast regression.
- Shared focus wrappers use release-based activation, changed-target/focus-loss cancellation and skip disabled controls. Modal traversal is scoped; focus reveal only adjusts enclosing Hub scroll containers. Tests cover these rules, nested-scroll math, Android input release and launch-size bounds.
- Relaunched the final binary in the same isolated fixture after confirming the previous QA process exited. English/light preference survived restart, the search placeholder was readable, Command-N focused the name input immediately, and four Tabs reached the checkbox with its outline clear of text. The final English form fit at 1000×640. Escape cancelled; no creation/start action was submitted.

## Native accessibility audit and limits

- AX state of shell, preferences and creation dialog exposes only the native window/title/menu and window-management buttons, with no Hub button, input, checkbox or picker nodes. Local Iced 0.13 core/widget/winit sources and the current dependency graph have no accessibility/AccessKit integration for these controls. Keyboard focus is not a native accessibility implementation.
- VoiceOver support is **not achieved**; spoken-output testing was not performed and no macOS accessibility setting was changed. The missing semantic tree is tracked separately in [US-043](../stories/US-043-native-hub-accessibility.md), as permitted by US-040's framework-gap clause. No framework rewrite is included in US-033–042.
- Minimum-size startup is verified; live window-drag resize is not claimed from the earlier unsuccessful attempts.
- System appearance transition while macOS itself changes has not been exercised. AppKit query and fallback paths are implemented; no global macOS preference was changed for QA.
- Theme-switch continuity with a running Android stream has automated identity-state coverage, not a new native running-game test in this pass.

US-040 is complete with the accessibility gap explicitly deferred to US-043. The active US-033–042 goal is not complete: US-033–036 retain their native gaps and US-041–042 remain planned.

## Cleanup

Final QA exited via Command-Q; native inventory and an exact process-path/fixture-ID check confirmed no remaining QA or test guest process. Removed the owned `/tmp/android-hub-focus-qa-20261117` bundle/data (20 MiB), four `hub-focus-*-20261117.log` task logs, `scripts/__pycache__` and `target/` (2.7 GiB). Verified those paths absent. Main `Android Hub.app`, personal runtime, SDK/AVDs and backups remain. No commit was created.
