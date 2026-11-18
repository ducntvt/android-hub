# ADR 0032 — Giữ cache lỗi riêng trước khi tải lại

- Status: Accepted
- Date: 2026-11-13
- Story: US-025

## Quyết định

Mỗi package có thao tác Tải lại cache gói này trong review; cần xác nhận riêng, nêu rõ package và dung lượng download tối đa. Xác nhận chỉ chuyển cache cũ sang thư mục riêng, không tự tải, chọn SDK hoặc chấp nhận license. Cache còn giữ trên disk, không phải chức năng giải phóng dung lượng.

quarantine_sdk_archive xác minh spec syntax, giữ exclusive download.lock và preflight cả bốn suffix zip/part/headers/response: chỉ regular file độc lập, không link. Không chạm extraction staging, SDK đã publish, AVD hoặc preferences. Không xóa byte; rename vào sdk-downloads/quarantine/reset-ID trên cùng filesystem. Các gói khác giữ nguyên.

## Immutable intent và recovery

Ghi cache-reset-pending.json trước moves, chứa checksum key, directory dev/ino, suffix và file dev/ino/size. Target directory/parent được sync trước intent. write_json dùng .new + fsync + rename. Không cập nhật intent trong lúc moves; khi hoàn tất, rename intent thành receipt.json ở target rồi sync dirs.

Recovery chạy dưới download.lock trước download, extract và workflow disk admission; reset mới cũng recover lượt cũ trước. Validate schema/key/id/suffixes, directory identity và tất cả source/destination pairs trước move tiếp. Một file phải ở đúng một phía và fingerprint khớp. Đã moved thì bỏ qua; còn source thì rename; cả hai/không phía nào hoặc external changes thì dừng, giữ dữ liệu. Không overwrite destination.

Nếu chỉ có pending.json.new mà chưa có pending, chưa có move được phép bắt đầu: giữ .new ở quarantine/aborted-intent-ID.json và tiếp tục, không đụng payload. Receipt sau hoàn tất là evidence location; pending tồn tại là việc chưa xong. Không giả pending đã hoàn tất từ tên thư mục alone.

## UI và failure

Ask/confirm/cancel riêng, có busy/command_epoch guard; nút bắt đầu setup bị khóa trong lúc đang hỏi reset. Future chạy background; lỗi/success hiển thị cả panel và notice chung. Lần tải sau nhận cache namespace trống cho package và tải lại theo checksum pinned. Consent giữ theo review hiện tại, không suy ra từ reset.

Lock tranh chấp báo bận, không chạm file đang tải. Nếu external mutation làm recovery không xác định được ownership, cần kiểm tra đường dẫn báo lỗi; không tự xóa/sửa để vượt gate. Chưa có UI undo hoặc xóa quarantine; dữ liệu có thể tăng disk usage sau redownload. Không tuyên bố host power-loss durability hoặc actual SIGKILL cache-reset test; test recovery dựng lại mixed state từ intent/fingerprint thật đã emit.

## Evidence

Tests giữ bytes bốn suffix và file unrelated, reset rồi fresh full partial verify/publish không network; mixed one-file-moved recovery idempotent; đổi source/symlink chặn không overwrite; lock contention; aborted .new giữ bytes và cache nguyên. UI confirmation không bắt đầu download.

HTTP regression chạy curl localhost: Range/If-Range/ETag, server bỏ Range/đổi ETag, stalled cancel/reap và SIGKILL driver giữ khóa qua curl. Worker helper no-op không tính scenario độc lập. Không reset cache thật của người dùng hoặc tải SDK candidate trong test.
