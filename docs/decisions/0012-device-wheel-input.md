# 0012 — Wheel input qua giao thức emulator

- **Status**: Accepted
- **Date**: 2026-11-13

## Decision

Desktop chuyển wheel sang Input::Wheel chứa vị trí framebuffer và delta theo đơn vị 120/nấc của proto SDK đã vendored. Lines nhân 120; Pixels nhân 3 (40 pixel/nấc là lựa chọn ban đầu cần kiểm chứng UX), giữ phần lẻ giữa event, giới hạn 10 nấc/event và bỏ dữ liệu không hữu hạn. Không can thiệp chiều cuộn do hệ điều hành gửi.

Infrastructure gửi một streamInputEvent gồm MouseEvent buttons=0 rồi WheelEvent theo thứ tự. Vị trí được gửi trước để cuộn đúng vùng trong Android. Dùng session token/port/PID và hàng đợi bounded hiện có; không replay sau reconnect. Wheel trong touch drag bị capture nhưng không gửi để tránh trộn hai cử chỉ. Ngoài canvas để parent scrollable xử lý.

## Limits

Unit tests chứng minh routing và đổi đơn vị, chưa chứng minh phản hồi Android thực tế hay chiều natural scrolling trên macOS. Cần smoke native US-011; mỗi wheel event hiện tạo một RPC hữu hạn (hai input messages), chưa coalesce nhiều wheel events. Queue đầy báo lỗi thay vì tích lũy vô hạn.

## Alternatives

Giả lập swipe từ wheel có thể kích hoạt game gesture/tap không mong muốn. Một stream input lâu dài hiệu quả hơn nhưng cần đổi thiết kế reconnect và timeout cho toàn bộ input; để sau số đo cụ thể.
