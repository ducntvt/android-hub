# US-050 — Cài image Android theo nhu cầu

- **Status**: implemented
- **Owner**: Android Hub maintainer
- **Created**: 2026-11-18
- **Risk lane**: high-risk
- **Related**: [Epic E01](README.md), [ADR 0056](../../../decisions/0056-immutable-android-image-per-device.md)
- **Depends on**: [US-049](US-049-immutable-device-image-identity.md).

## Context

Là người dùng, tôi chỉ muốn tải phiên bản Android mình cần và tái sử dụng image đã có. Một lỗi tải không được làm hỏng SDK hoặc máy đang hoạt động.

## Contract (acceptance criteria)

- [x] Tách công cụ dùng chung khỏi image tùy chọn; bỏ giả định plan luôn có đúng bốn package và chỉ có PlayImage36. Không tạo năm bản sao toàn bộ SDK.
- [x] Tạo install plan cho image được chọn từ catalog đã kiểm duyệt; xác minh URL nguồn, host macOS ARM64, revision, checksum, license và minimum engine. Không nhận destination tùy ý từ UI.
- [x] Image đã cài đúng metadata được tái sử dụng; trạng thái thiếu, đang tải, lỗi, sẵn sàng phân biệt rõ. SDK do người dùng chọn bên ngoài Hub được nhận diện nhưng không tự quản lý/xóa/cập nhật như SDK do Hub cài.
- [x] Trước tải hiển thị dung lượng download và kiểm tra chỗ trống cho archive, giải nén/staging và dữ liệu máy; giữ phân biệt ước tính với dung lượng guest sẽ tăng theo thời gian.
- [x] Có tiến trình, hủy và retry; tái sử dụng cơ chế download/cache/checksum/recovery hiện có. Chỉ publish image hoàn chỉnh sau verify; không nhận phần cài dở là sẵn sàng.
- [x] Khóa theo thao tác/image phù hợp để chống tải/cài trùng; lỗi mạng, checksum sai, disk-full, đóng app và khởi động lại không thay image đã dùng hoặc ảnh hưởng máy khác.
- [x] Không tự cập nhật image/revision hay engine đang phục vụ máy hiện có; thiếu engine tương thích phải báo và dừng plan thay vì âm thầm nâng cấp. Không đổi SDK root đang gắn với máy/backup.
- [x] License chưa được chấp thuận cần được người dùng xác nhận qua luồng có sẵn; không tải cả năm image theo mặc định.

## Out of scope

Cập nhật image tại chỗ, cập nhật engine tự động, nguồn image tùy ý, Android x86, AOSP/Google APIs không Play Store và catalog cập nhật qua server riêng.

## Affected areas

`crates/infrastructure/src/sdk/`, bundled SDK manifests/resources, scripts tạo manifest và desktop SDK progress/state.

## Implementation notes

Kế thừa các cơ chế transaction, consent, checksum, cancel và recovery hiện có; không tạo bộ tải song song thứ hai. Không ghi đè package directory khi vẫn có máy/backup tham chiếu revision cũ. Ghi nguồn gốc ownership để phân biệt nội dung Hub quản lý với SDK bên ngoài.

## Verify

Các lệnh sau dành cho lúc triển khai story, chạy từ repo root; không chạy khi chỉ viết tài liệu:

```bash
bash scripts/check.sh
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Test bằng fixture download: cache hit, checksum sai, hủy/retry, mất mạng, thiếu dung lượng, hai request cùng image, crash trước publish và image cũ đang được dùng. Smoke một image thật vào SDK QA riêng, xác nhận image cài lần hai không tải lại và guest khác vẫn dùng được. Không chạy bulk download bằng shared check/CI.

Chỉ đánh dấu `implemented` khi các lệnh và ca kiểm chứng bắt buộc của story đều đạt; với spike, phân biệt kết luận nghiên cứu với chứng nhận sản phẩm.

## Evidence

[On-demand installation report](../../../reports/android-image-installation-20261118.md):
full checks, bundle build/strict codesign, cached fixtures, explicit crash/cancel/
interrupted-transfer retry, actual API 33 installation/reuse while API 31 remained
operational, and native EN/VI review at minimum window size. QA app and guest were
closed. SDK/AVDs remain isolated inputs for the remaining epic stories. Real disk
exhaustion was not induced; budget boundaries are fixture-tested.
