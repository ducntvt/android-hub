# Scaffold review và cleanup — 2026-11-16

> Báo cáo này ghi nhận trạng thái trước refactor. Cấu trúc hiện tại xem [scaffold optimization](scaffold-optimization-20261116.md) và [Development](../DEVELOPMENT.md).

## Kết luận

Workspace hiện tại phù hợp cho ứng dụng desktop cá nhân, không cần scaffold mới hay thêm backend. Chưa phải cấu trúc tối ưu để mở rộng lớn: desktop tập trung nhiều logic vào hai file, và đường dẫn runtime còn mang tên thử nghiệm. Review này không thay hành vi app hay di chuyển dữ liệu.

| Thành phần | Đánh giá |
| --- | --- |
| `crates/domain` | Đúng ranh giới: Profile/Runtime/validation, chỉ phụ thuộc serde. |
| `crates/infrastructure` | SDK, process, storage, display transport; phụ thuộc domain, không phụ thuộc UI. Các module đều được khai báo trong lib.rs, không thấy thư mục source bỏ rơi. |
| `crates/desktop` | Iced UI gọi infrastructure; dependency đi đúng chiều. `main.rs` khoảng 1.932 dòng, `ui.rs` khoảng 1.094 dòng, gồm cả tests ở main: nên tách theo feature khi sửa tính năng liên quan, chưa cần refactor hàng loạt cho cleanup. |
| `tests/android-input` | Source fixture Android, không phải output test; cần giữ để tái tạo APK. |
| `scripts` | Công cụ build, probe, QA, metrics và Python tests. Tên lệnh CLI dùng kebab-case; tests Python dùng snake_case là hợp lý. |
| `docs/decisions`, `docs/stories`, `docs/reports` | Quyết định, hợp đồng triển khai và bằng chứng. Có ghi chú pending lịch sử; không nên coi toàn bộ reports là cache. |
| `experiments/p0/control` | Công cụ metrics riêng, được workspace exclude có chủ đích. Không phải crate app bị sót. |
| `experiments/p0/runtime` | Dữ liệu thật của app, khoảng 7,4 GiB. Tên gây nhầm là thử nghiệm nhưng backend đang dùng đường dẫn này; không đổi/xóa nếu chưa có migration. |
| `Android Hub.app` | Artifact bàn giao đang dùng, giữ nguyên. |

## Naming và điểm nên cải thiện

- Rust crate package `hub-domain`, `hub-infrastructure`, `android-hub` và module `snake_case.rs` tuân theo quy ước Rust. Tên thư mục ngắn `domain/infrastructure/desktop` không cần trùng hoàn toàn package name.
- Java `ScrollBridge.java` và `InputActivity.java` dùng tên lớp PascalCase đúng quy ước. `main.rs`, `lib.rs`, `build.rs`, `Cargo.lock` là các file có vai trò chuẩn, không phải file thừa.
- `sdk_*.rs` hiện dễ tìm trong một crate nhỏ. Nếu SDK tiếp tục mở rộng, có thể gom thành module `sdk/`; chưa đổi đường dẫn hàng loạt khi không có nhu cầu chức năng.
- `catalog.rs` là catalog backup, còn `sdk_catalog.rs` là catalog SDK; `storage.rs` là helper dung lượng, `usage.rs` đo usage. Tên generic có thể rõ hơn khi tách module, nhưng đây không phải module trùng/rác.
- Manifest SDK được `include_str!` từ `docs/reports/sdk-installer/install-plan.json` theo ADR 0030. Đây là coupling build → docs; cần giữ hiện tại. Hướng cải thiện là một bản canonical dưới resources của crate, cùng cập nhật ADR/tooling, không xóa như báo cáo sinh tự động.
- `crates/infrastructure/android/scroll-bridge.dex` được `include_bytes!` khi build. Đây là resource nhúng có script tái tạo, không xóa cùng cache.
- Workspace hiện không có `.git`; `.gitignore` chỉ có hiệu lực khi đưa vào Git. Không tự khởi tạo repository hoặc commit.

## Đã dọn

Manifest [scaffold-cleanup-20261116.json](scaffold-cleanup-20261116.json) ghi chính xác từng đường dẫn, dung lượng allocated trước xóa và kết quả. Tổng khoảng **6,07 GiB artifact allocated**; đây không phải cam kết số dung lượng trống tăng tương ứng trên APFS.

- `target/` của workspace (khoảng 5,9 GiB), `scripts/__pycache__/`.
- Các checkout, bundle QA, bản app trước thay đổi, APK test/classes/dex/key tạm và log contention tạm đã xác định dưới `/private/tmp`.
- Các app QA Empty/Recovery/APK Panel và app trong root QA chính đã đóng; xóa bundle, giữ phần data riêng.
- Giữ toàn bộ runtime cá nhân, runtime QA có máy đang chạy (khoảng 9,4 GiB), registry/backup và SDK hệ thống. Không xóa cache Cargo/SDK toàn máy.

Hash SHA-256 của app chính và hai registry (cá nhân/QA đang hoạt động) được lưu trước dọn và đối chiếu sau dọn. Xóa cache không thay source Rust nên không chạy lại build/test nặng để tạo lại chính thư mục vừa xóa.

Kiểm tra cuối: cargo fmt, metadata workspace offline/locked, cú pháp Python, strict codesign, tồn tại hai resource nhúng và hash bảo vệ đều đạt; mọi đường dẫn trong manifest đã vắng mặt và target không tái tạo. Workspace còn khoảng 7,4 GiB, chủ yếu là runtime. Lần đọc process cuối không thấy Hub hoặc emulator, khác lần đọc đầu; lượt cleanup này không gửi lệnh đóng/dừng process và không tự mở lại app. Runtime QA vẫn được giữ dù process đã không còn tại thời điểm kiểm tra cuối.

## Quy trình sau mỗi lượt test

Đóng và xác minh process QA đã thoát; xóa checkout/app/output tạm của lượt đó sau khi lưu bằng chứng cần thiết. Không dùng wildcard xóa runtime/AVD. Dùng Python không sinh bytecode khi kiểm tra. Giữ app bàn giao; cache Rust chỉ xóa sau khi đã hoàn thành build/verify cần thiết. Build kế tiếp sẽ tạo lại target và mất thời gian biên dịch nhiều hơn.
