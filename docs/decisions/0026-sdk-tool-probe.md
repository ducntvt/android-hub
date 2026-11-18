# ADR 0026 — Kiểm tra khả năng chạy SDK trong home riêng

- Status: Accepted
- Date: 2026-11-13
- Story: US-025

## Evidence và quyết định

Launcher avdmanager trong SDK Command-line Tools 23.0 hiện cài gọi Java class `com.android.sdklib.tool.AvdManagerCli` bằng lib/avdmanager-classpath.jar và yêu cầu Java >=17. sdkmanager/android của bản này có bootstrap tự tải CLI; không dùng chúng làm read-only probe.

Backend.probe_sdk_tools(java_home) là thao tác rõ ràng cho installer, không chạy trong refresh hoặc tự sửa SDK. Java root được canonicalize; cần executable Java, SDK adb/emulator và classpath jar. Package validator cũng yêu cầu jar này trước assembly.

Chạy `java -XshowSettings:properties -version` để kiểm tra specification version >=17 và os.arch aarch64/arm64. Sau đó `adb version`, `emulator -no-window -version`, Java trực tiếp gọi AvdManagerCli list device với toolsdir/classpath tương ứng. Mỗi process có timeout 20–30 giây, stdin null, output giới hạn khi đọc theo Backend.command. Không gọi adb devices/start-server, không truyền AVD name, không cài package hoặc chấp nhận license.

Mỗi lần tạo home 0700 riêng trong runtime, với tmp/android/avd riêng. Command env_clear rồi chỉ đặt PATH hệ thống, HOME, TMPDIR, SDK/Java và Android user/AVD paths; LC_ALL=C từ runner. Không kế thừa JAVA_TOOL_OPTIONS/JDK_JAVA_OPTIONS/JAVA_OPTS/AVDMANAGER_OPTS/DYLD_* của Hub. Java nhận user.home, java.io.tmpdir riêng và -XX:-UsePerfData. Không gọi shell wrapper có eval. Các env này chỉ thuộc process con, không thay env của Hub hay OS.

Thành công trả SdkToolProbe (Java home/major/arch và version lines) rồi xóa home tạm do probe tạo. Lỗi giữ home và báo đường dẫn kèm tên bước. Chưa thay cách chọn Java của launcher hiện hành; installer cần lưu lựa chọn Java và nối flow này trước khi publish/select SDK.

## Giới hạn

Chỉ chạy trên SDK tin cậy đã verify archive/package và assembly hoặc SDK hiện có được chọn rõ ràng; không coi API nhận đường dẫn là sandbox cho binary không tin cậy. Home isolation không phải OS sandbox. Version/list device thành công không chứng minh boot, GPU, game hoặc compatibility toàn bộ. ADB protocol version không phải package revision; revision vẫn kiểm tra source.properties. Không gọi probe từ loop refresh.

Java bootstrap/download và UI chọn Java còn thiếu; yêu cầu Java ARM64 là policy của managed installer trên Apple Silicon, không tuyên bố mọi Android tool đều không chạy được qua Rosetta. Probe không giữ process sống sau khi trả về hoặc khởi động guest.

## Kết quả kiểm chứng host

Probe SDK hiện cài đạt: Java 25/aarch64, ADB protocol 1.0.41, emulator 37.2.12.0, AvdManagerCli list device có pixel_5. Home tạm của lượt thành công được dọn, không tạo guest hoặc gọi bootstrap. Đây không phải candidate emulator 37.2.13 trong install plan.

Lượt sandbox dừng ở emulator version không có output; chạy test với quyền ngoài sandbox đi qua bước này. Lượt --help ngoài sandbox trả exit nonzero dù in usage; sửa probe dùng list device trả thành công và kiểm tra profile cần thiết, không bỏ qua exit failure. Logs giữ cả lượt lỗi và lượt đạt trong reports/sdk-probe.
