# US-051 — Chọn phiên bản Android khi tạo máy mới

- **Status**: implemented
- **Owner**: Android Hub maintainer
- **Created**: 2026-11-18
- **Risk lane**: normal
- **Related**: [Epic E01](README.md), [ADR 0056](../../../decisions/0056-immutable-android-image-per-device.md)
- **Depends on**: [US-049](US-049-immutable-device-image-identity.md), [US-050](US-050-on-demand-android-image-installation.md).

## Context

Là người dùng, tôi muốn chọn Android 12–16 khi bấm tạo máy, hiểu rõ có cần tải thêm hay không và giữ được các lựa chọn nếu thao tác thất bại.

## Contract (acceptance criteria)

- [x] Form tạo máy có selector Android 12–16; mặc định Android 16. Hiện Google Play và trạng thái cài đặt, không ép người dùng chọn ABI/API/revision kỹ thuật.
- [x] Bản đã sẵn sàng dùng hành động Tạo máy; bản chưa cài dùng Tải và tạo máy, kèm dung lượng. Plan/consent cần thiết vẫn được xem và xác nhận trước download.
- [x] Giữ tên/RAM/CPU/độ phân giải/FPS và lựa chọn version khi lỗi hoặc chuyển sang review plan; khóa đích thao tác sau xác nhận để callback cũ không tạo nhầm phiên bản.
- [x] Hủy trước tạo không có AVD/registry entry; nếu image đã cài xong thì có thể giữ để dùng lại và thông báo rõ, không coi đó là máy đã tạo.
- [x] Một xác nhận chỉ tạo một máy; backend dùng đúng image đã verify. Kiểm tra lại disk/SDK/image trước tạo và giữ journal/recovery hiện có.
- [x] Lỗi sau khi máy đã đăng ký không tạo lại khi retry; tùy chọn Khởi động sau khi tạo chỉ nhắm ID vừa tạo. Máy mới có dữ liệu/tài khoản riêng.
- [x] Sidebar/thông tin máy hiển thị đúng Android version mà không đổi tên tùy chỉnh. Device settings hiển thị version chỉ đọc và hướng dẫn tạo máy mới khi cần version khác.
- [x] Nhãn/trạng thái/lỗi Hub có EN/VI; bố cục, bàn phím, focus và loading/disabled phù hợp style hiện tại ở kích thước cửa sổ tối thiểu.

## Out of scope

Đổi Android trong settings, clone tài khoản, tự chọn version theo game và thiết kế lại toàn bộ sidebar/settings.

## Affected areas

`crates/desktop/src/app/` creation/update/SDK state/UI, `i18n.rs`, domain CreateRequest và infrastructure creation integration.

## Implementation notes

Dùng một luồng tạo máy với các giai đoạn review/download/create; đừng nối các callback bằng “máy cuối danh sách”. Gắn operation ID và image identity vào yêu cầu bất đồng bộ; trạng thái UI phải xuất phát từ kết quả backend.

## Verify

Các lệnh sau dành cho lúc triển khai story, chạy từ repo root; không chạy khi chỉ viết tài liệu:

```bash
bash scripts/check.sh
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Test state transitions cho đã cài/chưa cài, đổi lựa chọn, hủy, lỗi/retry, stale completion, double-submit và created-but-start-failed. Native QA EN/VI trên dữ liệu riêng: tạo ít nhất một máy từ image sẵn có và một máy qua tải mới; kiểm tra form nhỏ, version chỉ đọc và restart Hub.

Chỉ đánh dấu `implemented` khi các lệnh và ca kiểm chứng bắt buộc của story đều đạt; với spike, phân biệt kết luận nghiên cứu với chứng nhận sản phẩm.

## Evidence

[Creation QA report](../../../reports/android-version-creation-20261118.md): full
checks, bundle build and strict codesign passed. Native EN/VI creation covered an
installed Android 13 image and Android 14 download-and-create, minimum window,
keyboard cancellation, read-only version and reopen persistence. QA app closed.
