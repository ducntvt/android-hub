# US-016 — Gỡ máy với dữ liệu được giữ lại

- **Status**: implemented
- **Created**: 2026-11-13
- **Owner**: Codex
- **Risk lane**: high-risk

## Contract

- UI chỉ gỡ máy đã dừng, xác nhận đúng profile; giải thích chưa giải phóng dung lượng và dữ liệu nằm trong khu lưu trữ.
- Backend lock + stopped gate + so profile đã xác nhận; chuyển AVD/registration vào thư mục riêng cùng filesystem, rồi cập nhật registry.
- Journal trước khi move, recovery pending đưa máy cũ trở lại; committed giữ kết quả và biên nhận.
- Không ghi đè file/path lạ, không xóa backup hoặc dữ liệu cũ; không áp dụng lên A/B để test.
- Có luồng đưa máy đã gỡ trở lại trước khi bật nút trong UI. Xóa vĩnh viễn/giải phóng dung lượng là bước riêng chưa triển khai.

## Phân việc

R1 backend remove + crash recovery; R2 undo + tests; R3 UI confirm/list/undo + smoke. Chỉ bật UI sau R2.

## Verify

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --locked -p android-hub-infrastructure public_remove_undo_and_startup_recovery_respect_busy_port -- --ignored --nocapture
ANDROID_HUB_TEST_APK=/tmp/hub-apk-fixture-20261113/test.apk cargo test --locked -p android-hub-infrastructure installs_on_disposable_guest -- --ignored --nocapture
cargo clippy --workspace --all-targets --locked -- -D warnings
bash scripts/build-android-hub.sh
```

Bổ sung opt-in stopped/port integration và UI guest thử ở R2/R3. Chưa đủ các phần trên thì giữ in_progress.

## Evidence R1

- Backend remove_device nhận profile đã xác nhận; kiểm tra đúng profile, lock và stopped gate trước transaction. Chưa có nút UI gọi API.
- Fault injection ở Journal/Data/Ini/Registry/Committed: pending trả dữ liệu/registry về ban đầu, committed giữ archive và registry đã gỡ. Test còn kiểm tra recovery lặp sau một move rollback và từ chối directory/registry ngoài Hub.
- Archive giữ dữ liệu thật và registration; không xóa guest bytes. Journal `.new` còn lại cạnh journal active được chuyển vào archive làm bằng chứng, không xóa. `.new` mồ côi khi chưa có journal active vẫn giữ nguyên, chưa có cleanup riêng.
- Chưa có undo R2, integration port/startup hoặc native smoke; story giữ in_progress. Không gỡ máy A/B hay máy nào trong registry người dùng.
- 37 unit tests, fmt/clippy -D warnings/build/codesign pass. [Tests](../reports/removal/tests.log), [clippy](../reports/removal/clippy.log), [build](../reports/removal/build.log).

## Evidence R2

- undo_remove_device kiểm tra profile/archive/live paths và giữ port cũ; từ chối ID/port bị chiếm, không tự đổi hoặc ghi đè.
- 39 unit tests pass (5 integration opt-in ignored). Fault injection undo ở năm checkpoint, pending quay về archive rồi retry thành công, committed giữ máy trong registry và chặn replay. [Tests](../reports/removal/undo-tests.log).
- Public remove/undo + startup integration pass: cổng bận chặn thao tác và recovery; khi rảnh initialize recovery đúng cả pending remove lẫn undo. [Log](../reports/removal/port-recovery.log).
- R3 còn thiếu: danh sách máy đã gỡ, xác nhận remove/undo, thông báo recovery và smoke UI. Chưa có nút gỡ máy trên app.
- fmt/clippy -D warnings/build/codesign pass. [Clippy](../reports/removal/undo-clippy.log), [build](../reports/removal/undo-build.log).
- Guest smoke pass 132.11 giây: API remove gỡ profile và live path, undo đưa lại, guest boot và package local vẫn còn. Guest đã dừng, root tạm đã dọn. [Log](../reports/removal/undo-boot.log). Không thay dữ liệu A/B.

## Evidence R3 (UI integration, chưa smoke)

- Settings có Gỡ máy khỏi danh sách khi máy dừng; xác nhận hiển thị tên/ID và nêu dữ liệu giữ lại, chưa giải phóng dung lượng. Profile đổi/selection đổi khiến xác nhận bị hủy hoặc từ chối.
- Sidebar có Máy đã gỡ ngay cả khi danh sách active rỗng. Panel tải/làm mới archive, chọn máy, hiển thị path và lý do không thể undo; có bước xác nhận riêng. Invalid archive được đếm, không tự sửa/xóa.
- Callback catalog có epoch; đóng panel bằng nút hoặc mở Cấu hình hủy trạng thái loading, callback cũ không ghi đè. Sau gỡ máy cuối cùng, selected/draft được xóa đúng.
- 42 unit tests pass (5 integration opt-in ignored); fmt/clippy/build/codesign pass. [Tests](../reports/removal/ui-tests.log), [clippy](../reports/removal/ui-clippy.log), [build](../reports/removal/ui-build.log).
- CUA inventory thử lại vẫn `Sky Computer Use native pipe startup failed`; chưa smoke UI thật. Thông báo lịch sử recovery remove/undo khi mở app còn thiếu (backend recovery đã chạy qua integration). Giữ story in_progress.

### R3 history follow-up

- Đã nối lịch sử remove/undo vào banner chung sau initialize và mỗi kết quả thao tác. Phân biệt committed/rollback, báo receipt/archive không đọc được. Bản cập nhật này hoàn tất phần code thông báo recovery còn thiếu ở ghi chú trước; native smoke vẫn pending.
- 44 unit tests pass, 5 integration opt-in ignored. Test receipt state/filename sai bị từ chối, nhãn bốn loại lịch sử, callback cũ hoặc đã ẩn không ghi đè. [Tests](../reports/removal/history-tests.log).
- Startup recovery integration có assert biên nhận lịch sử pass 0.60 giây. fmt/clippy/build/codesign pass. [Recovery](../reports/removal/history-recovery.log), [clippy](../reports/removal/history-clippy.log), [build](../reports/removal/history-build.log).

### Native QA undo — 2026-11-16

Đã smoke UI archive picker → xác nhận → đưa máy QA trở lại sidebar, trạng thái dừng và lịch sử đúng; registry/registration/live path cùng biên nhận restored được kiểm tra. Xem [bằng chứng](../reports/ui-user-validation.md). Luồng remove confirm/cancel/last-device và Verify toàn story còn cần hoàn tất; giữ in_progress.

### Last-device native follow-up — 2026-11-16

Cancel → remove last profile → empty sidebar → archive picker → confirmation → undo passed on an isolated synthetic fixture. Marker/registration and both receipts verified on disk. [Report](../reports/removal/last-device-native-20261116.md). Long picker labels are clipped; fix before completing this story. Native removal recovery-history coverage remains separate.

Label follow-up: replaced clipped picker with wrapped name/ID selection items; rebuilt and verified native selection/details. Current 111 tests, fmt/check/clippy/build/codesign pass; see the report above. Recovery-history native coverage remains outstanding.

## Completion audit — 2026-11-16

Native cancel/remove-last/empty-list/undo and long-label fix passed. Native startup recovery now covers pending remove, pending undo and successful UI retry, with disk identity/marker/registration/receipt checks ([report](../reports/removal/recovery-native-20261116.md)). Running-device controls were checked in the earlier QA flow.

Verify satisfied: fmt, workspace 111 passed / 20 opt-in ignored, clippy and bundle build/sign passed after the label fix ([tests](../reports/removal/label-tests-20261116.log), [clippy](../reports/removal/label-clippy-20261116.log), [build](../reports/removal/label-build-20261116.log)). Required public remove/undo/startup integration rerun passed ([log](../reports/removal/final-recovery-20261116.log)). Required disposable-guest test passed with the equivalent fresh test APK at `/tmp/hub-apk-ui-qa-20261116/test.apk`: production remove/undo, reboot, package persistence and clean teardown ([log](../reports/restore/final-guest-20261116.log)); infrastructure code has not changed since that run. Unit fault tests cover transaction boundaries, foreign paths/registry and repeated recovery.

All contract items are verified for reversible removal. Permanent deletion/space reclamation and actual host power-loss testing remain outside this story. Earlier incomplete notes are historical.
