# E01 — Chọn phiên bản Android khi tạo máy

- **Status**: implemented
- **Created**: 2026-11-18
- **Scope confirmed by**: người dùng trong trao đổi ngày 2026-11-18
- **Design**: [ADR 0056 — accepted](../../../decisions/0056-immutable-android-image-per-device.md)

## Mục tiêu và quy tắc đã chốt

Người dùng chọn Android 12–16 khi tạo máy mới. Mỗi máy giữ nguyên phiên bản đã chọn
trong suốt vòng đời; muốn dùng Android khác thì tạo máy mới với dữ liệu và tài khoản
độc lập. Các máy khác phiên bản có thể chạy đồng thời trong Hub.

- Phạm vi host: macOS Apple Silicon; image Google Play ARM64 stable.
- Android 16 tiếp tục là mặc định khi tạo máy; không tự chọn version mới nhất.
- Máy cũ giữ dữ liệu và image hiện có; chỉ bổ sung metadata sau khi xác minh AVD/SDK.
- Không có chức năng đổi Android trong settings, nâng/hạ OS hoặc chuyển nguyên dữ liệu
  máy giữa các phiên bản. Image revision cũng không tự thay đổi khi catalog cập nhật.
- Chỉ tải image người dùng chọn. Công cụ SDK/engine dùng chung nếu đạt tương thích.
- Gỡ image chỉ khi Hub quản lý và không còn tham chiếu cần bảo toàn.

## Catalog mục tiêu

| Android | API | Image |
| --- | --- | --- |
| 12 | 31 | Google Play ARM64 |
| 13 | 33 | Google Play ARM64 |
| 14 | 34 | Google Play ARM64 |
| 15 | 35 | Google Play ARM64 |
| 16 | 36 | Google Play ARM64; mặc định |

Khảo sát ngày 2026-11-18 đã đọc [catalog Google](https://dl.google.com/android/repository/sys-img/google_apis_playstore/sys-img2-3.xml)
và thấy package tương ứng; đây là bằng chứng có image, chưa phải kết quả chạy Hub.
Revision/checksum đã được kiểm chứng và cố định trong catalog; kết quả chạy thực tế
xem [matrix US-053](../../../reports/android-versions-qualification-20261118.md). Android 12L/API 32,
Android 17+, preview, x86 và các biến thể page size không thuộc đợt này.

## Luồng người dùng

1. Bấm **+** → nhập tên/cấu hình và chọn phiên bản Android.
2. Image đã sẵn sàng → **Tạo máy**. Chưa có → **Tải và tạo máy**, xem dung lượng/plan
   và xác nhận license nếu cần trước khi tải.
3. Theo dõi tiến trình, có thể hủy/thử lại khi phù hợp; giữ form và không tạo trùng máy.
4. Máy mới hiển thị đúng version; thông tin Android trong settings chỉ đọc.
5. **Hub settings → Phiên bản Android** quản lý image, dung lượng và tham chiếu.

## Stories và thứ tự

| US | Kết quả | Phụ thuộc |
| --- | --- | --- |
| [US-048](US-048-android-version-compatibility-spike.md) | Spike Android 12/16, engine chung và helper | Bắt đầu |
| [US-049](US-049-immutable-device-image-identity.md) | Catalog, identity bất biến, metadata migration và backup gates | 048 có phương án khả thi |
| [US-050](US-050-on-demand-android-image-installation.md) | Cài thêm image theo nhu cầu | 049 |
| [US-051](US-051-choose-android-version-at-creation.md) | Chọn version, tải và tạo máy trong UI | 049, 050 |
| [US-052](US-052-manage-installed-android-images.md) | Quản lý/gỡ image có kiểm tra tham chiếu | 049, 050 |
| [US-053](US-053-qualify-five-android-versions.md) | Helper production và nghiệm thu cả năm bản | Helper sau 048; nghiệm thu sau 049–052 |

US-051 và US-052 có thể triển khai độc lập khi backend sẵn sàng. Thứ tự này là kế
hoạch công việc, không yêu cầu tự tạo agent hoặc chạy nhiều QA cùng lúc.

## Definition of done

Tất cả story hoàn tất với bằng chứng tương ứng; đủ năm bản tạo/start/stop và dùng
được chức năng Hub trong matrix của US-053; cặp 12/16 chạy đồng thời đúng đích.
Máy cũ không đổi dữ liệu/version; không có đường UI/backend đổi image giữa chừng.
Download/cancel/recovery và các gate backup/restore/gỡ image đạt test.

Play Store, game compatibility và hiệu năng là các kết quả riêng. Không suy luận
“chạy mọi game” từ “có image” hoặc “boot được”. Benchmark dài và tài khoản Google
cá nhân không phải điều kiện để viết/triển khai test thuần của feature.

## Kết quả triển khai

US-048–053 đã triển khai. Catalog/binding bất biến, tải theo nhu cầu, chọn version
khi tạo và quản lý/gỡ image đã có bằng chứng trong từng story. US-053 ghi matrix
cả năm bản, chạy cặp 12/16, dừng riêng và Quit cleanup. SDK bên ngoài có thể bị
công cụ khác sửa; Hub báo mismatch và không tự đổi binding.

Xem [báo cáo nghiệm thu](../../../reports/android-versions-qualification-20261118.md)
cho revision đã thử, các lần thử chưa đạt, giới hạn native automation và phạm vi
không kiểm thử. Không suy luận game compatibility hoặc FPS từ kết quả chức năng Hub.
Dữ liệu QA/cache build đã dọn; app chính và dữ liệu cá nhân được giữ nguyên.
Remote CI chưa chạy; chưa commit/push hoặc tạo tag/release.
