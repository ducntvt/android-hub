# 0024 — Tải SDK archive vào cache riêng trước cài đặt

- **Status**: Accepted
- **Date**: 2026-11-13

## Decision

Rust infrastructure dùng /usr/bin/curl với argv trực tiếp, --disable để không đọc curlrc, HTTPS repository dl.google.com/android/repository và URL path hạn chế. Không follow redirect, không gửi credential hoặc chạy package. Candidate SdkArchive gồm URL, byte count, checksum algorithm/value và license text hash. Caller phải cung cấp digest của license đã được người dùng chấp nhận; downloader không tự chấp nhận hoặc suy ra từ bool trong manifest. UI consent còn chưa nối.

Staging nằm ở runtime/sdk-downloads, directory 0700, file 0600. Lock riêng không khóa lifecycle VM trong lúc tải. Cache key theo thuật toán+checksum. .part được curl tiếp tục bằng Range; lỗi giữ partial. Chỉ rename thành .zip sau khi size và toàn bộ checksum khớp, sync file/directory. Cache hit cũng hash lại trước dùng. Sai checksum giữ file để kiểm tra; reset/quarantine có kiểm soát cần bổ sung trước UI.

Kiểm tra file thường với nlink=1, từ chối symlink/special files. Downloader chỉ kiểm tra budget archive còn lại +256 MiB; extracted budget thuộc bước giải nén chưa triển khai. Curl timeout 600 giây mỗi lượt, có thể gọi tiếp; backend guard 610 giây. Không bypass TLS, không tự restart từ đầu khi server không hỗ trợ Range.

## Consequences

Resume dùng strong ETag từ headers gốc và If-Range. Headers gốc giữ nguyên khi thử tiếp; response mỗi lượt ghi file riêng. Parser giới hạn 64 KiB, yêu cầu headers hoàn chỉnh, status 200/206 và đúng một strong ETag; weak/missing/duplicate đều từ chối resume. Response có ETag khác cũng bị từ chối. File partial đã đủ size sau crash được verify toàn bộ bằng pinned digest trước publish, không cần một request resume nữa. Nếu nguồn thay đổi, file trộn chỉ được dùng nếu vẫn khớp toàn bộ checksum; checksum sai không publish. SHA-1 upstream được kiểm tra dưới đúng thuật toán, không nâng thành cam kết SHA-256. API đã có progress/cancel, UI chưa nối progress/cancel/reset và chưa có thao tác tải SDK thật.

Không cài đặt SDK hoặc ghi vào SDK hiện tại. HTTP override chỉ chấp nhận loopback trong test build; production không cho URL override. Test payload 6 byte không phải package hoặc license thật.

ETag fixture thêm ba case: resume đúng, server bỏ Range, server đổi ETag. 63 unit tests và HTTP fixture pass; logs etag-* trong reports/sdk-download. Không tải SDK thật.

## Điều khiển một lượt tải

SdkDownloadControl dùng chung qua Arc, nhận số byte đang có trên disk và cờ cancel. Một control chỉ dùng một lần; retry tạo control mới để cờ hủy cũ không tác động lượt khác. Curl được theo dõi mỗi 100 ms, hủy/timeout/error dùng guard kill + wait process con do chính thao tác tạo, giữ partial. Kết thúc lỗi vẫn cập nhật bytes cuối cùng; 100% bytes không đồng nghĩa checksum đạt hoặc SDK installed.

Cancel trước start không tạo staging. Cancel trong transfer dừng curl; cancel trong bước shasum được quan sát sau khi lệnh verify kết thúc (bounded 300 giây), trước publish. Chưa có UI hoặc progress callback cho extraction. Vòng đời curl/lock khi parent chết đã có cơ chế fd inheritance và SIGKILL fixture bên dưới; installer end-to-end vẫn chưa được kiểm chứng.

## Lock tồn tại cùng writer khi parent bị kill

Curl kế thừa file descriptor đang giữ flock. Command::pre_exec chỉ dùng fcntl F_GETFD/F_SETFD để bỏ FD_CLOEXEC trong child sau fork; parent descriptor giữ nguyên CLOEXEC, không làm các spawn khác kế thừa lock. File được borrow giữ sống xuyên spawn/transfer. Curl exit đóng descriptor; lock chỉ hết hiệu lực khi không còn holder. Không gọi unlock tường minh vì sẽ làm mất khóa chung. Xem [Apple flock(2)](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/flock.2.html).

Fixture dùng curl thật với HTTP localhost: SIGKILL driver sau khi curl kết nối, xác nhận competing call bị lock chặn; cho curl nhận đủ payload rồi exit; xác nhận lock rảnh và retry verify/publish được payload. Không kill GUI/guest người dùng. Driver chết có thể để curl chạy đến network timeout (600 giây); không tuyên bố tự hủy ngay khi parent mất. Temporary log của driver bị kill có thể còn lại, không tự dọn.

libc 0.2.191 được thêm làm dependency trực tiếp từ phiên bản đã có trong Cargo.lock; không cập nhật package khác. Unsafe block được giới hạn ở pre_exec/fcntl, không allocation hay khóa Rust trong child callback.

### Normal-exit release correction

[ADR 0034](0034-sdk-cache-lock-release.md) supersedes the close-only release statement above: explicit unlock runs after owned curl is reaped. Parent SIGKILL still does not unlock, preserving the crash contract. CLOEXEC prevents unrelated inheritance across exec, not temporary sharing between fork and exec.
