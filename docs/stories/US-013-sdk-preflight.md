# US-013 — Báo thiếu thành phần SDK trước tạo/mở máy

- **Status**: implemented
- **Created**: 2026-11-13
- **Owner**: Codex
- **Risk lane**: normal

## Contract

- Tạo máy kiểm tra emulator, adb, avdmanager, Java và các file image Google Play ARM64 API 36 trước avdmanager; liệt kê toàn bộ mục thiếu trong một lỗi có hướng dẫn.
- Start kiểm tra emulator/adb trước tạo launcher, không bắt cài Java/avdmanager để mở máy đã có.
- Không tự tải image, nhận license, thay SDK hoặc chmod công cụ.
- Đường dẫn Java được chọn nhất quán giữa preflight và avdmanager: P0_JAVA_HOME, nếu không có dùng JBR Android Studio hiện tại.

## Verify

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --locked -p android-hub-infrastructure installed_sdk_prerequisites -- --ignored --nocapture
cargo clippy --workspace --all-targets --locked -- -D warnings
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Read-only kiểm tra môi trường SDK hiện có và fixtures thiếu file/quyền execute; không tạo guest để kiểm tra lỗi thiếu SDK. Đây là phần báo lỗi của setup; cài đặt có resume/rollback vẫn thuộc task riêng.

## Evidence

- 28 unit tests pass; 4 integration opt-in ignored trong suite thường. Fixture kiểm tra đầy đủ danh sách thiếu, phân biệt create/start, file rỗng và công cụ thiếu quyền execute.
- `installed_sdk_prerequisites` pass trên SDK host hiện có, chỉ đọc metadata, không chạy công cụ hay tạo guest.
- [Hướng dẫn setup](../SETUP.md) ghi đường dẫn, yêu cầu SDK/Java và các giới hạn hiện tại. Đây chưa phải trình cài tự động.
- fmt/clippy -D warnings/build/codesign pass. [Clippy](../reports/setup/clippy.log), [build](../reports/setup/build.log). Không thay widget; lỗi qua notice hiện có. Smoke UI tổng thể vẫn pending riêng.
