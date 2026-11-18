# 0009 — Kiểm tra image và đường dẫn trước restore

- **Status**: Accepted
- **Date**: 2026-11-13
- **Deciders**: triển khai restore của MVP
- **Supersedes**: mở rộng manifest ADR 0008, không bỏ verify v1

## Context

Disk QCOW2 có thể tham chiếu image ngoài AVD; checksum payload không chứng minh cùng image/đường dẫn backing hoặc khả năng boot. Khôi phục sang một SDK đã đổi có thể hỏng guest.

## Decision

Backup mới schema 2 ghi path SDK/AVD canonical và SHA-256/size của sáu file SDK: source.properties, system.img, vendor.img, ramdisk.img, kernel-ranchu, encryptionkey.img. Kiểm tra trước/sau backup. Gate restore yêu cầu đúng ID/port/path và fingerprint image cùng config.sysdir. V1 giữ kiểm tra payload, không cho automatic restore.

## Consequences

Hash image thêm I/O, chỉ chạy trong backup/restore, không trong frame/poll. Tạm giới hạn restore tại đúng đường dẫn với cùng image; không tự rewrite backing paths hoặc tải image. Đây là compatibility gate, không thay cho journal/rollback và boot smoke.

## Alternatives considered

Chỉ so source.properties/API version không phát hiện image bị thay nội dung. Cho restore v1 rồi thử boot dễ làm mất trạng thái; từ chối tự động là rõ ràng hơn.
