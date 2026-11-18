# US-003 — Android nằm trong Hub

- Status: implemented

## Contract

- Màn hình live trong Hub; chọn A/B để chuyển máy, tùy chọn xem hai máy.
- Click, kéo và bàn phím trên màn hình; Home/Back/Recent.
- Engine mới mở headless, không có cửa sổ Android riêng.
- Kết nối localhost có token; đóng Hub không dừng engine.
- Lỗi/reconnect không khóa UI, frame cũ không gửi input sai máy.

## Verify

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
bash scripts/build-android-hub.sh
```

Kiểm tra UI bằng máy thử riêng: hình ảnh, tap/drag/key, switch, đóng/mở Hub, stop. Không dừng A/B đang dùng để test.

## Evidence

[Validation report](../reports/embedded-display-validation.md): two live headless guests, tap, swipe, keyboard isolation, reconnect, lifecycle and six passing tests.
