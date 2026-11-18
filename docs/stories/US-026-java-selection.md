# US-026 — Chọn Java home trong Hub

- Status: implemented
- Created: 2026-11-13
- Related: US-025, ADR 0027

## Contract

- J1: lưu Java optional, tương thích schema cũ, env override rõ ràng; launcher tạo AVD dùng lựa chọn đã lưu.
- J2: kiểm tra Java và SDK tools trước lưu, Hub lock/stopped/port gates, atomic write; lỗi không thay cấu hình, SDK selection giữ Java.
- J3: UI cho sửa/lưu Java và thông báo lỗi/busy/override; native layout và interaction được kiểm chứng.

## Verify

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --locked -p android-hub-infrastructure configure_java_persists_validated_home_and_preserves_settings_on_failure -- --ignored --nocapture
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Native: mở Thiết lập SDK, xem Java path/scroll, invalid path không lưu, valid Java save/reload; chỉ dùng runtime thử khi thay preferences.

## Evidence

- [Tests](../reports/java-selection/tests.log): 79 đạt, 15 ignored mặc định. [Integration SDK/JBR thật](../reports/java-selection/installed-sdk.log) đạt trong temp runtime, không guest.
- [Clippy](../reports/java-selection/clippy.log), [build](../reports/java-selection/build.log).
- CUA getApp Android Hub báo `Sky Computer Use native pipe startup failed`; J3 native smoke chưa đạt. Không yêu cầu người dùng kiểm tra lại trong lúc họ chưa tiện benchmark.

### 2026-11-16 native follow-up

Native invalid/valid Java path and save/reopen checks now passed on the isolated QA runtime: [SDK/Java report](../reports/sdk-java-native-20261116.md). All Verify commands were rerun successfully on 2026-11-16: fmt, workspace tests (111 passed / 20 opt-in ignored), clippy, the installed SDK/JBR integration test (1 passed), app build and strict codesign verification. Logs: [tests](../reports/java-selection/final-tests-20261116.log), [clippy](../reports/java-selection/final-clippy-20261116.log), [integration](../reports/java-selection/final-integration-20261116.log), [build](../reports/java-selection/final-build-20261116.log). The launcher reads `java_home()` into `JAVA_HOME`; invalid-path persistence and SDK preservation are covered by the integration test.

CUA hoạt động trở lại; smoke một phần và sửa scrollbar clipping được ghi tại [UI evidence](../reports/ui-user-validation.md). Các giới hạn của lượt smoke ban đầu đã được bổ sung bằng kết quả native và Verify ở trên; phạm vi installer US-025 vẫn riêng.
