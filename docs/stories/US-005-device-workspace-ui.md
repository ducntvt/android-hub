# US-005 — Giao diện workspace theo ảnh Device Hub

- **Status**: implemented
- **Created**: 2026-11-13
- **Owner**: Codex
- **Risk lane**: normal

## Context

Người dùng muốn giao diện lấy cảm hứng từ ảnh Device Hub: sidebar sáng, toolbar gọn, thiết bị ở trung tâm.

## Contract

- Sidebar có tìm kiếm thiết bị và tạo máy.
- Thiết bị dừng có hình minh họa và nút khởi động; thiết bị chạy giữ canvas tương tác.
- Toolbar chuyển single/split, zoom, mở panel cấu hình.
- Cấu hình chỉ sửa khi máy dừng; giữ xác nhận dừng và thông báo lỗi.
- Không thay đổi engine, dữ liệu hay timer frame.

## Affected areas

Desktop `main.rs`, `ui.rs`; không đổi layer boundary.

## Verify

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
bash scripts/build-android-hub.sh
```

UI: xem stopped state, tìm kiếm, toggle panel/split, zoom; kiểm tra màn hình nhúng nếu có engine chạy.

## Evidence

- `cargo fmt --all -- --check`, 7 tests, clippy với `-D warnings`, build bundle: pass.
- Giao diện tách riêng sang `crates/desktop/src/ui.rs`; cập nhật hướng dẫn README.
- UI native chưa xác minh: CUA gọi getApp trả `Sky Computer Use native pipe startup failed`. Chưa có screenshot bản mới, chưa thử tương tác thực tế. Giữ trạng thái in_progress đến khi smoke UI hoàn tất.
- `Android Hub.app` đã đóng gói; cần đóng/mở lại app để nạp giao diện mới. Dữ liệu A/B không thay đổi.
- Cảnh báo gián tiếp `block 0.1.6` future compatibility vẫn còn như trước.

- Cập nhật xác nhận người dùng: sau yêu cầu mở lại Hub và kiểm tra sidebar, Hai máy, panel Cấu hình có cắt/chồng chữ không, người dùng trả lời “Hiển thị ổn”. Đây là evidence thủ công cho ba phần bố cục này; chưa xác nhận search, zoom, stopped state, các nút hoặc input mới. Story vẫn in_progress cho coverage còn lại.

### 2026-11-16 native follow-up

CUA hoạt động trở lại; smoke một phần và sửa scrollbar clipping được ghi tại [UI evidence](../reports/ui-user-validation.md). Không suy ra các thao tác chưa thực hiện là đạt; story vẫn in_progress theo phần kiểm chứng còn thiếu.

### Settings hints follow-up

Native QA phát hiện stopped/busy/stale dùng chung lời nhắc dừng máy. Sửa nhãn cấu hình và khôi phục theo trạng thái thực; không thay gate hoặc thao tác backend. `cargo fmt --all -- --check`, 22 desktop tests, build bundle và codesign verification pass. Logs: [build](../reports/restore/settings-hint-build.log), [tests](../reports/restore/settings-hint-tests.log). Bản QA đang mở chưa reload binary mới; chưa nhận là native verify nhãn mới.

### Final native acceptance — 2026-11-16

Zoom 100% → 80% → 110% → reset 100% and scaled tap verified on embedded QA Android. Stopped the guest through confirmation afterward. Search/create/stopped/live/split/settings/error coverage and all-Verify audit are consolidated in [UI acceptance report](../reports/ui-user-validation.md#zoom-and-final-workspace-acceptance--2026-11-16). Earlier pending notes above are historical; this story is now implemented. Input interruption and game performance remain separate unfinished stories.
