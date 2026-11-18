# Workspace and lifecycle interaction follow-up — 2026-11-17

## Scope and isolation

This follow-up covers the remaining US-033–035 checks. The two requested analysis
agents completed earlier; this is implementation verification, not a new audit.
The QA bundle and data live under `/tmp/android-hub-interaction-qa-20261117`.
Two blank Android 16/API 36 Google Play ARM64 guests use ports 5664/5666,
1536 MiB RAM, one CPU and 15 FPS. A third stopped registry profile has no AVD
and was never started. No personal account, game runtime or original SDK image
was changed. The input APK was built from `tests/android-input` and installed
only after checking the exact QA AVD identities.

## Observed native behavior

- Both guests progressed to Ready to control after starting through Hub. An
  initial display timeout recovered automatically.
- Fit at 1180 × 820 showed portrait and landscape together. The native macOS
  window zoom action enlarged the running window to the available desktop;
  both canvases refitted without workspace scrollbars. Restoring the window
  returned to the original fit, pair and running guests. This closes the live
  resize gap; it does not claim freehand border-drag coverage.
- 110% zoom explicitly displayed `110% · khung` and added workspace scrolling.
  Returning to Fit removed it. Earlier reports cover enlarged/scrolled edge
  coordinate mapping.
- Input fixture taps and keyboard events reached their selected guest with
  balanced Down/Up counters. Native drag attempts in the fixture still showed
  Moves 0, so continuous drag at manual zoom is **not** recorded as passed.
  The supported automation API does not expose separate mouse-down/mouse-up
  calls for switching a slot while a pointer remains held. The user subsequently confirmed on the main app: “Kéo/vuốt đúng ở cả hai
  mức” (Fit and 110%). This closes the manual-zoom drag smoke gap; it is
  user-reported evidence, not an automated Moves-counter pass.
- With QA A suspended for a bounded 45 seconds and automatically resumed in
  `finally`, an input RPC timed out. Hub removed A’s last frame and controls;
  QA B remained live and received a key. Clicking the disconnected placeholder
  and typing X did not change B’s key counters. A automatically reconnected to
  Ready when resumed. This verifies established-session loss/recovery, unlike
  the earlier startup timeout case.
- The fault exposed a presentation defect: boot revalidation masked the display
  error with `Android status unavailable`. The fix prioritizes an existing
  display error after lifecycle/stopped checks and shows `Display disconnected
  · Reconnecting…` (with Vietnamese translation), independent of pending or
  failed boot probes. It does not infer boot readiness from a frame.
- Quitting with two guests visibly showed QA Portrait stopping and QA Landscape
  waiting. Subsequent process inspection found both original guest PIDs and the
  original Hub PID gone. The screenshot API reopened the exited bundle; that
  extra empty QA window was closed too. Partial failure was **not** established
  by this suspension attempt and is not inferred from successful shutdown.

## Verification

The label regression test covers pending, failed and successful boot observations,
both languages, and returning to Connecting after the display error clears.
`scripts/check.sh` passed: 190 Rust tests (20 existing ignored), 26 Python
tests, formatting, clippy with warnings denied, and shell syntax checks.
Main app build and strict deep codesign verification passed. The QA copy was
updated to that binary and signed/verified before its follow-up run. The existing
`block 0.1.6` future-compatibility warning remains; no new compiler/clippy warning.

## US-034 input-release acceptance review

The native smoke checklist requires routing/drag, slot changes, target indication,
stop confirmation and VI/EN long names; those observations are recorded here and
in the workspace/feedback follow-ups. The separate contract on pending input is
verified at code/protocol level: `PickSlot`, `SplitView` and `choose` call
`release_inputs` before `sync_displays` can drop sessions; `release_inputs` clears
focus and increments the epoch; session drop sends a bounded Disconnect, whose
handler releases touch. Screen state resets gestures on epoch/generation/ID
changes. Existing gesture, modal epoch, stale input, queue saturation and
reconnect tests cover these invariants. A successful canvas selection keeps its
epoch, as checked by `canvas_selection_keeps_gesture_epoch_and_cancels_old_action_target`.

The earlier report treated simultaneous native held-pointer/slot switching as a
required additional smoke. The current tool has no independent down/up action;
that exact native sequence is not claimed. Contract acceptance uses the reviewed
release-before-drop ordering plus automated generation/gesture protections and
observed per-device routing. This limitation is retained rather than extending
the implementation scope or repeatedly reopening QA for the same unsupported
input sequence.

## Final shutdown follow-up

A QA-only SDK shim delegated every command to the original vendor SDK except,
while a disposable marker existed, A's `emu avd name` response. This safely
exercised the existing identity refusal without changing a guest or the vendor
SDK. Both guests had reached Ready through the rebuilt UI before the marker
was enabled.

Native EN shutdown showed QA Portrait / Could not stop with the identity refusal,
then QA Landscape / Stopping device. The settled screen showed A / Could not
stop, B / Stopped, Unable to close Hub, Retry and Return to Hub. A remained
running and B was absent in the process list. Thus one failure did not prevent
stopping the other device, and Hub did not exit with A running.

The fault marker was removed. Before the intended Retry check, the window had
changed externally to light theme, a larger size and a restarted B; the attempted
click is **not** evidence for Retry. A subsequent normal Command-Q stopped both
QA guests and exited Hub. Exact process inspection confirmed no QA Hub/guest
remained. Retry/return state transitions retain automated coverage. The rebuilt
Vietnamese connection label was also observed during an initial display retry;
established loss/recovery was observed earlier in this report.

## Acceptance audit for US-033–035

| Story / criteria | Evidence |
|---|---|
| 033: fit uses stage constraints; portrait/landscape, single/pair | `workspace::screen_size`, responsive stage, geometry tests; earlier minimum/default/panel native captures |
| 033: zoom meaning, scrolling, coordinate mapping | Explicit Fit-relative labels; earlier enlarged/scrolled edge tests and native input; user confirmed real dragging at Fit and 110% |
| 033: sidebar reachable, session preference, empty/search | Earlier native toggle/pair/selection checks, session state fields and distinct empty/search messages; live resize above |
| 033: required native sizes/languages and cleanup | Workspace and feedback follow-up reports, resize above, final cleanup below |
| 034: explicit pair, duplicates, stopped/empty, stable order, removal | `Slots` and three-profile regression; earlier native reverse pair, third profile, empty reconciliation and targeted Start |
| 034: focus, gesture, release-before-drop, hidden input | Native independent keyboard routing and modal focus; input-release review above, screen epoch/generation and release-order tests; native held-pointer limitation explicitly retained |
| 034: named stop, single/pair, VI/EN long names | Earlier named stop/cancel and routing smoke; replacement-PID/target tests; feedback follow-up EN 45-character name |
| 035: truthful phases, bounded identity probe, wait/error text | Lifecycle code, PID/epoch/failed-probe tests, native boot progression; no fabricated percentages |
| 035: display loss gate, no stale frame, other guest independent | Controlled 45-second fault above; automatic reconnect; label-priority regression and final native label |
| 035: named stop progress, partial failure, exit guard | Final shim/native refusal + B stopped + persistent result; normal two-guest exit; automated retry/return and partial-failure transitions |
| 035: no permanent polling or orphan probe | `boot_updates` completion/cap and subscription drop, cancellation/PID tests, earlier process evidence |

These are acceptance results for the original story contracts. Native scenarios
not produced by the automation API are identified as such; no additional
accessibility, game-performance or public-release certification is implied.

## Final cleanup

After checking exact QA bundle and guest process identities, removed the QA
bundle/data/SDK shim, input APK build directory, three temporary interaction
logs and workspace `target/`. The main app, personal runtime, installed SDK,
non-QA AVDs and backups remain. No commit was created.
