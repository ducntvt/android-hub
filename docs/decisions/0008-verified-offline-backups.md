# 0008 — Tạo backup offline có checksum trước khi hỗ trợ restore

- **Status**: Accepted
- **Date**: 2026-11-13
- **Deciders**: triển khai yêu cầu backup/restore trong MVP
- **Supersedes**: none

## Context

Dữ liệu AVD gồm userdata và khóa mã hóa; chỉ copy một disk là chưa đủ. Copy khi guest ghi dữ liệu hoặc công bố trước khi copy xong sẽ tạo bản khôi phục không đáng tin.

## Decision

Tạo thư mục mới trong runtime/backups, manifest schema 1 và payload toàn bộ file thường trong thư mục AVD. Chỉ làm offline, giữ lock Hub, kiểm tra process/port. Lập inventory SHA-256 source, copy bằng cp có timeout, verify bản sao, sync file, kiểm tra lại source rồi rename .partial thành tên cuối. Dùng shasum có sẵn trên macOS, args riêng và timeout; không thêm dependency crypto.

## Consequences

Hash/copy cần I/O và chỗ trống, không zero-copy. Có thể mất vài phút; chưa có progress/cancel. Không coi backup là tự chứa SDK/image hoặc snapshot RAM. Giữ cấu hình profile để restore trong bước sau. Từ chối symlink và file đặc biệt; backup cục bộ không mã hóa thêm. Không tự sao chép dữ liệu người dùng ngoài thao tác họ chọn.

Hub lock không kiểm soát người dùng mở emulator ngoài Hub; kiểm tra process/port và source trước/sau giúp phát hiện thay đổi, không phải snapshot filesystem nguyên tử. Restore cần verify, kiểm tra tương thích và rollback riêng trước khi được bật.

## Alternatives considered

Copy live hoặc chỉ userdata: không đủ tính nhất quán/khóa mã hóa. Archive nén: thêm chi phí CPU và rủi ro giải nén; dùng directory đơn giản để kiểm tra từng file. APFS clone: chưa chọn vì cần benchmark/tương thích; không hứa tiết kiệm dung lượng.
