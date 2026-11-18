# US-038 — Tạo máy qua biểu mẫu ngắn trước khi ghi dữ liệu

- **Status**: implemented
- **Owner**: Desktop / infrastructure
- **Created**: 2026-11-17
- **Risk lane**: high-risk
- **Related**: [UX roadmap](../UX-ROADMAP.md), [US-019](US-019-creation-recovery.md), [US-021](US-021-create-space-preflight.md), [US-036](US-036-feedback-and-confirmations.md)

## Context

Hiện nút + gọi create ngay với tên/cấu hình tự chọn. Cần bước xem trước ngắn để tránh tạo nhầm và cho biết image/điều kiện môi trường, đồng thời giữ journal recovery và bảo vệ dữ liệu hiện có.

## Contract (acceptance criteria)

- [x] + mở form; Hủy không tạo journal, AVD hoặc registry entry. Form có tên gợi ý chưa trùng, Android image hiện hỗ trợ và cấu hình mặc định; nâng cao thu gọn.
- [x] Kiểm tra tên/thông số trước tạo; backend tự cấp ID/port an toàn, không nhận đường dẫn tùy ý từ UI.
- [x] Hiện tình trạng SDK/Java và dung lượng trống nếu probe thành công; 12 GiB admission floor hiện có được gọi đúng là mức trống tối thiểu, không phải dung lượng game được bảo đảm.
- [x] Thiếu môi trường thì chỉ rõ cách sang thiết lập; không tự tải/chấp nhận license hoặc bắt đầu tạo rồi mới giải thích điều kiện đã biết thiếu.
- [x] Một lần xác nhận chỉ tạo một máy; nút và trạng thái phản hồi ngay, không double-submit. Lỗi giữ input và giải thích khi đã có dữ liệu dở, không tự xóa để retry.
- [x] Có Khởi động sau khi tạo (mặc định tắt); chỉ start đúng ID vừa tạo sau khi đăng ký hoàn tất. Nếu tạo xong nhưng start lỗi, không tạo lại máy.
- [x] Journal/recovery, preflight và identity checks vẫn hoạt động với cấu hình người dùng nhập; dữ liệu thật không bị đổi.

## Out of scope

- Chọn tùy ý image Android, clone máy/tài khoản, import AVD, tự tối ưu cấu hình theo host, wizard nhiều trang khi môi trường đã đủ.

## Affected areas

- `crates/infrastructure/src/{lib,creation,storage}.rs`
- `crates/domain/src/lib.rs`, `crates/desktop/src/app/`, `crates/desktop/src/i18n.rs`

## Implementation notes

ADR cho create request/result contract mới. Result trả ID rõ ràng, không suy ra bằng profiles.last(). Giữ port/path validation và recovery metadata. Chỉ làm readiness check cần thiết, timeout có giới hạn.

## Verify

```bash
bash scripts/check.sh
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Test cancel/no write, custom-profile validation, double-submit, interrupted create và created-but-start-failed. Native QA tạo một máy độc lập không tài khoản; kiểm tra registry và tùy chọn start, dừng đúng guest rồi dọn app/test data do ca này tạo. Không dùng runtime cá nhân để fault injection.

## Evidence

- [Verification report](../reports/ux-creation-20261117.md): 162 Rust tests, 25 Python tests, fmt/clippy/shell checks, build và codesign đạt.
- Native QA: tên rỗng bị chặn, Hủy không ghi dữ liệu, tạo đúng một máy 4096 MiB, tự khởi động đúng ID và hiển thị Android Home. Dừng máy và thoát QA được xác nhận qua UI/process.
