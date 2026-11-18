# US-034 — Explicit device pair and visible input target

- **Status**: implemented
- **Owner**: Codex
- **Created**: 2026-11-17
- **Risk lane**: normal
- **Related**: [US-033](US-033-adaptive-workspace.md), [US-012](US-012-input-release-order.md), [ADR 0013](../decisions/0013-release-order.md)

## Context

Two-device mode currently displays the first two running registry entries, then renders their sessions in map order. Clicking an Android screen changes keyboard focus without changing the selected device used by configuration and lifecycle actions. Users need explicit, stable device slots and an unambiguous target for each action.

## Contract (acceptance criteria)

- [x] Two-device mode exposes two ordered slots with explicit device selection. The same device cannot occupy both slots.
- [x] Slots may contain stopped devices and may be empty. A stopped device shows its name and a targeted start action; an empty slot invites choosing a device.
- [x] Starting, stopping or polling devices does not reorder the chosen pair or silently replace a selected stopped device with a different running device.
- [x] Selecting a third device has a defined visible effect on the active slot; it cannot leave the toolbar pointing at a device hidden outside the pair.
- [x] Removing a device reconciles its slot and selection without displaying stale data or issuing actions for the removed ID.
- [x] The device receiving keyboard input has a visible indicator. A successful click on a displayed device synchronizes the selected device and relevant configuration target.
- [x] Selection synchronization during a screen gesture does not reset its input epoch, discard the mouse release, or retarget input to another session. Existing generation and PID checks remain in force.
- [x] Changing a slot or leaving the workspace releases pending input before dropping sessions. Background or hidden devices cannot receive newly directed keyboard input.
- [x] Stop controls and confirmation identify the target by device name. Confirmation is bound to the intended device or canceled when selection changes; it cannot silently act on a new selection.
- [x] Single/two-device controls show the current mode, and switching back to single-device mode displays the selected device.
- [x] Regression tests cover three registered devices, duplicate selection, stopped/empty slots, stable order, removal reconciliation, and gesture/focus transitions.

## Out of scope

- Saving the pair or active selection across Hub launches.
- More than two simultaneous displayed screens, cloning, drag-to-reorder, and automatic vertical stacking.
- Relaxing global mutation locks or redesigning unsaved configuration handling.
- Changing guest processes, input protocol, backend lifecycle identity checks, or account data.

## Affected areas

- `crates/desktop/src/app/{mod,ui,update,tests}.rs` and a bounded slot-selection helper if useful.
- `crates/desktop/src/app/screen.rs` — input focus and gesture regressions.
- `crates/desktop/src/i18n.rs`.

## Implementation notes

Keep ordered slot IDs separate from the map of live display sessions. Reconciliation may open sessions only for running devices in those slots; a stopped slot still exists in the UI. Do not call the existing `choose()` unchanged from a mouse-down event: it calls `release_inputs()`, increments the epoch and can invalidate the current gesture. Use a narrowly scoped selection synchronization path that preserves the gesture, and retain cancellation of pending actions whose target changed. Avoid silently discarding an editable draft from unrelated canvas events; retain current stopped-only configuration rules.

## Verify

Run these commands in order; each must exit 0 before the story is marked implemented. Native checks below have their own evidence and must not be inferred from compilation.

```bash
PYTHONDONTWRITEBYTECODE=1 bash scripts/check.sh
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

## Native smoke checklist

- [x] With at least three QA profiles, select an explicit pair; confirm screen order matches selectors rather than registry/map order.
- [x] Select a stopped machine and an empty second slot; verify clear placeholders and correct targeted start action.
- [x] Click/type/drag in each displayed device; focus indicator and selected name agree and input does not reach the other device.
- [x] Switch a slot, collapse the sidebar, and toggle one/two-device mode; controls still identify their target.
- [x] Open a stop confirmation and verify its device name; cancel without stopping an unrelated device.
- [x] Inspect VI/EN labels and long device names.
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
