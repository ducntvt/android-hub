# US-033 — Adaptive Android workspace

- **Status**: implemented
- **Owner**: Codex
- **Created**: 2026-11-17
- **Risk lane**: normal
- **Related**: [US-005](US-005-device-workspace-ui.md), [US-034](US-034-explicit-device-pair.md)

## Context

The live Android canvas currently starts at a fixed width of 250 logical pixels, or 210 in two-device mode. Enlarging the Hub window therefore leaves usable space empty, while opening a side panel can crowd the content. The workspace should fit Android to its actual available area and allow the device sidebar to be collapsed.

## Contract (acceptance criteria)

- [x] The default display mode fits each Android screen into the available workspace while preserving its aspect ratio. The calculation uses the stage's layout size, including the effect of open panels and visible controls.
- [x] Both portrait and landscape screens fit in one-device and two-device layouts without requiring workspace scrolling at the default fit setting.
- [x] Zoom controls explicitly describe scaling relative to the fitted size; returning to Fit restores automatic sizing. No label claims that this is a native-pixel or physical-size 100% mode.
- [x] Manual enlargement may overflow into a scrollable workspace, while input still maps to the correct Android coordinates.
- [x] A sidebar toggle is always reachable. Collapsing and restoring the sidebar does not change the selected device, running engines, or active display pair.
- [x] Sidebar visibility and zoom mode are retained during this Hub session. This story does not promise persistence after quitting.
- [x] Empty registry and unmatched search results have different messages, and the collapsed layout retains a clear way to identify the selected device.
- [x] Geometry tests cover small/default windows, one/two screens, portrait/landscape, and the reduced stage area with a panel open. No engine or Android data change is needed for these tests.

## Out of scope

- Persisting window geometry, zoom, sidebar size or visibility across launches.
- Resizable sidebar, full-screen/focus mode, automatic vertical stacking, or native-pixel zoom.
- Engine rendering resolution, frame-rate changes, Android configuration, and guest restart.
- Redesigning all settings, SDK, archive and backup panels.

## Affected areas

- `Cargo.toml` — Iced responsive widget feature, if required.
- `crates/desktop/src/app/{mod,ui,update,tests}.rs` and a bounded layout helper if useful.
- `crates/desktop/src/app/screen.rs` — retain coordinate and gesture behavior.
- `crates/desktop/src/i18n.rs`.

## Implementation notes

Use the actual stage constraints, rather than subtracting estimated toolbar/sidebar heights from the window size. Keep layout calculation independent of filesystem and guest I/O. Reserve space for visible card controls and borders before sizing the image. Resizing must not recreate display sessions or reset the input epoch merely because the canvas dimensions changed. Preserve the existing cursor mapping and capture of scroll events over Android.

## Verify

Run these commands in order; each must exit 0 before the story is marked implemented. Native checks below have their own evidence and must not be inferred from compilation.

```bash
PYTHONDONTWRITEBYTECODE=1 bash scripts/check.sh
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

## Native smoke checklist

- [x] Inspect single and paired layouts at 1000 × 640 and 1180 × 820 logical window sizes, including a landscape screen.
- [x] Open/close the configuration panel and collapse/restore the sidebar; the device screens fit and the toggle remains accessible.
- [x] Zoom in, return to Fit, and resize; controls remain readable and no unrequested scrollbars appear at Fit.
- [x] Tap/drag at fitted and manually enlarged sizes, including near screen edges; input reaches the correct Android location.
- [x] Verify VI/EN labels and empty/unmatched-search states.
- [x] Close only QA apps/guests created for this verification and remove disposable artifacts; preserve the user's running Hub and Android sessions.

## Evidence

- [Final acceptance audit](../reports/ux-interaction-followup-20261117.md) maps
  the contract to code, automated tests, native observations and user feedback.
  It includes live resize, established-display recovery, partial shutdown failure
  and the corrected connection label. Verification: 190 Rust + 26 Python;
  fmt/clippy/shell checks, app build and strict codesign passed.
- [Workspace/native follow-up](../reports/ux-workspace-followup-20261117.md)
  records fitted layouts, coordinate routing, explicit slots and modal focus.
- [Feedback follow-up](../reports/ux-feedback-followup-20261117.md) covers
  long EN names. The user confirmed dragging works at Fit and 110%.
- [Initial lifecycle evidence](../reports/ux-lifecycle-20261117.md) covers bounded
  boot observation and backend stop progress. Earlier pending items are
  superseded only by the specific evidence in the final audit.
- Exact native held-pointer/slot switching and the interrupted final Retry click
  are not claimed; release ordering and retry/return use code/regression evidence.
  QA apps/guests and disposable build/data artifacts were closed and removed.
