# US-039 — APK và clipboard có đích rõ ràng

- **Status**: implemented
- **Owner**: Desktop
- **Created**: 2026-11-17
- **Risk lane**: normal
- **Related**: [UX roadmap](../UX-ROADMAP.md), [US-007](US-007-install-apk.md), [US-015](US-015-android-clipboard.md), [US-036](US-036-feedback-and-confirmations.md)

## Context

APK hiện chỉ qua drop toàn cửa sổ rồi xác nhận theo máy chọn; clipboard chép vào guest nhưng chưa tự dán. Cần làm rõ đích trong chế độ hai máy và tránh người dùng tưởng cài/dán không hoạt động.

## Contract (acceptance criteria)

- [x] Có Cài APK… mở file picker; picker/drop dùng cùng validation và xác nhận, chỉ hỗ trợ một .apk như hiện tại.
- [x] Xác nhận hiển thị tên file, dung lượng nếu đọc được, tên máy và phần đường dẫn chi tiết. Hủy không chạy install.
- [x] Drag hover hiển thị đích nếu API cung cấp tọa độ đáng tin; nếu không xác định được ô đích, hỏi chọn máy rõ ràng trước xác nhận, không đoán từ con trỏ cũ.
- [x] ID/PID/epoch gắn với xác nhận; đổi máy hoặc restart làm xác nhận cũ vô hiệu. Kết quả timeout giữ nhãn chưa xác nhận, không tự retry.
- [x] Clipboard ghi Chép vào clipboard Android, kèm tên đích trong phản hồi; thành công giải thích nhấn giữ ô nhập để dán. Không báo thành công trước ACK.
- [x] Không đưa nội dung clipboard vào toast/log; giữ giới hạn 64 KiB, generation gate, chỉ đọc khi click và không đồng bộ nền.
- [x] Back/Home/Recent có icon/tooltip và nhãn VI/EN; input chỉ gửi tới máy của control tương ứng.

## Out of scope

- XAPK/split APK, reinstall/update app, tự mở app sau cài, tự dán vào ô nhập, IME Unicode, keymap/game macro, clipboard hai chiều.

## Affected areas

- `crates/desktop/src/app/`, `crates/desktop/src/i18n.rs`
- `crates/infrastructure/src/{apk,display}.rs` chỉ nếu cần phản hồi typed, không đổi safety gates.

## Implementation notes

Phụ thuộc chọn máy rõ ràng của US-034. Đánh giá API file picker/drag của Iced/macOS trước khi hứa drop theo tọa độ; quyết định dependency mới cần ADR. Không dùng tọa độ của sự kiện mouse cũ để xác định drop.

## Verify

```bash
bash scripts/check.sh
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Test stale picker/drop/clipboard ACK, target restart, unsupported file và cancel. Native smoke với APK QA có sẵn và clipboard văn bản thử, hai guest QA nếu cần xác minh đích; xác nhận không gửi nhầm và dọn QA. Không đọc clipboard cá nhân khi chỉ thử layout.

## Evidence

- [Báo cáo APK/clipboard](../reports/ux-apk-clipboard-20261117.md): 166 Rust + 25 Python, build/codesign và native picker/cancel/install/clipboard trên guest QA riêng đã đạt.
- [ADR 0048](../decisions/0048-native-apk-picker-and-explicit-drop-target.md): picker AppKit, drop hỏi đích; stale target được kiểm chứng bằng regression tests. Native kéo file giữa hai cửa sổ chưa tái kiểm chứng trong lượt này, không tuyên bố drop theo tọa độ.
