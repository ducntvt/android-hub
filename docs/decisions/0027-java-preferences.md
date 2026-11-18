# ADR 0027 — Chọn và lưu Java home

- Status: Accepted
- Date: 2026-11-13
- Stories: US-026, US-025

## Quyết định

hub-settings.json schema 1 bổ sung java_home optional/null để đọc tương thích cấu hình cũ. Thứ tự ưu tiên: P0_JAVA_HOME, java_home đã lưu, JBR của Android Studio mặc định. Trường đã lưu phải là absolute path/string; dữ liệu sai báo lỗi, không âm thầm dùng fallback khi tạo máy. Backend.java_home trả Result; create truyền lựa chọn đã đọc vào JAVA_HOME của avdmanager trong lúc giữ Hub lock.

configure_java yêu cầu absolute path, không có override P0_JAVA_HOME, SDK session còn khớp, Hub lock, snapshot tất cả máy dừng và port gate. Sau đó probe Java/ADB/emulator/avdmanager theo ADR 0026 trước atomic write 0600/fsync/rename. Lỗi không sửa preferences. Khi ANDROID_SDK_ROOT ghi đè phiên hiện tại, vẫn giữ SDK đã lưu trong file cấu hình; chỉ Java thay đổi. Không migrate AVD hoặc sửa SDK/Java binary.

SDK selection giữ java_home đã lưu khi ghi lại. Luồng sửa SDK preferences bị hỏng vẫn dùng default/env Java để kiểm tra components trước khi thay cấu hình; đây là thao tác repair rõ ràng, không phải fallback trong launcher. Chưa tự tải Java.

Panel Thiết lập SDK có Java home, nút Kiểm tra và lưu Java, trạng thái override và yêu cầu dừng máy. Lưu chạy spawn_blocking; busy/command_epoch chặn thao tác chồng và snapshot cũ; thành công initialize trước thao tác tiếp theo. Không chạy probe trong refresh. UI scrolling sẵn có để chứa thêm fields.

## Kiểm chứng và giới hạn

Unit: schema cũ, optional Java, invalid field, preserve khi đổi SDK, reload backend, UI failure/success callback. Integration trên temp runtime: SDK/JBR thật, save/reload, invalid Java không sửa prefs, SDK save giữ Java, port bận chặn Java save. Không tạo hoặc boot guest trong test này.

Cần native visual/interaction smoke cho panel Java mới; CUA đang lỗi native pipe startup. Xác nhận UI cũ của người dùng không được coi là evidence cho phần mới. US-026 giữ in_progress. Installer US-025 vẫn thiếu assembly/publish/recovery, UI license/download orchestration và setup từ môi trường trống.

## Cập nhật kiểm chứng 2026-11-16

US-026 đã hoàn tất native invalid/valid/save/reopen trên runtime QA riêng và toàn bộ Verify; xem [báo cáo](../reports/sdk-java-native-20261116.md). Giới hạn native pipe ở trên là lịch sử. Installer US-025 có triển khai riêng, vẫn chưa đạt candidate end-to-end.
