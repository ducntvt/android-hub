# US-036 — Thông báo và xác nhận không làm co vùng Android

- **Status**: implemented
- **Owner**: Desktop
- **Created**: 2026-11-17
- **Risk lane**: normal
- **Related**: [UX roadmap](../UX-ROADMAP.md), [US-007](US-007-install-apk.md), [US-009](US-009-restore-recovery.md), [US-016](US-016-remove-device.md)

## Context

Hiện lịch sử, xác nhận và kết quả APK được thêm vào cột bên dưới stage, làm thay đổi diện tích chơi. Phần lớn lỗi/thành công dùng chuỗi notice chung, thiếu phân cấp và tên máy nhất quán. Các kiểm tra backend đang có phải được giữ khi thay presentation.

## Contract (acceptance criteria)

- [x] Thành công ngắn hiện toast không chiếm layout; lỗi cần hành động giữ lại đến khi xử lý hoặc đóng, không bị một poll thành công ghi đè.
- [x] Xác nhận dừng, cài APK, restore và lưu trữ máy dùng một overlay/sheet nhất quán; stage không đổi kích thước khi mở/đóng.
- [x] Hộp xác nhận nêu máy đích và tác động, giữ identity snapshot hiện có; chọn máy/đổi phiên không thể khiến xác nhận cũ chạy trên đích mới.
- [x] Modal chặn input Android phía sau, release input khi mở, có focus rõ; Esc hủy và trả focus hợp lý. Không tự xác nhận destructive action bằng Enter khi focus chưa rõ.
- [x] Thông báo phân biệt thành công, thông tin, lỗi, kết quả chưa xác định; lỗi có phần Chi tiết và Sao chép lỗi, không sao chép clipboard/game/account content.
- [x] Lịch sử dữ liệu mở trong panel riêng; biên nhận phục hồi sau gián đoạn vẫn được báo rõ và không tự biến mất trước khi người dùng thấy.
- [x] Chuỗi hiển thị hỗ trợ VI/EN, raw diagnostics được giữ nguyên trong Chi tiết.

## Out of scope

- Đổi transaction backup/restore/removal, tự retry thao tác ghi, log collector toàn hệ thống, thêm telemetry hoặc thay tất cả SDK consent bằng auto-approval.

## Affected areas

- `crates/desktop/src/app/{mod,update,ui,tests}.rs`
- `crates/desktop/src/i18n.rs`
- `crates/infrastructure/src/lib.rs` — expected PID gate cho dừng đã xác nhận;
  xem [ADR 0045](../decisions/0045-modal-feedback-and-confirmed-stop.md).

## Implementation notes

Dùng typed notification có severity, subject và ID để timer dismiss không xóa thông báo mới. Chỉ một destructive confirmation active. Không giảm backend guards để đơn giản hóa UI; giữ kết quả cài APK timeout là chưa xác nhận.

## Verify

```bash
bash scripts/check.sh
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Test late dismiss, confirmation invalidation, modal input gate và kết quả lỗi. Native smoke trên data QA: confirm/cancel/Esc và đổi ngôn ngữ, kiểm tra stage không đổi kích thước; không thực hiện restore dữ liệu thật. Đóng/dọn QA sau thử.

## Evidence

- [Final acceptance audit](../reports/ux-feedback-followup-20261117.md) maps all
  seven contract items to current source, regression tests and native evidence.
- [Initial implementation](../reports/ux-feedback-20261117.md): shared overlay,
  typed feedback, history panel, expected-PID stop, VI/EN, Tab/Enter/Esc and
  persistent errors. Its remaining-check list is superseded by the follow-ups.
- [Running-guest follow-up](../reports/ux-workspace-followup-20261117.md): fixed
  and verified keyboard focus after cancel; background mouse/key input blocked.
- Final native checks verified success-toast expiry, actual privacy-preserving
  clipboard output, compact Details, and startup rollback with persistent unread
  receipt presentation in VI/EN. APK/restore/archive variants have linked evidence
  from US-039/042 in the final audit.
- Final Verify: 189 Rust + 26 Python passed; fmt/clippy/shell, app build and strict
  codesign passed. Twenty existing integration tests remain ignored.
- Main app rebuilt. Owned QA apps/processes closed; target and disposable QA
  artifacts removed. Personal runtime and SDK preserved. No commit.
