# 0007 — Cài APK với máy đích và xác nhận cụ thể

- **Status**: Accepted
- **Date**: 2026-11-13
- **Deciders**: triển khai mục APK trong kế hoạch MVP
- **Supersedes**: none

## Context

Nhiều guest cùng chạy; ADB không có serial rõ ràng hoặc selection thay đổi giữa drop/confirm có thể đưa ứng dụng vào sai máy.

## Decision

Desktop lưu ID, tên, PID và đường dẫn từ file drop rồi yêu cầu xác nhận. Khi đã có một APK chờ xác nhận, các sự kiện drop tiếp theo bị từ chối và báo phải hủy lựa chọn hiện tại trước; không âm thầm thay file đang hiển thị. API window phát từng file riêng, nên nhiều file trong một lượt kéo không được gom thành một batch: chỉ file đầu được giữ để xác nhận, không tự cài bất kỳ file nào. Infrastructure nhận ID/PID/path, kiểm tra file thật không rỗng, tên AVD qua console, boot readiness và PID trước/sau kiểm tra. Giữ lock thao tác, chạy `adb -s emulator-PORT install ABSOLUTE_PATH` qua Command với args riêng, timeout 180 giây. Chỉ báo thành công khi ADB exit thành công và trả dòng Success.

## Consequences

Chưa hỗ trợ split APK, XAPK, cập nhật bằng -r, downgrade hoặc tự cấp runtime permission. Thao tác timeout có thể vẫn đã cài trong guest; UI hướng dẫn kiểm tra trước retry. Không tự gỡ package khi có lỗi. Khóa chỉ phối hợp các process Hub, không kiểm soát người dùng thao tác SDK bên ngoài.

## Alternatives considered

Tự cài ngay khi drop: dễ nhầm máy, không chọn. File picker thêm dependency chưa cần cho luồng kéo thả MVP. Cài không chỉ định serial: không phù hợp yêu cầu cách ly máy.

## References

SDK adb help đã kiểm tra tại máy; API `install`, `-s`, các flags replace/downgrade/grant-permissions. [US-007](../stories/US-007-install-apk.md).
