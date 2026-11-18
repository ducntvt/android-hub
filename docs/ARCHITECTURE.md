# Architecture

Theo [ADR 0051](decisions/0051-data-catalog-and-usage-presentation.md), catalog backup chỉ cung cấp metadata (kèm tên máy nguồn), không tuyên bố checksum/restore đã đạt. Dung lượng trả category có kiểu và thời điểm đo, nhận diện máy đang quản lý từ registry; archive, dữ liệu trước restore và staging được tách. UI phân biệt lỗi đọc với danh sách rỗng. Luồng chính hiện có xóa máy trực tiếp với xác nhận và tùy chọn sao lưu trước; archive cũ vẫn có thể khôi phục hoặc xóa trong Cài đặt Hub. Backend giữ journal/recovery và không tự tiếp tục xóa khi khởi động.

## Stack hiện tại

Rust workspace với Iced 0.13.1 (wgpu/Metal) cho Android Hub trên macOS Apple Silicon. Android Emulator dùng chung cho Google Play ARM64 Android 12–16 (API 31/33/34/35/36); game trước đây đã chạy trên Android 16; không có dependency hoặc checkout AVO trong source đang dùng. Nghiên cứu engine trước đây chỉ còn trong báo cáo lịch sử. Không có HTTP backend riêng.

## Ranh giới

- `crates/domain`: Profile, Runtime và kiểm tra dữ liệu; không I/O hay UI.
- `crates/infrastructure`: registry JSON, khóa thao tác, SDK/ADB, quản lý process và launcher macOS; phụ thuộc domain.
- `crates/desktop`: Iced state/update/view; gọi Backend qua tác vụ nền. Không chạy lệnh shell từ widget.
- `tools/metrics`: sampler hiệu năng riêng, không thuộc workspace Cargo app. Dữ liệu nằm ngoài source theo ADR 0054.

Đây là phạm vi manager theo [ADR 0004](decisions/0004-rust-device-hub-manager.md). Các lớp application/interface riêng chưa cần tách trong bản đầu. Các hướng engine lâu dài xem [PLAN](PLAN.md).

## Dữ liệu và vòng đời

Registry `~/Library/Application Support/Android Hub/hub-profiles.json` lưu cấu hình mong muốn, kiểm tra ID/port duy nhất và ghi qua rename. Schema 1 bọc `profiles` và lưu image identity bất biến theo [ADR 0056](decisions/0056-immutable-android-image-per-device.md). Registry array cũ được sao lưu trước migration metadata; máy chưa xác định được image giữ trạng thái chưa xác định, không tự gán Android 16. `hub.lock` khóa các thao tác thay đổi giữa những process Hub. Mỗi AVD có dữ liệu riêng trong `emulator-avds`; không sao chép tài khoản.

Chỉ sửa cấu hình khi engine dừng và port rảnh. Mở máy truyền RAM/CPU/FPS và kích thước framebuffer qua các flags SDK rồi spawn engine headless làm process con; không đọc/ghi lại config.ini của máy cũ. Tạo máy mới vẫn chuẩn hóa config.ini với đọc có timeout. Dừng máy kiểm tra tên AVD từ ADB và PID lần nữa trước `emu kill`; không cưỡng bức kill. Theo ADR 0039, đóng cửa sổ hoặc Quit dừng các engine đã đăng ký rồi mới thoát; chờ tác vụ ghi hiện tại và giữ UI mở nếu dừng thất bại. Desktop chặn AppKit terminate: để Cmd+Q/menu/Dock đi qua cùng luồng async; không thay delegate Winit. Force Quit/crash nằm ngoài bảo đảm này.

## Image Android theo phiên bản

Theo [ADR 0056](decisions/0056-immutable-android-image-per-device.md), catalog cố định
Android 12–16 và mỗi máy giữ package/revision/fingerprint đã xác minh. Tải image
là plan riêng trên SDK dùng chung; consent gắn với nội dung license của plan.
Creation recheck identity dưới lock trước khi tạo AVD. Không nâng/hạ image tại chỗ.
Gỡ image chỉ dành cho SDK do Hub quản lý, kiểm tra registry/AVD/backup/archive và
journal; SDK bên ngoài chỉ đọc. Lock theo thứ tự SDK-image rồi Hub, publish/remove
có journal và kiểm tra lại identity. Bảng kết quả từng phiên bản và giới hạn xem
[báo cáo nghiệm thu](reports/android-versions-qualification-20261118.md).

## Quan sát và giới hạn

US-041 đã triển khai theo [ADR 0052](decisions/0052-guest-connectivity-probe.md): kiểm tra kết nối theo yêu cầu dùng helper Java trong guest, trả mã mạng/DNS/HTTPS có giới hạn thời gian. Subscription riêng gắn máy/PID/boot/epoch, hủy khi đóng cấu hình hoặc đổi đích; không dùng lock mutation hay hàng đợi input. Không đổi cấu hình mạng, không suy kết nối mọi ứng dụng từ một endpoint. Native guest smoke đã trả Interface/DNS/HTTPS Passed; giới hạn kiểm chứng được ghi trong report US-041.

Refresh mỗi 3 giây; CPU là số đo process từ ps (100% = một core), RSS chưa gồm GPU/helper. UI phân biệt engine xuất hiện với Android boot xong. Lỗi được hiển thị trong Hub, log engine nằm ở runtime/<id>.log. Không thu thập mật khẩu Google hoặc nội dung game.

Màn hình Android nhúng theo ADR 0005: infrastructure/display dùng tonic gRPC localhost có token, stream RGBA giữ frame mới nhất, input cảm ứng có hàng đợi giới hạn. Desktop dùng canvas wgpu, tọa độ từ event và focus theo thiết bị; hủy session khi không hiển thị. Engine mở -no-window, cổng gRPC = ADB console port + 3000. Đã có gỡ/undo giữ dữ liệu trong archive và luồng chuẩn bị SDK với consent theo lượt; native setup trên SDK candidate còn chờ kiểm chứng. Đã có xóa máy trực tiếp và xóa archive với xác nhận; chưa có clone, keymap hay auto-update. Benchmark hai game và lựa chọn engine dài hạn vẫn thuộc P0 chưa hoàn tất.

Theo ADR 0006, UI nhận thông báo frame/error thay vì poll 33 ms; mỗi session giữ một thông báo chờ và một frame mới nhất. Nhịp nhận thông báo được giới hạn theo FPS đã lưu (15/30/60/90/120); subscription gồm ID, PID và FPS để thay đổi đúng session, không chạy theo draft chưa lưu. Đồng bộ session khi trạng thái/lựa chọn đổi.

Theo ADR 0007, cài APK yêu cầu file drop + xác nhận, cố định ID/PID/serial và kiểm tra boot/AVD trước lệnh install có timeout. Backend tạo sẵn AVD_HOME trước avdmanager để guest mới luôn được đăng ký đúng nơi.

Backup offline theo ADR 0008: payload AVD + manifest SHA-256, source/backup được kiểm tra trước publish bằng rename; bản sao mới có thư mục quyền 0700. Backend restore/rollback đã triển khai theo ADR 0010; UI restore đã qua smoke native picker/xác nhận/restore/boot/cancel trên guest QA riêng; startup recovery UI còn chờ kiểm chứng.

Manifest backup mới dùng schema 2 (ADR 0009), bổ sung fingerprint file image SDK và đường dẫn canonical SDK/AVD. V1 vẫn kiểm tra payload được nhưng không qua gate restore tự động. Boot drill đã đạt trên guest thử; backend transaction/journal/rollback cũng đã qua fault-injection và guest smoke; UI restore đã qua smoke native trên guest QA riêng.

Restore theo ADR 0010: lock Hub + stopped gate, stage/verify/sync, journal pending trước rename directory/registry, marker committed trước archive receipt. Lock thao tác và initialize tự recovery journal khi máy dừng; stop dùng khóa thô để có thể dừng guest mở ngoài Hub đang chặn recovery. Dữ liệu cũ giữ trong `.previous-*`; không tự xóa sau commit. Recovery kiểm tra inode/device và profile, từ chối ghi đè thay đổi ngoài Hub. UI đọc ba biên nhận gần nhất sau initialize/restore, hiển thị lịch sử committed/rollback. R3 đã qua smoke native restore/cancel; startup recovery banner còn chờ kiểm chứng.

Theo ADR 0011, backup/restore kiểm tra dung lượng khả dụng filesystem đích trước copy. Ước lượng dùng kích thước logic làm tròn block + 256 MiB dự phòng, không giả định APFS clone/sparse. Lỗi báo lượng cần/còn; không tự xóa dữ liệu. Preflight không giữ riêng dung lượng, nên lỗi copy vẫn qua cleanup/rollback.

Input queue đóng gate trong thời gian reconnect; ready chỉ sau handshake stream/release. Gate và drain đồng bộ; pending release giữ generation để không phát sang phiên mới (ADR 0013 bổ sung). Clipboard theo click dùng cùng gate.

Tạo máy theo ADR 0019: journal preparing trước SDK, ready sau configure và fingerprint metadata. Lock/initialize phục hồi đăng ký ready khi registry/data còn khớp và guest dừng. Preparing giữ nguyên file SDK, chuyển intent thành biên nhận incomplete; chưa có UI dọn dữ liệu tạo dở. Không tuyên bố bảo đảm payload khi host mất điện.

Tạo AVD mới kiểm tra available space trên filesystem AVD trước journal/SDK (ADR 0020): tối thiểu 12 GiB cho profile hiện tại. Đây là admission floor, không reserve disk hoặc bảo đảm dung lượng cho game về sau.

Theo ADR 0036, wheel dùng helper Java DEX nhỏ qua một kênh adb shell/app_process
cho mỗi display session; ACTION_SCROLL mang tọa độ guest và cả hai trục. Touch,
key và clipboard vẫn qua gRPC. Input xử lý tuần tự trong future riêng với nhận frame;
setup/ack có timeout và không replay sau reconnect. Kênh cố định transport ID,
boot ID và kiểm tra engine PID; helper không mở cổng mạng, không cần root.
Cache DEX publish atomic, upload staging riêng từng lần. Native QA đã đạt cuộn dọc,
đúng vùng/hai máy/outside, touch sau wheel và teardown/reconnect; người dùng xác nhận
trackpad ngang đổi bộ đếm H. Giới hạn kiểm chứng còn lại xem US-011 và ADR 0036.

## Tổ chức source

Theo [ADR 0053](decisions/0053-rust-module-conventions.md), nối tiếp ADR 0040, desktop có entry point nhỏ và root `app.rs`; các module con tách state/event, bootstrap/subscriptions, SDK handler, update, UI, screen và tests. Infrastructure tách snapshot parsing và shutdown aggregation khỏi facade; public exports không đổi. SDK nằm dưới `infrastructure/src/sdk/`; API public của crate giữ nguyên. Bản kê SDK canonical nằm trong `infrastructure/resources/`, build không đọc từ docs/reports. Manifest workspace quản lý version/dependencies dùng chung. Xem [development](DEVELOPMENT.md) cho cấu trúc và lệnh kiểm tra.

Theo ADR 0042, metadata runtime cục bộ đã chuyển sang Android Hub và bỏ alias VM. Engine headless được spawn làm process con của Hub để tránh lỗi quyền Desktop của helper LaunchServices độc lập; thread riêng reap process, logic dừng/quit vẫn kiểm tra danh tính như trước.

Theo [ADR 0043](decisions/0043-adaptive-device-workspace.md), desktop giữ hai slot
hiển thị độc lập session map và fit màn hình theo constraint stage thực. Pointer-down
đổi inspector bằng metadata-only path, không tăng input epoch. Zoom/sidebar/pair
chỉ lưu trong phiên; engine/backend và tốc độ stream không đổi.

Theo [ADR 0044](decisions/0044-observe-boot-and-shutdown-progress.md), readiness
là observation async riêng, gắn ID/PID/ADB transport/boot ID và có timeout. Desktop
hủy subscription khi guest kết thúc, bỏ kết quả cũ theo epoch, chỉ gọi sẵn sàng
khi boot hoàn tất và display/input gate cùng đạt. Shutdown phát tiến độ từng máy
nhưng vẫn cần snapshot cuối không còn engine trước khi thoát.

Theo [ADR 0045](decisions/0045-modal-feedback-and-confirmed-stop.md), notification
và modal nằm trên stack độc lập kích thước stage. Modal chặn input ở cả widget
và update; tác vụ nền đang chạy vẫn trả kết quả. Xác nhận dừng giữ ID/PID và
backend kiểm tra PID kỳ vọng sau khi lấy lock. Thông báo lỗi/kết quả chưa rõ
giữ đến khi đóng; sao chép lỗi chỉ lấy tóm tắt không chứa raw diagnostics.

Theo [ADR 0046](decisions/0046-settings-drafts-and-optimistic-save.md), draft giữ
profile gốc riêng. Poll chỉ cập nhật draft sạch; xung đột không bị ghi đè. Desktop
giữ navigation intent có kiểu và chỉ tiếp tục sau save thành công. Backend
`save_if_unchanged` so profile gốc dưới Hub lock trước các stopped/write guard.

Theo [ADR 0047](decisions/0047-create-request-and-registered-result.md), tạo máy
nhận CreateRequest có tên/thông số và phiên bản Android, backend tự cấp ID/port dưới lock.
CreatedDevice giữ profile đã đăng ký độc lập lỗi snapshot/biên nhận; desktop
chỉ khởi động đúng ID trả về khi người dùng chọn trước. Preview môi trường
chỉ đọc thành phần SDK/Java và filesystem; không tạo journal hay tải phần mềm.

Theo [ADR 0048](decisions/0048-native-apk-picker-and-explicit-drop-target.md),
picker APK dùng AppKit qua rfd, tạo future trên UI thread rồi chờ bất đồng bộ.
Picker và drop cùng kiểm tra file/xác nhận; drop hỏi máy nhận vì Iced không cung
cấp tọa độ đáng tin. Xác nhận giữ ID/PID/command epoch và lựa chọn gốc; cancel
hoặc đổi phiên bỏ kết quả cũ, backend vẫn giữ lock/AVD/boot gates. Clipboard
chỉ báo thành công sau ACK và nhắc người dùng dán trong Android.

Theo [ADR 0049](decisions/0049-host-appearance-and-semantic-palette.md), theme Hub
lưu riêng `ui-appearance` (System/Light/Dark). AppKit effectiveAppearance được
đọc trên main thread trong Tick 3 giây sẵn có; không helper hay đổi setting macOS.
Palette và vector icon thuộc desktop, không thay stream/session/profile Android.

Theo [ADR 0050](decisions/0050-keyboard-focus-and-scroll-reveal.md), control Hub
chia sẻ focus operation với text input. Tab release input Android rồi chuyển focus;
form tạo máy giới hạn traversal trong modal. Kích hoạt bằng key release chỉ giữ
khi action identity còn khớp; focus reveal chỉ cuộn các panel Hub liên quan.

Theo [ADR 0054](decisions/0054-app-owned-data-root.md), data root không còn phụ thuộc
source checkout. `ANDROID_HUB_ROOT` được hiểu là thư mục dữ liệu trực tiếp, dùng
cho QA. Backend và UI preferences dùng chung root; initialize tạo registry rỗng,
không tự nhập máy P0. Runtime cũ được reset theo yêu cầu, không migrate AVD/backup.
