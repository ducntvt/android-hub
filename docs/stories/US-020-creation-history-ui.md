# US-020 — Hiển thị lịch sử tạo máy

- **Status**: implemented
- **Owner**: Codex
- **Created**: 2026-11-13
- **Risk lane**: normal

## Contract

- Sau initialize và thao tác tạo máy, đọc ba biên nhận tạo máy mới nhất trong tác vụ nền.
- Phân biệt hoàn tất đăng ký với tạo dở chưa đăng ký; hiển thị tên và ID để tìm đúng dữ liệu.
- Đọc receipt độc lập với trạng thái hiện tại của guest: guest đã đổi config không làm lịch sử mất hiệu lực.
- Kiểm tra schema, profile/registry, tên file/state, định dạng fingerprint; bỏ qua receipt lỗi/symlink và thông báo số lượng không đọc được. Không coi receipt là bằng chứng guest đang boot hay file vẫn nguyên vẹn.
- Dùng receipt epoch hiện có để bỏ callback cũ và tôn trọng nút Ẩn. Lịch sử cuộn trong vùng cao 110 px để không đẩy toàn bộ workspace.

## Out of scope

Xóa file tạo dở, phục hồi thủ công conflict, thay đổi journal hoặc dữ liệu guest. Không thay contract backend US-019.

## Verify

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Native smoke: receipt incomplete/registered hiển thị đúng, scroll/Ẩn hoạt động, workspace còn dùng được ở chiều cao nhỏ. Giữ in_progress khi native smoke chưa có.

## Evidence

- Fixture xác nhận receipt vẫn đọc được khi guest thay đổi bình thường; sai state/tên và symlink bị bỏ qua có cảnh báo.
- Desktop regression xác nhận callback cũ và callback tới sau Ẩn không đưa lịch sử tạo máy cũ trở lại.
- [Tests](../reports/creation-history/tests.log), [clippy](../reports/creation-history/clippy.log), [build](../reports/creation-history/build.log).

### Native follow-up — 2026-11-16

Registered/incomplete labels with names/IDs, malformed receipt warning, scrolling and Hide passed in a synthetic QA history fixture. Workspace Settings remained usable after Hide. [Report](../reports/creation-history/native-20261116.md). Minimum-height native coverage remains missing because resize attempts did not change the window; retain in_progress.

## Completion audit — 2026-11-16

All required native cases now passed, including minimum size via a QA-only initial-window-size substitution with unchanged UI logic ([report](../reports/creation-history/native-20261116.md)). Registered/incomplete names and IDs, malformed-receipt count, scroll, Hide and usable workspace were observed. Receipt fixture preparation does not claim a real guest was created in this run.

All Verify commands passed on the restored product source: shared fmt, 111 tests / 20 opt-in ignored and clippy from the preceding label fix remain applicable ([tests](../reports/removal/label-tests-20261116.log), [clippy](../reports/removal/label-clippy-20261116.log)); a fresh normal app build and strict codesign verification completed after QA ([build](../reports/creation-history/final-build-20261116.log)). Tests include receipt schema/state/fingerprint checks and stale/dismissed history callbacks. UI code retains background history reads and a 110-point scroll region. Earlier incomplete notes are historical.
