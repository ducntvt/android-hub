# US-004 — Dọn code và giảm cập nhật khi Hub rảnh

- **Status**: implemented
- **Owner**: Codex
- **Created**: 2026-11-13
- **Risk lane**: normal
- **Related**: [ADR 0006](../decisions/0006-frame-notifications.md)

## Context

Hub đánh thức UI mỗi 33 ms ngay cả khi không có thiết bị/frame. Mỗi tick còn chọn lại thiết bị và clone profile.

## Contract (acceptance criteria)

- Không còn timer frame khi rảnh; frame mới vẫn hiển thị, tối đa khoảng 30 cập nhật/giây/máy.
- Giữ frame mới nhất và thông báo có giới hạn, không xếp hàng ảnh.
- Chuyển thiết bị/chế độ xem và input vẫn hoạt động.
- Ghi số đo CPU khi rảnh và phạm vi kiểm chứng; giữ dữ liệu A/B.

## Out of scope

Engine Android, launcher B cũ, benchmark game P0, xóa dữ liệu/thí nghiệm lịch sử.

## Affected areas

`crates/desktop/src/main.rs`, `crates/infrastructure/src/display.rs`, `crates/infrastructure/src/lib.rs`, tài liệu.

## Verify

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
bash scripts/build-android-hub.sh
```

UI: mở Hub mới, kiểm tra frame và input, chuyển single/split. Đo CPU idle bằng chênh lệch CPU time/wall time.

## Evidence

Xem [báo cáo](../reports/hub-performance.md).

### Idle measurement follow-up

Đã đo Hub thật 60 giây: 0.664% CPU theo delta CPU time, không engine Android. Xem [báo cáo](../reports/hub-performance.md). Native storage/search/stopped UI đã phản hồi; live frame/input/split vẫn thiếu, giữ in_progress.

### Final acceptance audit — 2026-11-16

Native live single/split input and workspace zoom now complete the previously missing smoke. Raw idle trace, current bounded-frame implementation, notification regression and all Verify results were rechecked in the [final audit](../reports/hub-performance.md#final-us-004-acceptance-audit--2026-11-16). Earlier pending notes are historical. This closes idle Hub optimization only; game/P0 performance remains open.
