# E02 — Thiết bị Android thật trong Android Hub

- **Status**: in_progress
- **Created**: 2026-11-19
- **Branch**: `feature/physical-devices`
- **Base**: `develop` tại `63aaaed` (v0.2.0)
- **Design**: [ADR 0060 — Accepted](../../../decisions/0060-physical-device-sessions.md)

## Mục tiêu

Nhận diện và điều khiển điện thoại Android cắm vào Mac, hiển thị ngay trong cửa sổ
Rust/Iced hiện tại. Một workspace có thể mở một điện thoại thật và một máy ảo.
Nhánh feature dành cho thử nghiệm; chưa thay app chính, commit/push hay phát hành.

## Phạm vi đề xuất

USB trước trên macOS Apple Silicon. Tái sử dụng ADB và transport stream phù hợp;
US-056 đã chọn scrcpy v5.0.1 cho triển khai tiếp; renderer prototype cần tối ưu. So sánh scrcpy với
Android Device Hub của Google nếu dùng được; khảo sát QtScrcpyCore làm nguồn tham
khảo. Vysor là sản phẩm đối chiếu, chưa xác nhận có SDK/API nhúng phù hợp. Không
triển khai nhiều engine production cùng lúc chỉ để so sánh.

Không suy hiệu năng từ ngôn ngữ Rust hoặc số FPS quảng cáo. Thiết bị thật tự chạy
Android/game; Hub nhận video và gửi input. Bài thử phải tách độ trễ end-to-end,
FPS khi có chuyển động và CPU/RAM của host. H.264/USB/60 FPS là cấu hình thử ban đầu,
không phải cam kết mọi điện thoại đạt 60 FPS. Chọn giải mã phần cứng theo kết quả
thực tế, không giả định luôn hardware decode hoặc zero-copy.

## Luồng người dùng mục tiêu

1. Có ADB hợp lệ hoặc thiết lập Platform-Tools; không bắt tải image/emulator/Java.
2. Cắm điện thoại, bật USB debugging và tự chấp nhận RSA trên điện thoại.
3. Hub hiển thị nhóm Thiết bị thật, tên/model, Android, kết nối và trạng thái quyền.
4. Chọn Mở màn hình; xem/điều khiển ngay trong Hub, có thể ghép với slot máy ảo.
5. APK và clipboard chỉ gửi theo thao tác chủ động, đích rõ ràng.
6. Rút cáp → slot báo mất kết nối; đóng Hub → kết thúc phiên điều khiển, điện thoại
   tiếp tục hoạt động. Không replay thao tác cũ khi cắm lại.

## Quy tắc dữ liệu và quyền sở hữu

Physical không phải AVD: không tạo/xóa/restore image, đổi CPU/RAM, factory reset
hoặc tắt điện thoại khi Quit. Máy ảo giữ hành vi hiện có. Không kill ADB server
hoặc tunnel/helper của công cụ khác. Uninstall Hub chỉ xóa tài nguyên host sở hữu,
không xóa nội dung điện thoại. Không log video, clipboard, pairing code hay tài khoản.
Tách device identity khỏi transport/session generation; xác minh lại đích trước ghi.

## Stories và thứ tự

| US | Kết quả | Phụ thuộc | Giai đoạn |
| --- | --- | --- | --- |
| [US-056](US-056-physical-device-streaming-spike.md) | So sánh stream trên thiết bị thật | Không có | MVP |
| [US-057](US-057-device-types-and-capabilities.md) | Phân biệt máy ảo và thiết bị thật | US-056 có phương án khả thi | MVP |
| [US-058](US-058-usb-discovery-and-adb-setup.md) | Nhận diện USB và thiết lập ADB tối thiểu | US-057 | MVP |
| [US-059](US-059-embedded-physical-display.md) | Nhúng màn hình điện thoại vào Hub | US-056 đã chốt transport; US-057, US-058 | MVP |
| [US-060](US-060-physical-input-and-navigation.md) | Điều khiển điện thoại và navigation | US-059 | MVP |
| [US-061](US-061-physical-apk-and-clipboard.md) | Cài APK và clipboard đúng điện thoại | US-057, US-058; clipboard cần US-060 | MVP |
| [US-062](US-062-mixed-device-workspace.md) | Workspace chung cho điện thoại và máy ảo | US-058–061 | MVP |
| [US-063](US-063-physical-session-lifecycle-and-packaging.md) | Vòng đời kết nối, cleanup và đóng gói | US-057–062 | MVP |
| [US-064](US-064-physical-device-qualification.md) | Nghiệm thu USB và hiệu năng workspace hỗn hợp | US-056–063 | MVP |
| [US-065](US-065-wireless-device-pairing.md) | Ghép đôi điện thoại qua Wi-Fi | MVP USB đạt US-064 | Future |
| [US-066](US-066-physical-device-audio.md) | Nghe âm thanh từ điện thoại | US-064; transport có hỗ trợ audio | Future |
| [US-067](US-067-physical-screen-capture.md) | Chụp ảnh và ghi màn hình điện thoại | US-064; audio ghi kèm phụ thuộc US-066 | Future |
| [US-068](US-068-physical-file-transfer.md) | Truyền file có xác nhận với điện thoại | US-064 | Future |

Thứ tự MVP: 056 → 057 → 058 → 059 → 060/061 → 062 → 063 → 064.
Có thể viết test/model độc lập, nhưng không chốt implementation stream trước kết
luận khả thi của 056. US-065–068 chỉ là backlog tương lai, không tự triển khai khi
đang thử USB. Không tự tạo agent hoặc mở nhiều app QA.

## Điều kiện bắt đầu thử nghiệm

Cần chốt model/Android của một điện thoại người dùng cho phép thử và xác nhận đã
cấp USB debugging. US-056 đã thử Samsung SM-G996B / Android 15 với quyền này. Không có thiết bị thì tiếp tục
khảo sát/code/test thuần có ích, ghi rõ thiếu bằng chứng phần cứng. Không cài APK,
đăng nhập hoặc thay đổi settings điện thoại chỉ để thu thập thông tin.

## Definition of done — MVP

US-056–064 implemented, có một điện thoại thật đã nghiệm thu, cặp physical/emulator
đạt và emulator-only không hồi quy. Ngân sách hiệu năng chốt từ 056 được kiểm lại
trong 064; ca chưa đo không được gọi là pass. QA giới hạn bài đo ngắn và soak 10 phút,
không tự mở rộng thành benchmark hàng giờ. Đóng app/helper QA, dọn build/cache/temp
mỗi lượt; giữ dữ liệu cá nhân và bằng chứng nhỏ cần thiết. Nâng version/publish cần
lệnh riêng của người dùng.

## Nguồn khảo sát

Các nguồn dưới đây đã được đối chiếu trong trao đổi ngày 2026-11-19; khi triển khai
phải ghi chính xác phiên bản/revision dùng thử.

- [ADB và USB/Wireless debugging](https://developer.android.com/tools/adb)
- [scrcpy](https://github.com/Genymobile/scrcpy)
- [scrcpy video/decoder](https://github.com/Genymobile/scrcpy/blob/master/doc/video.md)
- [scrcpy protocol và kiến trúc](https://github.com/Genymobile/scrcpy/blob/master/doc/develop.md)
- [Android Device Hub của Google](https://developer.android.com/tools/agents/android-cli/commands/device_hub)
- [QtScrcpyCore](https://github.com/barry-ran/QtScrcpyCore)
- [Vysor](https://www.vysor.io/)

## Trạng thái hiện tại

US-056 implemented: đã đo baseline, nhúng video/input Iced và chốt scrcpy v5.0.1.
Người dùng miễn đo camera; latency chưa kiểm chứng. ADR 0060 Accepted cho hướng
triển khai, không phải nghiệm thu renderer hiện tại. US-057 implemented: typed ID,
capabilities, session tokens và facade chặn AVD operations trên physical.
US-058 implemented: nhận diện USB/model/Android, theo dõi cắm/rút, hướng dẫn
EN/VI và thiết lập ADB riêng. Đã thử cắm/rút Samsung trên QA. US-059 implemented: video nhúng dùng VideoToolbox trực tiếp, Start/Stop,
30/60 FPS, ba mức stream, Fit/zoom/rotation và ca nội dung bảo vệ đã kiểm tra.
US-060–068 vẫn planned; chưa có control, MVP và nghiệm thu hiệu năng chưa hoàn tất.
Bước tiếp theo: US-060.
