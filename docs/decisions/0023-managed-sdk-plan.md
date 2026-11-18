# 0023 — Pin bản kê trước khi cài SDK riêng

- **Status**: Accepted
- **Date**: 2026-11-13

## Evidence and decision

Tài liệu chính thức đã deprecate sdkmanager để dùng Android CLI. Command-line Tools 23.0 trên host có `android` Mach-O arm64 bootstrap; gọi --help đã thất bại vì cố tạo bin directory ngoài sandbox. Strings trong binary có ANDROID_USER_HOME và URL tải CLI. Không nâng quyền để chạy bootstrap trên profile người dùng. Không coi --help của wrapper này là thao tác thuần đọc.

- https://developer.android.com/tools/sdkmanager
- https://developer.android.com/tools/agents/android-cli/commands/sdk
- https://developer.android.com/tools/agents/android-cli/commands/sdk_install

Installer của Hub sẽ dùng staging/SDK riêng và bản kê pinned, không gọi công cụ tự cập nhật `latest` trực tiếp trên SDK hiện tại. Python script hiện tại là resolver phục vụ chuẩn bị/review catalog; thực thi installer trong sản phẩm vẫn thuộc Rust, chưa triển khai.

Repository XML tải qua HTTPS chính thức được lưu cùng hash SHA-256 để tái lập bản kê. Chọn stable và macOS ARM64, giữ checksum nguyên thuật toán được công bố (hiện SHA-1). Không tuyên bố archive đã được SHA-256 xác thực khi chưa có digest đó từ nguồn tin cậy. Download/extraction cần chính sách integrity rõ trước implementation.

## Consequences

Catalog candidate tách khỏi SDK đang chạy. Review/acceptance license phải gắn đúng text hash và version, không suy ra từ các license từng chấp nhận. Chưa giải quyết Java bootstrap, disk budget extracted, resume HTTP, package extraction layout/permissions và crash recovery publish. I2–I5 trong US-025 giữ đầy đủ các yêu cầu này; không dùng manifest để đánh dấu installer đã xong.

## Alternatives considered

Gọi `sdkmanager` hoặc `android` latest trên SDK người dùng có thể tự tải CLI/update thành phần và không pin nội dung như kỳ vọng. Tự nâng emulator lên candidate cũng chưa có bằng chứng tương thích game. Reuse installer upstream có thể được đánh giá lại nếu xác minh được version pin, isolated home và lifecycle; hiện chưa chọn thực thi cách đó.
