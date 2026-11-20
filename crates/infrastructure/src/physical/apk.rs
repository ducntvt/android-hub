//! Single APK streaming to one validated USB transport; never retry or uninstall.
use super::wire;
use android_hub_domain::PhysicalSession;
use std::{path::Path, time::Duration};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallError {
    File,
    SessionChanged,
    UnknownOutcome,
    Signature,
    Version,
    Abi,
    Permission,
    Storage,
    AlreadyInstalled,
    InvalidApk,
    Rejected,
}
impl InstallError {
    /// Stable UI keys, without package-manager output or user file contents.
    pub fn message(self) -> &'static str {
        match self {
            Self::File => "APK không đọc được hoặc đã thay đổi; chọn lại file",
            Self::SessionChanged => "Phiên USB đã thay đổi; chọn lại thiết bị để cài APK",
            Self::UnknownOutcome => {
                "Chưa xác định kết quả cài APK; kiểm tra trên điện thoại trước khi thử lại"
            }
            Self::Signature => "Chữ ký APK không tương thích; Hub không tự gỡ ứng dụng hiện có",
            Self::Version => "Phiên bản APK không tương thích hoặc thấp hơn bản đã cài",
            Self::Abi => "APK không hỗ trợ kiến trúc CPU của thiết bị",
            Self::Permission => {
                "Điện thoại hoặc chính sách của hãng đã từ chối cài APK; kiểm tra quyền cài qua USB"
            }
            Self::Storage => "Điện thoại không đủ dung lượng để cài APK",
            Self::AlreadyInstalled => "Ứng dụng đã tồn tại; Hub không tự ghi đè hoặc gỡ ứng dụng",
            Self::InvalidApk => "APK không hợp lệ hoặc thiếu thành phần; chỉ hỗ trợ APK đơn",
            Self::Rejected => "Android từ chối cài APK; không tự thử lại hoặc gỡ ứng dụng",
        }
    }
}
fn parse_result(raw: &[u8]) -> Result<(), InstallError> {
    let text = std::str::from_utf8(raw).map_err(|_| InstallError::UnknownOutcome)?;
    // A success-looking substring in a failure message must not become success.
    if text.trim() == "Success" {
        return Ok(());
    }
    if !text.contains("Failure [") {
        return Err(InstallError::UnknownOutcome);
    }
    let error = if text.contains("UPDATE_INCOMPATIBLE")
        || text.contains("INCONSISTENT_CERTIFICATES")
        || text.contains("NO_CERTIFICATES")
    {
        InstallError::Signature
    } else if text.contains("VERSION_DOWNGRADE")
        || text.contains("OLDER_SDK")
        || text.contains("DEPRECATED_SDK_VERSION")
    {
        InstallError::Version
    } else if text.contains("NO_MATCHING_ABIS") || text.contains("CPU_ABI_INCOMPATIBLE") {
        InstallError::Abi
    } else if text.contains("USER_RESTRICTED")
        || text.contains("PERMISSION_DENIED")
        || text.contains("BLOCKED")
        || text.contains("VERIFICATION_FAILURE")
    {
        InstallError::Permission
    } else if text.contains("INSUFFICIENT_STORAGE") {
        InstallError::Storage
    } else if text.contains("ALREADY_EXISTS") {
        InstallError::AlreadyInstalled
    } else if text.contains("INVALID_APK")
        || text.contains("PARSE_FAILED")
        || text.contains("MISSING_SPLIT")
    {
        InstallError::InvalidApk
    } else {
        InstallError::Rejected
    };
    Err(error)
}
fn command(bytes: u64) -> String {
    // Generated numeric size only. -R explicitly disables Android's default
    // replacement behavior; no downgrade, permission grant, user path or serial.
    format!("exec:cmd package install -R --user current -S {bytes} -")
}
pub async fn install(
    token: &PhysicalSession,
    path: &Path,
    expected_bytes: u64,
) -> Result<(), InstallError> {
    if !path.is_absolute()
        || !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("apk"))
        || expected_bytes == 0
    {
        return Err(InstallError::File);
    }
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|_| InstallError::File)?;
    let meta = file.metadata().await.map_err(|_| InstallError::File)?;
    if !meta.is_file() || meta.len() != expected_bytes {
        return Err(InstallError::File);
    }
    tokio::time::timeout(Duration::from_secs(8), wire::validate(token))
        .await
        .map_err(|_| InstallError::SessionChanged)?
        .map_err(|_| InstallError::SessionChanged)?;
    // After requesting the install service, every transport/timeout failure is
    // ambiguous: the package manager may already have committed the install.
    tokio::time::timeout(Duration::from_secs(180), async {
        let mut stream = wire::service(token, &command(expected_bytes))
            .await
            .map_err(|_| InstallError::UnknownOutcome)?;
        let copied = tokio::io::copy(&mut (&mut file).take(expected_bytes), &mut stream)
            .await
            .map_err(|_| InstallError::UnknownOutcome)?;
        if copied != expected_bytes {
            return Err(InstallError::UnknownOutcome);
        }
        stream
            .flush()
            .await
            .map_err(|_| InstallError::UnknownOutcome)?;
        let mut response = Vec::new();
        (&mut stream)
            .take(8193)
            .read_to_end(&mut response)
            .await
            .map_err(|_| InstallError::UnknownOutcome)?;
        if response.len() > 8192 {
            return Err(InstallError::UnknownOutcome);
        }
        wire::validate(token)
            .await
            .map_err(|_| InstallError::UnknownOutcome)?;
        parse_result(&response)
    })
    .await
    .map_err(|_| InstallError::UnknownOutcome)?
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn command_is_streamed_and_disallows_replacement() {
        assert_eq!(
            command(123),
            "exec:cmd package install -R --user current -S 123 -"
        );
    }
    #[test]
    fn only_explicit_success_is_success() {
        assert_eq!(parse_result(b"Success\n"), Ok(()));
        for output in [
            b"".as_slice(),
            b"Success\nFailure [oops]",
            b"installing",
            b"Success-ish",
        ] {
            assert!(parse_result(output).is_err());
        }
        for (code, error) in [
            (
                "INSTALL_FAILED_UPDATE_INCOMPATIBLE",
                InstallError::Signature,
            ),
            ("INSTALL_FAILED_VERSION_DOWNGRADE", InstallError::Version),
            ("INSTALL_FAILED_NO_MATCHING_ABIS", InstallError::Abi),
            ("INSTALL_FAILED_USER_RESTRICTED", InstallError::Permission),
            ("INSTALL_FAILED_INSUFFICIENT_STORAGE", InstallError::Storage),
            (
                "INSTALL_FAILED_ALREADY_EXISTS",
                InstallError::AlreadyInstalled,
            ),
            ("INSTALL_FAILED_INVALID_APK", InstallError::InvalidApk),
        ] {
            assert_eq!(
                parse_result(format!("Failure [{code}]").as_bytes()),
                Err(error)
            );
        }
    }
}
