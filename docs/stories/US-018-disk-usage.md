# US-018 — Đo dung lượng theo yêu cầu

- **Status**: implemented
- **Owner**: Codex
- **Created**: 2026-11-13
- **Risk lane**: normal
- **Related**: [ADR 0018](../decisions/0018-on-demand-disk-usage.md)

## Contract

- Nút Dung lượng dữ liệu mở panel và chạy tác vụ nền một lần; không polling.
- Thống kê block đã cấp phát theo nhóm AVD, backup, máy đã gỡ, dữ liệu trước restore, log và dữ liệu khác trong runtime; hiển thị dung lượng còn trống của filesystem.
- Không đọc nội dung file, không xóa file; bỏ qua symlink, special file và filesystem khác khi duyệt; hardlink chỉ tính một lần.
- Giới hạn 100.000 mục và kiểm tra thời gian 10 giây giữa thao tác filesystem, kể cả khi liệt kê thư mục; không phải timeout cứng cho syscall bị treo.
- Bỏ kết quả nếu command epoch thay đổi. Số đo không phải snapshot nhất quán khi engine đang ghi, không phải dung lượng APFS có thể thu hồi.

## Out of scope

Xóa vĩnh viễn, dọn cache, đo SDK bên ngoài runtime, quét định kỳ.

## Verify

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Native smoke: mở panel, đo lại, đóng panel, đổi thiết bị; kiểm tra không chồng chữ và UI vẫn phản hồi. Đã đạt trong lượt kiểm tra completion bên dưới.

## Evidence

Fixture kiểm tra allocated blocks, hardlink, symlink và giới hạn mục. Logs: [tests](../reports/storage/tests.log), [clippy](../reports/storage/clippy.log), [build](../reports/storage/build.log). Xác nhận bố cục từ người dùng trước đó không bao gồm panel mới này.

### 2026-11-16 native follow-up

CUA hoạt động trở lại; smoke một phần và sửa scrollbar clipping được ghi tại [UI evidence](../reports/ui-user-validation.md). Không suy ra các thao tác chưa thực hiện là đạt; story vẫn in_progress theo phần kiểm chứng còn thiếu.


## Completion evidence

Native CUA trên Hub chính với cả ba guest dừng: mở Dung lượng dữ liệu, nhận kết quả, bấm Đo lại, đổi A → B trong khi panel mở, đóng panel, chọn lại A và mở/đóng lần nữa. Screenshot quan sát được tiêu đề/selection đúng và layout không chồng/cắt chữ. Không start/stop guest, lưu cấu hình hoặc thao tác dữ liệu. Kết thúc selection A, panel đóng như ban đầu.

Số đo hiển thị tại thời điểm kiểm tra: available 41.09 GiB; AVD/staging 14.99 GiB; Backup 2.07 GiB; Khác 11.08 GiB. Đây là allocated-block estimate, không phải dung lượng có thể thu hồi. Không dùng số này làm benchmark hiệu năng.

Đối chiếu contract: main.rs chỉ gọi disk_usage trong MeasureStorage qua spawn_blocking, có loading guard và command_epoch; Tick không gọi scan. usage.rs chỉ đọc metadata, chặn symlink/special/cross-device, deduplicate dev/inode và kiểm tra giới hạn entry/time. Fixture xác nhận hardlink/symlink và limit. UI xác nhận luồng thao tác; không khẳng định syscall timeout cứng hay APFS reclaimability.

Toàn bộ lệnh Verify vừa chạy đạt: [fmt](../reports/storage/native-completion/fmt.log), [100 tests](../reports/storage/native-completion/tests.log), [clippy](../reports/storage/native-completion/clippy.log), [build](../reports/storage/native-completion/build.log), [codesign](../reports/storage/native-completion/codesign.log). 20 tests ignored mặc định thuộc integration/helper khác. block 0.1.6 có future-incompatibility warning đã biết.

Chỉ US-018 hoàn tất; các luồng SDK/Java, live input, backup/restore UI và benchmark của goal vẫn giữ trạng thái riêng.
