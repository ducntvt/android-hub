//! Read-only prerequisites. Never installs SDK packages or accepts licenses.
use crate::Backend;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
const IMAGE: &str = "system-images/android-36/google_apis_playstore/arm64-v8a";

fn usable(path: &Path, executable: bool) -> bool {
    fs::metadata(path).is_ok_and(|m| {
        m.is_file() && m.len() > 0 && (!executable || m.permissions().mode() & 0o111 != 0)
    })
}
fn missing(sdk: &Path, java: Option<&Path>) -> Vec<String> {
    let mut result = vec![];
    for (relative, label) in [
        ("emulator/emulator", "Android Emulator"),
        ("platform-tools/adb", "Android SDK Platform-Tools"),
    ] {
        let path = sdk.join(relative);
        if !usable(&path, true) {
            result.push(format!(
                "{label}: {} (thiếu hoặc không thực thi được)",
                path.display()
            ));
        }
    }
    if let Some(java) = java {
        if !usable(&sdk.join("emulator/lib/hardware-properties.ini"), false) {
            result.push("Android Emulator: thiếu lib/hardware-properties.ini".into());
        }
        if let Err(error) = super::package::require_userdata_source(&sdk.join(IMAGE)) {
            result.push(format!("Google Play ARM64 API 36: {error}"));
        }
        for (path, label) in [
            (
                sdk.join("cmdline-tools/latest/bin/avdmanager"),
                "Android SDK Command-line Tools (latest)",
            ),
            (
                java.join("bin/java"),
                "Java của Android Studio (hoặc ANDROID_HUB_JAVA_HOME)",
            ),
        ] {
            if !usable(&path, true) {
                result.push(format!(
                    "{label}: {} (thiếu hoặc không thực thi được)",
                    path.display()
                ));
            }
        }
        for name in [
            "source.properties",
            "system.img",
            "vendor.img",
            "ramdisk.img",
            "kernel-ranchu",
            "encryptionkey.img",
        ] {
            let path = sdk.join(IMAGE).join(name);
            if !usable(&path, false) {
                result.push(format!(
                    "Google Play ARM64 API 36: {} (thiếu hoặc rỗng)",
                    path.display()
                ));
            }
        }
    }
    result
}
impl Backend {
    fn saved_settings(&self) -> Result<Option<(PathBuf, Option<PathBuf>)>, String> {
        let path = self.runtime().join("hub-settings.json");
        let meta = match fs::symlink_metadata(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
            Ok(meta) => meta,
        };
        if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 65536 {
            return Err("hub-settings.json không phải file cấu hình hợp lệ".into());
        }
        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).map_err(crate::err)?).map_err(crate::err)?;
        let sdk = value["sdk"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or("Thiếu đường dẫn SDK trong hub-settings.json")?;
        let sdk = PathBuf::from(sdk);
        if value["schema"] != 1 || !sdk.is_absolute() {
            return Err("Schema/đường dẫn SDK đã lưu không hợp lệ".into());
        }
        let java = match value.get("java_home") {
            None | Some(serde_json::Value::Null) => None,
            Some(value) => {
                let path = PathBuf::from(value.as_str().ok_or("java_home không phải đường dẫn")?);
                if !path.is_absolute() {
                    return Err("java_home đã lưu phải là đường dẫn tuyệt đối".into());
                }
                Some(path)
            }
        };
        Ok(Some((sdk, java)))
    }
    pub(crate) fn saved_sdk(&self) -> Result<Option<PathBuf>, String> {
        Ok(self.saved_settings()?.map(|(sdk, _)| sdk))
    }
    /// Select an already-installed SDK; never downloads or accepts licenses.
    pub fn configure_sdk(&self, path: &Path) -> Result<PathBuf, String> {
        if std::env::var_os("ANDROID_SDK_ROOT").is_some() {
            return Err("ANDROID_SDK_ROOT đang ghi đè SDK; bỏ biến này rồi mở lại Hub để lưu lựa chọn trong app".into());
        }
        if !path.is_absolute() {
            return Err("Nhập đường dẫn SDK tuyệt đối".into());
        }
        let sdk = path.canonicalize().map_err(crate::err)?;
        let candidate = Self {
            root: self.root.clone(),
            sdk: sdk.clone(),
        };
        // Explicit SDK repair may replace malformed preferences; normal launch fails closed.
        let java = self
            .java_home()
            .unwrap_or_else(|_| Self::default_java_home());
        let absent = missing(&candidate.sdk, Some(&java));
        if !absent.is_empty() {
            return Err(format!(
                "Môi trường Android chưa sẵn sàng:\n{}",
                absent.join("\n")
            ));
        }
        let _guard = self.lock()?;
        self.ensure_sdk_switch_allowed(&sdk)?;
        let state = self.snapshot()?;
        if !state.runtime.is_empty() {
            return Err("Dừng tất cả máy Android trước khi đổi SDK".into());
        }
        for profile in &state.profiles {
            self.ensure_stopped(profile)?;
        }
        self.write_sdk_preference(&sdk)?;
        Ok(sdk)
    }
    fn ensure_sdk_switch_allowed(&self, sdk: &Path) -> Result<(), String> {
        let current = match self.saved_sdk() {
            Ok(Some(path)) => path,
            Ok(None) => self.sdk.clone(),
            Err(_) => PathBuf::new(),
        };
        let current = if current.as_os_str().is_empty() {
            current
        } else {
            current.canonicalize().unwrap_or(current)
        };
        if current == sdk {
            return Ok(());
        }
        match fs::read_dir(self.avds()) {
            Ok(mut entries) => {
                if let Some(entry) = entries.next() {
                    entry.map_err(crate::err)?;
                    return Err("Runtime còn dữ liệu AVD/archive gắn với SDK cũ. Chưa hỗ trợ chuyển SDK cho dữ liệu này; hãy chọn đúng SDK hiện tại.".into());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
        Ok(())
    }
    fn write_sdk_preference(&self, sdk: &Path) -> Result<(), String> {
        let java = self
            .saved_settings()
            .ok()
            .flatten()
            .and_then(|(_, java)| java);
        self.write_sdk_settings(sdk, java.as_deref())
    }
    fn write_sdk_settings(&self, sdk: &Path, java: Option<&Path>) -> Result<(), String> {
        use std::{io::Write, os::unix::fs::OpenOptionsExt};
        let target = self.runtime().join("hub-settings.json");
        match fs::symlink_metadata(&target) {
            Ok(meta) if !meta.is_file() || meta.file_type().is_symlink() => {
                return Err("Không ghi đè đường dẫn cấu hình không phải file thường".into());
            }
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.to_string()),
            _ => {}
        }
        let temporary = target.with_extension(format!(
            "json.{}.{}.new",
            std::process::id(),
            crate::SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let result = (|| {
            let mut file = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(&temporary)
                .map_err(crate::err)?;
            file.write_all(
                &serde_json::to_vec_pretty(
                    &serde_json::json!({"schema":1,"sdk":sdk,"java_home":java}),
                )
                .map_err(crate::err)?,
            )
            .map_err(crate::err)?;
            file.sync_all().map_err(crate::err)?;
            fs::rename(&temporary, &target).map_err(crate::err)?;
            crate::restore::sync_dir(&self.runtime())
        })();
        if result.is_err() {
            let _ = fs::remove_file(temporary);
        }
        result
    }
    /// Preferred product variable, with compatibility for existing P0 setups.
    pub fn java_override() -> Option<std::ffi::OsString> {
        std::env::var_os("ANDROID_HUB_JAVA_HOME").or_else(|| std::env::var_os("P0_JAVA_HOME"))
    }
    fn default_java_home() -> PathBuf {
        Self::java_override().map(PathBuf::from).unwrap_or_else(|| {
            PathBuf::from("/Applications/Android Studio.app/Contents/jbr/Contents/Home")
        })
    }
    pub fn java_home(&self) -> Result<PathBuf, String> {
        if Self::java_override().is_some() {
            return Ok(Self::default_java_home());
        }
        Ok(self
            .saved_settings()?
            .and_then(|(_, java)| java)
            .unwrap_or_else(Self::default_java_home))
    }
    /// Select an installed Java only after SDK tool probes succeed; never downloads Java.
    pub fn configure_java(&self, path: &Path) -> Result<PathBuf, String> {
        if Self::java_override().is_some() {
            return Err(
                "ANDROID_HUB_JAVA_HOME/P0_JAVA_HOME đang ghi đè Java; bỏ biến này rồi mở lại Hub để lưu lựa chọn".into(),
            );
        }
        if !path.is_absolute() {
            return Err("Nhập đường dẫn Java home tuyệt đối".into());
        }
        let _guard = self.lock()?;
        self.ensure_sdk(false)?;
        let state = self.snapshot()?;
        if !state.runtime.is_empty() {
            return Err("Dừng tất cả máy Android trước khi đổi Java".into());
        }
        for profile in &state.profiles {
            self.ensure_stopped(profile)?;
        }
        let probe = self.probe_sdk_tools(path)?;
        // Preserve the saved SDK even when ANDROID_SDK_ROOT overrides this session.
        let saved = self.saved_sdk()?.unwrap_or_else(|| self.sdk.clone());
        self.write_sdk_settings(&saved, Some(&probe.java_home))?;
        Ok(probe.java_home)
    }
    pub(crate) fn configure_sdk_and_java(
        &self,
        path: &Path,
        java: &Path,
    ) -> Result<(PathBuf, PathBuf), String> {
        if std::env::var_os("ANDROID_SDK_ROOT").is_some() || Self::java_override().is_some() {
            return Err(
                "Bỏ ANDROID_SDK_ROOT/ANDROID_HUB_JAVA_HOME/P0_JAVA_HOME và mở lại Hub trước khi chọn SDK kèm Java".into(),
            );
        }
        if !path.is_absolute() || !java.is_absolute() {
            return Err("SDK và Java cần đường dẫn tuyệt đối".into());
        }
        let sdk = path.canonicalize().map_err(crate::err)?;
        let _guard = self.lock()?;
        self.ensure_sdk_switch_allowed(&sdk)?;
        let state = self.snapshot()?;
        if !state.runtime.is_empty() {
            return Err("Dừng tất cả máy trước khi chọn SDK".into());
        }
        for profile in &state.profiles {
            self.ensure_stopped(profile)?;
        }
        let absent = missing(&sdk, Some(java));
        if !absent.is_empty() {
            return Err(format!("SDK/Java thiếu thành phần:\n{}", absent.join("\n")));
        }
        let candidate = Self {
            root: self.root.clone(),
            sdk: sdk.clone(),
        };
        let probe = candidate.probe_sdk_tools(java)?;
        self.write_sdk_settings(&sdk, Some(&probe.java_home))?;
        Ok((sdk, probe.java_home))
    }
    pub(crate) fn ensure_sdk(&self, creating: bool) -> Result<(), String> {
        if std::env::var_os("ANDROID_SDK_ROOT").is_none()
            && let Some(saved) = self.saved_sdk()?
            && saved != self.sdk
        {
            return Err("SDK đã được đổi bởi phiên Hub khác; mở lại Hub trước khi tiếp tục".into());
        }
        self.ensure_sdk_components(creating)
    }
    fn ensure_sdk_components(&self, creating: bool) -> Result<(), String> {
        let java = creating.then(|| self.java_home()).transpose()?;
        let missing = missing(&self.sdk, java.as_deref());
        if missing.is_empty() {
            return Ok(());
        }
        Err(format!(
            "Môi trường Android chưa sẵn sàng:\n{}\nMở Android Studio → SDK Manager để cài/khôi phục các thành phần trên. Hub đang dùng SDK tại {}. Có thể đặt ANDROID_SDK_ROOT khi dùng SDK khác.",
            missing.join("\n"),
            self.sdk.display()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn preference_fixture() -> Backend {
        let root = std::env::temp_dir().join(format!(
            "hub-sdk-preferences-{}-{}",
            std::process::id(),
            crate::SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let backend = Backend {
            sdk: root.join("sdk"),
            root,
        };
        fs::create_dir_all(backend.runtime()).unwrap();
        backend
    }
    #[test]
    fn sdk_switch_rejects_existing_avd_or_archive_but_accepts_same_path() {
        let b = preference_fixture();
        let different = b.root.join("different-sdk");
        assert!(b.ensure_sdk_switch_allowed(&different).is_ok());
        fs::create_dir_all(b.avds().join(".removed-fixture")).unwrap();
        assert!(
            b.ensure_sdk_switch_allowed(&different)
                .unwrap_err()
                .contains("AVD/archive")
        );
        assert!(b.ensure_sdk_switch_allowed(&b.sdk).is_ok());
        b.write_sdk_preference(&different).unwrap();
        assert!(b.ensure_sdk_switch_allowed(&b.sdk).is_err());
        assert!(b.ensure_sdk_switch_allowed(&different).is_ok());
        fs::remove_dir_all(b.root).unwrap();
    }
    #[test]
    fn sdk_preferences_roundtrip_corruption_and_atomic_repair() {
        let b = preference_fixture();
        assert_eq!(b.saved_sdk().unwrap(), None);
        let sdk = b.root.join("SDK with spaces");
        b.write_sdk_preference(&sdk).unwrap();
        assert_eq!(b.saved_sdk().unwrap(), Some(sdk.clone()));
        let path = b.runtime().join("hub-settings.json");
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        fs::write(&path, b"broken").unwrap();
        assert!(b.saved_sdk().is_err());
        b.write_sdk_preference(&sdk).unwrap();
        assert_eq!(b.saved_sdk().unwrap(), Some(sdk));
        fs::write(&path, br#"{"schema":1,"sdk":"relative/path"}"#).unwrap();
        assert!(b.saved_sdk().is_err());
        fs::remove_file(&path).unwrap();
        let outside = b.root.join("outside");
        fs::write(&outside, "keep").unwrap();
        std::os::unix::fs::symlink(&outside, &path).unwrap();
        assert!(b.saved_sdk().is_err());
        assert!(b.write_sdk_preference(&b.sdk).is_err());
        assert_eq!(fs::read_to_string(outside).unwrap(), "keep");
        fs::remove_dir_all(b.root).unwrap();
    }
    #[test]
    fn java_preference_is_optional_validated_and_preserved_when_sdk_changes() {
        let b = preference_fixture();
        let target = b.runtime().join("hub-settings.json");
        fs::write(
            &target,
            serde_json::to_vec(&serde_json::json!({"schema":1,"sdk":b.sdk})).unwrap(),
        )
        .unwrap();
        assert_eq!(b.saved_settings().unwrap(), Some((b.sdk.clone(), None)));
        let java = b.root.join("Java with spaces/Contents/Home");
        b.write_sdk_settings(&b.sdk, Some(&java)).unwrap();
        assert_eq!(
            b.saved_settings().unwrap(),
            Some((b.sdk.clone(), Some(java.clone())))
        );
        let next = b.root.join("next-sdk");
        b.write_sdk_preference(&next).unwrap();
        assert_eq!(
            b.saved_settings().unwrap(),
            Some((next.clone(), Some(java.clone())))
        );
        if Backend::java_override().is_none() {
            assert_eq!(
                Backend::new(b.root.clone()).unwrap().java_home().unwrap(),
                java
            );
        }
        for invalid in [
            serde_json::json!("relative"),
            serde_json::json!(false),
            serde_json::json!(""),
        ] {
            fs::write(
                &target,
                serde_json::to_vec(&serde_json::json!({"schema":1,"sdk":next,"java_home":invalid}))
                    .unwrap(),
            )
            .unwrap();
            assert!(b.saved_settings().is_err());
        }
        fs::remove_dir_all(b.root).unwrap();
    }
    #[test]
    #[ignore = "installed SDK/JBR probes and process gates, writes only temporary preferences"]
    fn configure_java_persists_validated_home_and_preserves_settings_on_failure() {
        assert!(Backend::java_override().is_none());
        assert!(std::env::var_os("ANDROID_SDK_ROOT").is_none());
        let mut b = preference_fixture();
        b.sdk = Backend::environment_sdk(b.root.clone()).unwrap().sdk;
        b.initialize().unwrap();
        let java = b
            .configure_java(Path::new(
                "/Applications/Android Studio.app/Contents/jbr/Contents/Home",
            ))
            .unwrap();
        assert_eq!(b.java_home().unwrap(), java);
        assert_eq!(
            Backend::new(b.root.clone()).unwrap().java_home().unwrap(),
            java
        );
        let before = fs::read(b.runtime().join("hub-settings.json")).unwrap();
        assert!(b.configure_java(&b.root.join("missing-java")).is_err());
        assert_eq!(
            fs::read(b.runtime().join("hub-settings.json")).unwrap(),
            before
        );
        b.configure_sdk(&b.sdk).unwrap();
        assert_eq!(b.java_home().unwrap(), java);
        let profile = android_hub_domain::Profile::standard(
            "hub-java-fixture".into(),
            "Java fixture".into(),
            5680,
        );
        b.write_profiles(&[profile]).unwrap();
        let listener = std::net::TcpListener::bind(("127.0.0.1", 5680)).unwrap();
        assert!(b.configure_java(&java).unwrap_err().contains("Port"));
        assert_eq!(
            fs::read(b.runtime().join("hub-settings.json")).unwrap(),
            before
        );
        drop(listener);
        fs::remove_dir_all(b.root).unwrap();
    }
    #[test]
    #[ignore = "read-only installed SDK validation plus process/loopback gates on a temporary root"]
    fn configure_installed_sdk_persists_without_touching_guest_data() {
        assert!(
            std::env::var_os("ANDROID_SDK_ROOT").is_none(),
            "Run without SDK environment override"
        );
        let b = preference_fixture();
        b.initialize().unwrap();
        let installed = Backend::environment_sdk(b.root.clone()).unwrap().sdk;
        let saved = b.configure_sdk(&installed).unwrap();
        assert_eq!(Backend::new(b.root.clone()).unwrap().sdk, saved);
        assert!(b.ensure_sdk(false).unwrap_err().contains("phiên Hub khác"));
        let settings = fs::read(b.runtime().join("hub-settings.json")).unwrap();
        assert!(b.configure_sdk(&b.root.join("missing-sdk")).is_err());
        assert_eq!(
            fs::read(b.runtime().join("hub-settings.json")).unwrap(),
            settings
        );
        let profile = android_hub_domain::Profile::standard(
            "hub-sdk-fixture".into(),
            "SDK fixture".into(),
            5680,
        );
        b.write_profiles(&[profile]).unwrap();
        let listener = std::net::TcpListener::bind(("127.0.0.1", 5680)).unwrap();
        assert!(b.configure_sdk(&installed).unwrap_err().contains("Port"));
        assert_eq!(
            fs::read(b.runtime().join("hub-settings.json")).unwrap(),
            settings
        );
        drop(listener);
        fs::remove_dir_all(b.root).unwrap();
    }
    #[test]
    fn reports_all_missing_components_and_distinguishes_start_from_create() {
        let root = std::env::temp_dir().join(format!(
            "hub-sdk-check-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let java = root.join("jbr");
        assert_eq!(missing(&root, None).len(), 2);
        assert_eq!(missing(&root, Some(&java)).len(), 12);
        for relative in [
            "emulator/emulator",
            "emulator/lib/hardware-properties.ini",
            "platform-tools/adb",
            "cmdline-tools/latest/bin/avdmanager",
            "jbr/bin/java",
        ] {
            let path = root.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, b"fixture only").unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        assert!(missing(&root, None).is_empty());
        assert_eq!(missing(&root, Some(&java)).len(), 7);
        for name in [
            "source.properties",
            "system.img",
            "vendor.img",
            "ramdisk.img",
            "kernel-ranchu",
            "encryptionkey.img",
            "data/empty_data_disk",
        ] {
            let path = root.join(IMAGE).join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"fixture only").unwrap();
        }
        assert!(missing(&root, Some(&java)).is_empty());
        fs::set_permissions(
            root.join("platform-tools/adb"),
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        fs::write(root.join(IMAGE).join("system.img"), b"").unwrap();
        let issues = missing(&root, Some(&java));
        assert_eq!(issues.len(), 2);
        assert!(issues.iter().any(|v| v.contains("Platform-Tools")));
        assert!(issues.iter().any(|v| v.contains("system.img")));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    #[ignore = "read-only check of installed host Android SDK"]
    fn installed_sdk_prerequisites() {
        let backend = Backend::new(std::env::temp_dir()).unwrap();
        backend.ensure_sdk(true).unwrap();
        backend.ensure_sdk(false).unwrap();
    }
}
