//! Presentation-only language preference; guest locale and SDK licenses are unchanged.
use std::path::Path;
mod diagnostics;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Language {
    #[default]
    Vi,
    En,
}
impl Language {
    pub fn load(root: &Path) -> Self {
        match std::fs::read_to_string(root.join("ui-language")) {
            Ok(value) if value.trim() == "en" => Self::En,
            _ => Self::Vi,
        }
    }
    pub fn save(self, root: &Path) -> std::io::Result<()> {
        let dir = root;
        std::fs::create_dir_all(dir)?;
        let staging = dir.join(format!(".ui-language-{}.tmp", std::process::id()));
        std::fs::write(&staging, if self == Self::En { "en\n" } else { "vi\n" })?;
        let result = std::fs::rename(&staging, dir.join("ui-language"));
        if result.is_err() {
            let _ = std::fs::remove_file(staging);
        }
        result
    }
    pub fn tr(self, value: &str) -> &str {
        if self == Self::Vi {
            return value;
        }
        match value {
            "Image đã cài tách biệt với cache tải. Hub không tự gỡ image khi xóa máy." => {
                "Installed images are separate from download cache. Deleting a device does not remove its image."
            }
            "Phiên bản Android" => "Android versions",
            "Transaction có image chưa xác định; không gỡ" => {
                "Transaction image is unresolved; removal blocked"
            }
            "Gỡ image" => "Remove image",
            "Tiếp tục gỡ image" => "Continue image removal",
            "Đã gỡ image Android" => "Android image removed",
            "Gỡ image chưa hoàn tất" => "Image removal is incomplete",
            "Không thể gỡ image Android" => "Cannot remove Android image",
            "Đang kiểm tra image trước khi gỡ…" => "Checking image before removal…",
            "Đang gỡ image Android…" => "Removing Android image…",
            "Gỡ image không dùng khỏi SDK do Hub quản lý. Muốn tạo máy sau này cần tải lại image." => {
                "Removes an unused image from the Hub-managed SDK. Creating another device later requires downloading the image again."
            }
            "Đang được tham chiếu bởi:" => "Referenced by:",
            "Chưa xác định dung lượng" => "Size unknown",
            "SDK bên ngoài Hub: chỉ xem, không gỡ image" => {
                "External SDK: view only; image removal is unavailable"
            }
            "Image đang được sử dụng" => "Image is in use",
            "Không đọc được dung lượng hoặc đường dẫn image" => {
                "Cannot read image size or location"
            }
            "Gỡ image chưa hoàn tất; mở Thiết lập SDK để tiếp tục" => {
                "Image removal is incomplete; open SDK setup to continue"
            }
            "Image không có biên nhận do Hub quản lý" => {
                "This image has no Hub ownership receipt"
            }
            "Biên nhận không khớp image hiện tại; không gỡ" => {
                "Receipt does not match the current image; removal blocked"
            }
            "Dữ liệu image thay đổi sau xác nhận; không gỡ" => {
                "Image data changed after confirmation; removal blocked"
            }
            "Image thay đổi sau xác nhận" => "Image changed after confirmation",
            "SDK thay đổi sau xác nhận" => "SDK changed after confirmation",
            "Biên nhận image không hợp lệ" => "Invalid image receipt",
            "Biên nhận gỡ image không hợp lệ" => "Invalid image removal receipt",
            "Biên nhận gỡ image không khớp SDK" => {
                "Image removal receipt does not match the SDK"
            }
            "Biên nhận SDK không hợp lệ" => "Invalid SDK receipt",
            "Thiếu biên nhận SDK" => "SDK receipt is missing",
            "Tên SDK không hợp lệ" => "Invalid SDK name",
            "Đường dẫn image không thuộc thư mục SDK thật" => {
                "Image path is not a real SDK directory"
            }
            "Image có symlink hoặc file đặc biệt; không gỡ" => {
                "Image contains a symlink or special file; removal blocked"
            }
            "Image có quá nhiều file" => "Image contains too many files",
            "Dung lượng image vượt giới hạn" => "Image size exceeds the limit",
            "Đường dẫn gỡ image đã thay đổi" => "Image removal path changed",
            "Lượt gỡ image đã thay đổi" => "Image removal operation changed",
            "Không quét tham chiếu image qua symlink/file" => {
                "Cannot scan image references through a symlink or file"
            }
            "Quá nhiều tham chiếu image để quét an toàn" => {
                "Too many image references to scan safely"
            }
            "Metadata tham chiếu image không hợp lệ" => "Invalid image reference metadata",
            "Dữ liệu AVD không phải thư mục thật" => "AVD data is not a real directory",
            "Backup có image chưa xác định; không gỡ" => {
                "Backup image is unresolved; removal blocked"
            }
            "Tải và tạo máy" => "Download and create",
            "Đang cài image Android…" => "Installing Android image…",
            "Phiên bản Android cố định sau khi tạo. Muốn dùng bản khác, hãy tạo máy mới." => {
                "Android version is fixed after creation. Create a new device to use another version."
            }
            "Image đã cài; chưa tạo máy. Có thể dùng lại khi tạo máy mới." => {
                "Image installed; no device created. You can reuse it when creating a new device."
            }
            "Image trả về không khớp phiên bản đã chọn" => {
                "The installed image does not match the selected Android version"
            }
            "Image đã thay đổi sau khi tải. Kiểm tra lại trước khi tạo máy." => {
                "The image changed after installation. Check again before creating the device."
            }
            "Cài image Android" => "Install Android image",
            "Image đã sẵn sàng" => "Image ready",
            "Đã cài image Android" => "Android image installed",
            "Không cài được image Android" => "Android image installation failed",
            "Tải và cài image" => "Download and install image",
            "Công cụ SDK dùng chung · macOS Apple Silicon" => {
                "Shared SDK tools · macOS Apple Silicon"
            }
            "Cài vào SDK hiện tại. Ước tính thêm 6 GiB staging và 12 GiB cho máy mới, ngoài archive. Hub kiểm tra dung lượng trước tải và trước giải nén; không cập nhật engine hoặc thay image đã có." => {
                "Installs into the current SDK. Allow an estimated 6 GiB for staging and 12 GiB for a new device, plus the archive. Hub checks space before download and extraction; existing images and the engine are not updated."
            }
            "Cần Emulator, Platform-Tools, Command-line Tools và Java. Cài riêng image Google Play ARM64 cho phiên bản Android muốn dùng." => {
                "Requires Emulator, Platform-Tools, Command-line Tools and Java. Install the Google Play ARM64 image separately for the Android version you want."
            }
            "Chưa xác định image máy cũ. Kiểm tra SDK và mở lại Hub." => {
                "Legacy device image is unresolved. Check the SDK and reopen Hub."
            }
            "Phiên input đã thay đổi; hãy thao tác lại" => {
                "Input session changed; try the action again"
            }
            "Input đang bận; hãy thử lại" => "Input is busy; try again",
            "Clipboard không có văn bản" => "The clipboard contains no text",
            "Văn bản clipboard vượt giới hạn 64 KiB" => {
                "Clipboard text exceeds the 64 KiB limit"
            }
            "Thiếu HOME" => "HOME is not set",
            "Đang chờ cổng màn hình của emulator…" => {
                "Waiting for the emulator display port…"
            }
            "Thiếu cổng màn hình" => "Display port is missing",
            "Máy mở bằng phiên bản cũ. Dừng rồi mở lại máy để dùng màn hình trong Hub." => {
                "This device was started by an older version. Stop and restart it to use its display in Hub."
            }
            "Không đọc được engine" => "Could not inspect the engine",
            "Engine đã dừng hoặc thay đổi" => "The engine has stopped or changed",
            "Cổng màn hình không thuộc phiên Hub này; mở lại máy từ Hub" => {
                "The display port does not belong to this Hub session; restart the device from Hub"
            }
            "Cổng không hợp lệ" => "Invalid port",
            "Token không hợp lệ" => "Invalid token",
            "Stream màn hình bị ngắt" => "Display stream disconnected",
            "Engine đã đóng stream" => "The engine closed the display stream",
            "Thiếu định dạng frame" => "Frame format is missing",
            "Frame không hợp lệ" => "Invalid frame",
            "Thiết lập cuộn quá thời gian" => "Scroll setup timed out",
            "Input bị ngắt" => "Input disconnected",
            "Chưa kết nối cuộn" => "Scrolling is not connected",
            "Input quá thời gian" => "Input timed out",
            "Mở stream màn hình quá thời gian" => "Opening the display stream timed out",
            "Không mở được stream màn hình" => "Could not open the display stream",
            "Kết nối input quá thời gian" => "Input connection timed out",
            "Không kết nối được input" => "Could not connect input",
            "Android chưa sẵn sàng sau thời gian chờ; có thể kiểm tra lại." => {
                "Android is not ready after the timeout; you can check again."
            }
            "Engine không còn chạy; chưa xác định nguyên nhân." => {
                "The engine is no longer running; the cause is unknown."
            }
            "APK phải là file thường, không rỗng" => {
                "The APK must be a non-empty regular file"
            }
            "Tên kỹ thuật không thuộc Hub" => "The internal name does not belong to Hub",
            "ID máy không hợp lệ" => "Invalid device ID",
            "Port máy không hợp lệ" => "Invalid device port",
            "Độ phân giải hoặc FPS không được hỗ trợ" => {
                "Unsupported resolution or FPS"
            }
            "Java trả về properties trùng" => "Java returned duplicate properties",
            "Java không trả về specification version" => {
                "Java did not return its specification version"
            }
            "Java cần phiên bản 17 trở lên" => "Java 17 or later is required",
            "avdmanager yêu cầu Java 17 trở lên" => "avdmanager requires Java 17 or later",
            "Java không trả về kiến trúc" => "Java did not return its architecture",
            "Cần Java ARM64 để chạy SDK native trên Apple Silicon" => {
                "ARM64 Java is required to run the native SDK on Apple Silicon"
            }
            "Version output không hợp lệ" => "Invalid version output",
            "Java home cần đường dẫn tuyệt đối" => {
                "Java home requires an absolute path"
            }
            "Java không có executable hợp lệ" => "Java has no valid executable",
            "SDK và Java cần đường dẫn tuyệt đối" => {
                "SDK and Java require absolute paths"
            }
            "avdmanager không liệt kê được profile pixel_5" => {
                "avdmanager could not list the pixel_5 profile"
            }
            "hub-settings.json không phải file cấu hình hợp lệ" => {
                "hub-settings.json is not a valid settings file"
            }
            "Thiếu đường dẫn SDK trong hub-settings.json" => {
                "SDK path is missing from hub-settings.json"
            }
            "Schema/đường dẫn SDK đã lưu không hợp lệ" => {
                "Invalid saved settings schema or SDK path"
            }
            "java_home không phải đường dẫn" => "java_home is not a path",
            "java_home đã lưu phải là đường dẫn tuyệt đối" => {
                "The saved java_home must be an absolute path"
            }
            "ANDROID_SDK_ROOT đang ghi đè SDK; bỏ biến này rồi mở lại Hub để lưu lựa chọn trong app" => {
                "ANDROID_SDK_ROOT overrides the SDK; unset it and reopen Hub to save an SDK choice in the app"
            }
            "Nhập đường dẫn SDK tuyệt đối" => "Enter an absolute SDK path",
            "Dừng tất cả máy Android trước khi đổi SDK" => {
                "Stop all Android devices before changing the SDK"
            }
            "Runtime còn dữ liệu AVD/archive gắn với SDK cũ. Chưa hỗ trợ chuyển SDK cho dữ liệu này; hãy chọn đúng SDK hiện tại." => {
                "Existing AVD or archive data is tied to the old SDK. Migrating this data is not supported; select the current SDK."
            }
            "Không ghi đè đường dẫn cấu hình không phải file thường" => {
                "Cannot overwrite a settings path that is not a regular file"
            }
            "Nhập đường dẫn Java home tuyệt đối" => "Enter an absolute Java home path",
            "Dừng tất cả máy Android trước khi đổi Java" => {
                "Stop all Android devices before changing Java"
            }
            "Dừng tất cả máy trước khi chọn SDK" => {
                "Stop all devices before selecting an SDK"
            }
            "SDK đã được đổi bởi phiên Hub khác; mở lại Hub trước khi tiếp tục" => {
                "Another Hub session changed the SDK; reopen Hub before continuing"
            }
            "đã gỡ khỏi danh sách; dữ liệu được giữ" => {
                "removed from the list; data retained"
            }
            "gỡ máy bị gián đoạn; đã đưa máy về trạng thái trước đó" => {
                "removal was interrupted; the previous device state was restored"
            }
            "đã đưa máy trở lại danh sách" => "restored to the list",
            "đưa máy trở lại bị gián đoạn; dữ liệu vẫn trong archive" => {
                "restoration was interrupted; data remains in the archive"
            }
            "tạo bị gián đoạn; file tạo dở được giữ, chưa đăng ký máy" => {
                "creation was interrupted; partial files retained, device not registered"
            }
            "đã hoàn tất đăng ký máy" => "device registration completed",
            "90/120 FPS: không chọn cao hơn tần số màn hình Mac. Mức cao có thể tăng tải CPU/GPU, nhất là khi chạy nhiều máy." => {
                "90/120 FPS: do not exceed your Mac display's refresh rate. Higher rates may increase CPU/GPU load, especially with multiple devices."
            }
            "Tùy chọn thiết bị" => "Device options",
            "Xóa máy…" => "Delete device…",
            "Dừng máy trước khi xóa" => "Stop the device before deleting",
            "Sao lưu trước…" => "Back up first…",
            "Bản sao được giữ riêng. Hiện chưa hỗ trợ tạo lại máy đã xóa từ bản sao trong giao diện Hub." => {
                "Backups are kept separately. Recreating a deleted device from a backup is not yet available in Hub's interface."
            }
            "Máy đã thay đổi; hãy xác nhận xóa lại" => {
                "Device changed; confirm deletion again"
            }
            "Máy đã thay đổi; đóng hộp thoại và thử lại." => {
                "Device changed. Close this dialog and try again."
            }
            "Chưa xóa hoàn tất. Kiểm tra máy và dữ liệu đã giữ trong Cài đặt Hub." => {
                "Deletion incomplete. Check the device and retained data in Hub Settings."
            }
            "Xóa dữ liệu máy và giải phóng dung lượng. Cần xác nhận trước khi xóa." => {
                "Delete device data to reclaim space. Confirmation is required."
            }
            "Đã sao lưu. Máy chưa bị xóa; bạn có thể Hủy hoặc xác nhận Xóa vĩnh viễn." => {
                "Backup saved. The device has not been deleted; choose Cancel or confirm permanent deletion."
            }
            "Xóa vĩnh viễn…" => "Delete permanently…",
            "Xóa vĩnh viễn" => "Delete permanently",
            "Xóa trước đó chưa hoàn tất; chỉ có thể tiếp tục xóa." => {
                "Previous deletion was interrupted. Only retrying deletion is available."
            }
            "Đang kiểm tra dung lượng…" => "Checking size…",
            "Đang xóa vĩnh viễn máy…" => "Deleting device permanently…",
            "Đã xóa vĩnh viễn máy" => "Device permanently deleted",
            "Chưa xóa hoàn tất. Làm mới danh sách để thử lại." => {
                "Deletion incomplete. Refresh the list to retry."
            }
            "Không thể chuẩn bị xóa máy" => "Could not prepare device deletion",
            "Dữ liệu vẫn chiếm dung lượng. Chọn máy để khôi phục hoặc xóa vĩnh viễn." => {
                "Archived data still uses disk space. Select a device to restore or permanently delete."
            }
            "Toàn bộ ứng dụng, tài khoản và dữ liệu trong máy này sẽ bị xóa, không thể hoàn tác. SDK/image dùng chung và backup riêng được giữ nguyên." => {
                "All apps, accounts and data in this device will be deleted. This cannot be undone. Shared SDK/images and separate backups are kept."
            }
            "Xoay thiết bị" => "Rotate device",
            "Không đọc được hướng thiết bị" => "Could not read device orientation",
            "Cảm biến xoay thiết bị không khả dụng" => {
                "Device rotation sensor unavailable"
            }
            "Sao chép chi tiết" => "Copy details",
            "Đạt" => "Passed",
            "Chưa xác định: không chạy được phép thử" => {
                "Unknown: probe unavailable"
            }
            "Chưa có địa chỉ mạng hoạt động" => "No active network address",
            "Chưa phân giải được tên miền" => "Name resolution failed",
            "Không thiết lập được kết nối TLS" => "TLS connection failed",
            "Quá thời gian chờ" => "Timed out",
            "Kết nối thất bại" => "Connection failed",
            "Máy chủ trả mã HTTP khác dự kiến" => "Unexpected HTTP response",
            "Kết nối Android" => "Android connectivity",
            "Chỉ kiểm tra khi bạn yêu cầu. Không đổi mạng hoặc dữ liệu ứng dụng." => {
                "Runs only on request. Does not change network settings or app data."
            }
            "Đang kiểm tra kết nối…" => "Checking connectivity…",
            "Kiểm tra kết nối" => "Check connectivity",
            "Máy cần khởi động xong để kiểm tra kết nối." => {
                "The device must finish booting before checking connectivity."
            }
            "Giao diện mạng" => "Network interface",
            "Giao diện mạng có địa chỉ không đồng nghĩa đã có Internet." => {
                "An interface address does not prove Internet access."
            }
            "Endpoint kiểm tra truy cập được. Game hoặc YouTube vẫn có thể lỗi riêng." => {
                "The test endpoint is reachable. Games or YouTube may still have app-specific errors."
            }
            "Xem kết quả từng bước; thử lại hoặc kiểm tra mạng và thời gian trong Android. Hub không tự sửa cấu hình." => {
                "Review each step; retry or check network settings and time in Android. Hub does not repair settings automatically."
            }
            "Chưa xác định được kết nối. Không chạy được phép thử hoặc phiên Android đã thay đổi. Hãy thử lại." => {
                "Connectivity is unknown. The probe could not run or the Android session changed. Try again."
            }
            "Kiểm tra giao diện mạng, DNS và HTTPS từ guest; có thể mất khoảng 45 giây." => {
                "Checks the interface, DNS and HTTPS inside the guest; may take about 45 seconds."
            }
            "Ẩn chi tiết" => "Hide details",
            "Máy nguồn" => "Source device",
            "Thời gian tạo" => "Created",
            "Dung lượng theo metadata" => "Size from metadata",
            "Mới đọc metadata; chưa kiểm tra checksum dữ liệu hoặc khả năng khôi phục." => {
                "Metadata only; payload checksums and restore compatibility have not been verified."
            }
            "Máy đang quản lý" => "Managed devices",
            "Bản sao lưu" => "Backups",
            "Dữ liệu thao tác dở" => "Incomplete operation data",
            "Dữ liệu trước khôi phục" => "Pre-restore data",
            "Nhật ký máy" => "Engine logs",
            "Dữ liệu runtime khác" => "Other runtime data",
            "Đo lúc" => "Measured at",
            "Chỉ đo dữ liệu trong runtime; không gồm SDK/image bên ngoài." => {
                "Measures runtime data only; excludes external SDKs and images."
            }
            "Chưa đo được dung lượng. Hãy thử lại." => {
                "Could not measure storage. Try again."
            }
            "Đây không phải dung lượng có thể giải phóng." => {
                "This is not an estimate of reclaimable space."
            }
            "Ước lượng block đã cấp phát, không cộng lặp hardlink. APFS có thể chia sẻ block; dữ liệu đang chạy có thể thay đổi. Symlink, file đặc biệt và volume khác được bỏ qua." => {
                "Allocated blocks, with hardlinks counted once. APFS may share blocks; running devices can change data. Symlinks, special files and other volumes are skipped."
            }
            "Dữ liệu thao tác dở chỉ gồm staging khôi phục và backup partial đã nhận diện. Mục chưa rõ được giữ trong dữ liệu khác; không tự xóa." => {
                "Incomplete data includes recognized restore staging and partial backups only. Unknown entries remain in Other data; nothing is deleted automatically."
            }
            "Các thao tác dữ liệu yêu cầu máy đã dừng." => {
                "Data changes require a stopped device."
            }
            "Tạo bản sao có thể cần vài phút và dung lượng tương đương dữ liệu máy." => {
                "A backup may take several minutes and space comparable to the device data."
            }
            "Không đọc được danh sách. Hãy thử làm mới." => {
                "Could not read the list. Try refreshing."
            }
            "Chưa tìm thấy bản sao có metadata phù hợp cho máy này." => {
                "No backup with matching metadata was found for this device."
            }

            "Cài đặt Hub" => "Hub settings",
            "Phím tắt Hub" => "Hub shortcuts",
            "Tab/Shift-Tab: chuyển control · Enter/Space: chọn · ↑/↓: đổi giá trị. Từ Android, dùng ⌘F để quay lại Hub." => {
                "Tab/Shift-Tab: move focus · Enter/Space: activate · ↑/↓: change a value. From Android, use ⌘F to return to Hub."
            }
            "⌘, Cài đặt Hub · ⌘F Tìm máy\n⌘N Tạo máy · ⌘1/2 Số máy\n⌘0 Vừa khung · ⌘B Danh sách" => {
                "⌘, Hub settings · ⌘F Find device\n⌘N Create device · ⌘1/2 Device count\n⌘0 Fit · ⌘B Sidebar"
            }
            "Ngôn ngữ" => "Language",
            "Giao diện" => "Appearance",
            "Hệ thống" => "System",
            "Sáng" => "Light",
            "Tối" => "Dark",
            "Môi trường và dữ liệu" => "Environment and storage",
            "Chỉ thay đổi giao diện Hub; máy Android tiếp tục chạy." => {
                "Only changes Hub appearance; Android devices keep running."
            }
            "Không lưu được giao diện" => "Could not save appearance",
            "Phóng to" => "Zoom in",
            "Thu nhỏ" => "Zoom out",

            "Tên máy phải có 1–48 ký tự" => "Use 1–48 characters for the device name.",
            "Tên máy đã có trong danh sách. Chọn tên khác trước khi tạo." => {
                "This device name is already in use. Choose another name."
            }
            "Đã tìm thấy các thành phần SDK và Java." => {
                "SDK and Java components were found."
            }
            "Thiếu thành phần SDK/Java hoặc cấu hình không còn hợp lệ." => {
                "SDK/Java components are missing or settings are no longer valid."
            }
            "Chưa đọc được dung lượng trống." => "Available space could not be read.",
            "Đang kiểm tra môi trường…" => "Checking prerequisites…",
            "Cần tối thiểu 12 GiB trống để tạo máy; không bảo đảm dung lượng cho game về sau." => {
                "Creation requires at least 12 GiB of free space; this does not guarantee space for future games."
            }
            "Ẩn cấu hình nâng cao" => "Hide advanced settings",
            "Cấu hình nâng cao" => "Advanced settings",
            "Khởi động sau khi tạo" => "Start after creation",
            "Có dữ liệu tạo dở cần kiểm tra. Hub không tự xóa hoặc tạo lại." => {
                "Creation may have retained data. Review it before creating again; Hub does not delete or retry automatically."
            }
            "Tạo máy" => "Create device",
            "Máy đã được tạo. Không cần tạo lại." => {
                "The device was created. Do not create it again."
            }
            "Máy đã được tạo. Làm mới trạng thái trước khi khởi động." => {
                "The device was created. Refresh its status before starting it."
            }
            "Chưa lưu" => "Unsaved",
            "Đã lưu" => "Saved",
            "Thiết bị" => "Device",
            "Hiệu năng" => "Performance",
            "Dữ liệu" => "Data",
            "Tên máy phải có 1–48 ký tự, không có ký tự điều khiển." => {
                "Use 1–48 characters without control characters."
            }
            "Cấu hình đã thay đổi bên ngoài. Bản nháp của bạn vẫn được giữ." => {
                "Settings changed elsewhere. Your draft has been kept."
            }
            "Nạp lại cấu hình" => "Reload settings",
            "Giá trị chưa được hỗ trợ." => "This value is not supported.",
            "FPS là giới hạn cấu hình, không phải FPS game đang đo." => {
                "FPS is a configured limit, not a measured game frame rate."
            }
            "Thay đổi áp dụng ở lần khởi động máy tiếp theo." => {
                "Changes apply the next time this device starts."
            }
            "Hủy thay đổi" => "Discard edits",
            "Ở lại" => "Stay",
            "Bỏ thay đổi" => "Discard changes",
            "Lưu và tiếp tục" => "Save and continue",
            "Cấu hình đang có thay đổi chưa lưu. Chọn cách tiếp tục." => {
                "You have unsaved settings. Choose how to continue."
            }
            "Cấu hình đã thay đổi bên ngoài. Ở lại để xem bản nháp hoặc bỏ thay đổi để dùng cấu hình mới nhất." => {
                "Settings changed elsewhere. Stay to review your draft or discard it to use the latest saved settings."
            }
            "Chỉ lưu được cấu hình hợp lệ khi máy đã dừng và Hub có trạng thái mới nhất." => {
                "Saving requires valid settings, a stopped device and up-to-date device status."
            }
            "Máy đích không còn trong danh sách." => {
                "The target device is no longer in the list."
            }
            "Lưu hoặc hủy thay đổi trước khi thao tác dữ liệu." => {
                "Save or discard your edits before changing device data."
            }
            "Thành công" => "Success",
            "Thông tin" => "Information",
            "Cần xử lý" => "Action needed",
            "Chưa xác định kết quả" => "Outcome unknown",
            "Thao tác chưa hoàn tất" => "Operation could not be completed",
            "Không đọc được trạng thái máy" => "Could not read device status",
            "Chưa đọc được trạng thái máy" => "Device status unavailable",
            "Chưa tạo được máy" => "Could not create device",
            "Không khởi động được máy" => "Could not start device",
            "Chưa dừng được máy" => "Could not stop device",
            "Không lưu được cấu hình" => "Could not save settings",
            "Đã cài APK. Hub không tự mở ứng dụng." => {
                "APK installed. Hub does not launch the app automatically."
            }
            "Chưa xác nhận cài APK thành công" => "APK installation not confirmed",
            "Chưa xác nhận đưa máy trở lại" => "Device recovery not confirmed",
            "Máy đã đổi phiên; hãy xác nhận lại." => {
                "The device session changed. Please confirm again."
            }
            "Bản tóm tắt không chứa đường dẫn hoặc nội dung Android." => {
                "This summary excludes paths and Android content."
            }
            "Sao chép lỗi chỉ lấy bản tóm tắt, không lấy nội dung Android hoặc đường dẫn." => {
                "Copy error includes only a summary, excluding Android content and paths."
            }
            "Các ứng dụng trên máy này sẽ đóng. Máy khác vẫn tiếp tục chạy." => {
                "Apps on this device will close. Other devices will keep running."
            }
            "Hub sẽ cài file đã chọn và không tự mở ứng dụng." => {
                "Hub will install the selected file without launching the app."
            }
            "Tab: chuyển nút · Enter: chọn nút có viền · Esc: đóng" => {
                "Tab: switch button · Enter: activate outlined button · Esc: close"
            }
            "Chi tiết" => "Details",
            "Sao chép lỗi" => "Copy error",
            "Lịch sử" => "History",
            "Lịch sử · Có cập nhật" => "History · New updates",
            "Lịch sử dữ liệu" => "Data history",
            "Chi tiết thao tác tạo và lưu trữ máy" => {
                "Device creation and archive details"
            }
            "Chưa có lịch sử dữ liệu." => "No data history yet.",
            "Đã xem" => "Mark as read",
            "Không đọc được lịch sử restore" => "Could not read restore history",
            "Không đọc được lịch sử lưu trữ máy" => {
                "Could not read archive history"
            }
            "Không đọc được lịch sử tạo máy" => "Could not read creation history",
            "Đã khôi phục dữ liệu. Bản cũ vẫn được giữ." => {
                "Data restored. Previous data is retained."
            }
            "Chưa xác nhận khôi phục thành công" => "Data restore not confirmed",
            "Đã sao lưu dữ liệu" => "Data backed up",
            "Không sao lưu được dữ liệu" => "Could not back up data",
            "Đã giữ cache SDK cũ" => "Previous SDK cache retained",
            "Không chuyển được cache SDK" => "Could not move SDK cache",
            "Không chuẩn bị được SDK" => "Could not prepare SDK",
            "Đã chuẩn bị SDK" => "SDK prepared",
            "Thông tin thiết lập Hub" => "Hub setup information",
            "Không gửi được văn bản tới Android" => "Could not send text to Android",
            "Cần kiểm tra máy" => "Device needs attention",
            "Đang kiểm tra Android…" => "Checking Android…",
            "Kiểm tra lại" => "Check again",
            "Chưa dừng được" => "Could not stop",
            "Đang chờ dừng" => "Waiting to stop",
            "Android đã khởi động" => "Android boot completed",
            "Sẵn sàng điều khiển" => "Ready to control",
            "Mất kết nối màn hình · Đang kết nối lại…" => {
                "Display disconnected · Reconnecting…"
            }
            "Android đang khởi động…" => "Android is booting…",
            "Chưa xác minh được Android" => "Android status unavailable",
            "Thao tác máy thất bại" => "Device operation failed",
            "Đang mở engine…" => "Starting engine…",
            "1 máy" => "1 device",
            "2 máy" => "2 devices",
            "Lưu cấu hình trước khi khởi động máy." => {
                "Save settings before starting the device."
            }
            "Chưa có máy Android" => "No Android devices yet",
            "Tạo máy để bắt đầu sử dụng Android trong Hub." => {
                "Create a device to start using Android in Hub."
            }
            "Tạo máy Android" => "Create Android device",
            "Hiện danh sách" => "Show sidebar",
            "Ẩn danh sách" => "Hide sidebar",
            "Vừa khung" => "Fit",
            "khung" => "of Fit",
            "Chọn máy" => "Choose device",
            "Chọn một máy để hiển thị tại đây." => {
                "Choose a device to display here."
            }
            "Đang nhận bàn phím" => "Keyboard active",
            "Chép vào Android" => "Copy to Android",
            "Lưu cấu hình trước khi chuyển sang máy khác." => {
                "Save settings before switching to another device."
            }
            "Màn hình chính" => "Home",
            "Ứng dụng gần đây" => "Recent apps",
            "Chép vào clipboard Android; nhấn giữ ô nhập để dán." => {
                "Copy to the Android clipboard; long-press a text field to paste."
            }

            "●  Đang chạy" => "●  Running",
            "Đã dừng" => "Stopped",
            "Không tìm thấy thiết bị" => "No devices found",
            "Tìm thiết bị…" => "Search devices…",
            "THIẾT BỊ" => "DEVICES",
            "Máy đã lưu trữ" => "Archived devices",
            "Dừng tất cả máy trước khi lưu SDK hoặc Java." => {
                "Stop all devices before saving SDK or Java."
            }
            "Chưa hỗ trợ đổi SDK khi còn máy hoặc bản lưu trữ." => {
                "Switching SDKs is not supported while devices or archives exist."
            }
            "Chọn thư mục chứa bin/java." => "Choose the folder containing bin/java.",
            "Thông tin thiết lập" => "Setup details",
            "Thiết lập SDK" => "SDK setup",
            "Dung lượng dữ liệu" => "Storage usage",
            "Kéo APK vào để cài lên máy đã chọn" => {
                "Drop an APK to install on the selected device"
            }
            "Chưa thể đóng Hub" => "Unable to close Hub",
            "Đang chờ thao tác hiện tại hoàn tất…" => {
                "Waiting for the current operation to finish…"
            }
            "Đang dừng các máy Android…" => "Stopping Android devices…",
            "Hub sẽ đóng sau khi các máy đã dừng." => {
                "Hub will close once all devices have stopped."
            }
            "Quay lại Hub" => "Return to Hub",
            "Thử lại" => "Retry",
            "Thiết bị của bạn" => "Your devices",
            "Android · Hai thiết bị" => "Android · Two devices",
            "Một máy" => "Single view",
            "Hai máy" => "Dual view",
            "Đóng cấu hình" => "Close settings",
            "Cấu hình" => "Settings",
            "Đang xác minh trạng thái thiết bị" => "Checking device status",
            "Thiết bị Android độc lập" => "Independent Android device",
            "Đang xử lý…" => "Working…",
            "Khởi động" => "Start",
            "Chép văn bản vào Android" => "Copy text to Android",
            "Đang kết nối màn hình…" => "Connecting display…",
            "Lịch sử thao tác dữ liệu gần nhất" => "Recent data operations",
            "đã đưa dữ liệu cũ trở lại sau gián đoạn" => {
                "previous data recovered after interruption"
            }
            "đã khôi phục thành công" => "restored successfully",
            "Ẩn" => "Hide",
            "Dữ liệu và cấu hình hiện tại sẽ được thay bằng bản sao. Dữ liệu cũ được giữ để phục hồi." => {
                "The backup will replace current data and settings. Previous data is retained for recovery."
            }
            "Hủy" => "Cancel",
            "Khôi phục dữ liệu" => "Restore data",
            "Khôi phục dữ liệu đã giữ và cấu hình cũ. Máy chưa tự khởi động." => {
                "Restore archived data and settings. The device will not start automatically."
            }
            "Đưa máy trở lại" => "Restore device",
            "Đóng" => "Close",
            "Cài APK…" => "Install APK…",
            "Chọn một file APK" => "Choose one APK file",
            "Chọn file APK hiện chỉ hỗ trợ macOS" => {
                "The APK picker is currently supported on macOS only"
            }
            "Khởi động một máy trước khi cài APK" => {
                "Start a device before installing an APK"
            }
            "Thiết bị đã thay đổi; chọn lại APK để xác nhận" => {
                "The device changed; select the APK again to confirm"
            }
            "Không đọc được file APK" => "Could not read the APK file",
            "Đang đọc thông tin file APK…" => "Reading APK file information…",
            "Chọn máy nhận APK. Hub sẽ hỏi xác nhận trước khi cài." => {
                "Choose the APK destination. Hub will ask for confirmation before installing."
            }
            "Thả một file APK, sau đó chọn máy nhận" => {
                "Drop one APK, then choose its destination"
            }
            "Đường dẫn" => "Path",
            "Cài APK" => "Install APK",
            "Dừng máy sẽ đóng game đang chạy." => {
                "Stopping the device will close running games."
            }
            "Dừng máy" => "Stop",
            "Đang đo…" => "Measuring…",
            "Đo lại" => "Measure again",
            "Số đo gần nhất, ước lượng block đã cấp phát trong runtime; không gồm SDK/image bên ngoài. APFS có thể chia sẻ block nên đây không phải lượng có thể giải phóng. Máy đang chạy có thể làm số đo thay đổi." => {
                "Latest estimate of allocated runtime blocks, excluding external SDK/images. APFS may share blocks, so this is not reclaimable space. Running devices may change these values."
            }
            "Không có file nào được xóa khi đo dung lượng." => {
                "Measuring storage does not delete files."
            }
            "Đang đọc…" => "Loading…",
            "Làm mới" => "Refresh",
            "Không có máy đã lưu trữ trong danh sách này." => {
                "No archived devices in this list."
            }
            "Đưa máy trở lại…" => "Restore device…",
            "Chuẩn bị SDK mới" => "Prepare a new SDK",
            "Quay lại" => "Back",
            "Tải lại cache gói này" => "Download this package again",
            "Giữ riêng và tải lại sau" => "Set aside and retry later",
            "Bỏ qua" => "Dismiss",
            "Tôi chấp nhận điều khoản này" => "I accept these terms",
            "Java home (Java 17+ ARM64, đã cài trên máy)" => {
                "Java home (Java 17+ ARM64, already installed)"
            }
            "Chuẩn bị…" => "Preparing…",
            "Kiểm tra Java…" => "Checking Java…",
            "Kiểm tra dung lượng…" => "Checking disk space…",
            "Giải nén, ghép và kiểm tra SDK…" => {
                "Extracting, assembling and verifying SDK…"
            }
            "Đã hoàn tất" => "Completed",
            "Đã hủy" => "Canceled",
            "Không hoàn tất" => "Incomplete",
            "Hủy chuẩn bị SDK" => "Cancel SDK setup",
            "Tải và chuẩn bị SDK" => "Download and prepare SDK",
            "Cần thêm tối thiểu 6 GiB trống để giải nén, ngoài archive tải xuống. SDK mới không được chọn tự động. Nếu hủy/lỗi, có thể thử lại từ cache; đóng màn hình sẽ xóa lựa chọn điều khoản." => {
                "Allow at least 6 GiB of free space for extraction, in addition to downloads. The new SDK is not selected automatically. Retry from cache after cancellation or errors; closing this screen clears license selections."
            }
            "Đường dẫn Android SDK đã cài" => "Installed Android SDK path",
            "Kiểm tra và lưu SDK" => "Verify and save SDK",
            "ANDROID_SDK_ROOT đang ưu tiên. Bỏ biến môi trường rồi mở lại Hub để đổi SDK tại đây." => {
                "ANDROID_SDK_ROOT overrides this setting. Unset it and reopen Hub to change the SDK here."
            }
            "Đường dẫn được lưu cho những lần mở Hub sau." => {
                "This path is saved for future launches."
            }
            "Cần Emulator, Platform-Tools, Command-line Tools, Google Play ARM64 API 36 và Java. Đây là chọn SDK có sẵn; chưa tải hoặc cài SDK." => {
                "Requires Emulator, Platform-Tools, Command-line Tools, Google Play ARM64 API 36 and Java. This selects an existing SDK without downloading it."
            }
            "Kiểm tra và lưu Java" => "Verify and save Java",
            "Biến môi trường Java đang ưu tiên. Bỏ ANDROID_HUB_JAVA_HOME/P0_JAVA_HOME rồi mở lại Hub để đổi Java tại đây." => {
                "A Java environment variable overrides this setting. Unset ANDROID_HUB_JAVA_HOME/P0_JAVA_HOME and reopen Hub to change Java here."
            }
            "Chọn thư mục chứa bin/java. Kiểm tra Java và công cụ SDK hiện tại trước khi lưu; có thể mất khoảng một phút." => {
                "Choose the folder containing bin/java. Java and current SDK tools are verified before saving; this may take about a minute."
            }
            "SDK do Hub chuẩn bị" => "SDKs prepared by Hub",
            "Làm mới danh sách" => "Refresh list",
            "Chưa có SDK hoàn tất." => "No completed SDKs yet.",
            "Kiểm tra và sử dụng" => "Verify and use",
            "Chọn một thiết bị để cấu hình" => "Select a device to configure",
            "Cấu hình thiết bị" => "Device settings",
            "Tên thiết bị" => "Device name",
            "Tên máy" => "Device name",
            "Bộ nhớ · MiB" => "Memory · MiB",
            "Số CPU" => "CPU cores",
            "Giới hạn FPS" => "FPS limit",
            "Chiều khung màn hình" => "Display orientation",
            "Áp dụng lần khởi động sau. Game vẫn có thể tự chọn chiều màn hình." => {
                "Applies on next start. Games may choose their own orientation."
            }
            "Độ phân giải" => "Resolution",
            "Lưu cấu hình" => "Save settings",
            "Đang xử lý thao tác. Vui lòng chờ." => {
                "An operation is in progress. Please wait."
            }
            "Đang chờ cập nhật trạng thái thiết bị." => "Waiting for device status.",
            "Dừng máy để chỉnh cấu hình." => "Stop the device to edit settings.",
            "Tài nguyên đang dùng" => "Resource usage",
            "CPU 100% = một core.\nRSS chưa gồm GPU/helper." => {
                "CPU 100% = one core.\nRSS excludes GPU/helper processes."
            }
            "Sao lưu dữ liệu" => "Back up data",
            "Tạo bản sao khi máy đã dừng. Có thể cần vài phút và dung lượng tương đương dữ liệu máy." => {
                "Back up a stopped device. This may take a few minutes and space equivalent to its data."
            }
            "Tạo bản sao lưu" => "Create backup",
            "Đang tìm bản sao…" => "Finding backups…",
            "Tìm / làm mới bản sao" => "Find / refresh backups",
            "Chọn bản sao…" => "Choose a backup…",
            "Bản sao sẽ được kiểm tra đầy đủ trước khi thay dữ liệu." => {
                "The backup is verified before replacing data."
            }
            "Khôi phục…" => "Restore…",
            "Dừng máy để khôi phục bản sao." => "Stop the device to restore a backup.",
            "Mỗi thiết bị giữ dữ liệu ứng dụng và tài khoản riêng." => {
                "Each device keeps its own app data and accounts."
            }
            "Dọc" => "Portrait",
            "Ngang" => "Landscape",
            "Image đã cấu hình" => "Configured image",
            "Chưa xác định phiên bản Android" => "Android version unavailable",
            "Không lưu được ngôn ngữ" => "Could not save language",
            "Đang đọc danh sách máy…" => "Loading devices…",
            "Đang tạo máy mới…" => "Creating a device…",
            "Đang đưa máy trở lại…" => "Restoring device…",
            "Đang mở Android trong Hub…" => "Opening Android in Hub…",
            "Đang dừng máy…" => "Stopping device…",
            "Đang lưu cấu hình…" => "Saving settings…",
            "Đang cài APK…" => "Installing APK…",
            "Đang tải…" => "Loading…",
            "Máy đã thay đổi; hãy xác nhận lưu trữ lại" => {
                "The device changed; confirm archive again"
            }
            "Máy đã thay đổi; hãy chọn và xác nhận backup lại" => {
                "The device changed; select and confirm the backup again"
            }
            "Đang kiểm tra và khôi phục dữ liệu…" => "Verifying and restoring data…",
            "Đang sao lưu và kiểm tra dữ liệu…" => "Backing up and verifying data…",
            "Đang chờ xác nhận một APK. Hủy lựa chọn hiện tại trước khi kéo file khác." => {
                "An APK is awaiting confirmation. Cancel it before dropping another file."
            }
            "Chọn máy đang chạy trước khi kéo APK vào Hub" => {
                "Select a running device before dropping an APK"
            }
            "Chỉ hỗ trợ một file .apk; chưa hỗ trợ XAPK/APK split" => {
                "Only a single .apk is supported; XAPK/split APK is not supported yet"
            }
            "Thiết bị đã thay đổi; kéo lại APK để xác nhận" => {
                "The device changed; drop the APK again to confirm"
            }
            "Nhập Java home tuyệt đối trước khi bắt đầu" => {
                "Enter an absolute Java home path before starting"
            }
            "Đang chuẩn bị SDK mới…" => "Preparing a new SDK…",
            "Đang chuyển cache cũ sang thư mục riêng…" => "Moving the old cache aside…",
            "Đang hủy; chờ thao tác hiện tại kết thúc…" => {
                "Canceling; waiting for the current operation…"
            }
            "Đang kiểm tra lại SDK và Java trước khi sử dụng…" => {
                "Verifying SDK and Java before use…"
            }
            "Đã chọn SDK và Java; lựa chọn được lưu cho lần mở sau." => {
                "SDK and Java selected; preferences saved for next launch."
            }
            "Đang kiểm tra Java và các công cụ SDK…" => "Verifying Java and SDK tools…",
            "Đã lưu Java; Hub dùng lựa chọn này khi tạo máy Android mới." => {
                "Java saved; Hub will use it to create new Android devices."
            }
            "Đang kiểm tra và lưu SDK…" => "Verifying and saving SDK…",
            "Đã lưu SDK; lựa chọn này áp dụng cho những lần mở Hub tiếp theo." => {
                "SDK saved for future launches."
            }
            "Màn hình đang kết nối lại; hãy thử lại sau" => {
                "Display reconnecting; please try again shortly"
            }
            "Đang gửi văn bản tới Android…" => "Sending text to Android…",
            "Đã chép. Nhấn giữ trong Android để dán." => {
                "Copied. Long-press in Android to paste."
            }
            "Clipboard không có văn bản đọc được" => {
                "No readable text in the clipboard"
            }
            "Sẵn sàng" => "Ready",
            "Đã tạo máy Android độc lập" => "Independent Android device created",
            "Đã mở engine. Android có thể cần thêm thời gian để boot." => {
                "Engine started. Android may need more time to boot."
            }
            "Đã dừng máy" => "Device stopped",
            "Đã lưu. Cấu hình áp dụng khi mở máy." => {
                "Saved. Settings apply on next start."
            }
            "Đã cài APK vào máy đã chọn" => "APK installed on the selected device",
            "Đã đưa máy trở lại. Chọn máy và Khởi động để dùng." => {
                "Device restored. Select it and press Start to use it."
            }
            _ => value,
        }
    }
}
macro_rules! localized_format {
    ($lang:expr, $vi:literal, $en:literal $(, $arg:expr)* $(,)?) => {
        match $lang {
            crate::i18n::Language::Vi => format!($vi $(, $arg)*),
            crate::i18n::Language::En => format!($en $(, $arg)*),
        }
    };
}
pub(crate) use localized_format;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn language_roundtrip_preserves_other_settings_and_unknown_text() {
        let root = std::env::temp_dir().join(format!(
            "hub-language-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert_eq!(Language::load(&root), Language::Vi);
        Language::En.save(&root).unwrap();
        let settings = root.join("hub-settings.json");
        std::fs::write(&settings, "keep-sdk-settings").unwrap();
        assert_eq!(Language::load(&root), Language::En);
        assert_eq!(Language::En.tr("Khởi động"), "Start");
        assert_eq!(
            Language::En.tr("My Android /test/path"),
            "My Android /test/path"
        );
        Language::Vi.save(&root).unwrap();
        assert_eq!(Language::load(&root), Language::Vi);
        assert_eq!(
            std::fs::read_to_string(settings).unwrap(),
            "keep-sdk-settings"
        );
        std::fs::write(root.join("ui-language"), "unknown").unwrap();
        assert_eq!(Language::load(&root), Language::Vi);
        std::fs::remove_dir_all(root).unwrap();
    }
}
