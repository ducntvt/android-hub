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
- Google Play ARM64 cho Android API 36, package `system-images;android-36;google_apis_playstore;arm64-v8a`.

Tạo máy ưu tiên `ANDROID_HUB_JAVA_HOME`, rồi biến tương thích cũ `P0_JAVA_HOME` trong môi trường khởi chạy Hub, sau đó Java home đã lưu trong Thiết lập SDK, cuối cùng JBR mặc định tại `/Applications/Android Studio.app/Contents/jbr/Contents/Home`. Mở máy đã tạo không cần avdmanager/Java.

Hub kiểm tra file tồn tại, không rỗng và quyền execute cho công cụ. Kiểm tra này không chạy binary để chứng nhận phiên bản, không bảo đảm SDK/image không hỏng và không thay thế kiểm tra boot. Nếu SDK Manager yêu cầu chấp nhận license, người dùng tự xem và chấp nhận trong công cụ đó.

## Sử dụng bản app hiện tại

Mở `Android Hub.app` đã build. Dữ liệu nằm trực tiếp ở `~/Library/Application Support/Android Hub`, độc lập với thư mục source. Nút `+` tạo AVD sạch, không sao chép tài khoản. Chọn máy, cấu hình khi máy đã dừng, lưu rồi khởi động. Đăng nhập Play Store trực tiếp trong Android. Muốn dùng hai máy, tạo máy thứ hai rồi chọn chế độ **2 máy**.

Khi thiếu thành phần, Hub báo danh sách đường dẫn cần kiểm tra và dừng trước avdmanager/launcher. Cài/khôi phục bằng SDK Manager rồi thử lại. Hub không tự chmod công cụ hoặc tải package trong thao tác này.

## Dữ liệu

Dữ liệu nằm trực tiếp tại `~/Library/Application Support/Android Hub` (hoặc `ANDROID_HUB_ROOT` khi chạy QA); không xóa thư mục này để sửa lỗi SDK. Mỗi máy có AVD riêng. Đóng cửa sổ Hub hoặc Cmd+Q sẽ dừng các máy đã đăng ký trước khi thoát; nếu dừng thất bại, Hub giữ cửa sổ mở để xử lý. Force Quit/crash không chạy được luồng này.

Backup/restore cần máy đã dừng, SDK image tương thích và dung lượng cho bản sao. Backup nằm ở `backups/` trong thư mục dữ liệu; dữ liệu trước restore được giữ trong `.previous-*`. Chưa có giao diện dọn các bản dữ liệu cũ.

## Những phần còn đang kiểm chứng

Luồng SDK có download/resume/recovery và xoay log engine đã triển khai. Cài SDK candidate trên máy sạch và benchmark hai game dài hạn còn thiếu. Backup/restore và wheel đã có smoke riêng; giới hạn cụ thể nằm trong story tương ứng, không suy ra gameplay hay hiệu năng từ unit tests.

## Nếu tạo máy bị gián đoạn

Hub báo ID và hai đường dẫn có thể còn dữ liệu dở (`ID.avd`, `ID.ini`) khi avdmanager/configuration/ghi registry thất bại. Các file được giữ lại để kiểm tra. US-019 đã có journal recovery: máy đã chuẩn bị xong được hoàn tất đăng ký khi registry/data còn khớp; file tạo dở chưa sẵn sàng được giữ và ghi biên nhận incomplete. Chưa tự thu hồi file dở. Không xóa toàn bộ runtime hoặc chạy avdmanager với tùy chọn ghi đè để thử lại.

Nếu registry đã ghi thành công nhưng refresh thất bại, thông báo sẽ ghi “Đã tạo máy … nhưng chưa đọc lại được trạng thái”. Khi đó làm mới/mở lại Hub để xem máy, không cần tạo lại. Hub từ chối tạo vào tên có directory, registration hoặc symlink đã tồn tại.

## Dung lượng tạo máy

Hub yêu cầu ít nhất 12 GiB trống trên filesystem chứa AVD trước khi tạo máy mới. Đây là ngưỡng tối thiểu cho profile Pixel 5/API 36 hiện tại, không phải dung lượng cấp phát ngay hoặc cam kết đủ chỗ cho game về sau. Hub không xóa dữ liệu để lấy chỗ. Máy cũ không bị đổi kích thước disk bởi kiểm tra này.

## Log engine

Hub ghi stdout/stderr engine vào data-root/<id>.log. Trước lần khởi động tiếp theo, nếu file đạt 8 MiB, Hub chuyển nó thành .log.1, dịch bản cũ sang .log.2/.log.3 và xóa .log.3 cũ nhất. Tối đa ba archive được giữ cho mỗi máy; sao chép log ra ngoài trước lần mở máy tiếp theo nếu cần giữ lịch sử lâu dài. Không xoay hoặc cắt log khi engine đang chạy, nên một phiên dài vẫn có thể tạo log lớn hơn 8 MiB. Panel Dung lượng dữ liệu tính cả archive vào Log engine.

## Chọn SDK trong Hub

Mở **Thiết lập SDK**, nhập đường dẫn tuyệt đối tới SDK đã cài, rồi bấm **Kiểm tra và lưu SDK** khi tất cả máy đã dừng. Hub kiểm tra Emulator, Platform-Tools, Command-line Tools, Google Play ARM64 API 36 và Java hiện có; không tải hay cài thành phần mới.

Lựa chọn lưu trong data-root/hub-settings.json, áp dụng lần mở sau. ANDROID_SDK_ROOT được ưu tiên cao hơn; nếu biến này đang đặt, bỏ override và mở lại app trước khi lưu từ UI. Khi runtime còn AVD/archive, chỉ chọn lại đúng SDK hiện tại: migration sang SDK khác chưa được hỗ trợ. File cấu hình lỗi cần phục hồi đúng SDK cũ nếu đã có dữ liệu; runtime trống có thể lưu lại từ panel.

## Chọn Java trong Hub

Trong **Thiết lập SDK**, nhập Java home (thư mục chứa `bin/java`) và bấm **Kiểm tra và lưu Java** khi tất cả máy đã dừng. Hub kiểm tra Java 17+ ARM64 và chạy version/list device của SDK hiện tại trong home tạm riêng trước khi lưu. Không tải Java, không khởi động guest. Lựa chọn được giữ qua restart và khi lưu SDK; `ANDROID_HUB_JAVA_HOME` (hoặc `P0_JAVA_HOME` khi biến mới chưa đặt) ưu tiên cao hơn và khóa nút lưu khi đang đặt.

Kiểm tra lỗi giữ nguyên lựa chọn cũ và báo bước thất bại. Panel Java và backend đã qua kiểm tra trên runtime QA riêng; xem US-026 cho bằng chứng. Không thay SDK/Java của máy thật chỉ để kiểm thử.

## Tìm SDK do Hub chuẩn bị

Mục **SDK do Hub chuẩn bị** trong Thiết lập SDK đọc các thư mục SDK đã hoàn tất trong data-root/managed-sdks. Bấm **Làm mới danh sách** để đọc lại. **Kiểm tra và sử dụng** kiểm tra lại SDK/Java rồi lưu cả hai cùng lúc; không chọn tự động sau restart. SDK cài dở hoặc receipt/payload thiếu không được đưa vào danh sách dùng được, dữ liệu vẫn được giữ.

Cần dừng mọi máy và bỏ environment overrides trước khi chọn. Nếu runtime còn AVD/archive gắn SDK khác, thao tác vẫn bị chặn. Luồng tải/chuẩn bị SDK có trong màn hình bản kê; danh sách này chỉ tìm SDK đã được backend assembly hoàn tất. Native panel mới chưa qua smoke trực tiếp.

## Xem bản kê SDK và điều khoản

**Thiết lập SDK → Xem bản kê cài SDK** hiển thị bốn package pinned, tổng download và nguyên văn hai bộ điều khoản. Xem và tự chấp nhận từng bộ điều khoản, nhập Java home đã cài, rồi bấm **Tải và chuẩn bị SDK**. Checkbox không chọn sẵn và không lưu qua đóng/mở review. Chỉ mở màn hình không tải gì.

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
