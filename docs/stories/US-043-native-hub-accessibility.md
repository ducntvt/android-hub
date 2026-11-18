# US-043 — Native accessibility cho control Hub

- **Status**: planned
- **Owner**: Desktop
- **Created**: 2026-11-17
- **Risk lane**: normal
- **Related**: [US-040](US-040-desktop-visual-language-accessibility.md), [ADR 0050](../decisions/0050-keyboard-focus-and-scroll-reveal.md)

## Context

Audit native US-040 thấy AX chỉ có cửa sổ, title/menu và nút quản lý cửa sổ. Các button, input, checkbox và lựa chọn do Iced vẽ chưa được công bố thành accessibility nodes. Focus bằng Tab hoạt động nhưng không đồng nghĩa VoiceOver đọc và thao tác được các control này.

## Contract (acceptance criteria)

- [ ] Đánh giá khả năng tích hợp accessibility của phiên bản Iced/backend thực tế; chốt ADR trước khi nâng dependency hoặc thay backend.
- [ ] Sidebar, toolbar, form và dialog có role, tên VI/EN, giá trị, trạng thái enabled/selected/focused và action đúng đích trên macOS AX.
- [ ] VoiceOver đọc và thao tác được control Hub; focus modal không thoát ra nền. Kiểm chứng nội dung đọc thực tế, không suy từ tooltip hoặc viền focus.
- [ ] Không đưa frame/game/account/clipboard Android vào cây AX hoặc log. Phân biệt rõ accessibility của Hub với Android canvas.
- [ ] Không làm tăng vòng lặp render/poll hoặc phá keyboard/input, dirty draft, session identity và quit guards.

## Out of scope

- Cây accessibility của ứng dụng bên trong Android; viết lại engine; tự bật VoiceOver cho máy người dùng.
- Story này là backlog tách từ gap được US-040 cho phép, không tự mở rộng đợt US-033–042 thành framework rewrite.

## Affected areas

- `crates/desktop/src/app/`, backend accessibility của Iced và Cargo dependencies nếu cần.

## Verify

```bash
bash scripts/check.sh
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Native QA riêng: AX tree và VoiceOver spoken output cho danh sách máy, tìm kiếm, cấu hình, tạo máy, xác nhận/hủy. Kiểm tra VI/EN, focus order và stale target. Đóng QA và dọn artifact sau kiểm tra.

## Evidence

- [US-040 audit](../reports/ux-appearance-20261117.md): AX không có control Hub ở shell, preferences hoặc form tạo máy. Chưa có bằng chứng VoiceOver đọc được chúng.
