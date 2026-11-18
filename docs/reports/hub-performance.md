# Android Hub — cleanup / performance, 2026-11-13

## Thay đổi

- Bỏ `FrameTick` 33 ms toàn cục. Session báo frame/error qua Notify; Iced chỉ phát message khi có dữ liệu mới, tối đa khoảng 30 lần/giây/máy. Slot frame và permit đều giới hạn một, không xếp hàng ảnh.
- Chuyển chọn session/clone profile từ đường xử lý mỗi frame sang snapshot, selection và split changes. Message có PID để bỏ thông báo từ engine đã đổi.
- Bỏ trường `landscape` lưu trùng trong transport và UI; suy ra từ kích thước frame.
- Dùng array thay Vec cho các lựa chọn RAM/CPU/FPS/resolution cố định trong view.
- Lọc nhanh process không liên quan trước khi cấp phát danh sách argument; vẫn kiểm tra chính xác executable, AVD và port.
- Không nuốt `Disconnect` trong lúc bỏ input cũ sau lỗi stream, tránh reconnect không cần thiết khi session đang đóng.

## Kiểm chứng

- `cargo fmt --all -- --check`: pass.
- `cargo test --workspace --locked`: 7 tests pass; bao gồm tọa độ, gesture và notification đến trước subscription, gộp 100 thông báo, không phát message khi idle, nhận thông báo tiếp theo.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: pass.
- `bash scripts/build-hub-app.sh`: pass; bundle cập nhật, ad-hoc signing. `codesign --verify --deep --strict` pass. SHA-256 bundle khác target/debug do bước ký binary.
- Logs: [tests](performance/tests.log), [clippy](performance/clippy.log), [build](performance/build.log).
- Cargo vẫn cảnh báo future compatibility của dependency gián tiếp `block 0.1.6`; không phải regression mới. Không nâng dependency trong lượt tối ưu này.
- Đã đọc dependencies trực tiếp; chưa thấy dependency có thể bỏ an toàn. `prost` dùng trong code sinh từ protobuf. Công cụ cargo-machete không được cài. P0 nằm ngoài workspace; giữ các thử nghiệm làm bằng chứng engine, không xóa tùy tiện.

## Số đo và giới hạn

Hub cũ PID 73367, A/B đều dừng, không thao tác trong các khoảng đo. Dùng `ps -p PID -o time=,rss=`; CPU = delta CPU seconds / elapsed wall seconds × 100, 100% là một core.

| Lần | CPU time đầu/cuối | Wall | CPU trung bình | RSS đầu/cuối KiB |
| --- | --- | --- | --- | --- |
| 1 | 7:25.10 → 7:26.32 | 15.017 s | 8.12% | 34224 → 34224 |
| 2 | 7:32.40 → 7:33.46 | 15.011 s | 7.06% | 38784 → 38016 |

Chưa có số đo sau thay đổi hoặc benchmark game. Công cụ CUA sau lần screenshot baseline liên tục báo `Sky Computer Use native pipe startup failed`; thử lại và reset vẫn lỗi, inventory trả về lỗi native. Lệnh đóng Hub qua CUA không thành công; kiểm tra process vẫn thấy PID cũ. Vì vậy chưa kiểm tra UI bản mới/single-split/input thực tế, không công bố phần trăm cải thiện. Cần đóng/mở lại Hub để nạp bundle mới và hoàn tất kiểm chứng này. US-004 giữ in_progress.

Dữ liệu, registry, cấu hình và engine A/B không bị sửa trong lượt này. Lỗi native launcher B cũ vẫn nằm ngoài phạm vi. Không có Git repository ở thư mục hiện tại; không tạo commit.


## Bổ sung phép đo sau thay đổi

[Trace idle](performance/hub-idle-20261116-100806.json): Hub hiện tại PID 65453, ba profile đều dừng theo UI ở lượt kiểm tra trước. Mỗi lần lấy mẫu xác nhận không có process qemu-system-aarch64. Không có thao tác UI chủ động trong khoảng đo. macOS 27.0.1 (26A434), ARM64; không khảo sát tải nền/nhiệt/nguồn điện trong lượt này.

- Wall time: 60.251 giây, 5 mẫu cách khoảng 15 giây.
- CPU time: 266.16 → 266.56 giây.
- CPU trung bình từ delta CPU time / wall time: **0.664% của một core**.
- RSS đầu/cuối: 46528 / 42640 KiB. Đây là phiên đã chạy lâu, không phải cold start hoặc tổng GPU/helper.

Kết quả thấp hơn hai mẫu lịch sử 7–8%, nhưng khác ngày, thời lượng, phiên process và tải nền; không công bố phần trăm cải thiện nhân quả. Source hiện chỉ tạo frame subscription khi có display session, notification coalescing đã có regression test, còn Tick trạng thái 3 giây.

Đã bổ sung bằng chứng CPU idle còn thiếu. US-004 vẫn in_progress vì frame/input live và đổi single/split sau thay đổi chưa được smoke đầy đủ trên guest thử. Không biến phép đo idle thành benchmark game hoặc hoàn tất goal.

## Final US-004 acceptance audit — 2026-11-16

The previous pending live-smoke note is superseded by [native single/split scrolled input](input/scrolled-cursor-native-20261116.md) and [workspace/zoom acceptance](ui-user-validation.md#zoom-and-final-workspace-acceptance--2026-11-16). These observations include selecting QA guests, live Android frames, Home/drag on individual canvases, single/split routing and scaled tap. Personal A/B data were not changed.

Current source review confirms `frame_updates` waits 33 ms then a Notify permit; subscriptions exist only for display sessions. The sole remaining periodic idle subscription is the 3-second state refresh. Transport stores one replaceable latest frame, and `take_frame` consumes it. The notification regression sends 100 notifications before subscription, observes one event and then an idle timeout, followed by delivery of a fresh notification. This proves bounded notification behavior, not a measured game FPS claim.

Re-read the raw five-sample idle trace: same PID/start time across samples, CPU 266.16 → 266.56 seconds, monotonic interval 60.2514009997 seconds, yielding 0.66388498% of one core. Retain the earlier stated environmental and historical-comparison limitations. No new measurement or causal speedup percentage is claimed.

All required verification has passed on current production source: formatting; [113 workspace tests, 20 opt-in ignored](input/desktop-epoch-tests-20261116.log); [clippy -D warnings](input/desktop-epoch-clippy-20261116.log); [app build](input/desktop-epoch-build-20261116.log) and strict codesign. Only a regression test and formatting were added after that app build; production behavior is identical. The frame-notification implementation has not changed since the native evidence.

US-004 is implemented within its explicit scope. Two-game benchmark, active-hold interruption and sleep/wake remain separate open requirements.
