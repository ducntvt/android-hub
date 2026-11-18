# Android Hub — UX roadmap

Ngày: 2026-11-17. Người dùng yêu cầu hai agent phân tích review UI/UX, đưa thành US và bắt đầu triển khai. Đây là kế hoạch công việc có phạm vi, không phải tuyên bố toàn bộ review đã hoàn tất hoặc thay thế backlog engine/SDK/input cũ.

## Mục tiêu

Ưu tiên dùng một/hai máy Android thuận tiện, hiểu đúng máy đang điều khiển và nhận phản hồi rõ. Tiếp tục Rust/Iced và engine hiện tại. Bảo toàn game/tài khoản/runtime; đóng QA do mỗi lượt thử mở và dọn disposable build/test artifacts. Không tự mở thêm ca stress dài để tăng số story hoàn tất.

## Mapping review sang story

| Mục review | Story chịu trách nhiệm | Phạm vi và giới hạn |
|---|---|---|
| 1. Bố cục, diện tích Android, zoom | [US-033 — Fit workspace/sidebar](stories/US-033-adaptive-workspace.md) | Vừa khung và sidebar; focus mode/persistence chỉ hoàn tất khi nằm trong contract và có bằng chứng |
| 2. Hai máy, máy nhận input | [US-034 — Explicit pair/focus](stories/US-034-explicit-device-pair.md) | Chọn đúng cặp máy, biểu thị focus và đích điều khiển |
| 3. Sidebar | US-033, US-040 | Bố cục/empty state ở 033; cài đặt Hub, ngôn ngữ và typography ở 040 |
| 4. Toolbar và nút | US-033, US-034, US-040 | Bố cục/đích thao tác trước, icon/tooltip/phím tắt sau |
| 5. Màu, chữ, icon | [US-040](stories/US-040-desktop-visual-language-accessibility.md) | Theme/token/control Hub, không làm hiệu ứng nặng |
| 6. Khởi động và chờ | [US-035](stories/US-035-device-lifecycle-feedback.md) | Readiness có bằng chứng; không đồng thời mở khóa mọi mutation backend |
| 7. Thông báo/xác nhận | [US-036](stories/US-036-feedback-and-confirmations.md) | Overlay, toast, lịch sử riêng và giữ guards |
| 8. Cấu hình | [US-037](stories/US-037-device-settings-drafts.md) | Dirty/save/cancel/conflict và nhóm fields; preset tự tối ưu host để sau |
| 9. Tạo máy | [US-038](stories/US-038-create-device-form.md) | Form ngắn, một image hỗ trợ, recovery không đổi |
| 10. APK/clipboard | [US-039](stories/US-039-apk-and-clipboard-actions.md) | Picker/đích rõ/copy semantics; tọa độ drop phụ thuộc capability thực |
| 11. Ngôn ngữ/nội dung | US-040 và mọi story có label mới | VI/EN, thuật ngữ nhất quán; live Android version nằm cùng readiness 035 nếu được bổ sung, không gọi configured version là guest probe |
| 12. Chẩn đoán mạng | [US-041](stories/US-041-on-demand-connectivity-check.md) | On-demand/read-only, không tự sửa cấu hình |
| 13. Sao lưu/lưu trữ/dung lượng | [US-042](stories/US-042-device-data-management-presentation.md) | Presentation, không thêm xóa dữ liệu |
| 14. Thoát/phím tắt/chi tiết | US-035, US-040, US-033/034 | Thoát từng máy; keyboard/a11y; layout/session preference theo contract workspace |

US-033–042 đã implemented theo contract từng story. Gap native accessibility từ audit US-040 nằm ở [US-043](stories/US-043-native-hub-accessibility.md), ngoài đợt US-033–042; không tuyên bố VoiceOver đã hoạt động. File story và Evidence là nguồn trạng thái chính xác; bảng này không tự đánh dấu acceptance đạt.

## Thứ tự đề xuất

1. **US-033 + US-034**: diện tích màn hình, sidebar, cặp máy và focus. Chốt semantics selected/settings/input trước để các luồng sau dùng cùng đích.
2. **US-035 + US-036**: state/readiness, đóng Hub, feedback và confirmations không làm co stage. Hai story tách contract backend với presentation.
3. **US-037 → US-038 → US-039**: draft không mất, tạo máy rõ ràng, APK/clipboard đúng đích. Dùng overlay từ 036 và target selection từ 034.
4. **US-040**: polish token/theme/VI/EN/keyboard/a11y sau khi luồng ổn định. Label/contrast cần thiết vẫn làm cùng từng story, không để UI mới thiếu bản dịch chờ tới đây.
5. **US-041 + US-042**: chẩn đoán theo yêu cầu và quản lý dữ liệu dễ hiểu. Không buộc chạy hết trước khi giao bản cải thiện workspace cho người dùng dùng.

## Các phụ thuộc và quyết định cần giữ rõ

- Runtime hiện chỉ có PID/CPU/RSS; readiness/progress mới cần contract + ADR, timeout và bảo vệ stale response. Frame xuất hiện không đồng nghĩa Android boot xong.
- Các mutation hiện serialize qua busy và Hub lock. Không bỏ guard để tạo cảm giác nhanh; concurrent mutations cần story/ADR riêng. Việc chọn/xem máy khác có thể cải thiện mà vẫn serialize ghi dữ liệu.
- API create hiện không nhận form và trả Snapshot, nên US-038 phải trả ID đã tạo rõ ràng; không dựa vào thứ tự danh sách.
- Đổi nhãn lưu trữ không đổi archive path hay schema. Không dùng cleanup giao diện để xóa AVD/backups.
- Iced/file-drop/VoiceOver cần kiểm chứng capability; không cam kết drop đúng card hoặc accessibility chỉ vì đã vẽ icon/tooltip.
- Nút chuột, bàn phím, clipboard và confirmation phải chia sẻ quy tắc đích; modal phải release input và không để sự kiện lọt vào guest phía sau.
- Theme/readiness/network diagnostics không thêm timer dày đặc hoặc helper chạy nền vô hạn.

## Những đề xuất được giữ cho giai đoạn sau

Preset hiệu năng theo phần cứng, keymap/IME Unicode, native menu đầy đủ, focus mode nâng cao, lưu mọi bố cục qua phiên, live guest-version probe, xóa vĩnh viễn và dọn dữ liệu cần contract riêng hoặc mở rộng story được ghi rõ trước khi làm. Mapping review không có nghĩa các mục này được âm thầm thêm vào đợt triển khai đầu.

## Kiểm chứng và bàn giao

Mỗi US chỉ implemented sau các lệnh Verify và smoke trong chính story. Dùng fixture/unit test cho race/identity/dirty transitions; dùng QA riêng cho UI và guest operations thật khi cần. Không dừng guest cá nhân hoặc thử khôi phục tài khoản thật để đổi layout. Ghi giới hạn chưa kiểm chứng thay vì lặp QA vô hạn; đóng app/guest do ca thử tạo, dọn target/temp cuối lượt, giữ app chính và dữ liệu thực. Không tự commit.

Latest workspace evidence: [native follow-up and modal focus fix](reports/ux-workspace-followup-20261117.md). Main app rebuilt; 189 Rust + 26 Python tests passed. US-033–035 retain specific native acceptance gaps; the later feedback audit below closes US-036.

US-036 completed: [feedback acceptance audit](reports/ux-feedback-followup-20261117.md), including actual clipboard, toast expiry and startup recovery. Empty-slot VI/EN wording corrected.

## Final acceptance — 2026-11-17

US-033–042: 10/10 implemented. [Final workspace/lifecycle audit](reports/ux-interaction-followup-20261117.md)
closes the remaining stories with live resize, user-confirmed Fit/110% dragging,
release-order evidence, display recovery and partial shutdown results. Earlier
progress paragraphs are historical; story statuses and final audits are current.

Main app rebuilt and codesign verified; 190 Rust + 26 Python tests, fmt, clippy
and shell checks passed (20 existing integration tests remain ignored). Native
held-pointer slot switching and an externally interrupted Retry click are not
claimed; corresponding invariants have code/regression coverage. US-040's
explicit framework-gap clause leaves VoiceOver in US-043, outside this batch.
All owned QA sessions and disposable build/runtime artifacts were cleaned up.

Bổ sung ngày 2026-11-18: người dùng đã yêu cầu xóa vĩnh viễn máy đã lưu trữ. Contract và cách dùng hiện tại nằm trong [README](../README.md#lưu-trữ-khôi-phục-và-xóa-thiết-bị); đây là mở rộng riêng sau US-042.

Luồng chính tiếp tục được đơn giản hóa: ⋯ trên từng thiết bị → Xóa máy, có sao lưu tùy chọn và một xác nhận xóa. Archive cũ chuyển vào Cài đặt Hub; không còn tạo archive từ UI chính.
