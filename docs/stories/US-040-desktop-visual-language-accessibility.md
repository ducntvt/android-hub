# US-040 — Ngôn ngữ thị giác, bàn phím và VI/EN nhất quán

- **Status**: implemented
- **Owner**: Desktop
- **Created**: 2026-11-17
- **Risk lane**: normal
- **Related**: [UX roadmap](../UX-ROADMAP.md), [US-027](US-027-ui-language-polish.md), [US-036](US-036-feedback-and-confirmations.md)

## Context

Đã có VI/EN lưu preference và ba kiểu nút. Còn icon ký tự, nhiều cỡ chữ, theme sáng cố định và một số nhãn tiếng Anh/thuật ngữ kỹ thuật. Story này hoàn thiện shell/control của Hub; không hứa accessibility cho nội dung Android chỉ là ảnh stream.

## Contract (acceptance criteria)

- [x] Token màu/khoảng cách/cỡ chữ/radius nhất quán; trạng thái có chữ/icon ngoài màu, primary được dùng có chủ đích. Icon vector không làm lệch +/− giữa font/ngôn ngữ.
- [x] Có theme Hệ thống/Sáng/Tối, lưu preference không chứa dữ liệu guest; palette custom tuân theo theme và đủ tương phản. Thay theme không restart guest.
- [x] VI/EN bao phủ controls/tooltip/empty state mới, giữ nguyên tên riêng/path/license/raw diagnostics; thuật ngữ máy Android, RAM, FPS và phiên bản Android dùng nhất quán.
- [x] Cài đặt Hub gom ngôn ngữ/theme/môi trường/dung lượng; giữ truy cập nhanh không bắt người dùng vào panel sâu để đổi ngôn ngữ. Không viết lại SDK workflow.
- [x] Controls quan trọng có focus thấy được và thao tác bàn phím; phím tắt Hub không đồng thời gửi vào Android. Esc hủy modal; không chiếm text input khi gõ tên/search.
- [x] Kiểm chứng tên dài, VI/EN, cửa sổ tối thiểu và dark/light; chữ/control không bị cắt và không phụ thuộc màu để hiểu trạng thái.
- [x] Audit VoiceOver với bộ control Hub thực tế, ghi rõ hỗ trợ/giới hạn của Iced. Chỉ đánh dấu accessibility đã đạt cho phần có bằng chứng; nếu cần đổi framework/backend, tách story mới và ghi gap rõ. **AX chưa có control Hub; VoiceOver chưa đạt, chưa kiểm tra spoken output; gap tách thành US-043.**

## Out of scope

- Framework rewrite, Liquid Glass/blur nặng, accessibility cây UI bên trong Android, theme game, mọi shortcut nâng cao hoặc menu bar native hoàn chỉnh.

## Affected areas

- `crates/desktop/src/app/`, `crates/desktop/src/i18n.rs`, module preference/theme mới nếu cần
- `crates/desktop/Cargo.toml` chỉ nếu thiếu capability, cần đánh giá dependency trước.

## Implementation notes

Chốt semantics US-033/034/036 trước visual polish. Hệ thống theme phải dùng sự kiện/preference thực, không suy ra từ giờ trong ngày. Accessibility audit là bằng chứng phạm vi, không phải lời hứa VoiceOver hoạt động chỉ vì có tooltip.

## Verify

```bash
bash scripts/check.sh
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Test preference round-trip/fallback và shortcut routing trong/ngoài input/modal. Native checklist VI/EN, sáng/tối, resize, keyboard focus và VoiceOver; ghi limitation nếu có. Chỉ mở QA riêng và đóng/dọn sau kiểm tra.

## Evidence

- [Báo cáo triển khai và QA](../reports/ux-appearance-20261117.md): theme/token/vector icons, Cài đặt Hub, Command shortcuts, Tab/Shift-Tab, Enter/Space/↑↓ và tự cuộn theo focus; 176 Rust + 26 Python, build/codesign đạt.
- Native System/light ban đầu, Dark, VI/EN, tên dài, ⌘F/⌘,/⌘N, chọn máy bằng bàn phím và form đã kiểm tra. Cửa sổ 1000×640, màu sidebar cuối cùng, autofocus/checkbox/placeholder đều quan sát trên QA riêng.
- [US-043](US-043-native-hub-accessibility.md) giữ gap native accessibility. Không tuyên bố VoiceOver đã đạt, live-drag resize hay đổi appearance macOS trong lúc game chạy đã được kiểm chứng; xem giới hạn trong report.
