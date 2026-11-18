# 0005 — Embedded Android display

- Status: Accepted
- Date: 2026-11-13

Người dùng làm rõ Device Hub phải chứa màn hình thiết bị và yêu cầu triển khai. Thay quyết định cửa sổ riêng trong ADR 0004.

Dùng Android Emulator gRPC streamScreenshot RGBA và sendTouch/sendKey. Proto Apache-2.0 đi kèm SDK được vendored trong infrastructure, tonic sinh client. Chạy engine với -no-window, -grpc-use-token và port riêng trên localhost. Token đọc từ discovery file theo đúng PID, không lưu/log ra UI. Không tắt auth.

Mỗi session có task async riêng, chỉ giữ frame mới nhất, bounded input queue, hủy khi màn hình không còn hiển thị. UI Iced wgpu/Metal vẽ frame, map tọa độ và focus bàn phím. Không dùng screenshot ADB lặp hay nhúng cửa sổ macOS bằng API private. Giới hạn ban đầu: một pointer, bàn phím ASCII; không tuyên bố đạt FPS gaming trước benchmark.

Cấu hình runtime dùng -memory, -cores, -skin WxH và -vsync-rate. SDK help xác nhận -skin WxH đặt framebuffer chính xác. Tránh đọc lại config.ini của AVD cũ trong mỗi lần start; macOS đã được quan sát treo open trên B khi chạy từ app.
