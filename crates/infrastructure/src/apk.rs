//! Single-file APK installation into an explicitly selected, verified engine.
use crate::{Backend, Snapshot};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

fn apk_path(path: &Path) -> Result<PathBuf, String> {
    if !path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("apk"))
    {
        return Err("Chọn một file .apk; chưa hỗ trợ APK split/XAPK".into());
    }
    let path = path
        .canonicalize()
        .map_err(|e| format!("Không mở được APK: {e}"))?;
    let meta = path
        .metadata()
        .map_err(|e| format!("Không đọc được APK: {e}"))?;
    if !meta.is_file() || meta.len() == 0 {
        return Err("APK phải là file thường, không rỗng".into());
    }
    Ok(path)
}
fn same_engine(state: &Snapshot, id: &str, pid: u32) -> Result<(), String> {
    if state.runtime.get(id).and_then(|r| r.pid) != Some(pid) {
        return Err("Máy đã dừng hoặc khởi động lại. Chọn lại APK để xác nhận phiên mới.".into());
    }
    Ok(())
}
fn install_command(adb: &Path, port: u16, path: &Path) -> Command {
    let mut cmd = Command::new(adb);
    // Absolute file path is a single argument. Never select the first ADB device
    // implicitly or enable replace/downgrade/grant-permissions options.
    cmd.args(["-s", &format!("emulator-{port}"), "install"])
        .arg(path);
    cmd
}
impl Backend {
    pub fn install_apk(
        &self,
        id: &str,
        expected_pid: u32,
        path: &Path,
    ) -> Result<Snapshot, String> {
        let _guard = self.lock()?;
        let path = apk_path(path)?;
        let p = self.existing(id)?;
        same_engine(&self.snapshot()?, id, expected_pid)?;
        let adb = self.sdk.join("platform-tools/adb");
        let serial = format!("emulator-{}", p.port);
        let mut identity = Command::new(&adb);
        identity.args(["-s", &serial, "emu", "avd", "name"]);
        if self.command(identity, 5)?.lines().next().map(str::trim) != Some(id) {
            return Err("ADB trả về máy khác; đã từ chối cài APK".into());
        }
        let mut ready = Command::new(&adb);
        ready.args(["-s", &serial, "shell", "getprop", "sys.boot_completed"]);
        if self.command(ready, 5)?.trim() != "1" {
            return Err("Android chưa boot xong; hãy thử lại sau".into());
        }
        same_engine(&self.snapshot()?, id, expected_pid)?;
        let output = self.command(install_command(&adb, p.port, &path), 180)
            .map_err(|error| format!("Cài APK chưa được xác nhận: {error}. Nếu quá thời gian, kiểm tra ứng dụng trong Android trước khi thử lại."))?;
        if !output.lines().any(|line| line.trim() == "Success") {
            return Err("ADB chưa xác nhận cài thành công; kiểm tra ứng dụng trong Android".into());
        }
        self.snapshot()
            .map_err(|error| format!("APK đã cài, nhưng chưa làm mới được trạng thái: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use android_hub_domain::Runtime;
    use std::{collections::BTreeMap, ffi::OsString};

    #[test]
    fn installation_targets_one_serial_and_preserves_literal_path() {
        let path = Path::new("/tmp/a b ' $(touch nope).apk");
        let cmd = install_command(Path::new("/sdk/adb"), 5582, path);
        let args: Vec<_> = cmd.get_args().map(OsString::from).collect();
        assert_eq!(
            args,
            ["-s", "emulator-5582", "install", path.to_str().unwrap()].map(OsString::from)
        );
    }
    #[test]
    fn installation_rejects_stopped_restarted_or_other_guest() {
        let mut state = Snapshot {
            android_versions: BTreeMap::new(),
            profiles: vec![],
            runtime: BTreeMap::new(),
        };
        assert!(same_engine(&state, "hub-a", 10).is_err());
        state.runtime.insert(
            "hub-b".into(),
            Runtime {
                pid: Some(10),
                cpu_percent: None,
                rss_kib: None,
            },
        );
        assert!(same_engine(&state, "hub-a", 10).is_err());
        state.runtime.insert(
            "hub-a".into(),
            Runtime {
                pid: Some(11),
                cpu_percent: None,
                rss_kib: None,
            },
        );
        assert!(same_engine(&state, "hub-a", 10).is_err());
        assert!(same_engine(&state, "hub-a", 11).is_ok());
    }
    #[test]
    fn apk_requires_existing_nonempty_file() {
        let dir = std::env::temp_dir().join(format!("hub-apk-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let apk = dir.join("example.APK");
        assert!(apk_path(&apk).is_err());
        std::fs::write(&apk, []).unwrap();
        assert!(apk_path(&apk).is_err());
        std::fs::write(&apk, b"placeholder; Android validates package content").unwrap();
        assert!(apk_path(&apk).unwrap().is_absolute());
        assert!(apk_path(&dir).is_err());
        assert!(apk_path(&dir.join("example.xapk")).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
    /// Explicit opt-in: creates a disposable guest in a separate temporary root.
    #[test]
    #[ignore = "requires local Android SDK and ANDROID_HUB_TEST_APK fixture"]
    fn installs_on_disposable_guest() -> Result<(), String> {
        use std::{
            thread,
            time::{Duration, Instant, SystemTime, UNIX_EPOCH},
        };
        let fixture = std::env::var_os("ANDROID_HUB_TEST_APK").ok_or("Missing APK fixture")?;
        let apk = apk_path(Path::new(&fixture))?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let root = std::env::temp_dir().join(format!("hub-apk-smoke-{stamp}"));
        std::fs::create_dir(&root).map_err(|e| e.to_string())?;
        println!("Disposable root: {}", root.display());
        let backend = Backend::new(root.clone())?;
        backend.initialize()?;
        let mut profile = backend.create()?.profiles.pop().ok_or("Missing profile")?;
        profile.ram_mb = 1024;
        profile.width = 480;
        profile.height = 854;
        if std::env::var_os("ANDROID_HUB_TEST_LANDSCAPE").is_some() {
            (profile.width, profile.height) = (854, 480);
        }
        profile.fps = 15;
        backend.save(profile.clone())?;
        let state = backend.start(&profile.id)?;
        let pid = state.runtime[&profile.id].pid.ok_or("Missing PID")?;
        let serial = format!("emulator-{}", profile.port);
        let test = (|| -> Result<(), String> {
            let deadline = Instant::now() + Duration::from_secs(120);
            loop {
                let mut ready = Command::new(backend.sdk.join("platform-tools/adb"));
                ready.args(["-s", &serial, "shell", "getprop", "sys.boot_completed"]);
                if backend.command(ready, 5).is_ok_and(|s| s.trim() == "1") {
                    break;
                }
                if Instant::now() >= deadline {
                    return Err("Guest boot timeout".into());
                }
                thread::sleep(Duration::from_secs(2));
            }
            let mut dimensions = Command::new(backend.sdk.join("platform-tools/adb"));
            dimensions.args(["-s", &serial, "shell", "wm", "size"]);
            let dimensions = backend.command(dimensions, 10)?;
            let expected = format!("Physical size: {}x{}", profile.width, profile.height);
            if !dimensions.lines().any(|line| line.trim() == expected) {
                return Err(format!(
                    "Unexpected framebuffer: {dimensions}; expected {expected}"
                ));
            }
            println!("Framebuffer verified: {}x{}", profile.width, profile.height);
            assert!(
                backend
                    .install_apk(&profile.id, pid.wrapping_add(1), &apk)
                    .is_err()
            );
            backend.install_apk(&profile.id, pid, &apk)?;
            let mut query = Command::new(backend.sdk.join("platform-tools/adb"));
            query.args([
                "-s",
                &serial,
                "shell",
                "pm",
                "path",
                "local.androidhub.installsmoke",
            ]);
            assert!(backend.command(query, 10)?.trim().starts_with("package:"));
            println!("APK installed and package found on disposable guest only");
            Ok(())
        })();
        // Preserve files on cleanup failure for inspection; never remove a live guest.
        backend.stop(&profile.id)?;
        // Process exit may precede successful rebinding of the SDK's ports.
        // Keep the production offline gate; wait for it instead of weakening it.
        let stopped_deadline = Instant::now() + Duration::from_secs(75);
        loop {
            match backend.ensure_stopped(&profile) {
                Ok(()) => break,
                Err(error) if Instant::now() >= stopped_deadline => return Err(error),
                Err(_) => thread::sleep(Duration::from_millis(500)),
            }
        }
        if test.is_ok() {
            let backup = backend.backup(&profile.id)?;
            backend.verify_backup(&backup)?;
            backend.verify_restore_backup(&profile.id, &backup)?;
            println!("Stopped guest backup and SDK compatibility verified");
            let source = backend.data(&profile);
            std::fs::write(
                source.join("post-backup-marker"),
                b"newer data retained in rollback directory",
            )
            .map_err(|e| e.to_string())?;
            let mut newer = profile.clone();
            newer.name = "After backup".into();
            backend.save(newer)?;
            let digest = backend.backup_manifest_digest(&backup)?;
            let outcome = backend.restore(&profile.id, &backup, &digest)?;
            assert_eq!(outcome.snapshot.profiles.first(), Some(&profile));
            assert!(!source.join("post-backup-marker").exists());
            assert!(outcome.previous_data.join("post-backup-marker").is_file());
            let removed = backend.remove_device(&profile)?;
            assert!(removed.snapshot.profiles.is_empty());
            assert!(!source.exists());
            let returned = backend.undo_remove_device(&removed.archive, &profile)?;
            assert_eq!(returned.profiles, vec![profile.clone()]);
            println!("Removed and returned disposable guest through production APIs");
            let restored = (|| -> Result<(), String> {
                backend.start(&profile.id)?;
                let deadline = Instant::now() + Duration::from_secs(120);
                loop {
                    let mut ready = Command::new(backend.sdk.join("platform-tools/adb"));
                    ready.args(["-s", &serial, "shell", "getprop", "sys.boot_completed"]);
                    if backend.command(ready, 5).is_ok_and(|s| s.trim() == "1") {
                        break;
                    }
                    if Instant::now() >= deadline {
                        return Err("Restored guest boot timeout".into());
                    }
                    thread::sleep(Duration::from_secs(2));
                }
                let mut query = Command::new(backend.sdk.join("platform-tools/adb"));
                query.args([
                    "-s",
                    &serial,
                    "shell",
                    "pm",
                    "path",
                    "local.androidhub.installsmoke",
                ]);
                if !backend.command(query, 10)?.trim().starts_with("package:") {
                    return Err("Installed package missing after backup restore".into());
                }
                println!("Restored backup booted; installed test package persisted");
                Ok(())
            })();
            if backend.snapshot()?.runtime.contains_key(&profile.id) {
                backend.stop(&profile.id)?;
            }
            restored?;
        }
        std::fs::remove_dir_all(&root).map_err(|e| e.to_string())?;
        test
    }
}
