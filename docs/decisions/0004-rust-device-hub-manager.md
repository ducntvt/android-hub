# 0004 — Rust Device Hub quản lý Android Emulator

- Status: Accepted
- Date: 2026-11-13
- Deciders: Người dùng chọn bước 1 (Hub quản lý Rust)

## Context

Người dùng đã chơi được trên A, mở được A/B và chọn triển khai Hub quản lý trước. Gate gameplay/benchmark P0 vẫn chưa hoàn tất; lựa chọn mới cho phép làm manager song song thay vì chờ toàn P0.

## Decision

Iced cho desktop Rust, engine Android Emulator đã có trên máy, tiếp tục cửa sổ riêng. Tạo/list/start/stop/config/metrics local. Adopt hai AVD P0 có sẵn qua registry riêng; không clone tài khoản hoặc thay disk của A/B. Profile mới có tên kỹ thuật và port riêng. Cấu hình chỉ lưu khi VM dừng. Không cần backend mạng.

Domain giữ validation/models; infrastructure quản lý registry/process/AVD/ADB; desktop điều phối async, không chạy subprocess trong render/update. Dùng thư mục project cho bản cá nhân đầu, gói .app tạo đường dẫn tuyệt đối. Đóng Hub giữ các engine đang chạy; stop là hành động rõ ràng trên từng máy, để tránh đóng game đang dùng.

## Consequences

Có UI dùng được trước khi giải quyết Avo overlay; không tuyên bố chọn engine cuối hoặc đạt benchmark. Chưa nhúng framebuffer, keymap, backup UI, xóa instance hay remote device. Rust UI không thay chi phí tài nguyên của Android. Iced 0.13.1 được pin cho API ổn định trong bản đầu; Cargo.lock pin dependencies thực tế.

## Alternatives

Tauri không phù hợp lựa chọn UI Rust trực tiếp hiện tại. Tự viết engine không phục vụ mục tiêu sớm có bản dùng được.
