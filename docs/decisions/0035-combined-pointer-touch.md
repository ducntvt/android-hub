# 0035 — Kết hợp pointer và touch cho wheel Android

- **Status**: Rejected
- **Date**: 2026-11-16
- **Deciders**: Codex
- **Supersedes**: none; bổ sung ADR 0012

## Context

Native QA cho thấy wheel không tới kernel với cấu hình multitouch mặc định. Engine source chọn mouse/tablet hoặc multitouch; bật pointer đơn thuần có thể bỏ thiết bị touch cần cho game.

## Experiment

Thử bật pointer qua feature flags và thêm `-qemu -device virtio_input_multi_touch_pci_1` để giữ main-display touch. Chỉ đóng gói QA; main app chưa đổi.

Tablet + touch: boot, chạm mở Settings và kéo mở drawer pass. Kernel nhận ABS_X/Y và REL_WHEEL nhưng Android TouchInputMapper không có pointer (0 hovering/touching); Settings không cuộn. Không chọn phương án tablet này.

Tiếp tục thử VirtioMouse + VirtioDualModeMouse, tắt VirtioTablet và giữ thiết bị touch riêng. Chưa chấp nhận cho sản phẩm trước khi kiểm chứng wheel, touch, tọa độ và tương thích guest.

## Constraints

Giữ token gRPC, queue/release/reconnect, không sửa guest system image hoặc cấu hình bảo mật. Main display=0; nhiều máy là nhiều engine độc lập. Không suy ra đa màn hình trong một guest đã được hỗ trợ. Các flag/device phụ thuộc engine, phải nghiệm thu lại khi nâng SDK.

## Alternatives

Wheel giả swipe thay đổi nghĩa input và có thể kích hoạt gesture game; chưa chọn. Chỉ bật mouse/tablet mà bỏ touch không đáp ứng yêu cầu game.

## References

- [ADR 0012](0012-device-wheel-input.md)
- [US-011](../stories/US-011-device-scroll.md)
- [Engine device initialization](https://android.googlesource.com/platform/external/qemu/+/refs/heads/emu-main-dev/android-qemu2-glue/main.cpp)
- [Engine input dispatch](https://android.googlesource.com/platform/external/qemu/+/refs/heads/emu-main-dev/android-qemu2-glue/qemu-user-event-agent-impl.c)

## Result

Dual-mode + explicit touch: guest boots, native touch opens drawer/Settings, wheel down/up moves Settings correctly. However Android mouse cursor stays at its initial center while ABS_X/Y changes. dumpsys shows a CursorInputMapper consuming relative axes and a separate TouchInputMapper with no active hover pointer. Thus wheel targets the wrong location for nested scroll areas. This does not meet US-011 routing contract.

Rejected both experimental flag configurations; reverted launcher source to original flags. Main app bundle never included experiments. Guest đã dừng và QA đã đóng; binary QA đã trả về bản main không có experimental flags, ký lại và verify. target/debug còn binary thử cho đến lần build tiếp theo; không đóng gói từ binary đó.

Next candidate: framework-level ACTION_SCROLL with explicit coordinates on the existing guest connection. Guest `input` usage confirms `input mouse scroll x y --axis HSCROLL,value --axis VSCROLL,value`. A persistent shell/companion could avoid process-per-event overhead and bind the channel to a single guest session. Must preserve no replay/release ordering and avoid blocking frame streaming. This is not yet implemented or selected as final architecture.
