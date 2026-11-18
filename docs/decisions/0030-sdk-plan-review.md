# ADR 0030 — Bản kê pinned và xem license trong app

- Status: Accepted
- Date: 2026-11-13
- Story: US-025

## Quyết định

SdkPlan nhúng bản kê đã chuẩn bị ở docs/reports/sdk-installer/install-plan.json bằng include_str để app không phụ thuộc workspace khi chạy. Giữ một bản canonical thay vì copy artifact sang assets. Không fetch latest hoặc nâng package tự động khi mở màn hình. Thay catalog cần review/regenerate artifact và build lại.

Parser Rust giới hạn JSON 512 KiB, yêu cầu schema/platform/review_required, đúng bốn package và destination cố định, stable revision, command-line tools khớp id 23.0, dependency image/emulator hợp lệ và tổng download bytes chính xác. Archive URL/size/checksum/license hash dùng validator hiện có. Phần metadata source đã được resolver/offline evidence xác nhận; runtime không tải lại XML.

License text phải có nội dung, <=128 KiB, SHA-256 khớp; cùng id không được khác nội dung/hash. Từ chối plan có accepted=true hoặc thiếu accepted=false. Dùng [RustCrypto SHA-2](https://docs.rs/sha2/latest/sha2/) với sha2 pin 0.11.1, thêm tám package vào lockfile; không tự viết hash. Đây là hash license, không thay đổi archive checksum upstream SHA-1 thành SHA-256.

SdkPlan giữ private package/license state và cung cấp read-only getters. approved_packages chỉ trả request chứa accepted digest khi caller truyền đúng tập hash của tất cả license, không thiếu hoặc thêm hash lạ. Đây là ranh giới API của app, không là chứng cứ người dùng đã đọc/đồng ý; UI phải cung cấp từ thao tác người dùng.

## UI hiện tại

Thiết lập SDK → Xem bản kê cài SDK mở màn hình 520px gồm package/revision/MB, tổng GB, danh sách hai license, nguyên văn điều khoản trong vùng cuộn và checkbox tương ứng license đang xem. Chưa có nút tải/cài ở bước này, thông báo rõ chỉ xem trước; không tạo task I/O, không tải package, không ghi acceptance/preferences.

Mở review luôn load plan lại và reset consent. Close/reopen/reset về SDK settings xóa lựa chọn. Handler chỉ nhận lựa chọn khi SDK/review đang mở và index khớp điều khoản đang hiển thị. State consent không được lưu qua restart. Downloader orchestration phải dùng cùng plan và exact hash set tại thời điểm user bấm bắt đầu, không suy ra đồng ý từ việc mở panel.

## Evidence và giới hạn

Tests bundled plan xác nhận bốn package, hai license, 2.474.135.889 bytes; thiếu/thừa consent bị từ chối. Mutation tests: terms đổi, preaccepted, duplicate package, total sai, dependency không đáp ứng, destination traversal và URL không chính thức. UI test xác nhận không preaccept, không nhận checkbox license chưa hiển thị, clear khi close/reopen và không bắt đầu download.

Native visual/interaction smoke vẫn pending do CUA native pipe lỗi ở lần kiểm tra trước. Chưa thực hiện user license acceptance, download SDK thật hoặc cài đặt. Còn nối workflow tiến độ/hủy/retry, preflight dung lượng tổng, package registration/native boot và dọn partial trước khi US-025 hoàn tất.

## Resource location update

ADR 0040 relocates the canonical manifest to `crates/infrastructure/resources/sdk-install-plan.json`. Its bytes and validation/consent contract are unchanged; `docs/reports` is no longer a build input.
