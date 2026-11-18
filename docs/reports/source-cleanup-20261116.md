# Rà soát và dọn source — 2026-11-16

Theo yêu cầu người dùng: rà code từng folder, xóa phần không cần thiết, cache/build và runtime thừa.

## Kết quả

Dung lượng block do `du` báo: **41.17 GiB → 7.42 GiB**, giảm **33.75 GiB**. APFS có sparse/clone/shared blocks nên con số này không đồng nghĩa dung lượng vật lý độc quyền được giải phóng. Dung lượng trống filesystem quan sát trước/sau khoảng 40,82/66,38 GiB; máy có thể có hoạt động khác đồng thời.

## Rà từng thư mục

| Thư mục | Xử lý và lý do |
| --- | --- |
| `crates/domain` | Giữ model/validation và tests đang được Hub sử dụng. |
| `crates/infrastructure` | Giữ các module SDK, dữ liệu, display, process và integration fixtures. Các dependency còn dùng; `prost` phục vụ code sinh từ protobuf, `flate2` bật Rust Deflate backend cho `zip` qua feature unification (ADR 0025). Thêm comment giải thích `flate2`. |
| `crates/desktop` | Giữ state/update/UI, data root và screen; không tìm thấy module bỏ rơi qua rà module, dependency và clippy. |
| `scripts` | Xóa serial console, dual smoke dựa trên socket Avo, GApps staging và test tương ứng. Giữ emulator setup/probe, APK fixture, packaging, SDK manifest, metrics collector/verifier và tests còn dùng. Xóa `__pycache__`. |
| `experiments/p0/control` | Xóa client control socket và ADB bridge Avo. Giữ `engine-metrics` vì collector đang gọi; giữ tên package/đường dẫn tương thích. Xóa build target. |
| `experiments/p0/viewer`, `vendor` | Xóa viewer Avo và minifb vendored/patch vì Hub dùng Iced/gRPC và không còn engine Avo. ADR 0003 chuyển Deprecated. |
| `experiments/p0/images` | Xóa image/archive Cuttlefish và GApps của thử nghiệm cũ. SDK Google Play thật ngoài project không thay đổi. |
| `experiments/p0/runtime` | Xóa guest Avo `a`, `b`, hai bản thất bại, restore drill, backup Avo trước GApps, GApps staging và launcher viewer cũ. Xóa 5 AVD smoke không còn trong registry cùng INI/launcher/log tương ứng. Giữ registry, 3 máy hiện tại, launcher/log đang dùng và metadata lịch sử smoke nhỏ. |
| `target` | Xóa toàn bộ cache build workspace sau kiểm tra. Lần build tiếp theo cần compile lại. |
| `Android Hub.app` | Giữ bản chạy hiện tại, không phụ thuộc `target` để mở app. |
| `docs` | Giữ kế hoạch, quyết định và bằng chứng kiểm thử (~5 MiB). Báo cáo cũ là lịch sử; đường dẫn tới artifact Avo đã xóa không còn có thể chạy lại. |

Dọn `.DS_Store` ngoài các cây được bảo vệ. Không dọn cache dùng chung của Cargo, Android SDK, macOS hoặc thư mục ngoài project.

## Bảo vệ dữ liệu và kiểm chứng

- Kiểm tra process trước khi xóa: không có emulator/Avo engine hoặc cargo/rustc đang dùng thư mục build; chỉ Hub chính đang chạy.
- Giữ khóa `hub.lock` trong lúc dọn runtime; đối chiếu registry với metadata smoke để xác định đúng 5 guest thử.
- Giữ nguyên Android A (`p0-google-a`), Android B (`p0-google-b`), Android 3 (`hub-1794550846159`), cả `.avd` lẫn `.ini`.
- Đối chiếu toàn bộ file giữ lại theo inode, size, mtime, mode; file nhỏ, registry và binary app thêm SHA-256: **không đổi**. Không hash lại toàn bộ disk guest và không khởi động guest để kiểm chứng gameplay.
- Workspace `cargo test --workspace --locked --offline`: **100 passed, 20 ignored**; `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`: pass. Chạy trước khi xóa build cache; không sửa logic workspace Rust trong lượt dọn này.
- Sau bỏ code Avo: `cargo fmt --all -- --check`, clippy standalone sampler, Cargo metadata và **20 Python tests**: pass.
- Sau dọn: `codesign --verify --deep --strict 'Android Hub.app'`: pass. Còn đúng 3 AVD trong registry; không còn `target` hoặc Python cache trong project.
- Cảnh báo tương thích tương lai `block 0.1.6` đã có từ trước; không thay dependency trong lượt dọn.

Danh sách artifact đã xóa, kích thước và manifest bảo vệ: [source-cleanup-20261116.json](source-cleanup-20261116.json).
