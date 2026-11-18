# US-024 — Chọn SDK đã cài từ Hub

- **Status**: implemented
- **Owner**: Codex
- **Created**: 2026-11-13
- **Risk lane**: normal
- **Related**: [ADR 0022](../decisions/0022-sdk-preferences.md)

## Contract

- Panel Thiết lập SDK mở được cả khi chưa chọn/có thiết bị; nhập đường dẫn tuyệt đối, kiểm tra và lưu SDK có sẵn.
- Ưu tiên ANDROID_SDK_ROOT, rồi hub-settings.json trong runtime, rồi đường dẫn macOS mặc định. Không sửa biến môi trường từ app.
- SDK candidate phải có các thành phần preflight tạo máy. Canonicalize trước lưu; ghi atomic, quyền 0600, không ghi file vào SDK.
- Chỉ lưu sau khi lock và kiểm tra tất cả profile dừng/port rảnh. Nếu runtime còn AVD/archive, chỉ cho chọn lại SDK hiện tại từ preference authoritative; chưa migrate dữ liệu sang SDK khác.
- Preference lỗi không làm desktop panic. UI báo lỗi; start/create từ chối dùng cấu hình lỗi. Runtime trống có thể lưu lại preference; khi có AVD và không xác định được SDK cũ, cần sửa cấu hình gốc trước, không đoán SDK thay thế.
- Phiên Hub cũ phát hiện preference đã đổi khi start/create thì yêu cầu mở lại. Callback save thất bại không đổi backend; save thành công cập nhật path rồi refresh snapshot trước thao tác tiếp.

## Out of scope

Download/install SDK, tự chấp nhận license, Java picker, migration AVD/image, thay SDK đang chạy.

## Verify

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
cargo test --locked -p android-hub-infrastructure configure_installed_sdk_persists_without_touching_guest_data -- --ignored --nocapture
```

Native smoke: mở panel khi danh sách trống, nhập path sai/đúng, lưu SDK và mở lại Hub; kiểm tra UI không cắt chữ. Chưa có native smoke, giữ in_progress.

## Evidence

[Unit tests](../reports/sdk-selection/tests.log), [clippy](../reports/sdk-selection/clippy.log), [build](../reports/sdk-selection/build.log), [SDK fixture](../reports/sdk-selection/installed-sdk.log). Test SDK chỉ ghi runtime tạm; không sửa SDK, A/B hoặc preference thật của người dùng.

57 unit tests, fmt/clippy/build/codesign pass. SDK integration test trên runtime tạm pass: lưu/đọc lại đúng SDK, từ chối candidate thiếu thành phần, guard port bận và phiên backend cũ. Native panel chưa smoke vì CUA pipe không kết nối được; không coi unit tests là xác nhận bố cục UI.

### 2026-11-16 native follow-up

Native invalid/valid path, running-device gate and save/reopen checks now passed on the isolated QA runtime: [SDK/Java report](../reports/sdk-java-native-20261116.md). Empty-registry native coverage remains outstanding.

CUA hoạt động trở lại; smoke một phần và sửa scrollbar clipping được ghi tại [UI evidence](../reports/ui-user-validation.md). Không suy ra các thao tác chưa thực hiện là đạt; story vẫn in_progress theo phần kiểm chứng còn thiếu.

## Completion audit — 2026-11-16

Empty-registry native smoke now passed: open settings, invalid/valid SDK, save, quit/reopen and layout ([report](../reports/sdk-selection/empty-native-20261116.md)). Populated and running-device cases were verified in the earlier native report. Saved file has mode 0600 and the empty registry remains unchanged.

Installed-SDK integration rerun passed ([log](../reports/sdk-selection/final-integration-20261116.log)): persisted selection, candidate rejection, busy-port gate and stale backend detection. Shared fmt, 111 workspace tests / 20 opt-in ignored, clippy, app build and strict codesign checks passed in the preceding verification on unchanged application source ([tests](../reports/java-selection/final-tests-20261116.log), [clippy](../reports/java-selection/final-clippy-20261116.log), [build](../reports/java-selection/final-build-20261116.log)). Tests cover settings corruption/atomic repair, switching restrictions, and success/failure callbacks. All Verify commands and required native checks are now satisfied; earlier incomplete notes are historical. SDK downloading and guest migration remain outside this story.
