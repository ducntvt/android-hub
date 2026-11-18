# US-006 — Không để poll cũ ghi đè trạng thái mới

- **Status**: implemented
- **Created**: 2026-11-13
- **Owner**: Codex
- **Risk lane**: normal

## Context

Poll định kỳ có thể hoàn thành sau thao tác save/start/stop. Không đánh số thế hệ khiến snapshot cũ ghi đè kết quả mới, gây UI hiển thị sai và mở/đóng session thừa.

## Contract

- Khi bắt đầu command, vô hiệu hóa mọi poll đã khởi chạy trước command đó.
- Bỏ cả dữ liệu và lỗi từ poll cũ; vẫn giải phóng cờ polling.
- Poll mới sau command vẫn cập nhật bình thường.
- Không thay đổi registry/engine API hoặc dữ liệu Android.

## Affected areas

`crates/desktop/src/main.rs`, chỉ state/update và tests; không đổi boundary.

## Verify

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
bash scripts/build-android-hub.sh
```

## Evidence

Regression tests mô phỏng thứ tự message khi poll trả về sau save và khi lỗi poll trả về trong command: pass. Tổng workspace 9 tests pass; fmt, clippy -D warnings, build bundle và codesign verify đều pass. Logs tại `docs/reports/lifecycle/clippy.log` và `build.log`.

Chỉ xác minh state machine trong lượt này; không thay thế smoke UI US-005 hoặc benchmark US-004. CUA inventory vẫn lỗi native pipe. Bundle đã cập nhật, không thay dữ liệu/engine Android.
