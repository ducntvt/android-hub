# Thiết lập Android Hub

[English](SETUP.md) · Tiếng Việt

Build app theo [README](../README.vi.md) trước. Tài liệu này dành cho SDK và dữ liệu trên máy của bạn.

Bản cá nhân hiện dùng SDK đã cài trên macOS Apple Silicon. Đã có luồng chuẩn bị SDK/image trong Hub nhưng chưa qua native setup/boot trên SDK tải mới; chưa phải gói mang sang máy khác chạy ngay.

## Thành phần cần có

Hub mặc định đọc SDK ở `~/Library/Android/sdk`. Có thể dùng `ANDROID_SDK_ROOT` khi khởi chạy từ môi trường có biến này. App mở từ Finder không tự kế thừa biến trong cấu hình shell.

Trong SDK Manager của Android Studio, chuẩn bị:

- Android Emulator (`emulator/emulator`).
- Android SDK Platform-Tools (`platform-tools/adb`).
- Android SDK Command-line Tools (latest), cung cấp `cmdline-tools/latest/bin/avdmanager`.
- Google Play ARM64 của phiên bản muốn dùng: Android 12/13/14/15/16 tương ứng API 31/33/34/35/36; package `system-images;android-<API>;google_apis_playstore;arm64-v8a`.

Tạo máy ưu tiên `ANDROID_HUB_JAVA_HOME`, rồi biến tương thích cũ `P0_JAVA_HOME` trong môi trường khởi chạy Hub, sau đó Java home đã lưu trong Thiết lập SDK, cuối cùng JBR mặc định tại `/Applications/Android Studio.app/Contents/jbr/Contents/Home`. Mở máy đã tạo không cần avdmanager/Java.

Hub kiểm tra file tồn tại, không rỗng và quyền execute cho công cụ. Kiểm tra này không chạy binary để chứng nhận phiên bản, không bảo đảm SDK/image không hỏng và không thay thế kiểm tra boot. Nếu SDK Manager yêu cầu chấp nhận license, người dùng tự xem và chấp nhận trong công cụ đó.

## Sử dụng bản app hiện tại

Mở `Android Hub.app` đã build. Dữ liệu nằm trực tiếp ở `~/Library/Application Support/Android Hub`, độc lập với thư mục source. Nút `+` tạo AVD sạch, không sao chép tài khoản. Chọn máy, cấu hình khi máy đã dừng, lưu rồi khởi động. Đăng nhập Play Store trực tiếp trong Android. Muốn dùng hai máy, tạo máy thứ hai rồi chọn chế độ **2 máy**.

Khi thiếu thành phần, Hub báo danh sách đường dẫn cần kiểm tra và dừng trước avdmanager/launcher. Cài/khôi phục bằng SDK Manager rồi thử lại. Hub không tự chmod công cụ hoặc tải package trong thao tác này.

## Dữ liệu

Dữ liệu nằm trực tiếp tại `~/Library/Application Support/Android Hub` (hoặc `ANDROID_HUB_ROOT` khi chạy QA); không xóa thư mục này để sửa lỗi SDK. Mỗi máy có AVD riêng. Đóng cửa sổ Hub hoặc Cmd+Q sẽ dừng các máy đã đăng ký trước khi thoát; nếu dừng thất bại, Hub giữ cửa sổ mở để xử lý. Force Quit/crash không chạy được luồng này.

Backup/restore cần máy đã dừng, SDK image tương thích và dung lượng cho bản sao. Backup nằm ở `backups/` trong thư mục dữ liệu; dữ liệu trước restore được giữ trong `.previous-*`. Chưa có giao diện dọn các bản dữ liệu cũ.

## Chọn phiên bản và quản lý image

Bấm **+**, chọn Android 12–16 (mặc định 16). Android 12/13/14/15/16 tương ứng
API 31/33/34/35/36; không gồm 12L, preview, x86 hoặc Android 17+. Chỉ tải image
cần dùng. Các máy chia sẻ công cụ/image nhưng có disk ghi riêng.

Image sẵn sàng thì **Tạo máy**; nếu thiếu, chọn **Tải và tạo máy**, xem package,
dung lượng và tự xác nhận điều khoản. Hủy giữ form/cache để thử lại, không tạo
máy trùng. Cài image cần phần archive còn thiếu + 6 GiB giải nén + 12 GiB dành
cho máy mới + 256 MiB dự phòng; Hub kiểm tra lại trước publish/tạo máy.

Phiên bản, revision và fingerprint cố định theo máy, kể cả sau restart/restore.
Muốn Android khác hãy tạo máy mới; không nâng/hạ OS tại chỗ. Registry cũ được
sao lưu rồi migration metadata; image chưa xác định không bị tự gán Android 16.

**Cài đặt Hub → Thiết lập SDK → Phiên bản Android** hiển thị image đã cài,
dung lượng và tham chiếu. Chỉ gỡ image do Hub quản lý, đã xác minh và không còn
máy, backup, archive hoặc thao tác dở tham chiếu. Máy đã dừng vẫn giữ tham chiếu.
SDK ngoài chỉ đọc trong mục này. Gỡ image giữ archive tải về, không xóa disk máy
hay công cụ SDK dùng chung. Nếu gián đoạn, cần chủ động tiếp tục gỡ; không tự xóa
khi mở lại. Không chia sẻ SDK riêng do Hub quản lý giữa các data root độc lập hoặc
sửa bằng công cụ khác.

Hub không tự nâng engine/thay revision. Nếu SDK ngoài bị thay image, hãy phục hồi
đúng image/revision đã gắn với máy; sửa trường version không phải cách phục hồi.
Play Store mở được không bảo đảm mọi game/dịch vụ đều tương thích.

## Những phần còn đang kiểm chứng

Luồng SDK có download/resume/recovery và xoay log engine đã triển khai. Cài SDK candidate trên máy sạch và benchmark hai game dài hạn còn thiếu. Backup/restore và wheel đã có smoke riêng; giới hạn cụ thể nằm trong story tương ứng, không suy ra gameplay hay hiệu năng từ unit tests.

## Nếu tạo máy bị gián đoạn

Bản v0.1.0 có thể báo cấu hình AVD mâu thuẫn với image khi `config.ini` thiếu
`target`, dù đường dẫn image, ABI và loại Google Play hợp lệ. Bản 0.1.1
chấp nhận trường này bị thiếu, vẫn xác minh image. Sau khi cài bản build
có bản sửa, mở lại Hub rồi tạo máy mới. File tạo dở của lần lỗi vẫn được giữ,
không tự đăng ký hay xóa. Không cần reset SDK hoặc xóa toàn bộ dữ liệu Hub.

Hub báo ID và hai đường dẫn có thể còn dữ liệu dở (`ID.avd`, `ID.ini`) khi avdmanager/configuration/ghi registry thất bại. Các file được giữ lại để kiểm tra. US-019 đã có journal recovery: máy đã chuẩn bị xong được hoàn tất đăng ký khi registry/data còn khớp; file tạo dở chưa sẵn sàng được giữ và ghi biên nhận incomplete. Chưa tự thu hồi file dở. Không xóa toàn bộ runtime hoặc chạy avdmanager với tùy chọn ghi đè để thử lại.

Nếu registry đã ghi thành công nhưng refresh thất bại, thông báo sẽ ghi “Đã tạo máy … nhưng chưa đọc lại được trạng thái”. Khi đó làm mới/mở lại Hub để xem máy, không cần tạo lại. Hub từ chối tạo vào tên có directory, registration hoặc symlink đã tồn tại.

## Dung lượng tạo máy

Hub yêu cầu ít nhất 12 GiB trống trên filesystem chứa AVD trước khi tạo máy mới. Đây là ngưỡng tối thiểu cho cấu hình máy hiện tại, không phải dung lượng cấp phát ngay hoặc cam kết đủ chỗ cho game về sau. Hub không xóa dữ liệu để lấy chỗ. Máy cũ không bị đổi kích thước disk bởi kiểm tra này.

## Log engine

Hub ghi stdout/stderr engine vào data-root/<id>.log. Trước lần khởi động tiếp theo, nếu file đạt 8 MiB, Hub chuyển nó thành .log.1, dịch bản cũ sang .log.2/.log.3 và xóa .log.3 cũ nhất. Tối đa ba archive được giữ cho mỗi máy; sao chép log ra ngoài trước lần mở máy tiếp theo nếu cần giữ lịch sử lâu dài. Không xoay hoặc cắt log khi engine đang chạy, nên một phiên dài vẫn có thể tạo log lớn hơn 8 MiB. Panel Dung lượng dữ liệu tính cả archive vào Log engine.

## Chọn SDK trong Hub

Mở **Thiết lập SDK**, nhập đường dẫn tuyệt đối tới SDK đã cài, rồi bấm **Kiểm tra và lưu SDK** khi tất cả máy đã dừng. Hub kiểm tra Emulator, Platform-Tools, Command-line Tools, metadata image đã chọn và Java hiện có; không tải hay cài thành phần mới.

Lựa chọn lưu trong data-root/hub-settings.json, áp dụng lần mở sau. ANDROID_SDK_ROOT được ưu tiên cao hơn; nếu biến này đang đặt, bỏ override và mở lại app trước khi lưu từ UI. Khi runtime còn AVD/archive, chỉ chọn lại đúng SDK hiện tại: migration sang SDK khác chưa được hỗ trợ. File cấu hình lỗi cần phục hồi đúng SDK cũ nếu đã có dữ liệu; runtime trống có thể lưu lại từ panel.

## Chọn Java trong Hub

Trong **Thiết lập SDK**, nhập Java home (thư mục chứa `bin/java`) và bấm **Kiểm tra và lưu Java** khi tất cả máy đã dừng. Hub kiểm tra Java 17+ ARM64 và chạy version/list device của SDK hiện tại trong home tạm riêng trước khi lưu. Không tải Java, không khởi động guest. Lựa chọn được giữ qua restart và khi lưu SDK; `ANDROID_HUB_JAVA_HOME` (hoặc `P0_JAVA_HOME` khi biến mới chưa đặt) ưu tiên cao hơn và khóa nút lưu khi đang đặt.

Kiểm tra lỗi giữ nguyên lựa chọn cũ và báo bước thất bại. Panel Java và backend đã qua kiểm tra trên runtime QA riêng; xem US-026 cho bằng chứng. Không thay SDK/Java của máy thật chỉ để kiểm thử.

## Tìm SDK do Hub chuẩn bị

Mục **SDK do Hub chuẩn bị** trong Thiết lập SDK đọc các thư mục SDK đã hoàn tất trong data-root/managed-sdks. Bấm **Làm mới danh sách** để đọc lại. **Kiểm tra và sử dụng** kiểm tra lại SDK/Java rồi lưu cả hai cùng lúc; không chọn tự động sau restart. SDK cài dở hoặc receipt/payload thiếu không được đưa vào danh sách dùng được, dữ liệu vẫn được giữ.

Cần dừng mọi máy và bỏ environment overrides trước khi chọn. Nếu runtime còn AVD/archive gắn SDK khác, thao tác vẫn bị chặn. Luồng tải/chuẩn bị SDK có trong màn hình bản kê; danh sách này chỉ tìm SDK đã được backend assembly hoàn tất. Native panel mới chưa qua smoke trực tiếp.

## Xem bản kê SDK và điều khoản

**Thiết lập SDK → Xem bản kê cài SDK** hiển thị ba package công cụ dùng chung đã cố định, tổng download và nguyên văn hai bộ điều khoản. Xem và tự chấp nhận từng bộ điều khoản, nhập Java home đã cài, rồi bấm **Tải và chuẩn bị SDK**. Checkbox không chọn sẵn và không lưu qua đóng/mở review. Chỉ mở màn hình không tải gì.

Trong lúc chuẩn bị SDK, panel hiển thị phase và số bytes, có nút **Hủy chuẩn bị SDK**. Hủy chờ thao tác hiện tại kết thúc; archive/partial được giữ. Có thể bấm bắt đầu lại để retry từ cache. Nếu cache checksum sai hoặc thiếu validator resume, chọn **Tải lại cache gói này** ở đúng package rồi xác nhận. Hub giữ cache cũ trong quarantine; bấm bắt đầu setup riêng để tải lại. Cache cũ vẫn chiếm dung lượng, không tự xóa.

Trước download, cần chỗ trống cho archive còn thiếu + 6 GiB headroom. Đây là admission policy; từng ZIP vẫn được kiểm tra dung lượng thực tế trước giải nén. SDK hoàn tất không được chọn tự động: xem lại trong danh sách rồi chọn **Kiểm tra và sử dụng**. Java 17+ ARM64 phải có sẵn; luồng chưa tải Java hộ. Workflow đã qua fixture tests, native panel/candidate SDK thật vẫn chưa được kiểm chứng.

Nếu reset bị gián đoạn, lần tải tiếp theo phục hồi intent trước khi dùng cache. Nếu file đã bị sửa bên ngoài, Hub dừng và báo đường dẫn để kiểm tra thay vì ghi đè. Chưa có UI xóa/undo quarantine.

## Đóng gói bản QA tách dữ liệu

Sau khi có `Android Hub.app`, tạo bản QA vào một thư mục **chưa tồn tại**:

```bash
python3 scripts/build-android-hub-qa.py '/tmp/android-hub-qa-new'
```

Mở `Android Hub QA.app` trong thư mục đó bằng Finder. Launcher dùng thư mục `data` cạnh bundle, không dùng dữ liệu cá nhân trong Application Support. Script dùng lại binary đã build, ký ad-hoc và kiểm tra chữ ký; không tự mở app, tạo guest hay chấp nhận điều khoản SDK. Không di chuyển thư mục QA sau khi tạo AVD vì SDK có lưu đường dẫn tuyệt đối.

## Xử lý lỗi thường gặp

- **Không clone được repo**: tài khoản GitHub cần được mời vào repo private; xem [GitHub](GITHUB.md).
- **Không tạo được máy**: kiểm tra SDK/Java trong panel, đủ 12 GiB trống và không có biến môi trường đang ghi đè lựa chọn.
- **Không kết nối được màn hình**: thử kiểm tra lại trạng thái; khi cần restart, dừng đúng máy từ Hub rồi mở lại. Không xóa disk để thử sửa.
- **Một app báo lỗi HTTP**: dùng kiểm tra kết nối trong cấu hình thiết bị; kết quả endpoint kiểm tra không bảo đảm mọi dịch vụ/game đều hoạt động.
- **Icon Dock mặc định**: chạy lại script build để cập nhật bundle/đăng ký icon, rồi mở lại app khi thuận tiện. Không cần xóa dữ liệu Android.
- **Cần báo lỗi**: ghi tag/commit, macOS/chip, phiên bản SDK/image, bước tái hiện và lỗi đã che thông tin riêng; không gửi mật khẩu, token hay userdata.

## Gỡ cài đặt (từ v0.2.0)

Vào **Cài đặt Hub → Môi trường và dữ liệu → Gỡ cài đặt Android Hub…** trước khi
xóa app trong Finder. Chờ thao tác ghi/tải hoàn tất hoặc hủy nó trước. Xem danh
sách đường dẫn và dung lượng, chọn **Giữ lại backup** nếu cần, nhập đúng `DELETE`
rồi xác nhận. Đổi tùy chọn backup sẽ xóa nội dung xác nhận. Hub dừng máy Android,
chuẩn bị helper rồi thoát trước khi xóa. Nếu còn Hub/engine khác đang chạy, việc
gỡ bị chặn.

Chỉ gỡ app đang chạy trong `/Applications/Android Hub.app` hoặc
`~/Applications/Android Hub.app` với data root mặc định. Bản chạy từ source,
đường dẫn dữ liệu tùy ý và app bị translocation bị từ chối. Bundle QA riêng trong
thư mục tạm cũng được hỗ trợ. SDK/Java bên ngoài, Android Studio, source, DMG đã
tải và các bản app khác vẫn được giữ. Nếu giữ backup, `backups/` còn ở vị trí cũ;
khôi phục về sau vẫn cần SDK/image tương thích. Dung lượng đã cấp phát không bảo
đảm số dung lượng thực tế thu hồi trên APFS.

Không xin quyền quản trị và không tự tiếp tục xóa. Lỗi sau khi Hub thoát sẽ mở
báo cáo văn bản trong thư mục tạm riêng của helper; một phần dữ liệu đã xác nhận
có thể đã bị xóa. App được xóa sau cùng nhưng lỗi ở bước này cũng có thể để lại
bundle dở. Nếu crash để lại `uninstall-pending.json`, các thao tác ghi bị chặn để
kiểm tra; không chạy lại helper hoặc xóa thư mục khác một cách tùy tiện. Kéo app
vào Trash không kích hoạt luồng này và vẫn để lại dữ liệu.

## Điện thoại USB (nhánh feature, US-058–059)

Có trong source nhánh `feature/physical-devices`, chưa phát hành. Chỉ cần Platform-Tools; không bắt cài Java/Emulator/system image.

1. Tải và giải nén [Platform-Tools chính thức](https://developer.android.com/tools/releases/platform-tools), tự xem và chấp nhận license.
2. Trong **Cài đặt Hub → Điện thoại qua USB**, nhập đường dẫn tuyệt đối đến file `platform-tools/adb`, bấm **Kiểm tra và lưu ADB**.
3. Nếu server chưa chạy, chạy lệnh `'<đường dẫn adb>' start-server` được hiển thị trong Terminal. Hub đọc server loopback mặc định cổng 5037, không tự dừng/thay server dùng chung và chưa hỗ trợ endpoint tùy biến.
4. Cắm cáp USB có truyền dữ liệu, mở khóa điện thoại, bật **Tùy chọn nhà phát triển → USB debugging**, tự chấp nhận hộp thoại RSA.
5. Chọn điện thoại trong **THIẾT BỊ KẾT NỐI**. Hub hiện model/Android và trạng thái cấp quyền/kết nối. Cắm/rút tự cập nhật; **Kiểm tra lại USB** thử lại khi đọc thông tin hoặc kết nối lỗi.

Transport mạng/chưa xác định có nhãn riêng; emulator ngoài Hub không được nhập như
điện thoại. Nhận diện không tự mở stream hay đổi settings. Chọn 30/60 FPS và cạnh
dài tối đa 720/1280/1920px rồi bấm **Xem màn hình**. Dừng xem trước khi đổi giới hạn.
Vừa khung/zoom và xoay giữ đúng tỉ lệ. Đây là giới hạn yêu cầu, không cam kết FPS
thực hoặc thay độ phân giải hệ thống điện thoại.

Server scrcpy 5.0.1 đi kèm được đưa vào đường dẫn tạm riêng trên điện thoại và dọn
khi dừng/thoát. Cần giải mã H.264 phần cứng VideoToolbox. Nội dung khóa/bảo vệ có
thể trống; Hub không vượt bảo vệ. Bấm màn hình để chọn đích điều khiển: click/kéo,
cuộn hai trục, Back/Home/Recent và ký tự ASCII dùng kênh control của phiên.
Chưa hỗ trợ IME/Unicode hoặc giữ phím chơi game. Mở cài đặt Hub tạm chặn input.
Trong chế độ hai máy, bấm đúng màn hình trước khi gõ; điện thoại có thể hiển thị
cạnh emulator. Chưa có nút đổi rotation hệ thống điện thoại; xoay điện thoại khi
ứng dụng hỗ trợ. Nếu xem được nhưng không điều khiển được, kiểm tra hướng dẫn
quyền điều khiển USB của hãng. Âm thanh, cài APK và clipboard cho điện thoại vẫn
là phần việc tương lai. Sau lỗi kết nối cần chủ động mở lại stream; input cũ không
được phát lại vào kết nối mới.
Chỉ lưu đường dẫn ADB, metadata điện thoại ở bộ nhớ. Khi xung đột phiên bản,
đồng bộ Platform-Tools với các app ADB khác; Hub không tự kill-server.
