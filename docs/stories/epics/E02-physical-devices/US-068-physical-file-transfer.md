# US-068 — Truyền file có xác nhận với điện thoại

- **Status**: planned
- **Owner**: Android Hub maintainer
- **Created**: 2026-11-19
- **Risk lane**: high-risk
- **Related**: [Epic E02](README.md), [ADR 0060](../../../decisions/0060-physical-device-sessions.md)
- **Depends on**: US-064
- **Phase**: Future

## Context

Là người dùng, tôi muốn gửi/nhận file với đúng điện thoại mà không làm mất file hiện có.

## Contract (acceptance criteria)

- [ ] Chọn rõ thiết bị, file nguồn và đích trong vùng được phép; kiểm tra phiên ngay trước khi ghi.
- [ ] Có tiến độ/hủy; mất kết nối để trạng thái chưa hoàn tất rõ ràng, không tự retry sang phiên mới.
- [ ] Không ghi đè/xóa dữ liệu điện thoại nếu chưa xác nhận; chống path traversal và xử lý tên có khoảng trắng/Unicode.
- [ ] Xử lý storage permissions, hết dung lượng và file trùng bằng fixture; gỡ Hub giữ file export ngoài data root.

## Out of scope

Root file manager, đồng bộ thư mục nền và truy cập dữ liệu app riêng tư.

## Affected areas

ADB transfer, file dialog/confirmation, task lifecycle và docs.

## Implementation notes

Độc lập với APK install; không tái sử dụng install như một API truyền file tổng quát.

## Verify

Lệnh dành cho lúc triển khai story, chạy từ repo root; không build/test Rust chỉ để viết US.

```bash
bash scripts/check.sh
git diff --check
```

Thêm test hồi quy đúng các lỗi/ranh giới nêu trên, chạy native QA tương ứng trên fixture và điện thoại được chỉ định. Khi sửa bundle/helper, build và verify chữ ký bằng script hiện có. Lệnh test thuần đạt không thay thế ca thiết bị thật.

## Evidence

Chưa triển khai hoặc chạy QA. Chỉ đánh dấu implemented khi các tiêu chí và kiểm tra bắt buộc đạt; ghi rõ ca thiếu bằng chứng. Dọn tài nguyên thử sau mỗi phiên theo CLAUDE.md.
