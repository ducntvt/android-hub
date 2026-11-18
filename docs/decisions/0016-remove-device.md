# 0016 — Gỡ thiết bị qua journal và archive

- **Status**: Accepted
- **Date**: 2026-11-13

Gỡ khỏi danh sách được tách khỏi xóa vĩnh viễn. Giữ AVD và registration trong `.removed-ID-TOKEN` dưới AVD_HOME để rename cùng filesystem. Thư mục archive quyền 0700. Profile và registry trước/sau nằm trong journal runtime/remove-pending-ID.json; không ghi credential mới. Backup và previous restore không bị đụng tới.

Journal pending → move AVD → move ini → registry → committed marker → receipt. Sync journal và parent directory. Recovery khi lock/initialize: pending rollback, committed hoàn tất receipt. Identity dev/inode và registry snapshot phải khớp trước move; giữ nguyên nếu có thay đổi ngoài Hub. Stop dùng raw lock như restore recovery. Không tự purge archive.

Backend R1 chưa được nối UI. Phải có undo R2 và xác nhận rõ chưa giải phóng dung lượng ở R3. Chưa chứng minh mất điện thực hoặc phòng được emulator mở bên ngoài Hub trong khoảng giữa các kiểm tra.

## R2 — Đưa máy trở lại

Undo đọc removed.json đúng archive suy ra từ ID/token, kiểm tra profile người dùng xác nhận và inode/device của archive/data/registration. Không chấp nhận path ngoài namespace hoặc archive đã có restored.json. Không ghi đè ID/port đang dùng hay live path tồn tại (kể cả symlink). Port cũ được giữ; nếu bị máy mới dùng, undo từ chối thay vì tự đổi mapping.

Undo dùng journal cùng loại, operation=undo và registry before/after hiện tại. Pending rollback về archive, committed giữ máy đã đưa trở lại. Biên nhận undo thất bại có tên riêng để retry không xung đột; biên nhận restored.json chặn replay. Journal .new còn lại được giữ với tên duy nhất trong archive. Startup recovery kiểm tra stopped/port cả khi profile không còn trong registry.

Không so checksum toàn bộ nội dung archive trong undo: kiểm tra identity chống nhầm directory, không chứng minh nội dung guest chưa bị chỉnh ngoài Hub. Đây là dữ liệu được gỡ tạm, không thay thế backup có SHA-256. UI R3 vẫn chưa bật.

R3 đã nối panel/xác nhận remove/undo sau khi R2 guest smoke pass. Catalog đọc receipt theo namespace, không sửa file; liệt kê lý do ID/port/path bận. UI không cho thao tác khi state stale/busy, backend vẫn kiểm tra lại. Lịch sử recovery và native smoke còn pending.

## Lịch sử R3

Startup và kết quả remove/undo đọc lại biên nhận từ archive. Receipt phải đúng path suy ra ID/token, state/operation và filename tương ứng; kiểm tra registry before/after, profile và archive identity như journal. Reader chỉ đọc, không phục hồi hoặc đổi dữ liệu. Mỗi loại lịch sử giới hạn ba biên nhận gần nhất; lỗi receipt được báo thiếu coverage thay vì gán success. Đây là lịch sử, không khẳng định sự kiện vừa xảy ra trong lần mở này.

Task lịch sử restore/removal dùng chung epoch: bỏ phản hồi cũ khi có lần đọc mới hoặc khi người dùng Ẩn. Không thêm timer/poll filesystem định kỳ. Native smoke banner còn pending.
