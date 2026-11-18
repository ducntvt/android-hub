# US-017 — Mở app khi đường dẫn source lúc build không còn

- **Status**: implemented
- **Created**: 2026-11-13
- **Owner**: Codex
- **Risk lane**: normal

## Contract

- Giữ ưu tiên ANDROID_HUB_ROOT do người dùng đặt; không tự đổi một override sang chỗ khác.
- Nếu root project lúc build còn tồn tại, tiếp tục dùng đúng runtime cũ, không migrate A/B.
- Khi root project không còn, dùng ~/Library/Application Support/Android Hub làm root mới và báo rõ trên UI; không panic do canonicalize đường dẫn build.
- Không dò/chuyển AVD cũ hoặc viết lại đường dẫn absolute trong image/backup. Không coi đây là migration hay installer SDK.

## Verify

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
bash scripts/build-android-hub.sh
codesign --verify --deep --strict 'Android Hub.app'
```

Fixtures filesystem cho root cũ, root mất, override; native smoke app độc lập với đường dẫn source lúc build thực sự vắng mặt.

## Evidence

- 45 unit tests pass; fmt/clippy -D warnings/build/codesign pass. [Tests](../reports/packaging/root-tests.log), [clippy](../reports/packaging/root-clippy.log), [build](../reports/packaging/root-build.log).
- Resolver fixture kiểm tra legacy còn/mất, explicit override, root là file hoặc dangling symlink; không tạo file tại root fallback.
- Một lần suite trước đó lỗi ENOENT ở fixture removal.write_profiles; fixture dùng timestamp đơn lẻ có nguy cơ trùng khi song song. Đã thêm PID + atomic sequence và create_dir mới; nhóm removal chạy lặp 5 lần pass. Đây là xử lý fixture test, không thay dữ liệu sản phẩm. [Repeated runs](../reports/packaging/fixture-repeat.log).
- Native standalone smoke đạt: checkout thử vắng mặt, app mở và báo đúng fallback, registry mới rỗng; không di chuyển source/runtime A/B. [Báo cáo và verify audit](../reports/data-root/native-20261116.md). [ADR](../decisions/0017-data-root-fallback.md) giữ phạm vi không migration/installer.
