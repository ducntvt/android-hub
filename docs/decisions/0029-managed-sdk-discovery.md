# ADR 0029 — Tìm lại và chọn SDK đã publish

- Status: Accepted
- Date: 2026-11-13
- Story: US-025

## Quyết định

Backend.managed_sdk_catalog chạy on-demand khi mở Thiết lập SDK hoặc bấm Làm mới, không poll định kỳ. Nếu chưa có managed-sdks, trả danh sách rỗng mà không tạo file. Giữ shared install.lock khi đọc; installer giữ exclusive lock nên catalog báo đang ghép thay vì nhìn cây thay đổi.

Chỉ sdk-ID với ID ba nhóm số, thư mục thật và .hub-sdk.json hợp lệ mới được xem xét. Receipt <=1 MiB, schema/state/id khớp, Java metadata phù hợp, đúng bốn package/destination cố định và archive fields hợp lệ. Kiểm tra lại source.properties, required files, quyền execute, dependency và parent directories thật bằng package validator dùng chung. Không chạy binary hoặc hash lại nhiều GB payload khi mở panel. Receipt/layout check không chứng minh byte integrity hiện tại hoặc boot/game compatibility.

Tối đa 1.000 entries top-level; nếu vượt thì báo chưa quét hết thay vì trả kết quả có vẻ đầy đủ. Mục không đọc/validate được tính skipped. .staging-ID không có SDK publish hợp lệ tương ứng tính incomplete; journal còn lại sau commit không bị tính là cài dở. Không di chuyển/xóa/repair file trong catalog. Staging có marker ready vẫn không được đưa vào danh sách dùng được.

UI hiển thị revision emulator/path, số lượt dở và mục skipped, cùng nút Kiểm tra và sử dụng. Epoch guard bỏ response cũ hoặc response khi panel đã đóng. Async spawn_blocking giữ UI thread không bị I/O chiếm. Chưa có cleanup/reset UI.

## Chọn SDK và Java cùng transaction preferences

configure_managed_sdk đọc lại catalog để không tin entry cũ của UI. configure_sdk_and_java kiểm tra absolute/canonical SDK, env overrides, Hub lock, stopped/port gates và SDK migration guard; cần đủ payload. Probe SDK mới cùng Java trong receipt trước khi ghi SDK + Java chung một hub-settings.json bằng writer atomic hiện có. Điều này cho phép chọn SDK đã chuẩn bị cùng Java phù hợp dù Java mặc định trên host không có.

Không cho chọn qua UI khi ANDROID_SDK_ROOT/P0_JAVA_HOME đang override. Lỗi trước write giữ preferences cũ; thành công UI đổi backend path/drafts và initialize trước thao tác tiếp theo. Runtime còn AVD/archive gắn SDK khác vẫn bị chặn, không tự migrate. Không tự chọn chỉ vì tìm thấy SDK đã publish.

## Evidence và giới hạn

Unit kiểm tra catalog empty không tạo file, publish/dở/invalid/symlink, marker sai id, thiếu adb, lock contention; UI epoch/dismiss/failure/success. Integration dùng SDK/Java scripts giả trong temp root: chọn chung hai prefs, reload, Java thiếu giữ prefs, migration guard giữ prefs. SIGKILL fixture ba checkpoint nay gọi catalog thật sau worker chết: trước commit không chọn được, sau commit tìm được đúng SDK; retry vẫn đạt.

Không dùng fixtures như evidence chạy SDK thật. Native panel mới chưa qua visual/interaction smoke; công cụ CUA lỗi native pipe ở lần kiểm tra trước. Package catalog không xác minh checksum nội dung từng file đã publish, không sandbox binary bị sửa bởi tác nhân khác. Khi dùng SDK chỉ chạy code tin cậy từ archive verify/assembly. Còn installer license/download UI, cleanup/quarantine, archive thật/package registration/native boot và benchmark.
