# US-038 — Create-device form verification

Ngày: 2026-11-17. Phạm vi: form tạo máy, request/result contract và QA độc lập.

## Implementation

- Nút + mở form tên, image hiện hỗ trợ và kiểm tra thành phần SDK/Java/dung lượng chỉ đọc. Cấu hình nâng cao thu gọn; tùy chọn khởi động mặc định tắt.
- Kiểm tra tên/thông số và trùng tên trước tạo. Backend tự cấp ID/port dưới lock, giữ journal/recovery và mức trống tối thiểu 12 GiB. Mức này không bảo đảm dung lượng game.
- Chặn double-submit; giữ input khi lỗi và không cho tạo lại mù khi đã có dữ liệu dở hoặc kết quả chưa rõ.
- Result trả profile đã đăng ký riêng với lỗi snapshot/receipt; UI không suy ID từ thứ tự registry và không tạo lại khi start thất bại.
- [ADR 0047](../decisions/0047-create-request-and-registered-result.md).

## Automated verification

- `bash scripts/check.sh`: 162 Rust tests đạt (desktop 61, domain 4, infrastructure 97), 20 integration tests có sẵn vẫn ignored; 25 Python tests đạt. Fmt, clippy all-targets -D warnings và shell syntax đạt.
- `bash scripts/build-android-hub.sh` và `codesign --verify --deep --strict 'Android Hub.app'` đạt.
- Test bao phủ cancel/no-write, invalid request, duplicate/port allocation, double-submit, stale replies, custom-config recovery, committed result khi receipt/snapshot lỗi, create-then-start-failed, quit trong lúc tạo và giữ dữ liệu dở.
- Dependency block 0.1.6 vẫn có cảnh báo future incompatibility có sẵn.

## Native QA

App/data riêng `/tmp/android-hub-create-qa-20261117`; không sao chép tài khoản hoặc guest cá nhân.

1. + mở form với Android 1, Android 16/API 36 Google Play ARM64, thành phần môi trường và dung lượng trống. Nâng cao đóng, tự khởi động tắt.
2. Xóa tên: validation đỏ, nút Tạo bị khóa. Hủy đóng form. Đối chiếu registry vẫn rỗng, không AVD/journal/history được tạo.
3. Nhập QA Create Game, mở nâng cao, chọn RAM 4096 MiB, bật khởi động sau tạo. Xác nhận chuyển ngay sang trạng thái đang tạo, khóa các controls.
4. Tạo duy nhất `hub-1794894481228-385`, port 5584, RAM 4096 MiB. Đối chiếu registry/config.ini/registered receipt khớp; không pending journal. UI chọn đúng máy, Android Home hiển thị nhúng và sẵn sàng điều khiển.
5. Dừng mở xác nhận ghi đúng tên máy. Xác nhận xong UI hiện Đã dừng/Khởi động. Cmd+Q thoát QA; inventory không còn QA, ps theo bundle/guest ID không còn process (chỉ chính lệnh kiểm tra).

Không tuyên bố kiểm thử native mọi race/error branch; các nhánh đó được kiểm chứng bằng state/backend tests. Keyboard traversal đầy đủ thuộc US-040.

## Cleanup

QA app/guest đã dừng. Bundle và dữ liệu QA được xóa sau xác nhận process. Target được tái sử dụng cho US-039 trong cùng lượt làm việc và phải dọn khi bàn giao. Giữ app chính, runtime cá nhân, SDK/AVD/backups. Không commit.
