# 0017 — Dự phòng root dữ liệu khi source checkout không còn

> Superseded by [ADR 0054](0054-app-owned-data-root.md): default data now lives directly in Application Support after the user-authorized reset. The policy below is historical.

- **Status**: Superseded by 0054
- **Date**: 2026-11-13

Root được chọn theo thứ tự: ANDROID_HUB_ROOT nếu có; root project lúc build nếu metadata không xác nhận NotFound; nếu thực sự NotFound thì ~/Library/Application Support/Android Hub. Lỗi quyền truy cập, file thay directory hoặc dangling symlink tại root không được coi là lý do tự đổi dữ liệu. Backend giữ trách nhiệm báo lỗi truy cập.

Không canonicalize root bằng expect khi khởi động. Resolver chỉ đọc metadata, không tạo hoặc migrate file. Khi fallback, notice sau initialize báo rõ root mới và dữ liệu cũ chưa được di chuyển; panel cấu hình hiển thị root đang dùng. Bên dưới root vẫn giữ layout experiments/p0/runtime để không đổi đồng thời persistence backend.

App copy sang máy khác vẫn cần SDK/Java/image; đây chưa phải installer hoặc migration dữ liệu. AVD và backup chứa path tuyệt đối nên không tự dò/mở bản runtime bị di chuyển. Nếu root project xuất hiện lại, thứ tự trên lại ưu tiên project; dùng ANDROID_HUB_ROOT để cố định nơi lưu khi cần. UI chọn root/persist cấu hình còn là việc tiếp theo của packaging.
