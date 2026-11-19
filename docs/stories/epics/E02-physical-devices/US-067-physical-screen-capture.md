# US-067 — Chụp ảnh và ghi màn hình điện thoại

- **Status**: planned
- **Owner**: Android Hub maintainer
- **Created**: 2026-11-19
- **Risk lane**: normal
- **Related**: [Epic E02](README.md), [ADR 0060](../../../decisions/0060-physical-device-sessions.md)
- **Depends on**: US-064; audio ghi kèm phụ thuộc US-066
- **Phase**: Future

## Context

Là người dùng, tôi muốn chủ động lưu ảnh hoặc video từ màn hình đang xem.

## Contract (acceptance criteria)

- [ ] Chọn đúng thiết bị và nơi lưu qua UI; ghi hình có chỉ báo cùng nút dừng rõ ràng.
- [ ] Không ghi ngầm; mất kết nối/hết dung lượng/thoát app xử lý file dở và thông báo kết quả chính xác.
- [ ] Không ghi đè file nếu chưa xác nhận; giữ file export của người dùng khi gỡ Hub.
- [ ] Kiểm tra file đầu ra phát được và đo ảnh hưởng tới stream/input; nội dung được bảo vệ báo giới hạn.

## Out of scope

Ghi hình cloud, editor video và quay mọi thiết bị cùng lúc.

## Affected areas

Capture/record backend, save dialog, controls và docs.

## Implementation notes

Không lưu ảnh cá nhân vào báo cáo Git; dùng màn hình fixture.

## Verify

Lệnh dành cho lúc triển khai story, chạy từ repo root; không build/test Rust chỉ để viết US.

```bash
bash scripts/check.sh
git diff --check
```

Thêm test hồi quy đúng các lỗi/ranh giới nêu trên, chạy native QA tương ứng trên fixture và điện thoại được chỉ định. Khi sửa bundle/helper, build và verify chữ ký bằng script hiện có. Lệnh test thuần đạt không thay thế ca thiết bị thật.

## Evidence

Chưa triển khai hoặc chạy QA. Chỉ đánh dấu implemented khi các tiêu chí và kiểm tra bắt buộc đạt; ghi rõ ca thiếu bằng chứng. Dọn tài nguyên thử sau mỗi phiên theo CLAUDE.md.
