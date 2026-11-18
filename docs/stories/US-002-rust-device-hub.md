# US-002 — Hub quản lý máy Android bằng Rust

- Status: implemented
- Owner: Codex
- Created: 2026-11-13
- Related: ADR 0004; user chọn bước 1

## Contract

- [x] UI native Rust hiển thị A/B hiện có, trạng thái và CPU/RSS.
- [x] Tạo máy độc lập mới, không ghi đè máy cũ; nhận diện SDK thiếu và lỗi rõ ràng.
- [x] Mở/dừng đúng máy, không khởi động trùng hoặc ảnh hưởng A/B đang dùng.
- [x] Sửa tên, RAM, CPU, resolution, FPS khi máy dừng; cấu hình lưu qua restart Hub.
- [x] Tác vụ nền không khóa giao diện, có trạng thái đang xử lý/lỗi.
- [x] Gói .app mở từ Finder; đóng Hub không tự dừng game.

## Out of scope

Nhúng màn hình, keymap, xóa/clone máy đã đăng nhập, hoàn tất benchmark P0.

## Verify

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo build -p android-hub
bash scripts/build-android-hub.sh
```

Kiểm tra CUA: list A/B, create máy test mới, chỉnh cấu hình, start/stop máy test, reopen Hub kiểm tra persistence; không dừng A/B của người dùng. Ghi evidence trước khi implemented.

## Evidence

[Validation report](../reports/hub-validation.md), including real UI lifecycle and automated smoke. Test profiles archived; data retained.
