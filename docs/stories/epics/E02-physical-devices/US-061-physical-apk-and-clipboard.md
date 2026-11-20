# US-061 — Cài APK và clipboard đúng điện thoại

- **Status**: implemented
- **Owner**: Android Hub maintainer
- **Created**: 2026-11-19
- **Risk lane**: high-risk
- **Related**: [Epic E02](README.md), [ADR 0060](../../../decisions/0060-physical-device-sessions.md)
- **Depends on**: US-057, US-058; clipboard cần US-060
- **Phase**: MVP

## Context

Là người dùng, tôi muốn cài APK và gửi clipboard với đích rõ ràng khi Hub có cả máy thật và máy ảo.

## Contract (acceptance criteria)

- [x] Picker/drop APK cho phép chọn thiết bị ready, hiển thị tên/model trước xác nhận và giữ nguyên đích; không tự chọn máy khác khi rút cáp.
- [x] Kiểm tra lại transport/session trước install và khi nhận kết quả. Mất kết nối giữa install báo kết quả chưa rõ, không tự retry hoặc báo thành công.
- [x] Thể hiện lỗi chữ ký, version, ABI, quyền OEM và bộ nhớ bằng EN/VI; không tự uninstall app có dữ liệu để khắc phục lỗi.
- [x] Clipboard chỉ gửi theo click và chỉ báo thành công khi có xác nhận phù hợp. Không tự đồng bộ/ghi log clipboard hoặc nội dung tài khoản.
- [x] Test hai đích emulator/physical, hủy dialog, rút/cắm lại trước xác nhận và sau gửi; dùng APK QA riêng không ghi đè app cá nhân.

## Out of scope

File manager, đồng bộ clipboard nền, xóa app người dùng hoặc backup dữ liệu APK.

## Affected areas

APK/clipboard infrastructure, target selection/dialog, navigation bar và i18n.

## Implementation notes

Tái sử dụng UI xác nhận hiện có nhưng thay gate AVD/PID bằng gate đúng loại thiết bị; không bỏ kiểm tra đích để tái sử dụng code.

Hạ tầng đã triển khai:
- Install truyền APK qua stdin của `cmd package install -R --user current -S ...`
  trên smart socket gắn transport; `-R` chặn thay ứng dụng có sẵn. Chỉ size dạng số
  đi vào command; đường dẫn và nội dung APK không được chèn vào shell.
- Kiểm tra USB trước/sau tác vụ, timeout hữu hạn; lỗi transport sau khi gửi được xem là
  kết quả chưa rõ. Không tự retry, uninstall, downgrade hoặc cấp runtime permissions.
- Clipboard SET có sequence, chờ ACK rồi GET và so sánh nội dung trong bộ nhớ.
  ACK scrcpy một mình không chứng minh set thành công. Tắt paste/autosync, không
  log text; giới hạn 64 KiB. Response lỗi làm đóng control để tránh lệch giao thức.
- UI đã nối picker/drop APK với đích emulator/physical có session riêng và thêm
  nút cài APK, gửi clipboard trên thanh điều khiển điện thoại. Thông báo có EN/VI;
  nội dung clipboard được che trong Debug. Native QA đã hoàn tất; xem Evidence bên dưới.

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

Nghiệm thu ngày 2026-11-20 trên Samsung SM-G996B Android 15 và emulator Android 16
trong root QA riêng:

- Picker hiển thị đúng model/tên máy và file trước xác nhận. Hủy đóng dialog và
  không cài fixture. Cài thành công trên cả điện thoại và emulator, đối chiếu
  bằng `pm path local.androidhub.inputqa`.
- Người dùng kéo `input-qa.apk` từ Finder, xác nhận danh sách có cả US061 QA
  Emulator và SM-G996B · USB, rồi chọn Samsung. UI báo thành công; đối chiếu
  package xác nhận emulator không bị cài nhầm.
- Cài lại fixture trên Samsung bị từ chối với lỗi ứng dụng tồn tại; không ghi đè.
  Parser Samsung từ chối dấu `-` cuối lệnh nên đã bỏ đối số đường dẫn; Android
  đọc stdin khi có `-S` và không có đường dẫn, theo
  [AOSP Android 15](https://github.com/aosp-mirror/platform_frameworks_base/blob/android-15.0.0_r1/services/core/java/com/android/server/pm/PackageManagerShellCommand.java).
- Rút/cắm USB trước xác nhận làm xác nhận cũ bị từ chối; Hub yêu cầu chọn lại APK.
  Rút cáp khi đang cài fixture 256 MiB báo Outcome unknown đúng đích Samsung;
  cắm lại không tự retry. Fixture không xuất hiện trên điện thoại sau ca này.
- Clipboard explicit-click dùng văn bản QA do Hub tạo, báo thành công sau ACK
  và read-back; không tự dán hoặc mở ứng dụng. Thông báo đang hiển thị đổi đúng
  khi chuyển EN/VI.
- Regression tests bao phủ ID trùng giữa AVD/USB, boot chưa ready, AVD stale nhưng
  điện thoại ready, hủy, reconnect trước xác nhận, kết quả cũ đến sau reconnect,
  phân loại lỗi chữ ký/version/ABI/quyền/dung lượng và bản dịch. Clipboard từ chối
  read-back thiếu, sai loại hoặc quá lớn. Không gây các lỗi cài đặt bằng cách sửa
  app cá nhân hoặc làm đầy bộ nhớ điện thoại.
- `scripts/check.sh` đạt: 304 Rust tests, 36 Python tests, fmt, clippy và docs
  checker; 25 integration tests được bỏ qua theo mặc định. Test fixture cần quyền
  socket loopback/metadata ngoài sandbox. `git diff --check` đạt.

Giới hạn quan sát: picker macOS có một lượt Open bị disabled sau Go To;
hủy/mở lại và chọn file trực tiếp hoạt động bình thường.

Đã đóng QA, dừng emulator thử, gỡ fixture trên Samsung, xóa bundle/data/APK thử
và cache build. SDK và dữ liệu thiết bị người dùng được giữ nguyên.

## Preview handoff

Ngày 2026-11-20 đã cập nhật `~/Applications/Android Hub Preview.app`, build
`20261120.2`, từ working tree US-061 đã kiểm chứng. Bundle giữ identifier
`local.androidhub.preview` và data root Preview riêng; Latest không thay đổi.
Chữ ký ad-hoc verify đạt; SHA-256 executable đã cài khớp bản build. Người dùng
đã chấp nhận US trước khi yêu cầu commit/push.
