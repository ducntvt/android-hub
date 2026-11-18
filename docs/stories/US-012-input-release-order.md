# US-012 — Giữ thứ tự nhả touch khi input đầy

- **Status**: in_progress
- **Created**: 2026-11-13
- **Owner**: Codex
- **Risk lane**: normal

## Contract

- Khi queue đầy, chỉ một release chờ; release lặp cho cùng pointer không sinh thêm task.
- Input mới bị từ chối cho tới khi release chờ được enqueue, không chen một down/key/wheel trước release.
- Đổi selection hoặc single/split gửi release cho các session đang hiển thị, reset focus/gesture epoch.
- Không đổi giới hạn queue, auth, hoặc dữ liệu guest.

## Verify

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Smoke native kéo nhanh, đổi selection và single/split trên guest thử. Story giữ in_progress nếu chưa kiểm chứng thao tác thật.

## Evidence

- 27 unit tests pass, 3 integration opt-in ignored. Regression tạo queue capacity=1, enqueue down rồi release khi full, lặp 100 release, xác nhận down mới bị từ chối cho tới khi up được enqueue; kiểm tra không còn event thừa và có thể nhận down tiếp.
- Native smoke còn pending; lần inventory gần nhất trong US-011 trả lỗi native pipe. Không thao tác guest A/B để test.
- fmt/clippy -D warnings/build/codesign pass. [Clippy](../reports/input/release-clippy.log), [build](../reports/input/release-build.log). Giữ in_progress vì smoke native chưa pass.

### Reconnect follow-up

- Tìm thấy enqueue vẫn mở trong thời gian sleep/reconnect sau khi drain queue. Đã thêm ready/generation vào cùng gate và bỏ release async từ generation cũ.
- 34 unit tests pass; test mới kiểm tra clipboard/key trước lỗi bị drain, enqueue trong reconnect bị từ chối, chỉ input mới sau ready được nhận và Disconnect không bị mất khi drain.
- Đây là kiểm thử queue/state, chưa phải fault injection gRPC trên guest đang chơi. Không đọc/chuyển clipboard thật hoặc dừng guest A/B.
- Clippy -D warnings/build/codesign pass. [Clippy](../reports/input/reconnect-clippy.log), [build](../reports/input/reconnect-build.log).

## Scrolled viewport follow-up — 2026-11-16

Local Iced 0.13 scrollable source forwards the unchanged mouse event but translates
the cursor to content coordinates. Screen previously mixed translated press with
raw move, offsetting a drag when the stage was scrolled. Move now uses the same
translated cursor as press/wheel. An unavailable cursor preserves the last valid
point and release still sends up. Regression covers an 82px offset and release
after leaving the viewport. Native scrolled-drawer verification remains pending.

Verification: fmt, workspace tests (108 passed / 20 opt-in ignored), clippy
-D warnings, main app build and codesign verification passed. Main bundle on disk
contains this fix; existing running instances have not been restarted. Native
scrolled-drag recheck is still pending, so story remains in_progress. Logs in
`docs/reports/input/scrolled-cursor-*-20261116.log`.

Native follow-up: rebuilt QA bundle from the current main binary, signed and
reopened it. On QA Lifecycle, scrolled the Hub stage so the Android top was
clipped, pressed Hub Home, then dragged (1430,920) to (1430,460). The Android
app drawer visibly opened. This verifies the scrolled single-device drag fix;
it does not yet cover queue saturation or a selection change during a hold.

Split-mode follow-up: with stage still scrolled, left QA Lifecycle remained in
its app drawer. Pressed right Android3 Home, then dragged (1680,930) to
(1680,480); right drawer opened while left remained unchanged. Sidebar selection
was still QA Lifecycle. This verifies basic right-canvas routing at a different
scale and nonzero scroll offset. Both QA guests remain running; no personal
guest was operated.

## Same-PID reconnect gesture guard — 2026-11-16

Canvas state now resets on transport generation, not only profile/desktop epoch.
Touch/key/wheel messages and navigation buttons capture generation; infrastructure
validates it under the enqueue mutex. Regression skips the intermediate error
frame, reconnects with same ID/epoch, and verifies no continuation of old drag,
focus/fractional wheel reset, fresh-press routing and no input while reconnecting.
Sender regression rejects stale down/up/key/wheel after the new connection is
ready. Native active-hold fault injection remains pending.

Verification: 110 workspace tests passed / 20 opt-in ignored; fmt, clippy
-D warnings, build and codesign verification passed. Main app bundle on disk
updated; existing main/QA processes not restarted. Logs:
`docs/reports/input/gesture-generation-*-20261116.log`. Story remains in_progress
until native active-hold interruption checks are complete.

## Queued desktop input epoch — 2026-11-16

Review found DeviceInput carried connection generation but not the desktop epoch already used by clipboard. Added epoch capture to canvas and navigation messages and validation before sending. Old events are rejected after a desktop release/reset even when a split session retains its transport. Regression retains an emitted epoch-7 press, resets to epoch 8 without reconnect, verifies old drag stops and new press carries epoch 8. This is deterministic state coverage, not native fault injection.

Verification: [workspace tests](../reports/input/desktop-epoch-tests-20261116.log), [clippy](../reports/input/desktop-epoch-clippy-20261116.log), [build](../reports/input/desktop-epoch-build-20261116.log); fmt and strict codesign passed. Native interruption during active hold remains unverified; story stays in progress.

## Native hold-observation fixture — 2026-11-16

Extended tests/android-input/InputActivity.java with UP/CANCEL/MOVE counters, last touch action and Held state; previous fixture counted DOWN only and could not establish release. Built via scripts/build-input-qa-apk.sh into /tmp/hub-input-release-qa-20261116; apksigner verify and bash syntax check passed ([build](../reports/input/release-fixture-build-20261116.log)). Re-signed with the previous local QA fixture key and updated only emulator-5588 after exact AVD ID hub-1794811158106 check; adb install -r returned Success. This is fixture preparation, not Hub install UI or release-order acceptance evidence. Native controlled mid-hold split test still required.

### Controlled hold → split observation — 2026-11-16

Separate checkout `/tmp/android-hub-hold-split-20261116/checkout` adds a QA-only arm button: the next accepted press schedules the real SplitView message after 300 ms. Production release/epoch/transport logic is unchanged. Bundle uses the existing isolated QA root, targeting Android 3 (`hub-1794811158106`); personal guests are untouched.

Fixture began at Down 0 / Up 0 / Cancel 0 / Held false. After arming, native CUA click held for requested 2000 ms. Subsequent screenshot showed split layout, Down 1 / Up 1 / Cancel 0 / Held false / Last UP / Moves 0. A fresh native click in the resized canvas produced Down 2 / Up 2, Held false. This verifies balanced release and fresh input after the controlled layout transition. No intermediate screenshot or event timestamps establish the precise release timing; it is not evidence of queue saturation, reconnect interruption, or switching selection during hold. Story remains in progress for those checks.

QA build: [log](../reports/input/hold-split-build-20261116.log). Restored normal main executable via build-android-hub.sh after the experiment: [log](../reports/input/hold-split-final-build-20261116.log). The separate QA bundle retains instrumentation for reproducibility; it is not the main app.

### Controlled hold → selection observation — 2026-11-16

Reused the temporary checkout, changing only the armed callback to the real Select message for stopped QA Lifecycle (`hub-1794811024355`) after 300 ms. Packaged separately at `/tmp/android-hub-hold-select-20261116/bundle/Android Hub QA.app`; closed the split-test Hub first. No production source or personal guest changes.

Android 3 fixture baseline was Down 2 / Up 2 / Held false. Armed the callback and used native CUA click with requested duration 2000 ms in its canvas. Observed QA Lifecycle selected and stopped. Selected Android 3 again: Down 3 / Up 3 / Cancel 0 / Held false / Last UP. A fresh click then produced Down 4 / Up 4 / Held false. Thus release is balanced across this controlled selection transition and fresh input recovers after reattachment. Precise release latency was not recorded; no claim of saturated-queue or failed-transport release coverage. Story remains in progress.

Build logs: [QA](../reports/input/hold-select-build-20261116.log), [normal main restoration](../reports/input/hold-select-final-build-20261116.log). Normal app build and strict signature verification passed. Existing unit coverage is unchanged; this turn adds native evidence rather than a production code change.

### Bound reconnect establishment — 2026-11-16

Added local timeouts around screenshot establishment (8 seconds) and initial pointer release (2 seconds). Previously an unresponsive initial RPC could prevent the reconnect loop progressing; normal input already had a local deadline. Input ready remains set only after both operations succeed. ADR 0013 records timeout scope and remote-acceptance limitations.

Verification: fmt check, 113 workspace tests passed / 20 opt-in ignored, clippy with warnings denied, main bundle build and strict codesign passed. Logs: `docs/reports/input/handshake-timeout-{tests,clippy,build}-20261116.log`. Existing tests cover queue/generation guards, not a hung RPC server; native handshake timeout and active transport failure are still unverified. No running guest was restarted by this change. Story remains in progress.

### Native controlled transport interruption — 2026-11-16

Injected one error immediately after the first down RPC was acknowledged, only in a temporary QA build. Native UI showed the injected error then automatically restored the same guest canvas; fixture Down/Up advanced 4/4 → 5/5 with Held false. Fresh click advanced to 6/6, Held false. The real suspend/drain/retry/initial-release implementation handled recovery. [Detailed evidence and limitations](../reports/input/reconnect-native-20261116.md). This closes the controlled post-ack interruption case, not hung-server or saturated-queue checks; story remains in progress.

### Automated unresponsive RPC coverage — 2026-11-16

Extracted the unchanged handshake into establish_input and tested its generated tonic client with a Service that accepts requests but never returns headers. Both stream establishment and initial release return the expected timeout error (8 and 2 seconds respectively); an independent 12-second watchdog would fail a missing local bound. Recorded RPC paths prove stalled stream establishment never advances to release. Focused test passed in 10.01 seconds: [log](../reports/input/handshake-rpc-test-20261116.log). No new dependencies, server listener, guest mutation or timing override.

This covers client-side response hangs; it is not a native socket disconnect or a guest crash test. Saturated-queue native verification remains pending. Full verification logs are `docs/reports/input/handshake-rpc-{workspace,clippy,build}-20261116.log`.

Full verification passed: 114 tests, 20 opt-in ignored; fmt, clippy -D warnings, bundle build and strict codesign. Main bundle updated; already-running QA keeps its separate post-ack fault build until closed.

### Scope correction — 2026-11-16

User questioned prolonged QA and drift from the personal-use objective. No further native saturation actions were taken. The capacity-1 / paused-consumer QA bundle was opened but guest was at Home rather than the prior fixture; no saturation result is claimed. Do not continue this experiment automatically without reconciling scope with the user.

Expanded the existing automated queue test to capacities 1 and production 64, checking ordered queued events, coalesced repeated release, rejection of new down/key/wheel while release is pending, and fresh input afterward. 114 workspace tests passed, 20 opt-in ignored; fmt/clippy/build/strict signing passed. Logs: `docs/reports/input/queue-capacity-{tests,clippy,build}-20261116.log`. Main bundle restored to normal production source. Remaining stress QA is an unverified test gate, not evidence that the personal-use app lacks these features.
