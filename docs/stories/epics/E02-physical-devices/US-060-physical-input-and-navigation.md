# US-060 — Điều khiển điện thoại và navigation

- **Status**: planned
- **Owner**: Android Hub maintainer
- **Created**: 2026-11-19
- **Risk lane**: high-risk
- **Related**: [Epic E02](README.md), [ADR 0060](../../../decisions/0060-physical-device-sessions.md)
- **Depends on**: US-059
- **Phase**: MVP

## Context

Là người dùng, tôi muốn click, kéo/vuốt, cuộn và dùng bàn phím đúng thiết bị đang chọn.

## Contract (acceptance criteria)

- [ ] Click/drag/swipe, wheel hai trục, Back/Home/Recent và bàn phím hoạt động qua transport đã chọn; không spawn adb shell riêng cho từng chuyển động chuột.
- [ ] Ánh xạ tọa độ đúng tại Fit, zoom 110%, resize, portrait/landscape và hai slot. Thay đổi orientation giữa gesture hủy an toàn gesture cũ.
- [ ] Focus/selection rõ ràng; thao tác không lọt sang máy bên cạnh. Nhả phím/touch khi mất focus, mở modal, đóng stream hoặc rút cáp; không replay input cũ sau reconnect.
- [ ] Giới hạn hàng đợi và xử lý backpressure có thứ tự; không bỏ mất release event để rồi kẹt phím.
- [ ] Xoay chỉ cung cấp nếu capability thực hỗ trợ và mô tả rõ ảnh hưởng lên điện thoại; OEM chặn điều khiển phải có hướng dẫn phù hợp.
- [ ] Test native dùng app QA có bộ đếm touch/scroll/key; xác nhận các ký tự được hỗ trợ và giới hạn IME/Unicode, không mặc định nhận mọi kiểu gõ.

## Out of scope

Key mapping game, macro, gamepad, tự động chơi game và remote control qua Internet.

## Affected areas

Infrastructure input/control, desktop screen/focus, fixture tests/android-input/.

## Implementation notes

Input phải gắn transport/session generation thay vì chỉ serial; không gắn PID emulator giả cho điện thoại.

## Verify

Lệnh dành cho lúc triển khai story, chạy từ repo root; không build/test Rust chỉ để viết US.

```bash
bash scripts/check.sh
git diff --check
```

Thêm test hồi quy đúng các lỗi/ranh giới nêu trên, chạy native QA tương ứng trên fixture và điện thoại được chỉ định. Khi sửa bundle/helper, build và verify chữ ký bằng script hiện có. Lệnh test thuần đạt không thay thế ca thiết bị thật.

## Evidence

Chưa triển khai hoặc chạy QA. Chỉ đánh dấu implemented khi các tiêu chí và kiểm tra bắt buộc đạt; ghi rõ ca thiếu bằng chứng. Dọn tài nguyên thử sau mỗi phiên theo CLAUDE.md.
