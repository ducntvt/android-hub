# Thống kê và dọn thư mục — 2026-11-13

Dung lượng là allocated size từ `du`, làm tròn; không phải cam kết dung lượng vật lý lấy lại trên APFS có clone/snapshot.

| Thư mục | Dung lượng trước dọn | Vai trò / kết quả |
| --- | ---: | --- |
| `crates/domain` | 8 KiB | Giữ: model và validation, workspace chính |
| `crates/infrastructure` | 132 KiB | Giữ: SDK, process, registry, gRPC/protobuf |
| `crates/desktop` | 48 KiB | Giữ: UI Iced và input Android |
| `scripts` | 136 KiB | Giữ script build Hub và công cụ P0; xóa cache Python |
| `docs` | 3 MiB | Giữ: kế hoạch, quyết định, kiểm chứng và log bằng chứng |
| `Android Hub.app` | 16 MiB | Giữ: app đã đóng gói để sử dụng, không phụ thuộc target lúc chạy |
| `target` | 4,79 GiB | Đã xóa: cache và sản phẩm build workspace, tạo lại bằng Cargo |
| `experiments/p0/control` | 21 MiB | Giữ source/manifest/lock; xóa thư mục target bên trong |
| `experiments/p0/viewer` | 72 MiB | Giữ source/manifest/lock; xóa thư mục target bên trong |
| `experiments/p0/vendor` | 716 KiB | Giữ: minifb có bản vá cục bộ, được viewer tham chiếu bằng path dependency |
| `experiments/p0/upstream` | 52 MiB | Giữ: checkout Avo dùng bởi p0-build-engine.sh và p0-boot.sh |
| `experiments/p0/images` | 3,30 GiB | Giữ: image Cuttlefish/GApps dùng trong các script P0, không thuộc cache build |
| `experiments/p0/runtime` | 28,18 GiB | Giữ: chứa registry và dữ liệu AVD đang dùng, cùng guest/backup thử nghiệm |

## Đã xóa

Bốn thư mục: `target/`, `experiments/p0/control/target/`, `experiments/p0/viewer/target/`, `scripts/__pycache__/`.
Tổng allocated size trước xóa: **4.88 GiB**. Dọn thêm file metadata Finder `.DS_Store` ở root, crates, scripts, experiments nếu có.
Chi tiết từng thư mục: [folder-cleanup.json](folder-cleanup.json).

## Vì sao giữ P0

P0 không nằm trong workspace Hub, nhưng vẫn có workflow kiểm chứng được mô tả trong `experiments/p0/README.md`, `docs/TASKS.md` và story US-001 đang in_progress. Các script hiện có tham chiếu control/viewer/upstream/images; vendor có bản vá input macOS chưa có ở dependency gốc. Do đó không kết luận đây là source không dùng chỉ vì bị exclude khỏi workspace.

Đặc biệt `Backend::runtime()` vẫn trỏ vào `experiments/p0/runtime`; xóa cả experiments sẽ làm mất registry, dữ liệu game và tài khoản. Không xóa guest cũ hoặc backup chỉ dựa vào tên `failed`/`restorecheck`.

## Kiểm chứng sau dọn

- Không có tiến trình Cargo/rustc hoặc binary từ các thư mục target khi kiểm tra trước xóa.
- `cargo metadata --no-deps --offline --format-version 1`: pass, đủ 3 package workspace.
- `codesign --verify --deep --strict 'Android Hub.app'`: pass.
- `otool -L` app chỉ tham chiếu thư viện/framework hệ thống; không link động từ target.
- Bốn thư mục được xác nhận không còn tồn tại; source và runtime được giữ nguyên.
- Không rebuild sau dọn vì sẽ tạo lại cache vừa xóa. Lần build tiếp theo sẽ lâu hơn.

## Cập nhật tiếp theo — xóa Avo theo yêu cầu người dùng

Đã xóa `experiments/p0/upstream/avo` (~52 MiB), thư mục cha upstream rỗng và hai script `p0-build-engine.sh`, `p0-boot.sh`. Bỏ exclude Cargo trỏ vào checkout đã xóa. Script khảo sát môi trường không còn yêu cầu revision Avo. Các bảng phía trên ghi lại quyết định tại lần audit ban đầu; cập nhật này thay thế quyết định giữ upstream.

Không có engine Avo chạy và checkout sạch trước xóa. Giữ nguyên runtime, images, control/viewer/vendor và báo cáo lịch sử. Cargo metadata offline và kiểm tra chữ ký app sau xóa đều pass.
