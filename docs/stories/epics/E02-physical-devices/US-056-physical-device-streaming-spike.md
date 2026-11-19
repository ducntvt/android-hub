# US-056 — So sánh stream trên thiết bị thật

- **Status**: implemented
- **Owner**: Android Hub maintainer
- **Created**: 2026-11-19
- **Risk lane**: normal
- **Related**: [Epic E02](README.md), [ADR 0060](../../../decisions/0060-physical-device-sessions.md)
- **Depends on**: Không có
- **Phase**: MVP

## Context

Là người dùng Hub, tôi muốn chọn giải pháp có độ trễ thấp trên điện thoại thật trước khi đầu tư tích hợp vào app.

## Scope decision — 2026-11-19

Người dùng đồng ý bỏ đo độ trễ bằng camera và tiếp tục khép goal. Đây là phép đo
được **miễn trong US-056**, không phải kết quả đạt. Giữ số liệu CPU/RAM/FPS và
kết luận khả thi; không tuyên bố đã nghiệm thu độ trễ hoặc hiệu năng production.
Các giới hạn màn hình/cáp chưa xác minh và timing input không đồng bộ được giữ rõ
trong báo cáo, không suy diễn thành môi trường benchmark đã kiểm soát.

## Contract (acceptance criteria)

- [x] Ghi model/Android, chip/macOS, màn hình Mac, cáp, phiên bản công cụ, codec, độ phân giải, bitrate và FPS của từng bài thử. Không tự coi Android 12–16 của emulator là phạm vi hỗ trợ điện thoại.
- [x] Đo scrcpy nguyên bản qua USB làm baseline; thử Android Device Hub của Google cùng cấu hình nếu môi trường hỗ trợ. Khảo sát QtScrcpyCore về cách tích hợp/giấy phép; không cần triển khai mọi giải pháp. Công cụ không chạy được phải có lý do, không gọi là kết quả đạt.
- [x] Chạy cùng fixture chuyển động và mẫu input ba lượt, mỗi lượt hai phút. Ghi FPS, CPU/RSS, rớt kết nối và giới hạn timing. Frame-time p50/p95 chưa đo. Đo camera được miễn theo quyết định trên; không lấy thời gian decode làm độ trễ thực tế.
- [x] Thử một prototype hiển thị và gửi input ngay trong cửa sổ Iced. Ghi đường giải mã thực dùng và chi phí copy frame; cửa sổ scrcpy bên ngoài không chứng minh đã nhúng được.
- [x] Kết luận go/no-go bằng bằng chứng, chọn/pin server-client và cập nhật ADR 0060. Chốt ngân sách FPS/latency/resource cho US-064 trước triển khai production. Không có điện thoại thì ghi chưa đo và chưa chốt engine.
- [x] Giới hạn thử nghiệm vào một điện thoại người dùng chỉ định; không đăng nhập/chơi tài khoản cá nhân tự động. Đóng prototype và dọn đúng helper/tunnel/file QA do bài thử tạo.

## Out of scope

UI production, benchmark nhiều giờ, thay transport emulator, tải cả bộ Android Studio chỉ để benchmark.

## Affected areas

Prototype có thể bỏ đi, scripts đo có thể tái sử dụng, ADR 0060 và báo cáo trong docs/reports/.

## Implementation notes

Báo cáo phải phân biệt baseline độc lập, prototype nhúng và chứng nhận sản phẩm; ghi cả trường hợp không đo được. Điện thoại thử: Samsung SM-G996B, Android 15; chưa suy rộng sang model khác.

## Verify

Lệnh dành cho lúc triển khai story, chạy từ repo root; không build/test Rust chỉ để viết US.

```bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/check-docs.py
git diff --check
```

Nếu có prototype Rust, chạy thêm `bash scripts/check.sh`. Thực hiện bài đo và native prototype trong acceptance criteria; ghi lệnh, phiên bản, kết quả và phương pháp đo vào báo cáo, không chỉ ghi cảm nhận.

## Evidence and completion audit

[Báo cáo spike](../../../reports/physical-device-spike-20261119.md) và
[metrics](../../../reports/physical-device-spike-20261119-metrics.json) lưu kết quả.

| Hạng mục | Kết luận |
| --- | --- |
| Môi trường | Samsung SM-G996B / Android 15, M2 16 GB, direct USB; phiên bản công cụ và cấu hình video đã ghi. Màn hình/refresh thực dùng và loại cáp chưa xác minh, được ghi là giới hạn. |
| Đối chứng | Ba baseline scrcpy sạch; Google viewer chạy được nhưng CLI thiếu tham số để so sánh đồng cấu hình; đã khảo sát QtScrcpyCore/license. |
| Ba lượt Iced | Mỗi collector đủ 120 mẫu, ~60 FPS decode, 55,5–55,8 FPS chọn frame; CPU 40,6–51,5% một lõi. Input theo cùng mẫu click/drag, thời điểm không đồng bộ chính xác. |
| Nhúng/input | Fixture thật hiện trong Iced; click/drag cập nhật bộ đếm trên Samsung; đã ghi chi phí copy RGBA/pipe/upload. |
| Quyết định | Chọn scrcpy v5.0.1 làm transport/server-client cho triển khai tiếp; ADR 0060 Accepted. Không đưa renderer prototype nguyên trạng vào production; US-064 có ngân sách riêng. |
| Cleanup | Đã đóng viewer/helper/tunnel, gỡ APK fixture và xóa bundle/build/tools tạm. App chính và dữ liệu cá nhân giữ nguyên. |
| Camera | Miễn theo yêu cầu người dùng; latency chưa đo, không đánh dấu pass. Không còn buổi đo camera bắt buộc của US-056. |

`bash scripts/check.sh` đã đạt sau thay đổi code cuối: workspace tests, all-target
Clippy, metrics checks, 36 Python tests, docs và shell syntax/format. Source hashes
trong metrics khớp code hiện tại; sáu collector đủ 120 mẫu, thời gian/counters tăng
đơn điệu. Lần cập nhật quyết định chỉ sửa docs; chạy check-docs và diff-check, không
build lại sau cleanup. US-057 trở đi vẫn planned; chưa commit/push/release.
