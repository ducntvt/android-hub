# Kế hoạch Android Hub

Cập nhật theo source ngày 2026-11-18. Tài liệu này mô tả app hiện tại và phần còn
lại; nghiên cứu engine ban đầu được giữ trong [báo cáo P0](reports/p0-engine-validation.md).
Không còn kế hoạch build/tích hợp AVO trong source đang dùng.

## Mục tiêu và kiến trúc hiện tại

Ứng dụng cá nhân trên macOS Apple Silicon, quản lý ít nhất hai máy Android độc
lập để dùng Play Store và chơi Top Heroes/Hunt Royale. Tái sử dụng Android
Emulator Google Play ARM64 API 36; Rust + Iced cung cấp cửa sổ quản lý và màn
hình Android nhúng. Không có HTTP backend hoặc dịch vụ nền riêng.

Workspace giữ ba crate: `domain` cho cấu hình/validation, `infrastructure` cho
SDK/process/storage/display và `desktop` cho state/update/view. Desktop dùng
infrastructure/domain; domain không phụ thuộc I/O hay UI. Không thêm crate
application/CLI hoặc abstraction nhiều engine khi chưa có nhu cầu thật.
Chi tiết: [Architecture](ARCHITECTURE.md), [Development](DEVELOPMENT.md),
[ADR 0053](decisions/0053-rust-module-conventions.md).

Mỗi máy có AVD, process, port và dữ liệu riêng. Stream/input gắn đúng ID/PID và
phiên kết nối; dừng một máy không chủ động dừng máy khác. Đóng Hub bình thường
sẽ dừng các engine đã đăng ký trước khi thoát; lỗi dừng giữ app mở. Force Quit
hoặc crash nằm ngoài luồng này.

## Phần đã triển khai

- Danh sách máy, tìm kiếm, tạo mới, đổi tên, khởi động/dừng và xóa có xác nhận.
- Một/hai màn hình Android bên trong Hub; fit/zoom, input, cuộn, navigation,
  xoay thiết bị, clipboard theo thao tác và cài APK theo đúng máy.
- Cấu hình CPU/RAM/độ phân giải/chiều màn hình/FPS, kiểm soát thay đổi chưa lưu.
  Có mức 15/30/60/90/120; mặc định 30. Đây là giới hạn yêu cầu, chưa phải FPS đo được.
- EN/VI, System/Light/Dark, panel cấu hình/SDK/dữ liệu, thông báo và lịch sử.
- Backup offline có checksum; restore với compatibility gate, journal/rollback;
  recovery tạo máy, archive cũ và xóa dữ liệu có xác nhận.
- Chọn SDK/Java, kiểm tra môi trường; luồng SDK riêng có review/consent,
  tải/hủy/retry, kiểm tra archive và catalog. Xác thực trên máy sạch còn thiếu.
- Đo CPU/RSS, dung lượng theo yêu cầu, kiểm tra kết nối guest và xoay log engine.
- Bundle `.app` cho máy cá nhân, check script cho app, metrics và script Python.

Người dùng đã xác nhận gameplay và hai app đồng thời. Điều này không thay cho
benchmark hai game trong 60 phút. Bằng chứng/giới hạn theo từng phần nằm trong
[story index](stories/README.md) và [đối chiếu MVP](reports/mvp-coverage.md).

## Ưu tiên tiếp theo

Theo yêu cầu hiện tại, để người dùng dùng bản chính và phản hồi lỗi thực tế.
US-044 đã chuẩn hóa source/style/docs; US-045 chuyển công cụ và reset runtime theo yêu cầu. Không tự mở rộng
feature hoặc chạy thêm guest/QA dài hạn.

| Phần còn lại | Điều kiện và đầu ra |
| --- | --- |
| Lỗi sử dụng thực tế | Tái hiện đúng lỗi người dùng báo, sửa nhỏ và kiểm tra phạm vi liên quan |
| Native accessibility (US-043) | Kiểm chứng cầu nối accessibility của framework; keyboard support không đồng nghĩa VoiceOver đã đạt |
| Setup trên máy sạch (US-025) | Người dùng xem điều khoản; kiểm chứng tải/retry/recovery, tạo và boot guest bằng SDK candidate |
| Benchmark hai game | Khi người dùng sẵn sàng: ghi engine/image/game version, cấu hình và cảnh chơi; đo ≥60 phút |
| Input/crash/sleep-wake | Bổ sung các ca gián đoạn còn thiếu trong US-011/012; dùng dữ liệu QA riêng |
| Migration dữ liệu đã có | Default đã ở Application Support sau reset US-045; chuyển dữ liệu đang dùng sang vị trí khác vẫn cần migration riêng |
| Khôi phục máy đã xóa | UI tạo lại máy từ backup; restore hiện tại chỉ dành cho máy còn đăng ký |
| Phát hành công khai | Xem lại giấy phép/quyền phân phối engine và image, bundle độc lập, signing/notarization, onboarding và kiểm chứng máy sạch |

Key mapping, đa chạm, clone, snapshot RAM/thiết bị, auto-update và đa nền tảng
chưa có. Chỉ tách story triển khai khi có yêu cầu; backup dữ liệu không phải
snapshot toàn bộ máy đang chạy.

## Kiểm chứng hiệu năng và dữ liệu

Benchmark cần phân biệt CPU/RSS của một process với toàn bộ footprint/GPU;
phân biệt tần số stream/VSYNC với FPS gameplay. Ghi cold boot, mở game, frame
time nếu đo được, memory pressure/swap, lỗi input/audio/network và khả năng
phục hồi sau sleep/wake. Mục tiêu ban đầu 30 FPS ở 720p trên hai máy vẫn cần
bằng chứng, không phải kết quả đã đạt. Không dùng pause để tính là hai máy chơi
đồng thời, hoặc quy khác biệt giữa hai image cho riêng engine.

Bài thử dữ liệu dùng fixture/guest riêng. Không reset guest thật để dọn QA.
SDK, registry, AVD, backup, recovery receipt và tài nguyên DEX/manifest/protocol
là dữ liệu cần giữ. Dữ liệu mới nằm tại `~/Library/Application Support/Android Hub`; runtime P0 cũ
đã được xóa theo yêu cầu reset, không còn là dữ liệu đang dùng. Chỉ dọn target và artifact tạm đã xác định sau mỗi
lượt; đóng bản QA đã mở trước khi bàn giao.
