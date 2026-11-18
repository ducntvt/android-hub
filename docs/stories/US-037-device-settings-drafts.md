# US-037 — Cấu hình có trạng thái chưa lưu và nhóm nội dung rõ ràng

- **Status**: implemented
- **Owner**: Desktop
- **Created**: 2026-11-17
- **Risk lane**: normal
- **Related**: [UX roadmap](../UX-ROADMAP.md), [US-036](US-036-feedback-and-confirmations.md)

## Context

Header hiện đọc tên từ draft, chọn máy khác nạp đè draft, và nút Lưu vẫn bật dù chưa thay đổi. Panel dài trộn cấu hình hiệu năng với dữ liệu. Người dùng cần biết thay đổi nào đã có hiệu lực mà không làm mất bản sửa khi chuyển ngữ cảnh.

## Contract (acceptance criteria)

- [x] Dirty được so với profile đã lưu; Save chỉ bật khi có thay đổi hợp lệ và máy dừng. Có Hủy thay đổi, lỗi validation cạnh trường liên quan.
- [x] Header/danh sách dùng tên đã lưu; bản nháp được đánh dấu Chưa lưu trong panel.
- [x] Chọn máy khác, tạo máy, đóng panel hoặc thoát với draft dirty có hành vi Lưu/Bỏ thay đổi/Ở lại rõ ràng; khi Save thất bại giữ draft và không tiếp tục navigation đã chờ.
- [x] Poll không ghi đè bản nháp; dữ liệu lưu bị thay đổi bên ngoài không bị ghi đè âm thầm. Nêu xung đột và cho nạp lại trước khi Save.
- [x] Nhóm Thiết bị/Hiệu năng/Dữ liệu có vị trí ổn định; máy chạy vẫn thấy các trường chỉ đọc cùng lý do khóa.
- [x] Nhãn RAM, CPU, chiều màn hình và giới hạn FPS dễ hiểu, đơn vị chính xác; ghi rõ thay đổi áp dụng lần khởi động sau. Không gọi FPS cấu hình là FPS game đo được.
- [x] Không tự dừng guest để lưu; khởi động khi còn draft dirty phải giải quyết draft trước và không âm thầm chạy cấu hình cũ.

## Out of scope

- Profile hiệu năng tự điều chỉnh theo host, sửa cấu hình máy đang chạy, thay phạm vi CPU/RAM được backend hỗ trợ, persistence draft qua crash.

## Affected areas

- `crates/desktop/src/app/`, `crates/desktop/src/i18n.rs`
- Backend save chỉ khi cần thêm optimistic concurrency; trường hợp này cần ADR trước.

## Implementation notes

Giữ snapshot gốc để so sánh, pending navigation riêng và không tái dùng pending destructive actions. Không để chọn input focus của US-034 vô tình bỏ draft; chọn một quy tắc thống nhất giữa explicit pair và settings selection.

## Verify

```bash
bash scripts/check.sh
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Test dirty/cancel/save failure, poll preservation, external conflict và delayed navigation. Native smoke cấu hình QA: sửa tên/RAM, đổi máy rồi hủy/bỏ/lưu; xác minh cấu hình đúng máy, không restart guest thật. Dọn QA sau thử.

## Evidence

- [Báo cáo kiểm chứng](../reports/ux-settings-20261117.md): 150 Rust tests và
  25 Python tests đạt; fmt/clippy/syntax, build và codesign đạt.
- Native QA đạt sửa tên/RAM, chuyển máy với Ở lại/Bỏ/Lưu, đối chiếu registry đúng
  máy, conflict/reload và thoát với draft dirty. Không restart guest thật.
- [ADR 0046](../decisions/0046-settings-drafts-and-optimistic-save.md) ghi contract
  draft, deferred navigation và compare-under-lock. Giới hạn bằng chứng ghi trong report.
