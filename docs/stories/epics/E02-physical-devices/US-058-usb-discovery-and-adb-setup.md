# US-058 — Nhận diện USB và thiết lập ADB tối thiểu

- **Status**: implemented
- **Owner**: Android Hub maintainer
- **Created**: 2026-11-19
- **Risk lane**: high-risk
- **Related**: [Epic E02](README.md), [ADR 0060](../../../decisions/0060-physical-device-sessions.md)
- **Depends on**: US-057
- **Phase**: MVP

## Context

Là người dùng, tôi muốn cắm điện thoại để Hub nhận diện và hướng dẫn cấp quyền, kể cả khi chưa cài máy ảo.

## Contract (acceptance criteria)

- [x] Khám phá qua ADB và cập nhật cắm/rút với watcher hoặc polling có giới hạn; không spawn lệnh getprop cho mọi thiết bị ở mỗi frame. Chỉ đọc metadata khi phiên/thông tin cần đổi.
- [x] Phân biệt ready, unauthorized, offline, disconnected và lỗi ADB. Hướng dẫn USB debugging/RSA EN/VI; không tự xác nhận trên điện thoại.
- [x] Không nhận mọi serial khác emulator-* là điện thoại thật: đối chiếu transport/metadata, không nhập emulator ngoài Hub vào nhóm physical; trạng thái chưa rõ phải được ghi rõ.
- [x] Đọc model và Android khi đã được phép; xử lý nhiều thiết bị, serial lạ, phản hồi hỏng, timeout và ADB server restart bằng test.
- [x] Luồng physical chỉ yêu cầu Platform-Tools/ADB cùng dependency stream, không bắt cài Java/Emulator/system image. Có chọn ADB hợp lệ; tải Platform-Tools nếu hỗ trợ phải có nguồn, license/consent, checksum và hủy an toàn.
- [x] Dùng chung ADB một cách có kiểm soát; không tự adb kill-server để sửa lỗi vì có thể ảnh hưởng Android Studio/Hub khác. Hiển thị lỗi phiên bản/xung đột và cách xử lý.

## Out of scope

Wi-Fi pairing, mở stream tự động, cài APK hoặc thay cấu hình điện thoại.

## Affected areas

Infrastructure discovery/ADB/setup, desktop subscriptions và i18n.

## Implementation notes

USB là phạm vi đầu tiên. Kết nối Wi-Fi đã xuất hiện trong ADB được nhận biết là transport khác, không được gắn nhãn USB.

## Verify

Lệnh dành cho lúc triển khai story, chạy từ repo root; không build/test Rust chỉ để viết US.

```bash
bash scripts/check.sh
git diff --check
```

Thêm test hồi quy đúng các lỗi/ranh giới nêu trên, chạy native QA tương ứng trên fixture và điện thoại được chỉ định. Khi sửa bundle/helper, build và verify chữ ký bằng script hiện có. Lệnh test thuần đạt không thay thế ca thiết bị thật.

## Evidence

2026-11-19, nhánh `feature/physical-devices`:

- `infrastructure::physical`: smart socket `host:track-devices-l`, check protocol
  41, backoff 3 giây; không spawn getprop theo frame. Frame ban đầu/dở và metadata
  timeout 4 giây. Snapshot tối đa 128 thiết bị; metadata tối đa 4 KiB.
- Parser phân biệt marker USB, network, emulator prefix và unknown; đọc thêm qemu
  khi USB ready, loại emulator. Model/Android chỉ đọc khi có phiên mới; không có
  serial-derived filesystem path. Multiple devices có transport/token riêng.
- Desktop: sidebar connected devices, trạng thái ready/unauthorized/offline/
  disconnected/unknown, lỗi ADB riêng và retry. Kết quả metadata phải khớp phiên;
  EOF/restart/rút cáp/đổi authorization/quit vô hiệu phiên cũ. Metadata chỉ ở RAM.
- Setup riêng trong Hub settings: xác minh `adb version`, chọn executable tuyệt
  đối và ghi atomic `physical-adb-path`; không yêu cầu Java/Emulator/image. Không
  hỗ trợ tải tự động: UI/docs dẫn Platform-Tools chính thức và license thủ công.
  Khi thiếu server, hướng dẫn chạy lệnh start-server bằng Terminal. Chỉ hỗ trợ
  loopback:5037; Hub không gửi host:kill, không restart server hoặc đổi settings phone.
- Native QA với Samsung SM-G996B / Android 15 cắm trực tiếp Mac: app dữ liệu QA
  rỗng tự phát hiện model/Android. Người dùng rút cáp: tự hiện disconnected; cắm
  lại: tự trở về USB ready/Android 15, không cần refresh. Không reset RSA để thử
  unauthorized; ca đó được kiểm tra bằng fixture/state test. Không tuyên bố đã
  thử nhiều điện thoại thật cùng lúc, restart server thật hoặc transport Wi-Fi.
- QA bản cuối: tự chọn USB khi không có AVD; tiêu đề Android 15 · USB; Settings
  AVD disabled; verify/save ADB báo thành công, file path QA đúng; EN→VI đổi đúng
  trạng thái, hướng dẫn và thông báo. Hai QA bundle đều codesign verify strict/deep
  đạt và đã quit; inventory xác nhận không còn QA chạy.
- Test parser nhiều transport, malformed/duplicate IDs, metadata qemu; socket tests
  fragmented frames, EOF, timeout, malformed UTF-8, FAIL và version mismatch không
  gửi kill; desktop tests cache/no redundant metadata, reconnect/generation, nhiều
  USB độc lập, phone-only UI. Standalone ADB fixture không SDK/Java/AVDs vẫn lưu được.
- Full check phát hiện shared lock trong SDK catalog có thể bị giữ qua fork;
  sửa dùng FileLock guard có explicit unlock, thêm regression cho inherited shared
  descriptor. Ca lỗi đạt khi chạy riêng; full suite chạy lại sau fix.

- `bash scripts/check.sh` cuối: đạt — desktop 105, domain 11, infrastructure
  160 tests; 25 integration tests vốn ignored không tính đã chạy. Metrics 2,
  Python 36; Rustfmt, Clippy all-targets `-D warnings`, docs và shell syntax đạt.
- `git diff --check`: đạt. Cleanup xóa target/metrics target, hai bundle/data QA
  và log tạm US-058 sau khi xác nhận process đã thoát; không sửa app/dữ liệu chính,
  không dừng ADB server chung hoặc thay nội dung điện thoại. Chưa commit/push.
