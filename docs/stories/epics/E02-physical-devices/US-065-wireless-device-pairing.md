# US-065 — Ghép đôi điện thoại qua Wi-Fi

- **Status**: planned
- **Owner**: Android Hub maintainer
- **Created**: 2026-11-19
- **Risk lane**: high-risk
- **Related**: [Epic E02](README.md), [ADR 0060](../../../decisions/0060-physical-device-sessions.md)
- **Depends on**: MVP USB đạt US-064
- **Phase**: Future

## Context

Là người dùng, tôi muốn kết nối điện thoại không dây khi USB đã ổn định.

## Contract (acceptance criteria)

- [ ] Ưu tiên Wireless debugging Android 11+ bằng mã ghép đôi, hiển thị rõ địa chỉ đích; không tự mở legacy adb tcpip 5555.
- [ ] Phân biệt pair và connect; timeout/hủy/mDNS lỗi có hướng dẫn nhập địa chỉ. Không ghi log/lưu pairing code hoặc tự sửa Wi-Fi điện thoại.
- [ ] Xử lý cùng điện thoại qua USB và Wi-Fi, đổi transport/IP và mất mạng mà không gửi input nhầm hoặc replay APK.
- [ ] Đo độ trễ với cùng baseline USB, công bố khác biệt; Disconnect chỉ đóng kết nối do Hub quản lý.

## Out of scope

Kết nối Internet, port forwarding/router, QR pairing ở lượt đầu và tự bật Wireless debugging.

## Affected areas

ADB pairing/discovery, connection UI, session identity và docs.

## Implementation notes

Planned cho giai đoạn sau; chưa tự triển khai khi chỉ đang thử MVP USB.

## Verify

Lệnh dành cho lúc triển khai story, chạy từ repo root; không build/test Rust chỉ để viết US.

```bash
bash scripts/check.sh
git diff --check
```

Thêm test hồi quy đúng các lỗi/ranh giới nêu trên, chạy native QA tương ứng trên fixture và điện thoại được chỉ định. Khi sửa bundle/helper, build và verify chữ ký bằng script hiện có. Lệnh test thuần đạt không thay thế ca thiết bị thật.

## Evidence

Chưa triển khai hoặc chạy QA. Chỉ đánh dấu implemented khi các tiêu chí và kiểm tra bắt buộc đạt; ghi rõ ca thiếu bằng chứng. Dọn tài nguyên thử sau mỗi phiên theo CLAUDE.md.
