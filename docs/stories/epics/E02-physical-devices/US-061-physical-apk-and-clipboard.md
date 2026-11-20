# US-061 — Cài APK và clipboard đúng điện thoại

- **Status**: in_progress
- **Owner**: Android Hub maintainer
- **Created**: 2026-11-19
- **Risk lane**: high-risk
- **Related**: [Epic E02](README.md), [ADR 0060](../../../decisions/0060-physical-device-sessions.md)
- **Depends on**: US-057, US-058; clipboard cần US-060
- **Phase**: MVP

## Context

Là người dùng, tôi muốn cài APK và gửi clipboard với đích rõ ràng khi Hub có cả máy thật và máy ảo.

## Contract (acceptance criteria)

- [ ] Picker/drop APK cho phép chọn thiết bị ready, hiển thị tên/model trước xác nhận và giữ nguyên đích; không tự chọn máy khác khi rút cáp.
- [ ] Kiểm tra lại transport/session trước install và khi nhận kết quả. Mất kết nối giữa install báo kết quả chưa rõ, không tự retry hoặc báo thành công.
- [ ] Thể hiện lỗi chữ ký, version, ABI, quyền OEM và bộ nhớ bằng EN/VI; không tự uninstall app có dữ liệu để khắc phục lỗi.
- [ ] Clipboard chỉ gửi theo click và chỉ báo thành công khi có xác nhận phù hợp. Không tự đồng bộ/ghi log clipboard hoặc nội dung tài khoản.
- [ ] Test hai đích emulator/physical, hủy dialog, rút/cắm lại trước xác nhận và sau gửi; dùng APK QA riêng không ghi đè app cá nhân.

## Out of scope

File manager, đồng bộ clipboard nền, xóa app người dùng hoặc backup dữ liệu APK.

## Affected areas

APK/clipboard infrastructure, target selection/dialog, navigation bar và i18n.

## Implementation notes

Tái sử dụng UI xác nhận hiện có nhưng thay gate AVD/PID bằng gate đúng loại thiết bị; không bỏ kiểm tra đích để tái sử dụng code.

Hạ tầng đang triển khai:
- Install truyền APK qua stdin của `cmd package install -R --user current -S ... -`
  trên smart socket gắn transport; `-R` chặn thay ứng dụng có sẵn. Chỉ size dạng số
  đi vào command; đường dẫn và nội dung APK không được chèn vào shell.
- Kiểm tra USB trước/sau tác vụ, timeout hữu hạn; lỗi sau khi gửi được xem là
  kết quả chưa rõ. Không tự retry, uninstall, downgrade hoặc cấp runtime permissions.
- Clipboard SET có sequence, chờ ACK rồi GET và so sánh nội dung trong bộ nhớ.
  ACK scrcpy một mình không chứng minh set thành công. Tắt paste/autosync, không
  log text; giới hạn 64 KiB. Response lỗi làm đóng control để tránh lệch giao thức.
- UI đã nối picker/drop APK với đích emulator/physical có session riêng và thêm
  nút cài APK, gửi clipboard trên thanh điều khiển điện thoại. Thông báo có EN/VI;
  nội dung clipboard được che trong Debug. Native QA và nghiệm thu vẫn chưa hoàn tất.

Nguồn giao thức: [scrcpy Controller 5.0.1](https://github.com/Genymobile/scrcpy/blob/v5.0.1/server/src/main/java/com/genymobile/scrcpy/control/Controller.java),
[AOSP PackageManagerShellCommand](https://github.com/aosp-mirror/platform_frameworks_base/blob/master/services/core/java/com/android/server/pm/PackageManagerShellCommand.java).

## Verify

Lệnh dành cho lúc triển khai story, chạy từ repo root; không build/test Rust chỉ để viết US.

```bash
bash scripts/check.sh
git diff --check
```

Thêm test hồi quy đúng các lỗi/ranh giới nêu trên, chạy native QA tương ứng trên fixture và điện thoại được chỉ định. Khi sửa bundle/helper, build và verify chữ ký bằng script hiện có. Lệnh test thuần đạt không thay thế ca thiết bị thật.

## Evidence

2026-11-20: hạ tầng bước đầu, 23 tests nhóm `physical::` đạt (gồm phân loại
kết quả APK, ACK/read-back clipboard và hủy clipboard chờ khi đổi capture/đóng
phiên). Test socket chạy với quyền bind loopback; không thao tác thiết bị thật.
UI đã được nối trong bản triển khai tiếp theo. Chưa QA native và chưa nghiệm thu
các acceptance criteria; commit này lưu tiến độ, không phải bản release.
Các bản Latest/Preview đã cài cho người dùng không bị thay đổi.

Kiểm tra trước commit: `bash scripts/check.sh` đạt (297 Rust tests, 36 Python
tests, fmt, clippy và docs checker); 25 integration tests vẫn được bỏ qua theo
mặc định. `git diff --check` đạt. Đã sửa khóa dịch trùng và nút clipboard bị
thêm lặp trong navigation. Chưa kiểm chứng luồng mới trên thiết bị thật.
