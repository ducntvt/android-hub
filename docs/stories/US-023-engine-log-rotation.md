# US-023 — Xoay log trước khi khởi động engine

- **Status**: implemented
- **Owner**: Codex
- **Created**: 2026-11-13
- **Risk lane**: normal
- **Related**: [ADR 0021](../decisions/0021-engine-log-retention.md)

## Contract

- Start giữ lock, kiểm tra guest dừng/port rảnh và SDK sẵn sàng trước khi xoay log.
- Nếu `<id>.log` dưới 8 MiB hoặc chưa có, giữ nguyên. Nếu đạt ngưỡng, xoay thành `.log.1`, giữ thêm `.log.2` và `.log.3`, xóa bản `.log.3` cũ.
- Chỉ chạm đúng tên log của profile đã validate; kiểm tra tất cả destination trước khi sửa. Symlink, directory, special file hoặc hardlink bị từ chối.
- Không thay file guest, backup, cấu hình hoặc log thiết bị khác. Không xoay/truncate log đang chạy.
- Panel dung lượng tính cả ba archive vào nhóm Log engine.

## Out of scope

Giới hạn bytes trong phiên chạy dài, nén log, upload log, dọn log máy đã gỡ, crash-safe bảo toàn mọi bản log lịch sử.

## Verify

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Fixture: không có log, log nhỏ, ngưỡng đúng, retention ba bản, guest khác giữ nguyên, symlink/hardlink từ chối trước xóa. Không chạy xoay trên log người dùng trong phiên này.

## Evidence

[Tests](../reports/log-rotation/tests.log), [clippy](../reports/log-rotation/clippy.log), [build](../reports/log-rotation/build.log).

54 unit tests, fmt/clippy/build/codesign pass. Kiểm chứng thao tác xoay trên fixture; chưa thực hiện rotation trên log runtime người dùng. Không suy ra giới hạn dung lượng trong một phiên chạy từ retention theo số file.
