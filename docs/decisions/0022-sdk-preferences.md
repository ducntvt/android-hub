# 0022 — Lưu lựa chọn SDK theo runtime

- **Status**: Accepted
- **Date**: 2026-11-13

## Decision

Lưu schema 1 `{sdk: absolute canonical path}` tại runtime/hub-settings.json, ưu tiên sau ANDROID_SDK_ROOT và trước default ~/Library/Android/sdk. Desktop có panel độc lập với selected device. Validate SDK sẵn có trước ghi, dùng lock và stopped gates; file mới mode 0600, sync rồi rename. Temp duy nhất tránh việc temp dở từ crash chặn lần lưu sau; temp dở không được đọc thành preference.

Không đổi SDK khác khi runtime chứa AVD/archive vì config và backup có đường dẫn image tuyệt đối. Guard đọc preference mới nhất dưới lock thay vì tin SDK được load từ lúc app mở. Nếu preference hỏng với dữ liệu tồn tại, từ chối đoán SDK cũ; cần phục hồi cấu hình đúng. Migration phải là story riêng.

## Consequences

Chọn SDK lần đầu không cần biến môi trường/Terminal. SDK installer và Java vẫn ngoài phạm vi. Khi override môi trường có mặt, UI không cho lưu preference để tránh báo đã áp dụng nhưng thực ra bị ghi đè. Preference sai không panic app; các lệnh cần SDK báo lỗi. Start/create trong phiên Hub cũ bị chặn nếu preference đã đổi; mở lại phiên đó để dùng cấu hình mới.

Không download, chấp nhận license, copy image hoặc cập nhật SDK. Metadata preflight kiểm tra file/executable hiện diện, không chứng minh tương thích mọi phiên bản emulator.
