# US-035 — Trạng thái khởi động và đóng Hub theo từng máy

- **Status**: implemented
- **Owner**: Desktop / infrastructure
- **Created**: 2026-11-17
- **Risk lane**: normal
- **Related**: [UX roadmap](../UX-ROADMAP.md), [US-029](US-029-stop-on-quit.md), [Architecture](../ARCHITECTURE.md)

## Context

Hiện trạng thái chạy chủ yếu dựa vào PID; thông báo sau Start nói engine đã mở nhưng Android có thể chưa boot. Người dùng cần phân biệt khởi động Android, kết nối màn hình và lỗi màn hình mà không đoán qua một nhãn “Đang xử lý”. Đóng Hub đã dừng các guest có kiểm tra danh tính, nhưng chưa có tiến độ từng máy.

## Contract (acceptance criteria)

- [x] Từng máy phân biệt đã tắt, đang mở engine, Android đang khởi động, đang kết nối màn hình, sẵn sàng điều khiển và lỗi; mỗi nhãn dựa trên tín hiệu thực tế, không dùng PID hoặc frame đầu để suy ra boot hoàn tất.
- [x] Đo boot-ready bằng probe giới hạn thời gian và ràng buộc ID/PID/session; kết quả của phiên cũ không đổi trạng thái phiên mới.
- [x] Hiển thị thời gian chờ khi khởi động kéo dài, không dựng phần trăm hay thời gian hoàn tất giả. Lỗi mở engine và lỗi display có thông báo/cách xử lý riêng.
- [x] Mất display vô hiệu hóa input của phiên đó và hiện nhãn kết nối lại; máy khác vẫn xem/điều khiển được theo gate hiện có. Không hiển thị frame cũ như ảnh live.
- [x] Khi đóng Hub, hiển thị tên và kết quả dừng từng máy; một lỗi không ngăn thử dừng máy khác. Chỉ thoát khi xác minh không còn guest đã đăng ký chạy; giữ thử lại/quay lại như hiện tại.
- [x] Không thêm polling boot tần suất cao lâu dài; không giữ task probe sau khi phiên kết thúc.

## Out of scope

- Chạy đồng thời nhiều tác vụ ghi registry, reset Android, cưỡng bức kill, tự khởi động lại guest, benchmark game và thay engine.
- Thay semantics đóng Hub; Force Quit/crash vẫn không bảo đảm cleanup.

## Affected areas

- `crates/domain/src/lib.rs`, `crates/infrastructure/src/lib.rs`, `crates/infrastructure/src/display.rs`
- `crates/desktop/src/app/`, `crates/desktop/src/i18n.rs`

## Implementation notes

Viết ADR trước khi thêm contract readiness/progress giữa infrastructure và desktop. Tách observation khỏi mutation; giữ identity checks, command epochs và quit gates. Không nhét ADB/shell vào widget. Vẫn serialize các thao tác ghi; mở rộng concurrency là một story khác.

## Verify

```bash
bash scripts/check.sh
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Thêm test có ý nghĩa cho late readiness, PID đổi, timeout display và partial stop failure. Smoke trên guest QA riêng: boot, display reconnect, đóng Hub với hai guest; ghi kết quả từng bước và đóng/dọn mọi QA do lượt thử tạo. Không dùng game/tài khoản thật để fault injection.

## Evidence

- [Final acceptance audit](../reports/ux-interaction-followup-20261117.md) maps
  the contract to code, automated tests, native observations and user feedback.
  It includes live resize, established-display recovery, partial shutdown failure
  and the corrected connection label. Verification: 190 Rust + 26 Python;
  fmt/clippy/shell checks, app build and strict codesign passed.
- [Workspace/native follow-up](../reports/ux-workspace-followup-20261117.md)
  records fitted layouts, coordinate routing, explicit slots and modal focus.
- [Feedback follow-up](../reports/ux-feedback-followup-20261117.md) covers
  long EN names. The user confirmed dragging works at Fit and 110%.
- [Initial lifecycle evidence](../reports/ux-lifecycle-20261117.md) covers bounded
  boot observation and backend stop progress. Earlier pending items are
  superseded only by the specific evidence in the final audit.
- Exact native held-pointer/slot switching and the interrupted final Retry click
  are not claimed; release ordering and retry/return use code/regression evidence.
  QA apps/guests and disposable build/data artifacts were closed and removed.
