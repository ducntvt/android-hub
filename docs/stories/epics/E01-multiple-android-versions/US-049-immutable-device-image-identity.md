# US-049 — Lưu cố định image Android theo từng máy

- **Status**: implemented
- **Owner**: Android Hub maintainer
- **Created**: 2026-11-18
- **Risk lane**: high-risk
- **Related**: [Epic E01](README.md), [ADR 0056](../../../decisions/0056-immutable-android-image-per-device.md)
- **Depends on**: [US-048](US-048-android-version-compatibility-spike.md).

## Context

Là người dùng, tôi muốn mỗi máy giữ đúng Android đã chọn khi tạo, kể cả sau khi khởi động lại Hub hoặc cài thêm image. Máy cũ phải giữ nguyên dữ liệu và phiên bản hiện tại.

## Contract (acceptance criteria)

- [x] Có catalog cho Android 12/API 31, 13/33, 14/34, 15/35 và 16/36, giới hạn Google Play ARM64 stable; mỗi mục có định danh package, revision/checksum và điều kiện engine rõ ràng.
- [x] CreateRequest nhận lựa chọn từ catalog; backend xác thực và lưu identity image trong Profile/registry. Tên máy, port và đường dẫn vẫn do các luồng hiện có kiểm soát.
- [x] Image identity bất biến sau khi tạo: API update/settings từ chối thay đổi, kể cả khi gọi backend trực tiếp. Sửa RAM/CPU/FPS không được ghi đè identity.
- [x] Registry cũ được đọc tương thích; suy ra image từ cấu hình AVD và metadata SDK đang dùng, kiểm chứng trước khi ghi metadata bổ sung. Không mặc định gán API 36 khi thiếu hoặc mâu thuẫn dữ liệu.
- [x] Migration metadata có version schema, bản sao registry trước ghi, ghi atomically và chạy lại an toàn sau gián đoạn; không sửa userdata, tài khoản, config image hay image hệ thống. Máy chưa xác định được image có thông báo xử lý và không chặn các máy hợp lệ khác.
- [x] Readiness tách công cụ SDK chung khỏi image của từng máy; thiếu image Android 13 không chặn tạo/start máy Android 16 đã đủ điều kiện.
- [x] Backup, restore, archive/undo và journal giữ identity; backup cũ có metadata đáng tin được đối chiếu theo contract hiện hành. Restore khác image/revision/fingerprint bị chặn trước ghi dữ liệu; không giảm các gate SDK/path đang có.

## Out of scope

Chuyển version máy đã tạo, nâng/hạ Android, chuyển dữ liệu giữa các version, import AVD ngoài Hub và tự sửa image SDK bị thay đổi bên ngoài.

## Affected areas

`crates/domain/src/`, infrastructure creation/configuration/registry, SDK setup/package/plan, backup/restore/archive/recovery và snapshot version labels.

## Implementation notes

Thực thi quy tắc bất biến ở domain/backend, không chỉ ẩn control UI. Phân biệt Android version với image revision. Catalog cập nhật không được tự thay binding của máy cũ. Profile có identity chưa xác định cần biểu diễn tường minh, không dùng giá trị default che lỗi. Chốt schema chi tiết khi triển khai theo ADR 0056.

## Verify

Các lệnh sau dành cho lúc triển khai story, chạy từ repo root; không chạy khi chỉ viết tài liệu:

```bash
bash scripts/check.sh
```

Bổ sung fixture registry cũ/AVD thiếu/image mâu thuẫn, migration lặp/gián đoạn, update đổi image bị chặn, readiness theo máy, round-trip backup/archive và restore sai image không ghi dữ liệu. Native smoke trên bản sao dữ liệu QA: mở lại Hub và kiểm tra version, profile, dữ liệu guest không đổi.

Chỉ đánh dấu `implemented` khi các lệnh và ca kiểm chứng bắt buộc của story đều đạt; với spike, phân biệt kết luận nghiên cứu với chứng nhận sản phẩm.

## Evidence

[Image identity verification](../../../reports/android-image-identity-20261118.md)
records the complete check suite, migration/immutability/backup/archive fixtures,
and native reopen of API 31/36 QA profiles without config/userdata changes.
Schema, cache invalidation and legacy behavior are specified in ADR 0056.
