# US-014 — Cấu hình khung màn hình dọc/ngang

- **Status**: implemented
- **Created**: 2026-11-13
- **Owner**: Codex
- **Risk lane**: normal

## Contract

- Chọn dọc/ngang khi máy dừng, lưu bằng width/height hiện có; không thêm trường orientation dư thừa vào registry.
- Đổi độ phân giải giữ chiều đang chọn. Ba cặp kích thước hỗ trợ cả hai chiều, vẫn từ chối kích thước tùy ý.
- Mapping input dựa trên chiều frame thực tế và hai cạnh profile; không đảo sai khi profile vốn đã ngang.
- Đây là hình dạng framebuffer lúc khởi động, không ép game bỏ khóa orientation hoặc giả lập cảm biến xoay.

## Verify

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Smoke guest thử: lưu từng chiều, boot, xác nhận framebuffer và tap đúng tọa độ; đổi resolution giữ chiều; app có thể tự chọn orientation. Chưa smoke thì giữ in_progress.

## Evidence

- 31 unit tests pass; 4 integration opt-in ignored. Domain kiểm tra sáu kích thước, reject sai cạnh, mapping theo chiều frame. Desktop kiểm tra đổi resolution giữ ngang, draft không tự lưu và chặn khi trạng thái stale.
- [ADR 0014](../decisions/0014-framebuffer-orientation.md) ghi contract profile và kích thước stream.
- Guest smoke dùng `ANDROID_HUB_TEST_LANDSCAPE=1 ANDROID_HUB_TEST_APK=/tmp/hub-apk-fixture-20261113/test.apk cargo test --locked -p android-hub-infrastructure installs_on_disposable_guest -- --ignored --nocapture`.
- Guest smoke pass 139.02 giây: framebuffer báo Physical size 854x480, APK local cài được, backup/restore rồi boot và package còn. Guest đã dừng và root tạm đã dọn. [Log](../reports/orientation/guest-smoke.log).
- fmt/clippy/build/codesign pass ([clippy](../reports/orientation/clippy.log), [build](../reports/orientation/build.log)). Chưa có smoke tap/đổi orientation qua UI, giữ in_progress. Không thay đổi A/B.

## Native landscape follow-up — 2026-11-16

QA Android3 saved Ngang, registry1280×720, booted via Hub and rendered landscape.
ADB read-only verified physical1280×720. Native drawer drag, app-icon tap, fixture
touch and zone3 wheel passed. [Evidence](../reports/orientation/native-qa-20261116.md).
Resolution-change-preserves-landscape and return-to-portrait checks remain pending.

Native resolution follow-up: while QA Android3 was stopped, selected the small
resolution pair and saved. Registry854×480 and UI Ngang confirmed orientation
preservation. Return-to-portrait was interrupted by an unexpected selection
change; no further save/start performed. See native report.

## Completed native verification — 2026-11-16

Corrected resolution labels to follow selected orientation, sharing dimensions
with the save path. QA reopened with current build. Saved portrait480×854, booted
via Hub, ADB verified physical size; drawer drag, icon tap and fixture touch/wheel
passed. Combined with previous landscape save/boot/input and resolution retention,
all bounded native checks in this story are complete.111 workspace tests passed,
20 opt-in ignored; fmt/clippy/build/codesign passed. See native report and
`docs/reports/orientation/resolution-label-*-20261116.log`.
