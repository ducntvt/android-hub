# UX feedback / confirmations — 2026-11-17

US-036 is **in_progress**. US-033–035 remain in_progress; US-037–042 remain
planned. This report does not complete the full US-033 → US-042 goal.

## Implementation

- Stop, APK, restore, archive and undo use one modal presentation above the
  workspace. Removed the inline APK-result and confirmation rows.
- Modal widget captures keyboard, pointer and wheel events. Root update separately
  blocks background commands and stale clipboard/input messages, but accepts
  in-flight results. Opening/closing releases input and changes its epoch.
- Cancel initially has a visible focus border. Tab switches actions; Enter/Space
  activates the outlined action; Esc cancels. Restore Android focus only to the
  same ID/PID/display generation after cancel, when still usable.
- Stop binds ID/name/PID. `stop_session` checks expected PID after taking the Hub
  lock, then retains existing AVD identity/recheck/normal-stop safeguards.
- Typed Success/Info/Error/Unknown feedback: one six-second toast, persistent
  unresolved alerts, identical-error deduplication, ID-bound expiry/dismissal.
  Successful polls do not clear errors. Open toast details are pinned so a new
  toast or delayed timer cannot leave an invisible modal blocking the app.
- Notification Details holds raw diagnostics. Copy error exports only Hub version,
  translated severity/operation and a disclosure about omitted content; it does
  not copy diagnostics, device names, paths, clipboard or Android/account output.
- History is an explicit side panel with an unread marker. Acknowledging it does
  not erase retained history. Updated VI/EN strings.
- Successful restore now reloads the authoritative draft, consistent with the
  existing post-recovery refresh behavior.
- ADR: [0045](../decisions/0045-modal-feedback-and-confirmed-stop.md).

## Automated verification

`bash scripts/check.sh` passed after the functional and layout changes:

- Rust: **141 passed**, 20 existing ignored integration tests (desktop 47,
  domain 2, infrastructure 92).
- Python: **25 passed**; shell syntax checks passed.
- `cargo fmt --all -- --check` and clippy all-targets `-D warnings` passed.
- Final translation-only addition followed by desktop tests (47), fmt, app build
  and `codesign --verify --deep --strict`.
- Existing dependency warning: `block 0.1.6` future incompatibility; no new warning.

New regression cases cover error preservation across successful polls, repeated
error deduplication, old toast expiry, pinned details across new feedback, unknown
APK outcomes, copy-summary privacy, modal input/navigation gates, Cancel as the
initial keyboard action, changed PID confirmation, and asynchronous results while
details are open. Existing removal/APK/restore identity tests still pass.

## Native QA observed

Isolated app: `/tmp/android-hub-feedback-qa-20261117/Android Hub QA.app`.
Two registry-only fixture profiles, ports 5660/5662; no AVD payloads, accounts,
guest launches, real restores or personal runtime changes.

- Opened settings and archive confirmation in VI and EN. Title names QA Android A;
  impacts, Cancel/Remove actions and keyboard hints are readable.
- Before/during/after the modal, the stopped-device frame stayed at the same
  coordinates and size (Retina screenshot approximately x924–1396, y414–1256).
- Clicking QA Android B behind the modal did not change the selected device or
  target. Tab moved the visible outline to Remove; Esc closed the modal.
- Enter with the initial Cancel outline closed the EN confirmation without
  removing a profile.
- Requested backup on the deliberately payload-less fixture: expected failure
  appeared as a persistent translated alert. Details showed the original
  `No such file or directory (os error 2)` diagnostic. Closing Details retained
  the alert; closing the alert removed it.
- Opened the separate History panel; empty state and EN labels rendered.
- Cmd+Q closed QA normally. CUA inventory then omitted QA; read-only `ps -p 21020`
  returned no process. No engine was started by this session.

CUA occasionally returned a changed-state error and once timed out/reset. Rebinding
the same running app restored access; it was not relaunched or duplicated.

## Remaining checks

- Native running-guest checks for release/focus restoration, stop/APK/restore modal
  variants and their confirmations. This session only used stopped fixtures.
- Native success-toast expiry, clipboard copy result, recovery-receipt unread
  presentation, and smaller-window/long-details cases.
- Two small changes after visual inspection: alerts now sit above the footer so
  History/Stop remain reachable; short diagnostic text uses content height capped
  at 140 instead of always reserving 140. These compile/test/build successfully;
  their final pixel positions have not been rechecked natively.
  Follow-up: [US-037 QA](ux-settings-20261117.md) confirmed a save-success toast
  above the footer with History visible. Short-details sizing and the other
  remaining cases still need their own evidence.
- Native AX still exposes the window/menu but not Iced controls; do not claim
  VoiceOver coverage. This remains part of US-040.

## Cleanup

Removed this task's QA bundle/data, build `target/` and four temporary check/build
logs after verification. Confirmed QA and target paths no longer exist and the
rebuilt main `Android Hub.app` executable remains. Personal runtime, SDKs, AVDs,
backups and receipts are retained. No commit was made.

## Follow-up from US-038

[Create-device QA](ux-creation-20261117.md) observed the running-device Stop confirmation with the correct QA device name, followed by stopped UI and normal app exit. Other remaining checks above are not implied complete.
