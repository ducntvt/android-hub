# 0003 — Dùng minifb cho cửa sổ kiểm chứng P0

- **Status**: Deprecated
- **Date**: 2026-11-13
- **Deciders**: Chủ dự án
- **Supersedes**: none

## Context

P0 cần nhìn và thao tác guest trước khi đầu tư giao diện quản lý Iced. Engine cung cấp frame RGBA/BGRA qua shared memory và input qua control socket. Chưa cần widget quản lý máy.

## Decision

Thử minifb trong `experiments/p0/viewer` như một cửa sổ framebuffer Rust nhỏ, tách Cargo package khỏi ứng dụng sản phẩm. Iced vẫn là đề xuất cho UI sản phẩm; không thay stack sản phẩm bằng minifb.

## Consequences

- Positive: kiểm chứng đường frame và chuột trực tiếp bằng Rust, không cần video encoder.
- Negative / cost: CPU copy và chuyển pixel tạm thời; không dùng số đo viewer này để cam kết hiệu năng UI cuối. Viewer chỉ hỗ trợ display 0 cố định và tự đóng sau 600 giây.
- Follow-ups required: kiểm chứng trên VM, bổ sung test bounds/layout, thiết kế renderer sản phẩm sau P0. Chưa tuyên bố zero-copy hoặc hỗ trợ mọi resize.

## Alternatives considered

### Alternative A — Iced ngay từ đầu

Giữ cho UI sản phẩm; bước này chỉ cần cửa sổ đọc frame để tách lỗi engine khỏi quản lý UI.

### Alternative B — scrcpy

Hữu ích đối chứng nhưng thêm encoder/decoder và không trực tiếp kiểm chứng frame protocol của engine.

## References

- [ADR 0002](0002-build-personal-android-vm-with-rust.md).
- [minifb Window API](https://docs.rs/minifb/0.29.0/minifb/struct.Window.html).

## P0 implementation note

Thử nghiệm dùng bản minifb 0.29.0 tại `experiments/p0/vendor/minifb` với patch macOS về polarity focus và tọa độ NSEvent. Giữ license và ghi thay đổi trong `P0-PATCH.md`. Chỉ áp dụng viewer P0; chưa chọn dependency này cho sản phẩm. Tap/nhấn giữ/Home có bằng chứng thực tế; drag sau patch và focus nhiều cửa sổ cần kiểm chứng tiếp.

## Retirement — 2026-11-16

Viewer/minifb và runtime Avo đã được dọn theo yêu cầu người dùng. Hub dùng Iced/gRPC theo ADR 0005. Nội dung phía trên là quyết định thử nghiệm lịch sử, không phải hướng dẫn chạy hiện tại.
