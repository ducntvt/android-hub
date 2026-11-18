# Đối chiếu MVP với source và evidence — cập nhật 2026-11-16

Đây là đối chiếu lịch sử tại ngày ghi trong tiêu đề; các bổ sung sau đó nằm trong [kế hoạch hiện tại](../PLAN.md), [README](../../README.md) và [story index](../stories/README.md). Đặc biệt, xóa máy trực tiếp đã được bổ sung. Số lượng unit tests không thay thế bằng chứng gameplay/UI.

| Yêu cầu | Bằng chứng hiện có | Phần chưa đạt / bằng chứng cần bổ sung |
| --- | --- | --- |
| Tạo, đổi tên, start/stop, danh sách | Backend lib.rs + US-002; B mở từ Hub do người dùng xác nhận | Restart B lặp lại chưa kiểm chứng; US-019 backend pass gồm SIGKILL worker 6 mốc; UI xem biên nhận US-020 đã nối, chưa smoke; chưa có dọn file tạo dở |
| Xóa máy | US-016 có gỡ/undo với archive, journal/fault tests và guest boot; UI đã nối | US-016 implemented: native cancel/remove-last/undo, recovery cả hai nhánh và retry đạt; chưa xóa vĩnh viễn/giải phóng dung lượng |
| Hai Android độc lập | US-003, embedded-display-validation; người dùng thấy hai app chạy | Soak hai game đồng thời 60 phút chưa có |
| Android nằm trong Hub | US-003 đã có native smoke ban đầu; người dùng xác nhận sidebar/Hai máy/Cấu hình hiển thị ổn | Search/zoom/stopped state và các tính năng mới chưa đủ smoke; xem ui-user-validation.md |
| Play Store, Top Heroes, Hunt Royale | P0 có cài game; người dùng báo chơi được | Chưa có hồ sơ riêng chứng minh cả hai game với audio/network, đồng thời trên A/B |
| Cài APK | US-007 implemented: native drop/install/cancel, đúng guest và peer không có package; [evidence](apk/native-baseline-20261116.json) | Result panel preview dùng dữ liệu mẫu; peer dừng trong lúc cài; không hỗ trợ split APK/update |
| RAM/CPU/resolution/FPS/chiều | Profile/settings/launcher; US-014 UI lưu ngang và boot1280×720, tap/drag/wheel pass; [evidence](orientation/native-qa-20261116.md) | US-014 đạt: đổi resolution giữ ngang, đổi lại dọc480×854/boot/input qua UI; không giả lập cảm biến hoặc ép game xoay |
| Tap/drag/Home/Back/ASCII | Native kéo trong viewport bị cuộn, single/split pass; [evidence](input/scrolled-cursor-native-20261116.md); generation guard có regression | Native ngắt kết nối/đổi máy giữa hold còn thiếu |
| Cuộn | US-011: native đúng vùng/hai máy/outside; người dùng xác nhận trackpad ngang; [evidence](input/framework-scroll-native-20261116.md) | Forced disconnect khi input đang chạy và các tùy chọn natural scroll còn thiếu |
| Văn bản Unicode | US-015: native dán tiếng Việt/emoji, clipboard hai guest độc lập; [evidence](clipboard/native-qa-20261116.md); generation guard có regression | US-015 implemented: native callback trì hoãn có kiểm soát bị bỏ sau đổi selection; keyboard trực tiếp vẫn ASCII |
| Key mapping tap/hold/WASD nếu game cần | Chưa có keymap model/editor | Đã hỏi người dùng về nhu cầu tap/WASD, chưa có phản hồi; không coi bàn phím ASCII là keymap |
| Trạng thái/lỗi/tài nguyên | ps PID/CPU/RSS, notice, stale-poll guard, SDK preflight | Không bao gồm tổng GPU/helper; không suy ra FPS/game readiness từ PID |
| Persistence/backup/restore | US-008 implemented với native success/error và audit SHA-256 mới; US-009 implemented: native restore/cancel/boot QA và startup recovery, guest API restore/reboot 134.12 giây đạt; 22 file khớp SHA-256; [receipt](restore/native-qa-20261116.json) | Native startup rollback history đạt trên fixture riêng ([evidence](restore/startup-native-20261116.json)); dữ liệu game và khả năng chịu mất điện thực chưa kiểm chứng |
| Thiếu dung lượng | US-010 preflight backup/restore + sparse fixture | US-021 đã có admission floor 12 GiB khi tạo máy; US-025 có archive remainder + 6 GiB admission và kiểm tra expanded ZIP; chưa có actual candidate install |
| App Finder/setup | Build/sign app; SETUP hướng dẫn SDK hiện có | US-017 implemented: native standalone khi source mất đạt; migration ngoài phạm vi; US-024 implemented: empty/populated native invalid/valid/save/reopen đạt; US-025 đã nối download/resume/checksum/assembly/catalog/review/progress/cancel; candidate thật/native setup/boot chưa kiểm chứng |
| Quản lý image/cache/log | US-018 đo dung lượng theo yêu cầu; US-023 xoay log trước start, giữ ba archive | Cache có quarantine/retry; chưa có xóa vĩnh viễn cache/dữ liệu cũ; không giới hạn log trong phiên dài |
| Nhẹ/mượt, benchmark | US-004 implemented: bounded frame notification và native live/input/split đạt; có mẫu idle sau thay đổi khoảng 0,66% một core/60 giây (reports/hub-performance.md) | Cần workload một/hai game, minimized, crash/sleep-wake; mẫu idle ngắn không chứng minh hiệu năng game |
| Gate P0/engine decision | US-001, reports P0 lịch sử | `scripts/verify-p0.sh` đã triển khai và chạy trả exit 1 vì thiếu measurements.json; benchmark và ADR chốt engine chưa hoàn tất |

## Phần đã bổ sung và giới hạn bằng chứng

- US-025: review hai bộ điều khoản, consent từng lượt, Java preflight, download/resume/hủy/checksum, extraction, kiểm tra package, staging/publish, catalog/select và cache quarantine/recovery đã có code. Bài thử curl thực/SIGKILL, assembly fixture và avdmanager thật trên SDK thử đã qua; đây chưa phải candidate archive download + boot end-to-end. Xem US-025 và ADR 0023–0034.
- US-026: Java home được kiểm tra/lưu/dùng qua backend, test với JBR/SDK cài sẵn đạt; native invalid/valid/save/reopen đạt; US-026 implemented, [báo cáo](sdk-java-native-20261116.md).
- US-022 đã implemented: [hai engine thử thật](metrics-collection/real-idle-dual/collection.json) có 62 giây overlap, không chơi game, không có Hub display stream. Không dùng số đo này làm steady-state idle baseline hoặc bằng chứng P0/P4.
- [Gate audit mới](p0-completion-audit.txt): cả measurements và acceptance exit 1 do thiếu measurements.json. Không tạo hồ sơ thay thế bằng dữ liệu tổng hợp hoặc xác nhận gameplay ngoài phạm vi người dùng đã báo.

## Ưu tiên tiếp theo — điều chỉnh sau phản hồi người dùng

Ưu tiên bàn giao và dùng bản chính, sửa lỗi thực tế được báo. Không tự tiếp tục chuỗi QA gây lỗi/bản thử; người dùng đã phản hồi thời gian kiểm thử kéo dài và chệch mục tiêu cá nhân. Xem README để mở đúng app. Những mục còn thiếu dưới đây được giữ minh bạch, không phải điều kiện cản dùng app trên máy hiện tại:

- US-011/012: stress cuộn/input; nhiều ca native đã đạt, xem ghi nhận cuối mỗi story thay vì các ghi chú pending lịch sử.
- US-025: candidate SDK thật/native setup/boot; chưa có xác nhận điều khoản cho lượt cài.
- P0: benchmark hai game 60 phút và kết luận engine; người dùng đã hoãn.
- Key mapping: chỉ thuộc phạm vi nếu game cần; chưa có yêu cầu cụ thể.

Không coi số story hoặc số unit tests là phần trăm hoàn thiện hay bằng chứng về FPS/game. Chưa đánh dấu toàn bộ kế hoạch hoàn tất.

## Phạm vi đã thay đổi có căn cứ

Màn hình nhúng thay cửa sổ emulator riêng theo yêu cầu Device Hub và ADR 0005. Đóng Hub giữ engine chạy theo hành vi hiện tại được mô tả từ US-003; PLAN đã được chỉnh để không còn mâu thuẫn với yêu cầu “dừng khi Quit” cũ. Không bỏ các yêu cầu xóa máy/setup/benchmark chỉ vì chưa triển khai.
