# US-039 — APK và clipboard, 2026-11-17

## Kết quả

Mỗi máy chạy có Cài APK… mở NSOpenPanel với bộ lọc APK. Picker và drop kiểm tra một file thường, không rỗng, đuôi .apk; hộp xác nhận hiển thị tên file, bytes/MiB, đường dẫn và máy nhận. Drop luôn hỏi máy nhận, không đoán tọa độ. Hủy là lựa chọn bàn phím mặc định. ID/PID/command epoch/lựa chọn gốc được kiểm tra lại; không tự retry kết quả install chưa rõ.

Clipboard có tên đích trong phản hồi, chỉ báo thành công sau ACK, nhắc nhấn giữ ô nhập để dán; không hiển thị nội dung clipboard trong toast/log. Giữ generation gate và giới hạn 64 KiB.

## Kiểm chứng

- `bash scripts/check.sh`: 166 Rust (65 desktop, 4 domain, 97 infrastructure) + 25 Python đạt; 20 integration tests đang ignored giữ nguyên, không tính đã chạy. fmt, clippy và shell syntax đạt.
- `bash scripts/build-android-hub.sh` và `codesign --verify --deep --strict 'Android Hub.app'` đạt.
- Regression: drop cần chọn đích; modal không nhận file thứ hai; hủy/late picker không cài; PID/selection/epoch cũ bị từ chối; confirm một lần; mặc định Enter hủy; file không hỗ trợ/rỗng/thư mục bị từ chối. Clipboard generation/64 KiB regressions vẫn đạt.
- Native QA dữ liệu riêng: tạo QA APK A, mở picker thật, Cancel đóng; chọn test.apk 8,532 bytes hiển thị đúng metadata; Enter tại xác nhận hủy, kiểm tra package vẫn vắng. Lặp chọn và bấm Cài APK: kiểm tra đúng AVD/PID và package `local.androidhub.installsmoke` hiện diện.
- Native clipboard: tự tạo chuỗi `Hub clipboard QA 039` qua ô search của Hub rồi copy; gửi vào QA APK A, toast Success ghi tên máy và hướng dẫn dán, không chứa chuỗi. Nhấn giữ ô nhập Android và Paste: chuỗi đúng hiện trong ô nhập.
- Cmd+Q đóng QA; inventory không còn QA, process check không còn app hoặc guest `hub-1794897826265-36`. Không thao tác guest/tài khoản cá nhân.

## Giới hạn

Native picker/install/clipboard đã thử với một guest. Đích B và stale/restart kiểm chứng bằng regression tests; không tuyên bố smoke hai guest hoặc kéo thả liên cửa sổ trong lượt này. Native success toast APK hết hạn trước khi chụp; bằng chứng install là package manager sau kiểm tra danh tính. Back/Home/Recent giữ tuyến input theo ID và tooltip VI/EN; vector icon tiếp tục ở US-040.

QA/bộ APK/log dưới `/tmp/android-hub-apk-*20261117` là artifacts dùng một lần; không thuộc runtime cá nhân.
