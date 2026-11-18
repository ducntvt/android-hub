# UI refresh — 2026-11-16

Implemented EN/VI selection, localized interface labels/confirmations/settings and common status notices. Preference is scoped to each data root. Original backend diagnostics, saved operation history, SDK license text, user names and paths are preserved; this is not translation of guest Android or external error output.

Buttons share secondary/primary/danger styles and explicit hovered/pressed/disabled states. Add button is a centered 36×36 control with zero asymmetric padding. Android version is read from each AVD's configured target, carried separately from the running-engine map and displayed as configured image; no live getprop claim.

116 workspace tests passed, 20 opt-in ignored. fmt, clippy -D warnings, app build and strict signature verification passed. Tests cover language persistence/default fallback without changing SDK settings, and known/unknown/missing Android API metadata. Logs in this directory.

Native QA used one synthetic stopped profile `hub-ui-preview`, target android-36, in `/tmp/hub-ui-refresh-20261116/fixture`. No engine was started. Observed full English workspace, then clicked VI and opened settings: labels, selected language styling, portrait choice and action buttons switched. Clicked EN: settings and sidebar switched back, user device name unchanged. Header read Android 16 · API 36 · Configured image. Plus appeared centered with a square hit area; no device was created. English and Vietnamese panels fitted at the observed 1180×852 logical window size.

Preference file contained `en` after the native switch. Save/load across instances is covered by the unit test; a second native launch was not performed because the first getApp call took 690 seconds. No claim of testing every dialog or raw diagnostic translation. QA window was closed and read-only process inventory showed no QA process before cleanup.

Main bundle was signed and verified. Removed this task's temporary checkout/fixture/app outputs and workspace target after checks, keeping source, reports, main app and existing Android runtime intact. No commit, no personal guest restart or migration.
