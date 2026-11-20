# US-060 — Điều khiển điện thoại và navigation

- **Status**: implemented
- **Owner**: Android Hub maintainer
- **Created**: 2026-11-19
- **Risk lane**: high-risk
- **Related**: [Epic E02](README.md), [ADR 0060](../../../decisions/0060-physical-device-sessions.md)
- **Depends on**: US-059
- **Phase**: MVP

## Context

Là người dùng, tôi muốn click, kéo/vuốt, cuộn và dùng bàn phím đúng thiết bị đang chọn.

## Contract (acceptance criteria)

- [x] Click/drag/swipe, wheel hai trục, Back/Home/Recent và bàn phím hoạt động qua transport đã chọn; không spawn adb shell riêng cho từng chuyển động chuột.
- [x] Ánh xạ tọa độ đúng tại Fit, zoom 110%, resize, portrait/landscape và hai slot. Thay đổi orientation giữa gesture hủy an toàn gesture cũ.
- [x] Focus/selection rõ ràng; thao tác không lọt sang máy bên cạnh. Nhả phím/touch khi mất focus, mở modal, đóng stream hoặc rút cáp; không replay input cũ sau reconnect.
- [x] Giới hạn hàng đợi và xử lý backpressure có thứ tự; không bỏ mất release event để rồi kẹt phím.
- [x] Xoay chỉ cung cấp nếu capability thực hỗ trợ và mô tả rõ ảnh hưởng lên điện thoại; OEM chặn điều khiển phải có hướng dẫn phù hợp.
- [x] Test native dùng app QA có bộ đếm touch/scroll/key; xác nhận các ký tự được hỗ trợ và giới hạn IME/Unicode, không mặc định nhận mọi kiểu gõ.

## Out of scope

Key mapping game, macro, gamepad, tự động chơi game và remote control qua Internet.

## Affected areas

Infrastructure input/control, desktop screen/focus, fixture tests/android-control/.

## Implementation notes

Input gắn transport/session generation, stream epoch, input epoch và capture ID;
không gắn PID emulator giả cho điện thoại. Kênh scrcpy riêng truyền touch/scroll,
phím navigation và printable ASCII; phím down/up là một thao tác có thứ tự, không
hỗ trợ giữ phím game hoặc IME/Unicode. Hàng đợi 64 phần tử: reset/quá tải xóa input
chờ và ưu tiên CANCEL touch; không loại release để nhường chỗ cho move.

Dùng lại canvas mapping của emulator qua adapter riêng; không route serial qua
handler AVD. Khi orientation thay đổi, CANCEL được gửi với kích thước capture
mới và gesture cũ không tiếp tục. Quit chờ completion của các stream đã mở trước
khi thoát. Chưa quảng bá capability xoay hệ thống điện thoại, nên không hiện nút
xoay và không thay cài đặt rotation của người dùng.

Fixture `tests/android-control` giữ nguyên nguồn benchmark US-056. Kết quả kiểm thử và giới hạn được ghi bên dưới.

## Verify

Lệnh dành cho lúc triển khai story, chạy từ repo root; không build/test Rust chỉ để viết US.

```bash
bash scripts/check.sh
git diff --check
```

Thêm test hồi quy đúng các lỗi/ranh giới nêu trên, chạy native QA tương ứng trên fixture và điện thoại được chỉ định. Khi sửa bundle/helper, build và verify chữ ký bằng script hiện có. Lệnh test thuần đạt không thay thế ca thiết bị thật.

## Evidence

QA ngày 2026-11-20 trên Samsung SM-G996B / Android 15, USB trực tiếp:

- Click, drag, cuộn dọc; cuộn ngang bằng trackpad thật được người dùng hỗ trợ,
  bộ đếm H thay đổi. Input tại zoom 110% và landscape đã được kiểm tra.
- Home/Recent qua nút Hub chuyển đúng Samsung Launcher/RecentsActivity.
  Resize cửa sổ từ 2200×1584 lên 3840×1980: click/kéo/cuộn vẫn nhận,
  Held=false và V đổi từ -10 thành -20.
- Back và ASCII (gồm Shift, Enter, Backspace) có cặp down/up cân bằng.
  Mở cài đặt Hub chặn input bàn phím tới điện thoại.
- Hai slot với emulator QA Android 16: phím C chỉ tăng bộ đếm Samsung
  (8 down / 8 up); phím D chỉ tăng bộ đếm emulator (1 down / 1 up).
  Bộ đếm máy không nhận focus giữ nguyên, touch đều Held=false.
- Probe Rust dùng Session thật trên fixture Samsung: touch xuống ghi Held=true;
  `release_inputs()` chuyển Held=false, Cancel=1; đóng stream khi đang giữ touch
  chuyển Held=false, Cancel=2. Không gửi input ra ngoài fixture.
- Test hồi quy chặn ký tự Unicode có ASCII base key đạt; không tự thay `á` bằng `a`.
- `bash scripts/check.sh` và `git diff --check` đạt sau thay đổi mixed-slot.
  Đã đóng Hub QA; emulator QA được dừng cùng Hub.

- Rút/cắm USB thật: Hub đóng stream, khóa điều khiển, nhận transport mới.
  Phím Z gửi trong lúc disconnected không được phát lại khi mở phiên mới;
  bộ đếm phím vẫn 8 down / 8 up.
- Fixture tự xoay sau 500 ms, giữ chuột 2500 ms: portrait chuyển landscape,
  Down=1, Up=0, Cancel=1, Held=false. Gesture mới sau rotation ghi Down=2,
  Up=1; phím E ghi 1 down / 1 up. Gesture cũ không tiếp tục.
- Bộ kiểm tra cuối: 109 desktop + 11 domain + 169 infrastructure + 2 metrics
  tests đạt, 25 integration tests giữ ignored theo quy trình; 36 Python tests,
  fmt, clippy `-D warnings`, docs và shell syntax đạt. Bundle QA codesign hợp lệ.
- Đã đóng QA, xác nhận không còn process QA/emulator QA/server scrcpy, gỡ fixture
  do phiên này cài. Dọn bundle/data QA riêng và cache build; giữ app/data/SDK chính.

Giới hạn: khi cáp đã mất, không thể bảo đảm gói CANCEL cuối đến điện thoại;
Hub hủy trạng thái/queue local và không replay sang phiên mới. Cancel khi kênh
còn sống đã được xác minh bằng fixture. Chỉ Samsung SM-G996B / Android 15 được
kiểm thử native; không suy rộng sang mọi OEM. IME/Unicode, giữ phím game và xoay
hệ thống điện thoại không được cung cấp. Performance vẫn thuộc US-064.
