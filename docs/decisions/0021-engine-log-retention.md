# 0021 — Xoay log ở ranh giới khởi động

- **Status**: Accepted
- **Date**: 2026-11-13

## Context

Launcher append stdout/stderr vào `<id>.log` qua nhiều phiên. Plan yêu cầu quản lý log; hiện chưa có retention. Engine sở hữu file descriptor nên rename lúc đang chạy không chuyển được nơi ghi và truncate có thể mất thông tin lỗi đang phát sinh.

## Decision

Trước start, sau lock/stopped/SDK gates, nếu log đạt 8 MiB thì xoay sang ba archive cố định `.log.1`–`.log.3` và xóa archive cũ nhất. Kiểm tra regular file, không symlink, nlink=1 cho tất cả đường dẫn liên quan trước khi mutation. Lỗi filesystem dừng start và hiển thị lỗi; không bỏ qua để append qua đường dẫn bất thường.

## Consequences

Giới hạn số bản archive, không giới hạn bytes của một phiên dài hoặc kích thước từng archive. Không chạm log đang chạy. Retention có chủ ý bỏ dữ liệu log cũ nhất, được ghi trong SETUP. Crash giữa rename có thể để thiếu một archive; đây không phải journal guest và không ảnh hưởng AVD/registry. Không tự xoay dữ liệu lịch sử trong lần triển khai này.

## Alternatives considered

Truncate log active hoặc timer rotation cần phối hợp writer và có rủi ro mất log lúc crash. Pipe writer riêng có thể giới hạn bytes trong phiên nhưng cần thêm supervisor và lifecycle; để bước sau nếu số đo thực chứng minh cần thiết.
