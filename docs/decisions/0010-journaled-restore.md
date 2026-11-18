# 0010 — Khôi phục offline với journal và dữ liệu cũ được giữ lại

- **Status**: Accepted
- **Date**: 2026-11-13
- **Deciders**: triển khai US-009 R2
- **Supersedes**: none; bổ sung ADR 0009

## Decision

Restore giữ lock Hub, kiểm tra máy dừng và compatibility backup, copy/verify/sync staging cùng filesystem AVD. Journal pending ghi profile trước/sau, token số và định danh device/inode của directory; đường dẫn được suy ra từ ID/token đã validate, không lấy đường dẫn tùy ý trong journal.

Thứ tự: journal pending → live sang previous → stage sang live → registry mới → journal committed → archive biên nhận. File journal được sync rồi rename; sync parent directory sau thay đổi. Dữ liệu cũ được giữ trong `.previous-ID-TOKEN`, không tự xóa sau commit.

Khi khởi tạo hoặc nhận khóa thao tác mới, Hub kiểm tra journal. Pending rollback về directory/profile cũ; committed chỉ hoàn tất biên nhận. Kiểm tra engine/port trước recovery. Stop dùng khóa thô để người dùng có thể dừng một engine đã mở ngoài Hub trước khi recovery. Snapshot/read-only không tự thay dữ liệu.

## Failure behavior

Kiểm tra định danh directory trước mọi rename/delete recovery. Registry khác cả before/after hoặc directory lạ khiến recovery dừng, giữ dữ liệu/journal để kiểm tra. Recovery có thể lặp sau gián đoạn tại các bước đổi directory. Một file .json.new chưa publish không thể đã đổi live theo protocol; dọn file tạm khi máy dừng, không đoán/xóa directory không có journal.

Lỗi ghi biên nhận sau committed không được rollback kết quả đã commit. Lỗi refresh sau commit được báo rõ đã commit. Receipt giữ ở runtime/restore-history; UI thông báo recovery/rollback và lựa chọn giữ/xử lý dữ liệu cũ thuộc R3. Copy staging bị gián đoạn trước journal có thể để lại `.restore-*` cần dọn có kiểm tra ở bước quản lý dữ liệu sau.

## Limits

Kiểm chứng bằng fault injection trạng thái filesystem và guest thử; chưa kiểm thử mất điện/thất bại phần cứng. Không bảo đảm atomic với emulator do người dùng mở ngoài Hub giữa các phép kiểm tra; không được thao tác SDK ngoài Hub trong lúc restore. Backend đã nối UI ở R3; smoke UI còn pending.

## Alternatives

Ghi đè live: có thể mất bản gốc khi lỗi. Chỉ giữ backup mà không journal: không biết registry/directory đang thuộc phiên nào sau crash. Tự xóa directory lạ để recovery tiếp: từ chối vì có thể làm mất thay đổi ngoài Hub.


## R3 — Chọn và xác nhận trong Hub

Danh sách chỉ đọc manifest schema 2 đúng ID/port, không hash toàn bộ payload trong luồng render. Metadata được kẹp giữa hai lần hash manifest; bỏ qua bản sao thay đổi trong khi đọc. Lựa chọn giữ digest manifest, profile đã lưu và máy đích. Confirm kiểm tra lại profile/selection/máy dừng; backend so digest trước và sau verify/stage, từ chối backup đã thay đổi. Payload và SDK image vẫn được verify đầy đủ trước commit.

Kết quả tải danh sách mang ID và epoch, bỏ qua nếu đổi máy hoặc bắt đầu thao tác mới. Chọn backup chỉ mở bước xác nhận, không tự restore. UI hiển thị đường dẫn bản sao và cảnh báo thay dữ liệu/cấu hình; kết quả báo nơi giữ dữ liệu cũ. Lịch sử ba biên nhận gần nhất có thể ẩn và được đọc lại sau initialize/restore. Chưa có thao tác xóa dữ liệu previous trong UI.
