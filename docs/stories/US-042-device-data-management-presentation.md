# US-042 — Sao lưu, lưu trữ máy và dung lượng dễ hiểu

- **Status**: implemented
- **Owner**: Desktop / infrastructure presentation
- **Created**: 2026-11-17
- **Risk lane**: normal
- **Related**: [UX roadmap](../UX-ROADMAP.md), [US-008](US-008-offline-backups.md), [US-009](US-009-restore-recovery.md), [US-016](US-016-remove-device.md), [US-018](US-018-disk-usage.md), [US-036](US-036-feedback-and-confirmations.md)

## Context

Backend đã có backup/restore có kiểm tra, archive giữ dữ liệu và đo dung lượng không xóa. Người dùng dễ hiểu “Gỡ máy” là giải phóng ổ đĩa; path kỹ thuật và caveat dài làm luồng khó đọc. Chỉ thay cách trình bày và truy cập, giữ transaction hiện có.

## Contract (acceptance criteria)

- [x] Nhóm Dữ liệu tách khỏi CPU/RAM, gồm Sao lưu, Khôi phục, Lưu trữ máy và link Dung lượng; các thao tác yêu cầu guest dừng giữ nguyên gate.
- [x] Backup hiển thị máy nguồn, thời gian, dung lượng và mức kiểm chứng thực tế; không gọi backup compatible/đã kiểm tra checksum nếu mới đọc metadata.
- [x] Restore xác nhận tên máy đích, bản sao và dữ liệu sẽ thay; giữ dữ liệu trước restore theo contract hiện có, không thêm auto-delete.
- [x] Đổi nhãn Gỡ/Máy đã gỡ thành Lưu trữ/Máy đã lưu trữ (VI/EN nhất quán); giải thích ngắn giữ dữ liệu và không giải phóng dung lượng. Không đổi ID/path/registry/archive schema chỉ để đổi tên UI.
- [x] Dung lượng phân nhóm máy đang quản lý, máy lưu trữ, backups và dữ liệu tạm có căn cứ; hiển thị thời điểm đo, số bỏ qua và phạm vi không gồm SDK bên ngoài.
- [x] Không suy dung lượng có thể thu hồi từ allocated bytes trên APFS; không thêm nút dọn tất cả. Path đầy đủ và caveat đặt trong Chi tiết, vẫn truy cập được.
- [x] Empty/loading/error states có hành động đúng ngữ cảnh; lỗi danh sách không hiển thị như danh sách rỗng thành công.

## Out of scope

- Xóa vĩnh viễn, tự xóa cache/backup/archive, migration runtime/SDK, cloud backup, đổi checksum/journal hoặc bảo đảm restore game chưa kiểm chứng.

## Affected areas

- `crates/desktop/src/app/`, `crates/desktop/src/i18n.rs`
- `crates/infrastructure/src/{usage,catalog,backup,removal}.rs` chỉ metadata cần cho presentation

## Implementation notes

Giữ đường dẫn/schema cũ để không migration dữ liệu vì đổi nhãn. Nếu cần đổi contract phân nhóm usage thì bổ sung ADR; tránh double-count logical versus allocated hoặc follow symlink ra ngoài root.

## Verify

```bash
bash scripts/check.sh
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Test missing metadata, invalid backup, catalog error và usage categories bằng fixture. Native QA kiểm tra danh sách/confirm/cancel/labels, không phải chạy lại mọi destructive transaction đã có chỉ để đổi presentation. Đóng/dọn QA, bảo toàn runtime thật.

## Evidence

- Implemented 2026-11-17; [native/automated evidence](../reports/ux-data-management-20261117.md), [ADR 0051](../decisions/0051-data-catalog-and-usage-presentation.md).
- Verify: 181 Rust + 26 Python tests; fmt/clippy/shell syntax, app build và strict codesign đạt. Native fixture: empty/error/list, metadata-only, Details, restore/archive confirm + cancel, VI/EN, storage categories và compact backup picker đạt. Không chạy giao dịch restore/archive hay guest thật cho thay đổi presentation này.
