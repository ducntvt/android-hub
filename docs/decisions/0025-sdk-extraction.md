# ADR 0025 — Giải nén SDK vào staging riêng

- Status: Accepted
- Date: 2026-11-13
- Story: [US-025](../stories/US-025-managed-sdk-installer.md)

## Quyết định

Backend chỉ giải nén archive đã có trong cache, sau khi kiểm tra lại license digest, size và checksum. Giữ cùng download.lock xuyên verify/inspect/extract để không cạnh tranh downloader. Không gọi SDK manager, chạy executable, sửa SDK hiện tại hoặc chọn SDK mới.

Dùng zip 9.5.1, tắt default features, chỉ Stored/Deflate với flate2 1.1.11 rust_backend. Không dùng unzip subprocess: cần kiểm tra toàn bộ metadata trước ghi file, kiểm soát từng file/permission/CRC và hủy trong vòng copy.

Kiểm tra đường dẫn tương đối, không empty/dot/dotdot/backslash/colon; phiên bản đầu chỉ nhận ASCII graphic, không khoảng trắng, tối đa 1.024 bytes/path, 255 bytes/component, 32 components. Từ chối duplicate và alias hoa/thường kể cả thư mục cha ngầm định; từ chối file dùng làm parent, symlink/special file, encrypted ZIP, compression không hỗ trợ, directory chứa payload. Giới hạn 100.000 entries và tổng uncompressed 32 GiB. Đây là giới hạn sau khi thư viện parse central directory, không phải hard memory/CPU limit của parser. Chỉ nhận cache khớp digest được caller cung cấp từ manifest; không cung cấp chức năng mở ZIP tùy ý trong UI.

Disk preflight trên filesystem cache: tổng size làm tròn allocation block + dự phòng thư mục cha/metadata + 256 MiB, không tính lại dung lượng archive đã nằm trên disk. Không reserve disk; lỗi ENOSPC khi copy vẫn có thể xảy ra. Chưa có ước lượng tổng dung lượng installer trước khi bắt đầu download.

Mỗi lượt tạo thư mục staging mới bằng create (không ghi đè). Thư mục 0700; file create_new 0600, chỉ nâng thành 0700 nếu archive có executable bit. Không mang theo setuid/setgid/group/world permissions. Copy buffer 64 KiB, không ghi vượt size khai báo, đọc tới EOF để CRC được kiểm tra và sync từng file. Không tuyên bố durability toàn cây khi mất điện; publish/journal/sync directory thuộc bước transaction sau.

SdkExtractionControl dùng Arc: cancel, số byte đã ghi, single-use. Cancel được kiểm tra trước I/O, giữa entries và mỗi chunk; checksum subprocess vẫn bounded 300 giây, ZIP parser không có cancellation giữa parse. Bytes đủ không đồng nghĩa verify/install thành công. Lỗi sau tạo staging giữ lại partial và trả đường dẫn; không tự dọn khi retry hoặc đánh dấu usable. Caller chỉ được dùng SdkExtraction khi Result thành công.

## Giới hạn và bước tiếp

Chưa kiểm chứng archive SDK thật; policy tên/type có thể cần mở rộng sau khi có evidence. Đã có kiểm tra package layout/revision/dependency bên dưới; còn cần Java, executable compatibility, journal publish/rollback/recovery, reset cache/stage, UI license/progress/cancel và native smoke trước khi gọi đây là installer hoàn chỉnh. Không suy ra sự chấp nhận license từ unit fixture.

## Kiểm chứng

Fixture ZIP: traversal/absolute/dot/backslash/colon/control/non-ASCII, case aliases và parent conflict; symlink/empty; Stored/Deflate nhiều chunks, explicit directory, quyền 0600/0700; không overwrite; payload CRC sai; disk-budget overflow; cancel-before-start/single-use; backend license/lock/cache checksum gate và giải nén cache thành công. Chỉ dữ liệu tạm, không guest A/B.

## Xác minh package trước assembly

sdk_package.rs kiểm tra read-only staging có đúng một root theo bốn loại package hỗ trợ, rồi kiểm tra source.properties và các file cần thiết. Destination cố định theo enum, không nhận đường dẫn tùy ý từ metadata. Revision stable chuẩn hóa 1–3 thành phần số thành tuple ba số: 7 = 7.0.0, 23.0 = 23.0.0; so sánh đúng revision từ install plan, không nhận preview. Metadata <=64 KiB, key trùng/escape không hỗ trợ bị từ chối.

Command-line tools yêu cầu Pkg.Path và bin/avdmanager, sdkmanager, android có execute; emulator yêu cầu Pkg.Path, emulator và qemu/darwin-aarch64/qemu-system-aarch64; platform-tools yêu cầu adb. Image yêu cầu API36/ARM64/Google Play/vendor Google và năm payload không rỗng. Required files không được là symlink/hardlink hoặc có parent symlink trong package. Bộ package phải có đủ bốn loại, không trùng, emulator >= Pkg.Dependencies của image.

Quan sát read-only source.properties của SDK hiện cài được dùng để xác định format; không sửa hoặc chạy SDK trong bước này. Archive candidate chưa được tải nên cấu trúc ZIP thật vẫn cần kiểm chứng. VerifiedSdkPackage chỉ chứng minh layout/revision tại thời điểm kiểm tra, không phải chữ ký nguồn, kiến trúc Mach-O, khả năng chạy, Java, toàn vẹn đầy đủ hay bằng chứng đã install. Bước assembly/publish phải giữ ownership và kiểm tra lại khi có thể có thay đổi ngoài Hub; không serialize trạng thái verified rồi tin lại sau restart.

Tests dùng file fixture có nội dung giả, chủ ý không thực thi chúng: thiếu package/duplicate/dependency thấp, revision rút gọn/sai/preview, sai ABI/tag/API, duplicate metadata, thiếu/rỗng/mất execute, symlink/hardlink/parent link, extra root và oversized metadata. Không thay thế native setup smoke.
