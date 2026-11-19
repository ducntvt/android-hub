# US-052 — Quản lý các phiên bản Android đã cài

- **Status**: implemented
- **Owner**: Android Hub maintainer
- **Created**: 2026-11-18
- **Risk lane**: high-risk
- **Related**: [Epic E01](README.md), [ADR 0056](../../../decisions/0056-immutable-android-image-per-device.md)
- **Depends on**: [US-049](US-049-immutable-device-image-identity.md), [US-050](US-050-on-demand-android-image-installation.md).

## Context

Là người dùng, tôi muốn biết image nào đã tải, dung lượng và máy nào đang dùng, rồi gỡ image không còn cần mà không làm mất khả năng chạy/khôi phục máy.

## Contract (acceptance criteria)

- [x] Hub settings có mục Phiên bản Android với danh sách 12–16, trạng thái, dung lượng thực tế nếu đọc được và hành động tải/gỡ phù hợp; số dung lượng không xác định phải được ghi rõ.
- [x] Hiển thị tên/số máy và backup/archive đang tham chiếu từng image. Máy dừng vẫn tính là đang sử dụng image.
- [x] Chỉ cho gỡ image do Hub quản lý, không còn tham chiếu từ máy, archive, backup hoặc transaction đang chờ; tham chiếu chưa đọc/xác định được phải chặn gỡ và giải thích.
- [x] Gỡ cần xác nhận tên/version và dung lượng; backend kiểm tra lại ownership, phạm vi đường dẫn và tham chiếu dưới khóa ngay trước thao tác, kể cả khi danh sách UI đã cũ.
- [x] Tạo/tải/backup/restore không được đua với gỡ cùng image; gián đoạn khi gỡ có trạng thái khôi phục được và không để UI báo sẵn sàng cho package hỏng.
- [x] Tách image đã cài khỏi archive tải/cache; dọn cache không được xóa image hoặc dữ liệu máy. Không tự gỡ image sau khi xóa máy cuối cùng.
- [x] Image thuộc SDK bên ngoài Hub được hiển thị nhưng không có hành động xóa/cập nhật. Tải lại bản chưa cài không tự thay identity của máy cũ.
- [x] EN/VI, feedback gọn, loading/error/empty states và thao tác bàn phím tuân theo UI hiện tại.

## Out of scope

Force delete image đang dùng, xóa backup/AVD dây chuyền, cập nhật revision tại chỗ và tự dọn SDK bên ngoài.

## Affected areas

Infrastructure SDK catalog/storage/reference inventory, backup/archive metadata và desktop Hub settings.

## Implementation notes

Cần một nguồn kiểm kê tham chiếu dùng chung giữa preflight và mutation. Không chỉ đếm registry đang hiển thị: archive, backup và journal cũng có thể cần image. Giữ trách nhiệm xóa dữ liệu người dùng ở các luồng xác nhận riêng hiện có.

## Verify

Các lệnh sau dành cho lúc triển khai story, chạy từ repo root; không chạy khi chỉ viết tài liệu:

```bash
bash scripts/check.sh
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Fixture kiểm tra image không dùng, máy đang chạy/đã dừng, backup/archive, pending journal, tham chiếu hỏng, SDK bên ngoài, path escape và race create-versus-remove. Native QA tải/gỡ một image không dùng và xác nhận gỡ bị chặn khi tạo máy tham chiếu; không thao tác SDK thật.

Chỉ đánh dấu `implemented` khi các lệnh và ca kiểm chứng bắt buộc của story đều đạt; với spike, phân biệt kết luận nghiên cứu với chứng nhận sản phẩm.

## Evidence

[Image management QA](../../../reports/android-image-management-20261118.md): full
checks, bundle build and strict codesign passed. Native EN/VI verified installing,
canceling removal, confirmed removal and protecting an image used by a stopped
new device. QA app exited; the secondary SDK, AVD and cloned cache were removed.
The shared epic SDK/device fixtures remain for US-053; personal data was untouched.
