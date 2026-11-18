# US-021 — Kiểm tra dung lượng trước khi tạo máy

- **Status**: implemented
- **Owner**: Codex
- **Created**: 2026-11-13
- **Risk lane**: normal
- **Related**: [ADR 0020](../decisions/0020-create-space-policy.md)

## Contract

- Sau khi có thư mục AVD đích, đọc available space của chính filesystem đó, trước journal/avdmanager.
- Yêu cầu tối thiểu 12 GiB trống. Thiếu chỗ hoặc không đọc được filesystem thì dừng và báo đường dẫn/cần/còn; không chạy SDK, không tạo profile/journal.
- Không xóa file và không giữ riêng dung lượng. Thư mục AVD rỗng có thể đã được tạo để đo đúng filesystem.
- Không áp dụng ngưỡng này cho start máy cũ hoặc thay đổi partition của máy cũ.

## Verify

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Unit kiểm tra 0, thiếu một byte, đủ chính xác, u64::MAX và filesystem không đọc được. Guest disposable xác nhận create vẫn hoạt động trên filesystem đủ chỗ. Không giả lập disk đầy bằng cách lấp ổ đĩa thật.

## Evidence

- [Unit suite](../reports/create-space/tests.log): 52 tests pass; boundary tests cho admission policy và lỗi filesystem.
- [Clippy](../reports/create-space/clippy.log), [build](../reports/create-space/build.log), fmt/codesign pass.
- Không chạy thử nghiệm lấp đầy ổ đĩa; nhánh thiếu dung lượng được kiểm tra bằng unit tests. Không suy ra reservation hay quota từ kiểm tra này.

- [Guest smoke](../reports/create-space/guest-smoke.log) pass: tạo qua preflight trên filesystem đủ chỗ, boot/cài APK/backup/restore/gỡ/undo/boot lại và package còn nguyên; fixture đã được dọn. Không thao tác A/B.
