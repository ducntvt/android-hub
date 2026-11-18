# US-008 — Sao lưu dữ liệu máy đã dừng

- **Status**: implemented
- **Created**: 2026-11-13
- **Owner**: Codex
- **Risk lane**: normal
- **Related**: [ADR 0008](../decisions/0008-verified-offline-backups.md)

## Contract

- Nút tạo backup trong panel cấu hình chỉ hoạt động khi máy dừng, trạng thái đã xác minh.
- Backend giữ lock và kiểm tra engine/các port trước và sau sao chép.
- Manifest chứa profile, SDK root, danh sách kích thước/SHA-256; kiểm tra cả bản sao và source trước publish.
- Từ chối symlink/socket/file đặc biệt. Bản chưa xong dùng .partial; không ghi đè backup có sẵn hoặc source.
- Báo đường dẫn khi thành công; lỗi không được báo thành công. UI vẫn xử lý tác vụ nền.

## Out of scope

Khôi phục/rollback là task kế tiếp. Backup chỉ chứa dữ liệu AVD, không đóng gói SDK/image bên ngoài, không phải snapshot RAM. Chưa benchmark sao lưu disk nhiều GiB trên A/B.

## Verify

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --locked -p android-hub-infrastructure offline_backup_publishes_verified_copy_and_rejects_busy_port -- --ignored --nocapture
cargo clippy --workspace --all-targets --locked -- -D warnings
bash scripts/build-android-hub.sh
```

UI smoke: backup guest thử đã dừng từ panel, xác minh đường dẫn/error state. Không coi test backend là thay thế smoke UI.

## Evidence

- 15 unit tests pass (2 integration tests opt-in được ignore trong suite thường).
- Offline integration fixture pass: port bận chặn backup; publish có manifest; checksum/data đúng; source/registry không đổi; thư mục backup 0700. [Log](../reports/backup/offline-smoke.log).
- Integration guest Android thật pass: tạo guest riêng, boot/cài APK test, dừng, backup + verify rồi dọn root. 48.87 giây; không dùng A/B. [Log](../reports/backup/guest-smoke.log).
- fmt, clippy -D warnings, build và codesign verify pass. [Tests](../reports/backup/tests.log), [clippy](../reports/backup/clippy.log), [build](../reports/backup/build.log).
- CUA inventory mới vẫn lỗi native pipe. Chưa smoke nút backup/UI notice, giữ in_progress.
- Cập nhật US-009 R1: đã boot lại từ payload backup trên guest thử và package còn. Chưa có transaction restore sản phẩm; không coi checksum pass là thay thế kiểm thử rollback.
- Không backup/restore A/B; các root fixture/guest thử đã được dọn.

### Native QA — 2026-11-16

Đã kiểm chứng nút backup: bị khóa khi máy chạy; trước boot thiếu userdata báo lỗi; sau boot/dừng qua UI tạo bản sao và hiện đường dẫn thành công. Xem [native report](../reports/ui-user-validation.md) và [checksum audit](../reports/backup/native-qa-20261116.json). Chưa đánh dấu story implemented trong lượt này; gom Verify toàn bộ sau đợt native data flows để không build lại cache nhiều lần.

## Completion audit — 2026-11-16

Native success/error/disabled-while-running behavior is recorded above. Re-reading the retained UI-created backup independently verified its complete file set, sizes and SHA-256: 22 files / 1,830,786,506 bytes ([audit](../reports/backup/final-payload-audit-20261116.json)). This does not compare the live guest after subsequent QA activity.

The required offline integration was rerun successfully ([log](../reports/backup/final-integration-20261116.log)): busy port rejection, insufficient-space rejection before copying, verified publication, source preservation and private backup permissions. Current unchanged source also passed fmt, 111 workspace tests (20 opt-in ignored), clippy and app build in the immediately preceding US-026 verification ([tests](../reports/java-selection/final-tests-20261116.log), [clippy](../reports/java-selection/final-clippy-20261116.log), [build](../reports/java-selection/final-build-20261116.log)); no application code changed between these checks. Reused common checks avoid an unnecessary duplicate build.

Code audit confirms lock and stopped checks before/after copy, source/image revalidation before publish, special-file rejection, partial staging, manifest hashes and asynchronous UI work with distinct success/error notices. All US-008 Verify commands and native smoke requirements are satisfied. Restore/recovery and game-data benchmarks remain separate requirements.
