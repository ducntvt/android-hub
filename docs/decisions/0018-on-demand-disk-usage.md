# 0018 — Đo allocated blocks theo yêu cầu

- **Status**: Accepted
- **Date**: 2026-11-13

## Context

Backup và gỡ máy giữ dữ liệu trên disk. Người dùng cần biết dung lượng trước khi quyết định dọn dữ liệu, nhưng quét định kỳ gây I/O không cần thiết khi chơi game.

## Decision

Infrastructure cung cấp DiskUsage từ metadata runtime; desktop gọi bằng spawn_blocking khi người dùng bấm nút. Dùng st_blocks × 512 thay vì kích thước logic, deduplicate inode/device, không theo symlink. Giới hạn số mục/thời gian giữa syscall; không đọc payload. Kết quả phân nhóm là ước lượng, không phải số byte có thể thu hồi trên APFS.

## Consequences

Không thêm process du hoặc polling. Không giữ lock vòng đời trong khi đo; thay đổi runtime có thể làm phép đo lỗi hoặc không nhất quán. Callback khác command epoch bị bỏ. Không có chức năng xóa file. Smoke panel native vẫn cần thực hiện.

## Alternatives considered

Kích thước logic phóng đại sparse disk; quét định kỳ gây thêm I/O; đo block có thể thu hồi chính xác cần hiểu shared extents APFS, ngoài phạm vi hiện tại.
