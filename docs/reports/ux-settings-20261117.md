# US-037 — Settings drafts verification

Ngày: 2026-11-17. Phạm vi: bản nháp cấu hình và navigation có xác nhận; không
khởi động, dừng hoặc sửa dữ liệu của guest cá nhân.

## Implementation

- Draft giữ profile gốc; header/sidebar đọc tên đã lưu. Poll cập nhật draft sạch
  nhưng giữ draft dirty. Cấu hình đổi bên ngoài tạo conflict, chặn Save và cho reload.
- Save/Bỏ thay đổi/Ở lại áp dụng cho chuyển máy, tạo máy, đóng panel, mở các panel
  quản lý và thoát. Save thất bại giữ draft và hủy navigation đang chờ.
- Backend so profile gốc dưới Hub lock trước khi ghi, giữ stopped/identity guards.
  Xem [ADR 0046](../decisions/0046-settings-drafts-and-optimistic-save.md).
- Panel chia Thiết bị/Hiệu năng/Dữ liệu; có validation, đơn vị MiB và giải thích
  giới hạn FPS cấu hình. Máy chạy vẫn có các trường chỉ đọc và lý do khóa.

## Automated checks

- `bash scripts/check.sh`: 150 Rust tests đạt (desktop 55, domain 2,
  infrastructure 93); 20 integration tests có sẵn vẫn ignored. Không tính ignored
  là đã chạy. 25 Python tests đạt; fmt, clippy all-targets `-D warnings`, shell syntax đạt.
- `bash scripts/build-android-hub.sh`: đạt, cập nhật app chính.
- `codesign --verify --deep --strict 'Android Hub.app'`: đạt.
- Tám test settings bao phủ dirty/validation, Stay/Discard, save success/failure,
  poll/conflict, profile bị gỡ bên ngoài, navigation tới target đã mất, Start/Quit
  và Quit đến trong lúc Save. Test infrastructure xác minh stale editor không ghi
  đè registry mới hơn. Running guard được kiểm chứng bằng state test, không boot
  thêm guest để kiểm tra thay đổi presentation này.
- Cargo còn cảnh báo future incompatibility của dependency `block 0.1.6` có sẵn;
  không có lỗi build hoặc clippy.

## Native QA

Dùng `/tmp/android-hub-settings-qa-20261117/Android Hub QA.app`, data root riêng
với hai profile fixture, không AVD payload hoặc tài khoản. Thao tác qua native UI;
JSON chỉ dùng đọc đối chiếu và giả lập writer bên ngoài trên fixture.

1. Sửa tên A thành `QA Draft A`; chọn B mở modal. Header/sidebar vẫn giữ tên
   `QA Settings A`, panel hiện Chưa lưu. Esc giữ bản nháp và máy A đang chọn.
2. Chọn RAM 4096 MiB, chuyển B, Lưu và tiếp tục. Hub chuyển B sau save; đọc registry
   xác nhận chỉ A đổi tên/RAM, B vẫn 3072 MiB và tên gốc.
3. Sửa tên B thành `Discard me`, chọn A, Bỏ thay đổi. A được chọn; registry B
   giữ nguyên tên gốc. Quay lại B tiếp tục sửa bình thường.
4. Giữ draft tên `Conflict draft`, sửa CPU B trong fixture registry từ 2 thành 4.
   Poll hiện cảnh báo thay đổi bên ngoài, vẫn giữ tên draft. Nạp lại cấu hình
   khôi phục tên gốc, hiển thị CPU 4 và Đã lưu.
5. Sửa tên B, Cmd+Q mở modal. Enter mặc định chọn Ở lại, giữ cửa sổ và draft.
   Cmd+Q lần nữa, Tab/Enter chọn Bỏ thay đổi; QA thoát.
6. Toast save nằm phía trên footer, nút Lịch sử vẫn nhìn thấy. Đây cũng bổ sung
   bằng chứng vị trí notification cho US-036, không đóng các gap khác của story đó.

CUA inventory sau thử không còn QA; kiểm tra process theo đúng đường dẫn bundle
không tìm thấy tiến trình. Không có guest QA được tạo/chạy trong ca này.

## Scope of evidence

Native smoke yêu cầu của US-037 đã đạt. Race/save failure và chạy máy được kiểm
tra bằng test trạng thái/backend; không tuyên bố đã thao tác mọi nhánh đó trên
guest thật. Kết quả này không đánh dấu các story UX khác hoàn tất.

## Cleanup

Đã xóa QA bundle/data nêu trên, `target/` (2.4 GiB trước khi dọn) và bốn log
tạm `android-hub-settings-{check,tests,tests-final,build}-20261117.log`.
Đã xác nhận các đường dẫn tạm không còn, executable app chính và runtime cá nhân
vẫn tồn tại. Không tự commit.
