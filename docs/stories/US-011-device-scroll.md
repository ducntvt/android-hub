# US-011 — Cuộn chuột/trackpad trong màn hình Android

- **Status**: in_progress
- **Created**: 2026-11-13
- **Owner**: Codex
- **Risk lane**: normal

## Contract

- Wheel nằm trong canvas gửi đến đúng session thiết bị dưới con trỏ; ngoài canvas để UI Hub cuộn bình thường.
- Gửi ACTION_SCROLL với tọa độ guest đúng dưới con trỏ host qua helper giữ kết nối (ADR 0036), không giả lập swipe hoặc tap.
- Host giữ thang Lines: 120 đơn vị mỗi nấc; Pixels: 3 đơn vị mỗi pixel và phần lẻ giữa sự kiện. Helper đổi 120 đơn vị host thành 1 đơn vị trục Android. Giới hạn 10 nấc/event; bỏ NaN/infinity.
- Không phát wheel trong khi kéo touch; không replay input sau reconnect.
- Kiểm tra chiều cuộn và hiệu ứng thật trên guest thử trước khi đánh dấu implemented.

## Verify

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
bash scripts/build-android-hub.sh
```

Smoke native: hai thiết bị, con trỏ ở A/B/outside, wheel/trackpad theo cả hai trục, thao tác kéo không phát wheel. CUA chưa hoạt động thì story giữ in_progress.

## Evidence

- 26 unit tests pass (10 desktop, 1 domain, 15 infrastructure; 3 integration opt-in ignored). Test wheel kiểm tra scaling/remainder, NaN, clamp, hover B, outside, touch drag.
- CUA inventory vẫn lỗi `Sky Computer Use native pipe startup failed`; chưa kiểm chứng cuộn thật trong Android, hai máy và natural scrolling. Story giữ in_progress.
- [ADR 0012](../decisions/0012-device-wheel-input.md) ghi lựa chọn protocol và giới hạn.
- fmt/clippy -D warnings/build/codesign pass. Logs: [clippy](../reports/input/scroll-clippy.log), [build](../reports/input/scroll-build.log). App đã đóng gói; chưa reload app đang mở.

### Native failure reproduced — 2026-11-16

QA Android 3, API 36/Emulator 37.2.12: native drag opens app drawer and scrolls Settings. Native wheel down (1 and 3 pages) at top, down/up after drag do not move Settings. No stream error shown. Hub config panel scroll works independently. `getevent -lp` reports multitouch devices + keyboard, no REL_WHEEL-capable mouse/tablet. Read-only `getevent -lt` during UI wheel shows no kernel input events. See [devices](../reports/input/qa-wheel-devices-20261116.txt), [trace](../reports/input/qa-wheel-events-20261116.txt). This contradicts native completion; story stays in_progress.

Source investigation: current upstream [user_event_mouse_wheel](https://android.googlesource.com/platform/external/qemu/+/refs/heads/emu-main-dev/android-qemu2-glue/qemu-user-event-agent-impl.c) returns early when VirtioMouse and VirtioTablet are both disabled. [initialize_virtio_input_devs](https://android.googlesource.com/platform/external/qemu/+/refs/heads/emu-main-dev/android-qemu2-glue/main.cpp) chooses mouse/tablet OR multitouch devices. This supports an engine input configuration hypothesis, not proof of the exact installed binary implementation. Do not blindly enable mouse/tablet: it may remove touch required for games. Next: evaluate supported combined input configuration or a deliberate, tested scroll mapping; preserve touch/release and reconnect guarantees.

### Combined pointer/touch experiments

Tried tablet + explicit multitouch then dual-mode mouse + explicit multitouch, via QA builds only. Both boot and preserve native touch. Tablet delivers REL_WHEEL but no Android hover pointer; dual-mode scrolls both directions but targets the cursor at center rather than the pointer coordinates. Rejected for incorrect target behavior; launcher source reverted, main bundle unchanged. [ADR 0035](../decisions/0035-combined-pointer-touch.md) records evidence and next transport candidate. Do not mark native wheel complete from the successful center-list scroll alone.

### Framework bridge candidate — 2026-11-16

Source now routes wheel through a persistent guest-bound ACTION_SCROLL helper with explicit coordinates (ADR 0036). The host scaling contract stays 120 units per notch; the helper converts these to Android scroll-axis units. Frames and serial input run as separate async futures. This supersedes the candidate protocol pointer move, without adding fake taps or swipes. Native verification is pending; main app is not yet updated with this candidate.

Candidate verification: workspace tests pass (102 passed, 20 opt-in tests ignored), fmt check, clippy -D warnings, and cargo build -p android-hub --locked pass. Logs: docs/reports/input/framework-scroll-{tests,clippy,build}-20261116.log. The app bundle packaging/native smoke gate remains pending. target/debug/android-hub now contains this framework-scroll candidate, not the rejected pointer flags; do not copy to the main bundle before native validation.

Native candidate update: fixed unsupported adb transport lookup; QA Settings wheel down/up and subsequent touch drag pass. [Evidence](../reports/input/framework-scroll-native-20261116.md). Nested targeting, horizontal/two-device and reconnect/teardown checks remain pending.

Independent-zone native fixture confirms exact local coordinates at upper/lower targets and no synthetic touch events. Horizontal CUA page events produced no new guest event; unresolved. See expanded native report.

Two-guest native smoke passed for vertical wheel: correct hovered guest despite other sidebar selection, exact zone, unchanged peer and no outside-canvas events. Normal selection reconnect/teardown also passed. Horizontal trackpad remains pending; status remains in_progress.

### Physical trackpad confirmation — 2026-11-16

User explicitly confirmed “Số H có thay đổi” when asked to scroll horizontally with
two fingers over the colored Hub Input QA regions. This establishes real horizontal
trackpad delivery on the tested Mac/guest; CUA's earlier horizontal no-op is not
proof of a product failure. It does not by itself verify every horizontal direction
or natural-scroll preference. Native vertical direction, two guests, target zones,
outside routing, touch after wheel, and normal reconnect/teardown are documented
in the native report. Remaining detailed gesture/in-flight-failure checks keep this
story in_progress; this does not prevent packaging the verified practical fix.

Main bundle packaged and codesign verified after all Verify commands passed
(107 tests, 20 opt-in ignored). Native main launch shows the three personal profiles
and Sẵn sàng; personal guests remain stopped. Detailed native report records user
horizontal confirmation and release limitations. Full in-flight and combined
hold/scroll checks remain pending; no claim of full project completion.
