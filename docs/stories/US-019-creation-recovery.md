# US-019 — Recovery tạo máy bị gián đoạn

- **Status**: implemented
- **Owner**: Codex
- **Created**: 2026-11-13
- **Risk lane**: high-risk
- **Related**: [ADR 0019](../decisions/0019-creation-journal.md)

## Contract

- Ghi intent trước gọi avdmanager; ghi ready sau khi SDK và cấu hình hoàn tất, trước registry.
- Khi mở lại hoặc thực hiện thao tác có lock, intent preparing được lưu biên nhận incomplete; không đăng ký, di chuyển hay xóa file dở. avdmanager có thể vẫn sống sau khi Hub bị kill.
- Ready được hoàn tất đăng ký nếu registry là before/after dự kiến và metadata AVD, config/ini vẫn khớp. Kiểm tra process và port trước recovery ready.
- Không tự ghi đè registry đã chỉnh ngoài Hub hoặc dữ liệu khác. Journal giữ lại khi có xung đột.
- Biên nhận nằm trong creation-history. Chưa có UI xem biên nhận tạo máy hoặc dọn file tạo dở.

## Out of scope

Khôi phục payload sau mất điện, tiếp tục avdmanager giữa chừng, xóa file dở, migrate AVD, tự sửa conflict do bên ngoài.

## Verify

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Fixture: preparing, ready, registry committed, thay đổi dữ liệu/registry. Production initialize recovery với busy port và guest SDK disposable tạo/boot đã pass; không chạm A/B.

```bash
cargo test --locked -p android-hub-infrastructure startup_creation_recovery_waits_for_busy_port -- --ignored --nocapture
cargo test --locked -p android-hub-infrastructure creation_process_kill_recovery -- --ignored --nocapture
ANDROID_HUB_TEST_APK=/tmp/hub-apk-fixture-20261113/test.apk cargo test --locked -p android-hub-infrastructure installs_on_disposable_guest -- --ignored --nocapture
```

Lệnh guest cần APK fixture do scripts/build-test-apk.sh tạo. Hai lệnh recovery cần quyền đọc process và bind port localhost; SIGKILL chỉ nhắm child test do chính test spawn.

## Evidence

- 50 unit tests pass; fmt, clippy, app build và codesign pass. [Tests](../reports/creation/tests.log), [clippy](../reports/creation/clippy.log), [build](../reports/creation/build.log).
- [Startup recovery](../reports/creation/startup-recovery.log): public initialize từ chối khi port bận, giữ journal/registry, hoàn tất đúng một lần khi port rảnh.
- [Guest lần đầu](../reports/creation/guest-smoke-first-attempt.log): SDK tạo, boot và cài APK đạt; backup bị chặn vì port chưa bind lại được ngay sau stop. Test bổ sung chờ có giới hạn bằng chính ensure_stopped; không bỏ gate offline. Đây không phải một lượt integration pass.
- Native CUA kiểm tra lại vẫn lỗi “Sky Computer Use native pipe startup failed”.
- Không cam kết payload đã fsync toàn bộ hoặc an toàn mất điện. Chưa có UI xem/dọn biên nhận incomplete.

- [Guest smoke](../reports/creation/guest-smoke.log) pass 131.16 giây: tạo SDK, boot, cài APK, backup/restore, gỡ/undo và boot lại còn package. Guest tạm của cả lượt thành công và lượt thất bại đã được dọn sau khi dừng; không thao tác A/B. Đây là happy path SDK cùng fixture recovery, chưa phải kill Hub thực giữa mọi syscall.

- [SIGKILL drill](../reports/creation/process-kill-recovery.log): sáu child process riêng giữ lock thật, bị kill tại intent/partial/ready_temp/ready/registry/archived; xác nhận signal 9, lock được thả và public initialize chạy hai lần đều đúng. Partial journal .new không được nâng thành ready; file fixture giữ nguyên. Có guard dọn child nếu assert/timeout thất bại.
- [Unit suite mới](../reports/creation/kill-unit-tests.log): 50 pass; [clippy](../reports/creation/kill-clippy.log), fmt và codesign pass. Thay đổi lượt này chỉ trong cfg(test), không thay runtime app.
- Phạm vi SIGKILL là worker gọi các bước transaction thực với AVD metadata fixture; không phải GUI Hub + avdmanager thật bị kill giữa syscall và không mô phỏng mất điện. Guest SDK happy path là bằng chứng riêng. Backend story hoàn tất theo contract; UI biên nhận/dọn dữ liệu vẫn là việc riêng chưa hoàn tất.
