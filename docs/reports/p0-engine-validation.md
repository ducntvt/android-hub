# P0 engine validation

Status: in_progress — 2026-11-13.

Báo cáo lịch sử theo thời điểm thử nghiệm. Checkout `experiments/p0/upstream/avo` đã được xóa theo yêu cầu người dùng; không dùng mô tả checkout bên dưới như trạng thái hiện tại. Sản phẩm hiện dùng Android Emulator Google Play ARM64. Xem [đối chiếu MVP](mvp-coverage.md) để biết các gate còn thiếu.

## Bằng chứng hiện có

- [Môi trường](p0/environment.json): macOS ARM64, Rust/Cargo, Xcode và upstream revision. Đọc RAM bằng sysctl bị sandbox từ chối; trường lỗi được giữ nguyên, không giả thành số đo.
- Source Avo pin `08096b2a3cfa93f1ba924870e2ed18ac0b309f7a` tại `experiments/p0/upstream/avo`, không thay đổi source; LICENSE/NOTICE giữ nguyên trong checkout.
- Build `cargo build --release --locked -p avo-vmm` thành công. [Log chạy script tái lập](p0/build-engine.log), [SHA-256 binary](p0/engine.sha256).
- Android SDK có sẵn image Google Play ARM64 API 30 và 36 cho đối chứng. Chưa tạo hoặc sửa AVD hiện có.
- Ninja chưa có trong PATH; cần giải quyết nếu build renderer từ source.

## Lần boot đầu tiên

- Image Cuttlefish build `16644040`, Android 17, nguồn Google; checksum tải được kiểm tra bằng script upstream. [Manifest SHA-256](p0/image.json).
- Dùng renderer Android SDK 37.2.12 đang có trên máy. Engine xác nhận `Apple M2`, subscriber nhận `gpu active Apple M2`, `gpu context`, `gpu first-frame 0`.
- Console trả `P0_BOOT=1` và Android release `17`.
- [Ảnh màn hình khóa](p0/first-frame.png) đã kiểm tra trực quan: khung hình Android 720×1280 có nội dung hợp lệ.
- [Boot log](p0/boot-a.log). VM cấu hình 2 vCPU/3072 MiB/30 Hz. Đã gửi `reboot -p`, process engine kết thúc exit 0.
- Công cụ Rust `experiments/p0/control` build và clippy qua; kết nối socket thực, subscribe và snapshot thành công.
- Engine dùng shader cache mặc định tại `~/Library/Caches/Avo/gpu-cache`; cần chuyển cache riêng trong bước tích hợp để không dùng chung namespace với Avo.

## Input và mạng lần boot thứ hai

- Rust control gửi swipe thành công, [ảnh sau mở khóa](p0/unlocked.png) cho thấy launcher Android. Input đã được kiểm chứng ở thao tác swipe; chưa phải kiểm chứng game/key mapping.
- Thêm `scripts/p0-console.py`: câu lệnh guest được bao bằng marker riêng, timeout và giới hạn output; trả lại exit code guest. Thử `exit 42` thực tế cho process host exit 42.
- Trước khi join VirtWifi: không có route. Sau `cmd wifi connect-network VirtWifi open`: Android có IP 10.0.2.15, ping IP ngoài thành công 2/2. [Log](p0/network-a-after.txt).
- Android báo mạng VALIDATED và DNS resolver có 6 truy vấn NOERROR. Tuy nhiên `ping google.com` từ serial shell vẫn báo unknown host. Chưa kết luận DNS của ứng dụng đã đạt; cần kiểm tra qua ADB hoặc ứng dụng thử. Host DNS trả lời bình thường; không sửa resolver Mac.
- Guest không có curl (exit 127), nên phép thử HTTPS bằng curl chưa thực hiện được.
- VM đã shutdown sạch, engine exit 0. Boot script nay đặt `AVO_GPU_CACHE_DIR` riêng theo instance cho những lần chạy sau; lần boot thứ hai bắt đầu trước thay đổi này.

## ADB và DNS đã kiểm chứng

- Thêm binary Rust `adb-bridge` chuyển TCP loopback port ngẫu nhiên tới hybrid vsock 5555. Tự kết thúc sau 150 giây; đây là công cụ P0, chưa phải endpoint sản phẩm.
- Sửa socket được accept thành blocking: macOS kế thừa trạng thái nonblocking từ listener, làm relay `io::copy` kết thúc sớm và ADB offline. Sau sửa, ADB connect thành công.
- Bật `service.adb.listen_addrs=vsock:5555` trên guest userdebug và restart adbd. Không đổi cấu hình auth; log cho biết image hiện tại không yêu cầu authentication. Vì vậy bridge chỉ dùng tạm trên loopback, đã dừng sau kiểm chứng; sản phẩm phải dùng transport có xác thực/quyền truy cập phù hợp, không giữ listener này mặc định.
- [ADB network evidence](p0/adb-network.txt): DNS google.com và ping hoạt động 2/2. ADB shell có group 3003(inet); serial shell trước đó thiếu group này. Không cần sửa DNS host/engine để đạt phép thử.
- Cả root serial và ADB đều phân giải được tên miền. Chưa dùng bằng chứng này thay cho thử đăng nhập Google hoặc mạng trong game.

## Viewer Rust P0

- Thêm `experiments/p0/viewer` với minifb 0.29.0, libc, Cargo.lock riêng. Build và clippy qua.
- Subscribe display 0, mở POSIX shared memory read-only, kiểm tra kích thước/header và copy frame nhất quán trước đổi màu BGRA/RGBA sang RGB.
- Lần chạy đầu dùng pread thất bại Illegal seek trên macOS; đã sửa sang mmap read-only, giải phóng mapping bằng Drop.
- Đã mở bundle P0Viewer.app bằng CUA và kiểm tra screenshot: cửa sổ native hiển thị đúng màn hình khóa Android với đồng hồ và biểu tượng Wi-Fi.
- Đã viết input chuột và Esc/ H cho Back/Home nhưng chưa kiểm chứng thao tác qua cửa sổ trong lần chạy này. Engine và viewer có timeout riêng; cần thêm xử lý engine exit để tránh giữ frame cũ.
- [ADR 0003](../decisions/0003-use-minifb-for-p0-viewer.md) giới hạn minifb ở P0; chưa đổi đề xuất Iced cho sản phẩm.

## Lifecycle và kiểm tra input UI

- Viewer giữ luồng subscribe trên thread riêng, thoát khi engine disconnect hoặc display layout bị thay thế. Đã shutdown VM và kiểm tra CUA app inventory: P0Viewer không còn chạy.
- Timeout boot/viewer tăng lên 600 giây để đủ thao tác UI, không phải chế độ chạy sản phẩm lâu dài.
- CUA drag và phím Home chưa mở khóa màn hình; chưa chứng minh mouse/keyboard UI đúng. Đã đổi `get_mouse_pos` sang `get_unscaled_mouse_pos` để cùng đơn vị với `get_size`; cần thử lại sau build. Không đánh dấu P0.4 hoàn thành.

## Chẩn đoán input UI tiếp theo

- Thêm trace tùy chọn `P0_INPUT_TRACE=1` để thấy lệnh input thật gửi tới engine; chỉ bật trong VM thử nghiệm.
- Nguồn minifb 0.29 macOS: `mfb_is_active` trả native bool, wrapper Rust so sánh `== 0`. Thêm workaround đảo lại riêng macOS tại phiên bản pin, cùng tọa độ unscaled.
- Phát hiện hai process viewer cùng bundle ID trong phiên CUA; đã đóng process cũ. Đã ký ad-hoc lại bundle sau thay binary (trước đó chạy bundle có binary mới trả exit 137).
- CUA trên viewer mới vẫn chưa tạo input trace hoặc mở khóa. Vì vậy sửa focus chưa được chứng minh giải quyết vấn đề end-to-end. Cần kiểm tra sự kiện native hoặc thử chuột trực tiếp; không được coi ảnh màn hình khóa là bằng chứng input thành công.

## Chưa xác minh

Chưa có UI tương tác của sản phẩm; chưa kiểm chứng thao tác chơi, audio, mạng trong game, Play Store hoặc gameplay hai VM. Image mới có AOSP, chưa GApps. Boot và khung hình không chứng minh gameplay.

## Bước tiếp theo

P0.4: bổ sung cửa sổ hiển thị/input, xác minh network/audio. Sau đó cài GApps phù hợp và thử hai game. Các task tiếp theo ở [TASKS](../TASKS.md).

## Hai VM và restart cách ly

- [Cùng boot](p0/dual-both.json), [sau dừng A](p0/dual-after-a-stop.json), [sau restart A](p0/dual-after-a-restart.json): cả ba phase pass khi chạy trên guest thực.
- Script smoke lưu boot ID và uptime, yêu cầu B giữ nguyên boot ID và uptime tăng; A phải đổi boot ID sau restart. Marker instance-a/instance-b giữ nguyên trên disk riêng.
- A shutdown lần đầu exit 0; sau bài thử A/B đều shutdown sạch exit 0. Runtime, disk và GPU cache nằm trong namespace riêng theo boot script.
- P0.6 đạt phạm vi smoke lifecycle/dữ liệu. Chưa thử crash, hai game, tải GPU kéo dài hoặc tài khoản độc lập; không dùng kết quả này thay gate gameplay/benchmark toàn P0.

## Sửa input viewer và kiểm chứng tap/Home

- Thêm trace chuyển trạng thái focus/chuột (chỉ bật bằng P0_INPUT_TRACE). Lần chạy trước patch đã gửi touch qua viewer, mở khóa; guest `dumpsys window policy` trả `showing=false` và screenshot CUA thấy launcher.
- Phát hiện minifb lấy global cursor sau event dispatch, có thể ghi đè tọa độ NSEvent. Vendored minifb 0.29.0 tại `experiments/p0/vendor/minifb`, giữ license; patch dùng tọa độ down/up/drag/move, chỉ poll fallback khi không giữ chuột và trước dispatch. Sửa polarity `is_active` ngay trong dependency, bỏ workaround ở viewer.
- CUA từng mở lại viewer cũ sau thao tác close/getAXState, tạo process trùng bundle ID. Đã xác định và đóng process cũ trước kiểm chứng bản mới.
- Kiểm chứng bản mới bằng CUA: nhấn giữ Gallery mở menu, Esc đóng menu rồi tap mở Gallery, H trở về launcher. Home sửa từ Linux 102 sang 172 theo upstream AndroidKeys.swift. Screenshot đã quan sát trong phiên; trace tại [viewer-patched-input.log](p0/viewer-patched-input.log) và [viewer-home-input.log](p0/viewer-home-input.log).
- Swipe mở drawer trên bản patch chưa có bằng chứng đạt (không xuất hiện touch trace trong lần thử cuối). Cần kiểm chứng tiếp drag và mất focus; không coi toàn bộ input đã hoàn tất.
- Build/clippy viewer exit 0; dependency minifb còn warning upstream (dead code/lifetime, native unused parameter), không tuyên bố dependency sạch warning. Guest shutdown exit 0, viewer tự thoát exit 0 khi engine disconnect.

## GApps package preflight

- Release pin: MindTheGapps Android 17 ARM64 `20261022_132445`; checksum SHA-256 khớp digest từ GitHub release. [Manifest đầy đủ](p0/gapps-package.json) ghi source URL, checksum archive, destination/hash từng file và file bỏ qua.
- [Guest facts](p0/gapps-guest-facts.txt): SDK 37, arm64, vsoc_arm64_only; khớp build.prop của gói. Đã stage 35 file, 716481828 bytes trên host; chưa cài vào guest. Không chạy update-binary từ archive.
- `scripts/p0-gapps-package.py` kiểm tra checksum pin, SDK/arch, đường dẫn, loại file, giới hạn expanded size; chỉ chọn product/system_ext theo quy tắc upstream. SetupWizard và VelvetTitan được bỏ qua cho guest này. Staging dùng thư mục mới, xóa phần staging dở nếu giải nén thất bại.
- 5 test boundary qua: mismatch SDK, traversal/absolute path, symlink, partition ngoài danh sách, chọn file bỏ SetupWizard. Đây là kiểm chứng preflight, không phải kiểm chứng Play Store.
- [Guest preflight](p0/gapps-guest-preflight.txt): verity bật; /product và /system_ext không có dung lượng ghi; /data còn khoảng 7.46 GiB. Cần overlayfs/scratch theo quy trình Avo, không thể adb install vài APK thay thế bước system integration.
- Gói có app Google bổ sung (Velvet, Wellbeing, AndroidAutoStub, sync adapters…). Inventory được giữ minh bạch; chưa tuyên bố bộ GApps tối giản. Kiểm chứng tương thích trước khi cắt thành phần, không tự thêm app tài trợ.
- Guest đã shutdown sạch exit 0 sau khảo sát.

### Công việc cài guest còn lại

1. Backup bộ disk/metadata của instance thử khi đã dừng, xác minh khả năng restore.
2. Tắt verity chỉ trong guest thử; restart, remount tạo scratch/overlay, restart nếu được yêu cầu và probe writable. Không sửa image gốc dùng chung.
3. Kiểm tra dung lượng scratch và data sau remount, đẩy staging theo instance ADB cụ thể; ghi manifest/rollback, allowlist trước app, set owner/mode và restorecon theo upstream GoogleAppsCommit.
4. Restart; xác minh package hệ thống, crash log và mở Play Store; người dùng tự đăng nhập khi cần. Hoàn thiện rollback nếu cài gián đoạn.
5. Thử game và audio/network; chưa đánh dấu P0.5 đạt nếu chỉ cài được package.

## Backup/restore trước GApps và approval blocker

- Đã giữ khóa flock tương thích vm.lock của engine khi clone toàn bộ instance A bằng APFS clone vào `experiments/p0/runtime/backups/a-before-gapps`. SHA-256 source/backup của toàn bộ *.img khớp: [manifest](p0/gapps-backup.json).
- Boot một bản clone phục hồi riêng `restorecheck`, xác minh boot_completed=1, marker instance-a còn nguyên, verity vẫn enabled: [evidence](p0/gapps-restorecheck.txt). Đây là kiểm chứng phục hồi disk P0, không phải backup/restore sản phẩm hoàn chỉnh.
- Instance restorecheck shutdown exit 0. Sau đó A boot để chuẩn bị GApps.
- Automatic approval review từ chối lệnh `su 0 avbctl disable-verity`: persistent integrity/security change chưa được người dùng cho phép cụ thể. Lệnh không được chạy; chưa remount/cài system apps. Không thử đường vòng.
- A đã shutdown sạch exit 0 sau rejection. Chờ người dùng cho phép rõ việc tắt dm-verity **chỉ trong guest A thử nghiệm** trước bước phụ thuộc. Image gốc và macOS không thuộc phạm vi thay đổi.

## GApps overlay sau phê duyệt và rollback thực tế

- Người dùng đã cho phép rõ tắt dm-verity chỉ trên guest A thử nghiệm. Lệnh được chạy thành công: [kết quả](p0/gapps-disable-verity.txt). Approval blocker ở phần trước đã được giải quyết.
- Sau restart, remount báo overlayfs enabled, remount succeeded và yêu cầu reboot. Tuy nhiên ngay sau remount guest không chạy được printf hoặc /system/bin/reboot. Console verifier timeout; không coi exit của wrapper là kết quả remount. [Log console](p0/gapps-remount-console.txt).
- SIGTERM yêu cầu guest power off nhưng guest không tắt; engine kết thúc exit 1 sau timeout. Một lần boot quá sớm bị instance lock từ chối; đã chờ engine cũ terminal trước lần boot tiếp theo. Không có hai engine ghi cùng disk.
- Boot sau đó reset liên tục khoảng 1–2 giây/lần. [Boot log](p0/gapps-overlay-reboot2.log), [kernel capture](p0/gapps-overlay-failure-kernel.log) cho thấy giai đoạn mount scratch/overlay; chưa đủ bằng chứng kết luận nguyên nhân gốc. Không có GApps payload nào được copy vào guest.
- Dừng engine reset-loop (exit 143). Giữ nguyên bộ disk lỗi tại `experiments/p0/runtime/a-overlay-failed`, phục hồi A từ `backups/a-before-gapps` dưới flock.
- [Rollback evidence](p0/gapps-rollback-result.txt): boot_completed=1, marker instance-a còn nguyên, verity enabled trở lại. Sau kiểm chứng A shutdown sạch exit 0.
- Bước tiếp theo: chẩn đoán overlay/image compatibility trên bản clone thử riêng; kiểm tra khả năng remount có chọn partition thay vì mount toàn system. Nếu không giải quyết được bằng thay đổi có căn cứ, chạy đối chứng Google Play ARM64 theo plan trước khi mở rộng việc sửa engine. Play Store/gameplay vẫn pending.

## Chẩn đoán reset và thử targeted remount

- Boot script thêm `P0_EXIT_ON_REBOOT=1`, dùng cờ upstream để dừng reset đầu (exit 77) và giữ log. Đã sửa tương thích bash macOS khi không có cờ; cả nhánh mặc định và nhánh chẩn đoán chạy được, bash -n qua.
- [Kernel lần lỗi đầy đủ](p0/gapps-overlay-single-kernel.log): nhiều executable bị Permission denied; boringssl_self_test64 có reboot_on_failure dẫn tới reset. Chưa chứng minh nguyên nhân kernel/filesystem cụ thể; không suy diễn đây là lỗi SELinux chỉ từ AVC permissive.
- Guest remount --help xác nhận chọn partition được. Thử `remount /product /system_ext` trên A sạch sau disable-verity đã được cho phép. Lệnh thành công và guest shutdown sạch, nhưng sau reboot hwservicemanager trong system_ext không exec được; init reset khoảng 21 giây. [Kernel](p0/gapps-targeted-failure-kernel.log), [remount](p0/gapps-targeted-remount.txt).
- Probe trước boot hoàn tất thấy hai overlay còn ro; không coi đó là readiness pass. Guest đã exit 77 trước bước remount tiếp theo; không cài payload GApps.
- Giữ disk lỗi tại `runtime/a-targeted-failed`, rollback A từ backup và xác minh [boot_completed=1, marker, verity enabled](p0/gapps-targeted-rollback.txt). A shutdown sạch exit 0.
- Kết luận hiện tại: targeted remount chưa khắc phục compatibility blocker. Chuyển công việc tiếp theo sang đối chứng Android Emulator Google Play ARM64 theo kế hoạch, chưa chốt thay engine hoặc bỏ yêu cầu Rust/multi-instance.

## Android Emulator Google Play đối chứng

- Đã tạo AVD a/b trong `experiments/p0/runtime/emulator-avds`, ANDROID_USER_HOME riêng; không sửa AVD cá nhân. Dùng JBR có sẵn của Android Studio vì Java mặc định 8 không đáp ứng avdmanager.
- Image đã cài sẵn: API36/Android16 Google Play ARM64 revision 7, fingerprint và package versions tại [A](p0/emulator-a.json), [B](p0/emulator-b.json). Verity enforcing, Play Store và Play Services hiện diện. Chưa mở UI đăng nhập hoặc tải game.
- Hai guest cùng boot hoàn tất, marker riêng: [evidence](p0/emulator-dual-boot.json). Renderer log xác nhận Apple M2; mỗi máy 2 vCPU/3072 MiB/720x1280. Chưa kiểm chứng restart cách ly hoặc gameplay trên backend này.
- Lần smoke đầu chạy headless, audio tắt, refresh mặc định 60 Hz; không so số đo hiệu năng với Avo 30 Hz. Boot script đã thêm vsync-rate 30 cho lần sau và tùy chọn mở UI/audio; các tùy chọn mới chưa kiểm chứng thực tế.
- Script setup từ chối overwrite; probe có deadline, kiểm tra AVD name trước đọc guest và chỉ dùng serial cụ thể. Shell syntax/Python compile qua, probe A/B thực tế pass. Cả hai engine kết thúc exit 0 sau adb emu kill.
- Không chọn engine chỉ vì package tồn tại: bước kế tiếp mở UI, xác minh Play Store, audio/input và hai game; người dùng tự đăng nhập Google/game khi cần.

## UI đối chứng và bàn giao đăng nhập

- Boot A với cửa sổ và audio enabled; renderer Apple M2, boot log xác nhận guest vsync 30 Hz. Chưa nghe/đo audio thực nên không đánh dấu audio pass.
- Executable emulator ngoài bundle không xuất hiện trong inventory CUA. Launcher app ban đầu gọi script khác dưới Desktop bị Operation not permitted. Đã đổi thành launcher chứa trực tiếp nội dung boot đã review; mở được cửa sổ `Android Emulator - p0-google-a:5580` qua CUA.
- Thêm `scripts/p0-emulator-app.py` tạo launcher A/B trong runtime, dùng script boot hiện có làm nguồn; syntax của hai launcher và Python compile qua. Bản launcher tạo thủ công đã mở UI thành công; bản generator cần kiểm chứng launch lại sau phiên đăng nhập, không đóng A giữa thao tác của người dùng.
- CUA đã quan sát màn hình Google Sign in tải được, ô tài khoản trống. Đây chưa phải Play Store đăng nhập thành công hoặc gameplay pass.
- Người dùng trả lời sẽ tự đăng nhập và báo khi xong. Để A chạy phục vụ thao tác này; không chụp màn hình hoặc thao tác tiếp trong lúc nhập credential. B chưa mở UI.

## Rust engine metrics sampler

- Thêm `experiments/p0/control/src/bin/engine-metrics.rs`: chọn PID engine, giới hạn sample count/interval, kiểm tra process identity, ghi JSONL vào file mới, đánh dấu lượt thu bị gián đoạn. Không điều khiển UI/guest.
- Build/clippy và hai test parser qua. [Smoke JSONL](p0/emulator-metrics-smoke.jsonl) có ba mẫu thật, end complete=true, đã parse và kiểm tra số lượng/RSS. Đây chỉ là kiểm chứng công cụ, không phải benchmark workload hoặc kết luận hiệu năng.
- Chỉ ghi process RSS KiB và ps %CPU; không gồm GPU/helpers/FPS/memory pressure. Không suy ra tổng RAM VM từ RSS này.
- Trong lúc kiểm chứng sampler, giữ máy A mở để người dùng đăng nhập; không đọc màn hình hoặc account data. Vẫn chờ người dùng báo hoàn tất trước các thao tác Play Store/game.

## Play Store đăng nhập và cài hai game

- Người dùng đã báo đăng nhập hoàn tất. Qua CUA, Play Store mở được danh mục và tìm kiếm; đã cài đúng Top Heroes: Kingdom Saga (RiverGame) và Hunt Royale (BoomBit Games) từ Store trên emulator A.
- [Package evidence](p0/emulator-game-packages.json): Top Heroes 1.128.10/code 3237 và Hunt Royale 3.37.1/code 2221, cả hai arm64-v8a. Không thu credential hoặc dữ liệu game/account vào report.
- Đã mở Top Heroes bằng CUA và quan sát màn hình Start Game, hiển thị version 1.128.10. Chưa vào gameplay. Hunt Royale đã cài nhưng chưa thử mở.
- Start Game ghi rõ chấp nhận Terms of Service và xác nhận đọc Privacy Policy. Đã hỏi tại bước này; người dùng chọn tự xem và bấm. Giữ nguyên cửa sổ, chờ người dùng báo đã vào game; không tự chấp nhận điều khoản.
- Gameplay/input trong game/audio/FPS, phiên hai game đồng thời và soak test vẫn pending. Cài thành công không thay thế gate P0.

## Kiểm tra tính đầy đủ dữ liệu metrics

- Thêm `scripts/p0-metrics-summary.py`, kiểm tra schema/start/end, count/order, đơn vị RSS, giá trị hợp lệ, clock đơn điệu, gap và duration trước khi tính mean/peak mẫu.
- 4 test qua, gồm lượt thu bị gián đoạn, metrics hỏng và smoke không đủ thời lượng. CLI chạy trên dữ liệu thật tạo [summary](p0/emulator-metrics-smoke-summary.json); yêu cầu 3600 giây bị từ chối đúng exit 1 vì dữ liệu chỉ khoảng 2 giây.
- Không có benchmark/gameplay mới trong bước này. Người dùng vẫn đang tự xử lý điều khoản Top Heroes; không điều khiển hay chụp màn hình guest.

## Mở cửa sổ B theo yêu cầu người dùng

- Người dùng xác nhận đã chơi được trên A; ghi nhận là xác nhận của người dùng, chưa thay thế bài benchmark/gameplay có đo lường.
- Đã mở P0EmulatorB bằng launcher riêng; engine p0-google-b:5582 boot hoàn tất, cùng lúc A:5580 vẫn chạy. CUA inventory thấy cả hai app running; log B ghi created extended window và boot completed.
- Đọc screenshot/AX của B bằng CUA timeout; chưa xác minh trực quan nội dung cửa sổ B trong lượt này. Không restart hoặc dừng máy A.
