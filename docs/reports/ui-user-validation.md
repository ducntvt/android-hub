# Xác nhận UI từ người dùng — 2026-11-13

Câu hỏi: mở lại Android Hub.app rồi kiểm tra sidebar, chế độ Hai máy và panel Cấu hình có hiển thị đầy đủ, không bị cắt hoặc chồng chữ không.

Trả lời người dùng: “Hiển thị ổn”.

Phạm vi: bố cục ba phần trên. Không suy ra restore/backup, wheel, orientation, search/zoom hoặc các nút đều hoạt động. Không có screenshot tự động vì CUA native pipe vẫn lỗi. Không xác định chính xác binary đang mở, nên không dùng xác nhận này để chứng minh tính năng orientation vừa build.


## Native smoke và sửa clipping — 2026-11-16

CUA native đã hoạt động lại. Mở Android Hub.app trong workspace, các máy A/B/Android 3 đều ở trạng thái dừng. Không khởi động guest, lưu config, đổi SDK/Java hoặc chấp nhận điều khoản.

Đã quan sát bằng screenshot trong phiên CUA:
- Tìm “Android B” chỉ còn B trong sidebar; chọn kết quả đổi tiêu đề/placeholder sang B. Chuỗi không khớp hiện “Không tìm thấy thiết bị”; xóa query trả lại danh sách.
- Chuyển single/split đổi nút và nhãn; chưa kiểm tra hai frame live ở lượt này.
- Mở Dung lượng dữ liệu nhận số liệu, Đo lại và Đóng hoạt động. Chưa kiểm tra resize nhỏ hoặc lỗi filesystem qua UI.
- Panel SDK/Java và cấu hình trước sửa bị thanh cuộn che mép phải của text/input. Cả phần điều khoản cũng bị cắt.
- Sửa ui.rs: dành khoảng riêng cho thanh cuộn bằng spacing(8), width Fill cho scrollable; phần license text có width Fill. Áp dụng cho cấu hình, SDK, review, storage và removed-device panels.
- Sau build và mở lại, SDK/config có viền phải đầy đủ và text wrap trước scrollbar. Scroll cấu hình xuống cuối hiển thị đầy đủ nhóm backup/restore/remove; không kích hoạt các thao tác đó.
- Review SDK: chuyển giữa hai nội dung license, cuộn nội dung và panel riêng hoạt động; đoạn văn không bị che mép phải. Checkbox chưa chọn; nút tải disabled. Không download/accept/reset cache.
- Kết thúc ở màn hình chính, Android A được chọn, single-view, search rỗng, panel đóng; cả ba guest vẫn dừng.

Ảnh đã được quan sát trong tool output của phiên, không lưu thành file riêng. Đây là smoke ở kích thước cửa sổ hiện tại, không phải xác nhận toàn bộ UI hoặc responsive. Các story UI có gameplay/input, persistence, dữ liệu destructive hoặc standalone vẫn chưa hoàn tất.

Verification: fmt, 100 workspace tests (20 ignored), clippy -D warnings, build và codesign pass. block 0.1.6 vẫn có future-incompatibility warning.


## Isolated SDK/Java UI attempt after interruption

Prepared a separate QA bundle at /tmp/android-hub-ui-qa-20261116/Android Hub QA.app, with its own bundle ID and ANDROID_HUB_ROOT pointing to the sibling data directory. The initial shell launcher did not produce a QA window. Switched the fixture bundle to a copied Mach-O executable with LSEnvironment and verified its ad-hoc signature; no product source change.

CUA getApp timed out. Process inspection after that timeout showed only the existing workspace Android Hub process; QA registry/preferences did not exist. Finder could navigate to and select the QA bundle, but opening it did not produce a visible QA window; direct bundle-ID selection returned Invalid app. No matching Hub crash report was found. Root cause remains unconfirmed; do not classify as an SDK/Java validation failure or native test pass.

The isolated data root still has no runtime. No test path was saved into the personal Hub. SDK/Java invalid-path/save/reload smoke remains outstanding. Do not repeat the same launch indefinitely or infer that a timeout means an app is still launching.

## QA launcher fixed — 2026-11-16

Người dùng xác nhận bản QA không mở được. Log RunningBoard lúc 12:42 cho thấy LaunchServices vẫn chạy `Contents/MacOS/qa`; file đó dùng `exec /tmp/.../Android Hub QA.app/...` không quote, làm shell tách đường dẫn. Việc đổi Info.plist sang binary trước đó chưa thay entry mà LaunchServices đang dùng.

Đã sửa launcher bằng đường dẫn có quote, đặt lại CFBundleExecutable=qa, bổ sung version và ký lại bundle. CUA mở đúng bản `/tmp/android-hub-ui-qa-20261116/Android Hub QA.app` thành công, thấy standard window Android Hub; process QA PID 8837 tồn tại và registry đã được tạo dưới QA data root. Không thay dữ liệu A/B. Đây là bằng chứng launch thành công, chưa thay thế smoke toàn bộ chức năng.

Thêm `scripts/build-hub-qa-app.py` để đóng gói từ binary app hiện có vào thư mục mới, không overwrite, launcher đặt root dữ liệu riêng cạnh bundle. Hai regression tests kiểm tra dấu cách/argument/env isolation và từ chối destination đã tồn tại đều pass.

## QA native create/config smoke — 2026-11-16

Sau khi sửa launcher, quan sát empty state (không có máy), bấm + tạo hai máy QA riêng, thấy hai dòng sidebar ở trạng thái dừng và biên nhận hoàn tất đăng ký. Đổi tên máy thứ hai thành `QA Lifecycle`, bấm Lưu cấu hình, thấy tên sidebar cập nhật và thông báo đã lưu. Đường dẫn root hiển thị trong panel là `/tmp/android-hub-ui-qa-20261116/data`. Registry trên disk xác nhận tên đã lưu; registry project vẫn là Android A/B/Android 3.

Trong lúc chuẩn bị kiểm tra gỡ máy, công cụ báo UI bị thay đổi bởi người dùng; ảnh sau đó có cửa sổ lớn hơn và máy QA thứ ba. Dừng thao tác UI để tránh xung đột. Chưa xác nhận gỡ/undo, backup/restore, boot, live input hoặc cấu hình chiều ở lượt này. Các story liên quan tiếp tục in_progress cho đến khi đủ Verify và native coverage.

## QA native undo — 2026-11-16

Quan sát Android 3 (`hub-1794811158106`) đã được gỡ trong runtime QA riêng; sidebar còn hai máy, banner và trạng thái xác nhận dữ liệu được giữ. Mở Máy đã gỡ, chọn đúng ID, xem path archive, mở xác nhận Đưa máy trở lại và xác nhận. Kết quả native: Android 3 xuất hiện lại, được chọn, trạng thái dừng, panel archive đóng và banner ghi đã đưa máy trở lại. Không tự boot.

Đối chiếu disk: registry có lại ID/port 5588 và cấu hình 3072 MiB/2 CPU/720×1280/30 FPS; live .avd/config.ini và .ini tồn tại, archive có removed.json/restored.json. Registry project vẫn giữ ba ID ban đầu. Đây là smoke native undo trên máy QA mới chưa boot, không phải bằng chứng game data hoặc backup/restore. Gỡ bằng tay trước đó chỉ được quan sát kết quả, chưa tự kiểm tra đầy đủ confirm/cancel/remove-last ở UI. US-016 giữ in_progress; chưa chạy lại toàn bộ Verify trong lượt này.

## QA cancel/remove and first backup — 2026-11-16

Mở xác nhận gỡ Android 3 (`hub-1794811158106`), nội dung nêu đúng tên/ID và giữ dữ liệu trong archive; chọn Hủy: xác nhận đóng, máy vẫn trong sidebar. Không gọi gỡ ở lượt này.

Bấm Tạo bản sao lưu trước lần boot đầu: UI hiện busy rồi báo `AVD thiếu cấu hình hoặc userdata`, không báo thành công. Khởi động đúng guest QA qua Hub: thấy màn hình Android Home với Play Store; ADB chỉ đọc xác nhận AVD ID và sys.boot_completed=1. Bấm Dừng máy và xác nhận: sidebar chuyển Đã dừng. Trong lúc guest chạy, các nút backup/gỡ bị vô hiệu hóa.

Thử backup lại sau dừng: UI busy/disabled rồi báo đường dẫn thành công `.../backups/hub-1794811158106-1794811777468174000`. Đây là guest QA không đăng nhập, không chứa game data. Chưa chạy restore UI; tiếp tục từ bản sao này. Kết quả kiểm tra checksum độc lập ghi trong reports/backup/native-qa-20261116.json sau khi hoàn tất.

## QA native restore and boot — 2026-11-16

Trong root QA riêng, làm mới danh sách thấy đúng một backup Android 3 (1746 MiB), chọn bản sao, mở xác nhận. Xác nhận nêu đúng máy/path backup và việc thay dữ liệu/cấu hình, giữ dữ liệu cũ. Bấm Khôi phục dữ liệu: UI chuyển busy, khóa các thao tác dữ liệu; kết thúc có thông báo đã khôi phục và đường dẫn previous_data, lịch sử ghi thành công.

Đối chiếu độc lập trước boot: đủ 22 file trong live AVD khớp SHA-256 và kích thước manifest, không có file thừa; receipt state=committed và thư mục previous_data còn. Bằng chứng: `restore/native-qa-20261116.json`.

Khởi động bằng Hub, quan sát Android Home có Play Store; ADB chỉ đọc xác nhận sys.boot_completed=1. Dừng bằng Hub và xác nhận, sidebar về Đã dừng. Không đăng nhập hoặc thử game trong guest này; không suy ra dữ liệu game đã được kiểm chứng. Không thao tác guest A/B. Chưa chạy lại toàn bộ Verify story; giữ US-009 in_progress.

## Restore cancel and settings hint — 2026-11-16

Thu cửa sổ QA về 2360×1704 bằng native zoom. Làm mới/chọn backup, mở xác nhận và bấm Hủy: xác nhận đóng, máy vẫn dừng, không tạo receipt mới hoặc pending journal (chỉ có một receipt của lần restore thành công trước).

Phát hiện nhãn “Dừng máy để khôi phục bản sao” hiển thị ngay cả khi máy đã dừng; nhãn cấu hình cũng dùng lý do này khi busy/stale. Đã sửa view để thông báo theo busy/stale/running, không đổi điều kiện cho phép thao tác. Bản QA đang mở vẫn dùng binary cũ; cần kiểm tra lại bản build mới.

Kiểm chứng sửa nhãn: fmt pass, 22 desktop tests pass, build app và codesign verification pass. Bản main app đã được đóng gói; QA đang mở vẫn là bản cũ.

## Reload settings-hint build — 2026-11-16

Đóng QA qua Cmd+Q, sao chép binary build mới (so sánh byte trước khi ký lại), ký và verify bundle. Mở lại thành công bằng CUA. Chọn Android 3 đang dừng, mở cấu hình và cuộn xuống restore: lời nhắc dừng máy không còn xuất hiện. Dữ liệu QA/biên nhận trước vẫn được nạp. Chưa quan sát stale hoặc busy lâu ở build này.

## Zoom and final workspace acceptance — 2026-11-16

Native QA selected Android 3 in the isolated QA root and started it through Hub. Embedded Android Home rendered successfully. Minus twice changed 100% → 80%, visibly reducing content width from approximately 500 to 400 physical pixels. Plus three times changed 80% → 110%, visibly increasing it to approximately 550 pixels. Clicking the Phone launcher icon at 110% opened the Phone app in the embedded canvas; no number was entered or called. Clicking the percentage reset to 100% and original canvas width. A drag attempted at 80% did not open the app drawer, so that attempt is not counted as successful drag evidence. Earlier single/split drag evidence remains separately documented.

Stopped Android 3 with Hub's confirmation and observed all three QA devices stopped, placeholder restored, and status “Đã dừng máy”. No A/B operations, configuration saves or source edits in this follow-up.

### US-005 acceptance audit

- Search, unmatched query, clear and selection: native smoke above.
- Create and stopped illustration/start: native create/config smoke above and current boot/stop.
- Live embedded interaction and single/split: [scrolled canvas native report](input/scrolled-cursor-native-20261116.md).
- Zoom minus/plus/reset and scaled tap: current native follow-up.
- Settings stopped-only, save, stop confirmation and error feedback: native create/config, backup error/disabled controls, restore and settings-hint reload above; orientation native tests also exercise stopped settings.
- This UI work did not replace the engine or change frame timers; specialized input interruption and performance acceptance remain separate stories.

Verification reused unchanged production-source results: fmt passed in the preceding data-root follow-up; [workspace tests: 111 passed, 20 opt-in ignored](removal/label-tests-20261116.log), [clippy -D warnings](removal/label-clippy-20261116.log), [normal build and codesign](data-root/final-build-20261116.log). No production source changed between those checks and this UI follow-up. US-005's bounded workspace contract is now verified; this is not complete product acceptance.
