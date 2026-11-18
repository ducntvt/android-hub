# 0006 — Chỉ cập nhật màn hình khi có dữ liệu mới

- **Status**: Accepted
- **Date**: 2026-11-13
- **Deciders**: triển khai theo yêu cầu tối ưu của người dùng
- **Supersedes**: none; bổ sung ADR 0005

## Context

Timer 33 ms đánh thức UI kể cả khi không có frame. Việc đồng bộ session không cần chạy ở tốc độ frame.

## Decision

Session cung cấp thông báo `Notify` giữ tối đa một permit; pixel/error tiếp tục nằm trong slot latest có mutex. Mỗi subscription Iced theo ID/PID đợi thông báo và giới hạn khoảng 30 cập nhật/giây. Đồng bộ session khi snapshot/lựa chọn/chế độ xem thay đổi.

## Consequences

Không có tick frame khi rảnh, không tích lũy hàng đợi ảnh; giữ được thông báo đến trước khi subscription bắt đầu. Có thể bỏ frame trung gian để ưu tiên ảnh mới nhất. Lớp desktop dùng primitive Tokio vốn có trong stack. PID được kiểm tra khi nhận message để bỏ thông báo từ engine đã thay đổi. Poll trạng thái process mỗi 3 giây vẫn giữ nguyên.

## Alternatives considered

Timer chỉ khi có session: đơn giản nhưng vẫn đánh thức UI khi màn hình không thay đổi. Hàng đợi mọi frame: tăng RAM và độ trễ khi UI chậm.

## References

[ADR 0005](0005-embedded-emulator-display.md), [US-004](../stories/US-004-hub-performance.md).
