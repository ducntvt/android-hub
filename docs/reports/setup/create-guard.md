# Kiểm tra lỗi tạo AVD — 2026-11-13

- Source trước đó chỉ kiểm tra `.avd.exists()`, bỏ sót `.ini` và dangling symlink. Nay symlink_metadata kiểm tra cả hai path và fail closed nếu lỗi đọc metadata.
- Lỗi avdmanager/configuration/registry được bổ sung ID và đường dẫn dữ liệu có thể tạo dở. Giữ dữ liệu nguyên vẹn, chưa tự rollback/cleanup.
- Sau registry commit, lỗi snapshot báo đã tạo thành công; tránh hướng người dùng tạo thêm máy trùng mục đích.
- 29 unit tests pass, 4 integration opt-in ignored. Test fixture chứng minh `.ini` không bị ghi đè và dangling symlink không bị coi là path rảnh; nội dung registration giữ nguyên.
- Không claim đã fault-inject timeout avdmanager hoặc recovery sau crash. Đây là guard/error reporting, chưa phải transaction tạo máy.
- fmt/clippy -D warnings/build/codesign pass. Logs: [clippy](create-guard-clippy.log), [build](create-guard-build.log).
