# 0011 — Kiểm tra dung lượng cho bản sao offline

- **Status**: Accepted
- **Date**: 2026-11-13

## Decision

Dùng fs2 đã có trong workspace để đọc available_space cho user và allocation_granularity trên filesystem đích. Trước backup/restore copy, cộng kích thước logic các file làm tròn block đích, cộng 256 MiB dành cho metadata/journal và hoạt động khác. Phép cộng/làm tròn kiểm tra overflow. Lỗi đọc filesystem dừng thao tác, không coi là dung lượng vô hạn.

Không dựa vào dung lượng allocated của sparse file hoặc APFS clone: lệnh cp hiện tại không bảo đảm giữ các tối ưu đó. Ước lượng bảo thủ có thể từ chối copy vốn có thể thành công nhờ sparse/clone. Hiển thị cần/còn bằng GiB và đường dẫn đích khi từ chối.

Preflight không reserve disk: process khác vẫn có thể chiếm chỗ sau kiểm tra. Các cơ chế cleanup staging, journal và rollback giữ nguyên. Không tự xóa backup, cache hoặc dữ liệu cũ. Chưa áp dụng cho tạo máy/setup vì cần mô hình dung lượng engine/image riêng.
