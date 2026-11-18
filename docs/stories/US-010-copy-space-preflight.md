# US-010 — Kiểm tra dung lượng trước backup/restore

- **Status**: implemented
- **Created**: 2026-11-13
- **Owner**: Codex
- **Risk lane**: normal
- **Related**: US-008, US-009, ADR 0011

## Contract

- Backup và restore kiểm tra dung lượng khả dụng trên filesystem đích trước copy/staging; lỗi truy vấn cũng dừng thao tác.
- Tính theo kích thước logic từng file, làm tròn block filesystem đích và thêm 256 MiB dự phòng; không dựa vào APFS clone hoặc sparse compression.
- Thông báo thiếu dung lượng có lượng cần/còn và đường dẫn đích. Không tự xóa dữ liệu để lấy chỗ.
- Giữ xử lý lỗi copy/rollback hiện có: preflight không bảo đảm chỗ trống được giữ riêng.

## Scope

Chỉ backup/restore đã triển khai. Dung lượng cho tạo máy/setup image thuộc task setup riêng vì engine còn tạo/giãn dữ liệu khi chạy.

## Verify

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --locked -p android-hub-infrastructure offline_backup_publishes_verified_copy_and_rejects_busy_port -- --ignored --nocapture
cargo clippy --workspace --all-targets --locked -- -D warnings
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

## Evidence

- fmt check pass; 24 unit tests pass, 3 integration tests opt-in ignored trong suite thường. [Log](../reports/storage/tests.log).
- Integration backup fixture pass 2.68 giây: tạo sparse file có kích thước logic lớn hơn dung lượng khả dụng; backend báo thiếu chỗ trước copy, không tạo partial backup, userdata giữ nguyên. Xóa sparse fixture rồi backup/verify thành công. Không làm đầy ổ thật hoặc dùng A/B.
- Clippy --all-targets -D warnings pass. Build pass ([log](../reports/storage/build.log)); codesign --verify --deep --strict pass.
- Bài sparse thực tế kiểm chứng nhánh backup; restore dùng cùng hàm preflight trước tạo staging. Không tuyên bố đã mô phỏng ổ đầy giữa transaction restore. Journal/rollback giữ nguyên, bằng chứng riêng tại US-009.
- Không thay đổi widget; thông báo lỗi đi qua notice sẵn có. Smoke UI tổng thể vẫn pending ở US-005/US-009 vì CUA native pipe chưa hoạt động.
- Cảnh báo tương thích tương lai của dependency block 0.1.6 vẫn tồn tại, không phải lỗi Clippy của workspace.
