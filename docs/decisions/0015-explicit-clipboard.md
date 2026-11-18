# 0015 — Clipboard văn bản theo yêu cầu người dùng

- **Status**: Accepted
- **Date**: 2026-11-13

Proto vendored ghi rõ keyboard key/text chuyển sang evdev không hỗ trợ tùy ý Unicode. Dùng setClipboard(ClipData) để chuyển văn bản UTF-8, không giả định gửi text qua sendKey sẽ nhập tiếng Việt.

Nút dưới từng thiết bị gọi iced::clipboard::read đúng một lần theo click. Callback giữ ID/PID/input epoch; chỉ gửi khi session vẫn hiển thị frame hợp lệ, trạng thái không stale/busy và epoch còn khớp. Không tự đọc nền hay stream clipboard guest. Văn bản chỉ nằm trong bộ nhớ, hàng đợi bounded và RPC localhost có token; không lưu/log hoặc đưa vào notice. Giới hạn 64 KiB tính theo byte UTF-8.

RPC setClipboard trả OK khi emulator lên lịch cập nhật, không chứng minh ô nhập đã nhận text. Vì vậy thông báo chỉ ghi đã gửi và hướng dẫn nhấn giữ/Dán trong Android; không tự gửi phím Paste. Android/app có thể hạn chế clipboard. Không đọc ngược clipboard guest để xác nhận bằng dữ liệu người dùng.

Latest-frame mailbox có thêm feedback một lần để báo kết quả RPC, chung Notify hiện có; không thêm timer. display_probe bỏ qua feedback. Không claim native Unicode paste đã pass trước smoke US-015.

## Same-process reconnect — 2026-11-16

The callback now also captures the display input generation. PID and desktop
epoch alone do not detect a transport reconnect to the same engine. Infrastructure
checks the expected generation under the same mutex as enqueue, rejecting a late
read even after the new connection is ready. Existing queued data is still drained
on reconnect. This closes the callback/queue race without reading clipboard again
or retrying its payload. The error asks the user to explicitly copy again.

The generation is only returned while input is ready, so a click during reconnect
is rejected before requesting the host clipboard. A reconnect after this check
still increments generation and is rejected atomically when the callback sends.
