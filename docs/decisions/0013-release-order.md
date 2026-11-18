# 0013 — Serialize release đang chờ

- **Status**: Accepted
- **Date**: 2026-11-13

## Decision

Queue input vẫn bounded 64. Khi full, release của single pointer dùng một sender async chờ capacity; một mutex gate được giữ trong thao tác enqueue và khóa input mới đến khi release enqueue xong. Release lặp được gộp, không tạo task chờ không giới hạn. Các input mới bị từ chối với lỗi busy, người dùng thử lại; không replay click sau.

Trước đây mỗi release full tạo task riêng, một input mới có thể enqueue trước task đó khiến thứ tự down/up sai. Gate giữ thứ tự mà không chuyển UI sang blocking send và không tăng queue vô hạn.

Desktop dùng cùng helper release/reset epoch khi mất focus, chuyển selection và đổi single/split. Session còn hiển thị trong split cũng nhận release; không trông chờ Drop giải phóng touch. Chưa thay thiết kế reconnect.

## Bổ sung: reconnect

Input gate nay gồm ready/releasing/generation. Session bắt đầu chưa ready; chỉ mở gate sau khi stream screenshot và lệnh release ban đầu thành công. Khi run kết thúc/lỗi, giữ cùng mutex của enqueue để đóng gate, tăng generation và dọn queue. Như vậy UI chưa kịp nhận frame error cũng không enqueue input trong 2 giây chờ reconnect.

Release async chờ permit, rồi kiểm tra generation dưới gate trước enqueue; release từ phiên cũ bị bỏ, không được xóa cờ releasing của phiên mới. Input queue không tự replay clipboard/key/touch cũ. RPC đã được server nhận trước khi kết nối lỗi không thể thu hồi; không bảo đảm exactly-once khi mất kết nối. Người dùng chủ động thử lại.

## Canvas generation — 2026-11-16

Canvas input now carries the connection generation captured by its view. State
resets down/focus/fractional wheel when generation changes, even if error/ready
notifications coalesce and PID/desktop epoch remain unchanged. A not-ready view
does not emit guest input. Back/Home/Recent also carry the captured generation.
The existing gate validates generation atomically with enqueue, covering a
reconnect between event creation and desktop update. Rejected presses do not
change desktop input focus. Internal release/reset still uses the current gate.

This extends the clipboard generation boundary to touch/key/wheel without
replaying an old gesture or adding periodic polling. Native reconnect-during-hold
verification remains separate from deterministic event/queue regressions.

## Desktop epoch on queued messages — 2026-11-16

DeviceInput now carries the view's desktop input epoch as well as connection generation. Hub rejects messages whose epoch predates release/reset (unfocus, selection or single/split change). A retained split session can keep the same connection generation across these UI changes, so generation alone cannot invalidate a message already emitted by an older canvas. Back/Home/Recent capture the same epoch. Existing transport generation validation remains atomic under its queue gate. This does not retract an RPC already accepted by Android.

## Bounded handshake — 2026-11-16

Source review found that the initial release RPC lacked the 2-second local timeout used by normal input, and screenshot establishment could inherit the long stream deadline. Bound screenshot establishment to 8 seconds and initial release to 2 seconds with Tokio timeout. These limits end before the steady-state stream loop; they do not impose an 8-second frame-stream lifetime. Failure keeps the gate closed and uses the existing error/retry path. The remote may already have accepted a release when the local timeout fires; release is idempotent and no old user input is replayed. Native hung-server fault injection remains separate verification.

Handshake is factored into `establish_input`, using the same generated client for production and tests. A test-only tonic Service accepts requests but leaves response headers pending, first for streamScreenshot and then for sendTouch after stream headers succeed. The real 8/2-second deadlines return their stage-specific errors under an outer 12-second watchdog; recorded RPC paths verify that a stalled stream never advances to touch. This covers client-side response stalls without a network listener or guest. It does not claim socket-failure or Android-server behavior.
