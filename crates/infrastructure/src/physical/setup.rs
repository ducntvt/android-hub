//! Select an existing Platform-Tools executable without installing an emulator SDK.
use crate::Backend;
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};
impl Backend {
    pub fn physical_adb_path(&self) -> Result<PathBuf, String> {
        let path = self.root.join("physical-adb-path");
        match fs::read_to_string(path) {
            Ok(value) => {
                let value = PathBuf::from(value.trim());
                if !value.is_absolute() {
                    return Err("Đường dẫn ADB phải tuyệt đối".into());
                }
                Ok(value)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(self.sdk.join("platform-tools/adb"))
            }
            Err(error) => Err(error.to_string()),
        }
    }
    /// `adb version` is local and never contacts, kills or restarts the server.
    pub fn configure_physical_adb(&self, path: &Path) -> Result<PathBuf, String> {
        if !path.is_absolute() {
            return Err("Đường dẫn ADB phải tuyệt đối".into());
        }
        let path = path.canonicalize().map_err(crate::err)?;
        let metadata = fs::metadata(&path).map_err(crate::err)?;
        if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
            return Err("Chọn file thực thi adb từ Platform-Tools".into());
        }
        let _guard = self.lock_raw()?;
        let mut command = Command::new(&path);
        command.arg("version");
        let output = self.command(command, 4)?;
        if !output
            .lines()
            .any(|line| line.trim() == "Android Debug Bridge version 1.0.41")
        {
            return Err("ADB không tương thích; cần protocol 1.0.41".into());
        }
        let value = path
            .to_str()
            .filter(|s| !s.contains(['\n', '\r']))
            .ok_or("Đường dẫn ADB không hợp lệ")?;
        let staging = self.root.join("physical-adb-path.new");
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staging)
            .map_err(crate::err)?;
        let result = (|| {
            file.write_all(value.as_bytes()).map_err(crate::err)?;
            file.sync_all().map_err(crate::err)?;
            fs::rename(&staging, self.root.join("physical-adb-path")).map_err(crate::err)
        })();
        if result.is_err() {
            let _ = fs::remove_file(staging);
        }
        result?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn standalone_adb_selection_does_not_require_java_images_or_create_avds() {
        let root = std::env::temp_dir().join(format!("hub-us058-adb-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let adb = root.join("adb");
        fs::write(&adb,"#!/bin/sh\n[ \"$1\" = version ] || exit 9\nprintf 'Android Debug Bridge version 1.0.41\\nVersion test\\n'\n").unwrap();
        fs::set_permissions(&adb, fs::Permissions::from_mode(0o700)).unwrap();
        let backend = Backend {
            root: root.join("data"),
            sdk: root.join("missing-sdk"),
        };
        assert!(
            backend
                .configure_physical_adb(Path::new("relative/adb"))
                .is_err()
        );
        assert_eq!(
            backend.configure_physical_adb(&adb).unwrap(),
            adb.canonicalize().unwrap()
        );
        assert_eq!(
            backend.physical_adb_path().unwrap(),
            adb.canonicalize().unwrap()
        );
        assert!(!backend.sdk.exists());
        assert!(!backend.root.join("emulator-avds").exists());
        assert!(!backend.root.join("hub-profiles.json").exists());
        fs::write(&adb, "#!/bin/sh\necho wrong-tool\n").unwrap();
        assert!(backend.configure_physical_adb(&adb).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
