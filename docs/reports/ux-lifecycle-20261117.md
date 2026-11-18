# UX lifecycle and native workspace verification — 2026-11-17

## Implementation

US-035 now has bounded asynchronous guest boot observations, independent of
process/resource polling and display frames. Probes bind profile/PID, exact ADB
transport and boot identity; late results use a desktop epoch guard. Individual
commands time out after 3 seconds, the full probe after 12 seconds. Waiting probes
retry after 5 seconds, finish on success or a two-minute cap, and can be retried by
the user. Dropping the subscription kills its subprocess. Display reconnection
invalidates readiness; hidden devices do not claim to be ready for input.

Start/stop elapsed state and errors are separate from display errors. An engine
that disappears outside Hub's stop result is marked as needing attention rather
than silently treated as a requested stop. Shutdown exposes per-device
Waiting/Stopping/Stopped/Failed events, retains results on failure and still
requires an empty final engine snapshot before successful exit.

Native inspection also prompted fixes to US-033: aligned card headers, reserved
space for retry controls and a stable-width Settings button. These final layout
fixes have automated verification but have not yet been inspected in a new UI
session. Two initial readiness tests depended on the user's saved English
preference; their fixtures now explicitly select Vietnamese without writing it.

## Automated verification

- Final `bash scripts/check.sh`: exit 0; 135 Rust passed (41 desktop, 2 domain,
  92 infrastructure), 20 existing opt-in tests ignored; 25 Python passed.
- fmt, clippy `-D warnings` and shell syntax checks passed.
- Final `bash scripts/build-android-hub.sh` and strict deep codesign verification passed.
- Existing block 0.1.6 future-incompatibility warning remains.
- Cancellation regression proves the spawned sleep process is gone after its
  observation future is canceled; no long-lived probe process is left.
- State regressions cover late PID/epoch results, retry, unknown readiness and
  partial shutdown failure. Existing shutdown backend test confirms continuation
  after one failure and ordered progress events.
- A parallel SDK test exposed a false-busy install lock on immediate retry.
  Assembly now reuses the explicit-unlock guard from ADR 0034. Its deterministic
  shared-description test and the previously failing assembly test passed in
  the final suite. No contention retry/bypass was added.

## Native QA observed

Fresh, isolated QA root: `/tmp/android-hub-ux-lifecycle-20261117/data`.
Two blank Google Play API 36 AVDs, 1536 MiB / 1 CPU each, ports 5640/5642;
portrait 480×854 and landscape 854×480. Third registry-only stopped fixture
supported selector inspection and was never started. No account/game data copied.

Observed through CUA screenshots and actions on a 2360×1704 physical-pixel
window (Retina default logical window, including title bar):

- Stopped state, targeted Start and the Starting engine elapsed label.
- Both Android Home screens live in their respective portrait/landscape shapes.
- Explicit second-slot selection from three devices; toolbar/inspector selection
  followed the chosen slot.
- Tapping Phone on the portrait device opened Phone only there; a drag on the
  landscape device opened its app drawer. Focus border/header changed to that
  device. Keyboard `q`/`a` injection was attempted but no typed result was
  observed; **not counted as keyboard acceptance**.
- Sidebar collapse/restore, configuration panel opening/closing and VI/EN labels.
- Found staggered card headers and a clipped retry button; fixed in source.
- Native AX exposed window/menu controls but not Iced content controls. This is
  evidence for the pending US-040 accessibility audit, not accessibility success.

CUA getApp took 1161.9 seconds to return. Later coordinate input repeatedly
reported `noWindowsAvailable` even though AX/screenshots and the process remained
available; raising the window did not resolve it. The session was not restarted
just because this observation/control channel failed.

## Real two-device API verification

The first probe incorrectly used `adb -t <transport> emu avd name`. Real multi-
emulator testing returned `more than one emulator detected; use -s`. Fixed the
console query to use serial selection and recheck transport afterward; all guest
shell reads remain pinned to transport. This is why initial UI readiness stayed
unknown despite live frames; it was not treated as a network or boot failure.

A temporary Rust example exercised the corrected public API against both QA
AVDs. It observed completed boot for QA Portrait PID 16099 and QA Landscape PID
16791, rejected PID 1 for each, then used stop_all_with_progress:

```
QA Portrait: Waiting
QA Landscape: Waiting
QA Portrait: Stopping
QA Portrait: Stopped
QA Landscape: Stopping
QA Landscape: Stopped
Verified QA stop result: 2 stopped, zero engines remain
```

The example source was removed after verification. This proves backend readiness
and stop progress with two real guests; it does not prove the final native Quit
progress screen or reconnect UI.

## Session interruption and cleanup

During the first native run, the personal Hub PID 2295 and engine PID 2455 ended;
the personal engine log showed a graceful shutdown. Both QA engines also ended
around 11:04:49 with netsim cancellation/crash-reporter lines. No stop/kill command
was issued for the personal Hub or its guest in this task. The initiator and
causality were not established; do not attribute this to a confirmed Hub fix or
claim stability. The personal session was not automatically restarted.

The second QA run was explicitly stopped by the backend test above. After zero
QA engines were verified, QA Hub PID 15286 was terminated by exact executable
identity because CUA input was unavailable. Process exit was verified. The QA
bundle/blank fixture data, temporary example, task logs and target build cache
were removed. The main app, personal runtime, SDK and backups were retained.

## Remaining acceptance

US-033/034/035 remain in_progress. Still needed: inspect the final layout/readiness
binary, minimum window and long labels, manual zoom/edge gestures, successful
keyboard routing, empty-slot/search states, display reconnect and native Quit
progress with two guests. Do not infer these from geometry/unit/API tests.
US-036–042 remain planned and the complete US-033→042 goal stays active.
