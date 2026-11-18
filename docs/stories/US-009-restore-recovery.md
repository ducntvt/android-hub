# US-009 — Khôi phục backup với rollback

- **Status**: implemented
- **Created**: 2026-11-13
- **Owner**: Codex
- **Risk lane**: high-risk
- **Related**: US-008; [ADR 0009](../decisions/0009-restore-compatibility.md)

## Contract

1. Kiểm tra đúng ID/port/path SDK/AVD và fingerprint SDK image trước khi thay dữ liệu.
2. Sao chép/verify staging; chỉ swap khi máy dừng, giữ bản cũ đến khi commit.
3. Có journal bền vững và recovery/rollback nếu lỗi hoặc app dừng giữa các bước.
4. UI chọn backup, xác nhận máy đích và ảnh hưởng đến dữ liệu; không chạy restore chỉ từ file drop.
5. Guest thử boot từ payload backup, package/dữ liệu vẫn có; thử lỗi rollback trên fixture.

## Phân việc

- R1 Compatibility + boot drill: implemented, bằng chứng bên dưới.
- R2 Transaction/journal + fault injection: implemented, bằng chứng bên dưới.
- R3 UI xác nhận + lịch sử: code đã nối; smoke native còn phụ thuộc kết nối CUA.

## Verify

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --locked -p android-hub-infrastructure offline_backup_publishes_verified_copy_and_rejects_busy_port -- --ignored --nocapture
ANDROID_HUB_TEST_APK=/tmp/hub-apk-fixture-20261113/test.apk cargo test --locked -p android-hub-infrastructure installs_on_disposable_guest -- --ignored --nocapture
cargo test --locked -p android-hub-infrastructure reopening_hub_recovers_pending_transaction_only_when_stopped -- --ignored --nocapture
cargo clippy --workspace --all-targets --locked -- -D warnings
bash scripts/build-android-hub.sh
```

Các lệnh backend không đủ để pass R2/R3; thêm fault injection/journal tests và smoke UI khi triển khai. Không dùng guest người dùng để thử rollback.

## Evidence

Chưa hoàn tất restore sản phẩm. R1 bổ sung manifest schema 2 chứa SHA-256 của source.properties/system/vendor/ramdisk/kernel/encryptionkey SDK, đường dẫn SDK/AVD canonical. Backup v1 vẫn verify được payload nhưng từ chối restore tự động vì thiếu fingerprint image.


R1 evidence:

- 15 unit tests pass; fmt/clippy -D warnings/build/codesign pass. Logs: `docs/reports/restore/tests.log`, `clippy.log`, `build.log`.
- Fixture compatibility pass: đúng ID/path/image được chấp nhận; schema cũ, sai ID, sai SDK/AVD path, image manifest sai và nội dung system.img đổi đều bị từ chối. [Log](../reports/restore/compatibility-smoke.log).
- Boot drill 97.36 giây pass: guest riêng được tạo, cài package local, dừng/backup/verify, giữ original dir rồi copy payload về đúng AVD path. Guest boot hoàn tất và `pm path local.androidhub.installsmoke` xác nhận package còn. Dừng guest, đưa original dir trở lại, dọn root tạm. [Log](../reports/restore/boot-smoke.log).
- Đây là test-only swap trên guest dùng một lần, **chưa phải transaction restore sản phẩm**. R2/R3 còn thiếu. Không restore A/B; không thay đổi system image SDK.


R2 evidence:

- Backend `restore` đã có transaction thực; smoke hiện gọi API này thay cho swap test-only của R1. Sau restore, registry về profile backup, marker mới hơn nằm trong previous_data, guest boot và package vẫn còn. Root thử được dọn. [130.44 s smoke](../reports/restore/transaction-boot.log).
- 20 unit tests pass (3 integration tests opt-in được ignore trong suite thường). Fault injection bao phủ journal, old/new directory moves, registry, commit marker; pending rollback, committed giữ kết quả mới; recovery lặp sau các move; lỗi ghi registry; registry/directory đổi ngoài Hub; token journal traversal bị từ chối. [Tests](../reports/restore/transaction-tests.log).
- Startup recovery integration pass: Hub không recovery khi port bận; khi máy dừng, initialize phục hồi dữ liệu cũ; file journal .new chưa publish được dọn an toàn. [Log](../reports/restore/startup-recovery.log).
- fmt/clippy -D warnings/build/codesign pass. [Clippy](../reports/restore/transaction-clippy.log), [build](../reports/restore/transaction-build.log).
- Dữ liệu gốc giữ trong .previous-ID-TOKEN sau commit; biên nhận nằm ở restore-history. Chưa tự dọn các bản này.
- Chưa kiểm thử mất điện/hỏng ổ đĩa. R3 còn thiếu: picker/xác nhận restore, thông báo recovery khi mở app và smoke UI. Story giữ in_progress.


R3 evidence (2026-11-13):

- Panel cấu hình có tải/làm mới danh sách, chọn backup, xác nhận target/path và ảnh hưởng dữ liệu/cấu hình. Hiển thị lịch sử committed/rollback, có nút ẩn.
- Digest manifest gắn với lựa chọn, backend từ chối nếu manifest đổi; fixture offline pass, dữ liệu live còn nguyên. Listing bỏ qua backup sai máy.
- 22 unit tests pass; bao gồm kết quả danh sách cũ và thay profile giữa hỏi/confirm. [State tests](../reports/restore/ui-state-tests.log). Clippy -D warnings và build app pass ([clippy](../reports/restore/ui-clippy.log), [build](../reports/restore/ui-build.log)).
- CUA inventory vẫn trả `Sky Computer Use native pipe startup failed`; chưa kiểm chứng layout/picker/confirm bằng UI thật. Giữ story in_progress. Không restore guest A/B của người dùng.

- R3 guest smoke pass 128.13 giây: backup → restore qua API có digest → boot lại, package thử vẫn còn. Guest thử đã dừng và root tạm được dọn. [Log](../reports/restore/ui-boot-smoke.log). Chữ ký app đã verify bằng codesign --verify --deep --strict.

R3 native QA evidence (2026-11-16):

- Picker, xác nhận đúng target/path, busy, thông báo thành công và lịch sử đã được quan sát qua UI thật trên root QA riêng.
- Sau restore, 22 file live khớp manifest độc lập; receipt committed, previous_data được giữ. Khởi động từ Hub tới Android Home/Play Store, sys.boot_completed=1, sau đó dừng qua Hub. [Evidence](../reports/restore/native-qa-20261116.json), [UI notes](../reports/ui-user-validation.md).
- Không có tài khoản/game trong guest QA; đã kiểm tra hủy xác nhận không tạo transaction mới; startup recovery banner và toàn bộ Verify còn chờ. Giữ in_progress.

### Startup recovery native — 2026-11-16

A separate app/data root at `/tmp/android-hub-recovery-native-20261116` was prepared with a synthetic pending journal at the post-registry boundary and two tiny marker directories. Opening the unchanged Hub binary through native UI restored the original profile and displayed “Recovery original: đã đưa dữ liệu cũ trở lại sau gián đoạn”. Independent disk checks confirmed the original marker, zero pending journals/staging/previous directories and one rolled-back receipt. [Evidence](../reports/restore/startup-native-20261116.json).

The first fixture used an unsupported port (5690); Hub correctly rejected it. The fixture was corrected to 5680 before the successful run. This is a synthetic crash-state fixture, not a live SIGKILL or power-loss test. No guest was booted and no personal data was used. The stopped/busy-port startup integration was rerun and passed ([log](../reports/restore/startup-integration-20261116.log)). Native startup-history coverage is now complete; retain in_progress until the remaining Verify audit, including disposable-guest restore evidence, is completed.

## Completion audit — 2026-11-16

All five contract items now have evidence:

1. Compatibility, manifest digest and stopped gates: current offline integration passed ([log](../reports/backup/final-integration-20261116.log)); verifies ID/path/image mismatch and manifest mutation rejection without changing source.
2. Verified staging/swap and retained previous data: native 22-file checksum/receipt audit above, plus current production API guest test below.
3. Journal/recovery: current workspace tests cover all commit boundaries, repeated rollback, registry write failure and foreign data refusal. Startup busy-port integration and native rollback-history fixture passed ([integration](../reports/restore/startup-integration-20261116.log), [native](../reports/restore/startup-native-20261116.json)).
4. Native picker/target confirmation/cancel/busy/success/history: recorded in the QA evidence above. These are separate from backend tests.
5. Disposable guest test rerun on current source: install known no-code APK, stop/backup, mutate profile/marker, restore through production API with manifest digest, retain previous data, remove/undo, reboot and confirm the installed package persists. Passed in 134.12 seconds; guest stopped and temporary root removed ([log](../reports/restore/final-guest-20261116.log)). Used the freshly built equivalent fixture `/tmp/hub-apk-ui-qa-20261116/test.apk` instead of the historical path in Verify.

All Verify commands have passed on current unchanged application source. Shared fmt/workspace/clippy/build results from the preceding US-026 verification remain applicable (111 passed / 20 opt-in ignored; [tests](../reports/java-selection/final-tests-20261116.log), [clippy](../reports/java-selection/final-clippy-20261116.log), [build](../reports/java-selection/final-build-20261116.log)); no application code changed between checks. Backup and startup opt-in commands were run separately as linked above.

Limits: synthetic crash boundaries and normal app/guest restart do not prove host power-loss durability. Personal A/B data and game-account state were not used. Whole-project game benchmarks/setup remain open; completing this story does not complete the overall goal.
