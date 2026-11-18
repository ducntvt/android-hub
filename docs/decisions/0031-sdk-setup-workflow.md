# ADR 0031 — Luồng tải và chuẩn bị SDK từ UI

- Status: Accepted
- Date: 2026-11-13
- Story: US-025

## Quyết định

Backend.prepare_sdk nhận SdkPlan immutable, exact accepted license hash set, Java home và SdkSetupControl. Gate consent chạy trước I/O. Control chỉ dùng một lần; retry tạo control mới. Trình tự: kiểm tra Java độc lập với SDK, admission disk, tải/verify tuần tự bốn archive, assemble/extract/package check/probe/publish theo ADR 0028. Không tự chọn SDK hoặc thay preferences.

Java được kiểm tra bằng -XshowSettings:properties -version trong home/env riêng trước network; Java 17+ ARM64. Không cần SDK hiện có cho bước này. SDK mới vẫn qua full probe sau assembly.

Admission cần phần archive còn thiếu (đọc size cache .zip/.part, không tin đó là checksum verified) + 6 GiB dự phòng giải nén. Evidence read-only bốn package hiện cài: tools 180.486.444, platform-tools 38.832.970, emulator 1.309.784.053, image 2.419.952.711 bytes logical (~3,95 GB). 6 GiB là policy headroom, không phải trần expansion, disk reservation hoặc guarantee candidate. Downloader kiểm tra disk còn lại và extractor kiểm tra budget ZIP thực tế khi tới bước đó. Sai checksum/oversized cache không tự reset; lỗi rõ, giữ dữ liệu.

SdkSetupControl dùng atomics cho phase, cancel, single-use và bộ controls con. Progress bytes archive gồm cache nhận diện + dữ liệu tải; bytes đủ chưa là verify complete. Cancel chuyển tới tất cả download/extraction controls, được kiểm tra giữa phases. Java/tool probe vẫn bounded riêng, không hủy tức thời; UI báo đang hủy cho tới callback. Thành công sau publish commit thắng cancel đến muộn để không báo hủy giả khi SDK đã tồn tại.

## UI

Màn hình review nay có Java home, nút Tải và chuẩn bị SDK, phase/bytes, Hủy và thông báo kết quả. Bắt đầu chỉ sau đủ consent và absolute Java path. Task chạy spawn_blocking; busy/command_epoch bảo vệ thao tác chồng và snapshot cũ. Progress dùng subscription Tick 3 giây hiện có, không thêm timer/frame polling.

Không đóng panel trong lúc setup để giữ nút hủy truy cập được. Inputs/consent không sửa khi busy. Lỗi/hủy giữ cùng plan/consent trong review cho retry chủ động; đóng/mở review vẫn xóa consent. Thành công xóa consent, quay lại danh sách và refresh catalog; người dùng chọn Kiểm tra và sử dụng riêng, qua migration/stopped gates.

## Evidence và giới hạn

Tests toàn workflow chạy bằng bốn ZIP cached và executables giả: early Java failure không tải, retry control mới đạt, checksum/cache/extract/package/probe/publish nối đủ, preferences cũ giữ nguyên. Tests gate thiếu consent/cancel trước start không tạo runtime; budget subtract cached bytes và reject partial quá lớn. UI task test không poll future nên không thể tải SDK: gate consent, busy/cancel, failure/retry, complete refresh catalog mà không select.

Installed Java test bổ sung Backend có SDK path chưa tồn tại: Java thật vẫn validate được, không tạo SDK; sau đó existing SDK probe vẫn đạt. Downloader HTTP/ETag/cancel và assembly SIGKILL có evidence các bước riêng trước đây; chưa có network-to-UI end-to-end của candidate thật.

CUA getApp Android Hub vẫn báo native pipe startup failed khi kiểm tra workflow mới. Native layout/interaction chưa đạt. Chưa có SDK archive thật được tải/đồng ý license qua agent, chưa boot candidate, chưa xử lý package registration/layout thực tế hoặc reset/quarantine UI. Không gọi installer hoàn chỉnh hoặc đánh dấu US-025 implemented.
