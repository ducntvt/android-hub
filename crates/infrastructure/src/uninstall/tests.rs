use super::*;
use std::sync::atomic::Ordering;

pub(super) struct Fixture {
    pub backend: Backend,
    pub app: PathBuf,
    work: PathBuf,
}
impl Fixture {
    pub fn new() -> Self {
        let work = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "hub-uninstall-test-{}-{}",
            std::process::id(),
            crate::SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let app = work.join("Android Hub QA.app");
        let root = work.join("data");
        fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
        fs::write(app.join("Contents/Info.plist"), r#"<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>local.androidhub.qa.test</string></dict></plist>"#).unwrap();
        fs::write(app.join("Contents/MacOS/android-hub"), b"fixture only").unwrap();
        fs::create_dir_all(root.join("backups")).unwrap();
        fs::write(root.join("backups/keep.txt"), b"backup").unwrap();
        fs::create_dir_all(root.join("managed-sdks/sdk")).unwrap();
        fs::write(root.join("managed-sdks/sdk/image"), b"owned").unwrap();
        let backend = Backend {
            root,
            sdk: work.join("external-sdk"),
        };
        fs::create_dir(&backend.sdk).unwrap();
        fs::write(backend.sdk.join("keep"), b"external").unwrap();
        backend.write_profiles(&[]).unwrap();
        Self { backend, app, work }
    }
    pub fn plan(&self) -> UninstallPlan {
        self.backend.uninstall_plan_at(&self.app).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.work);
    }
}

#[test]
fn preview_and_delete_keep_external_sdk_symlink_target_and_optional_backups() {
    for keep in [false, true] {
        let f = Fixture::new();
        std::os::unix::fs::symlink(&f.backend.sdk, f.backend.root.join("external-link")).unwrap();
        let plan = f.plan();
        assert!(plan.bytes_to_remove(false) >= plan.bytes_to_remove(true));
        assert!(f.backend.root.join("managed-sdks/sdk/image").exists());
        plan.remove_data(keep).unwrap();
        assert_eq!(f.backend.root.join("backups/keep.txt").exists(), keep);
        assert!(!f.backend.root.join("managed-sdks").exists());
        assert!(!f.backend.root.join("external-link").exists());
        assert_eq!(fs::read(f.backend.sdk.join("keep")).unwrap(), b"external");
        assert!(f.app.exists()); // App removal belongs to the after-exit helper.
    }
}
#[test]
fn changed_entries_replaced_root_and_symlink_scope_are_rejected_without_deletion() {
    let f = Fixture::new();
    let plan = f.plan();
    fs::write(f.backend.root.join("unexpected"), b"keep").unwrap();
    assert!(plan.remove_data(false).is_err());
    fs::remove_file(f.backend.root.join("unexpected")).unwrap();
    let original = f.work.join("original-data");
    fs::rename(&f.backend.root, &original).unwrap();
    fs::create_dir(&f.backend.root).unwrap();
    assert!(plan.remove_data(false).is_err());
    fs::remove_dir(&f.backend.root).unwrap();
    std::os::unix::fs::symlink(&original, &f.backend.root).unwrap();
    assert!(plan.remove_data(false).is_err());
    assert!(original.join("backups/keep.txt").exists());
}
#[test]
fn arbitrary_roots_dev_bundles_and_symlink_backups_cannot_be_removed() {
    let f = Fixture::new();
    assert!(validate_scope(&f.app, &f.work, &home().unwrap()).is_err());
    let dev = f.work.join("Android Hub.app");
    fs::rename(&f.app, &dev).unwrap();
    assert!(validate_scope(&dev, &f.backend.root, &home().unwrap()).is_err());
    fs::rename(dev, &f.app).unwrap();
    fs::remove_dir_all(f.backend.root.join("backups")).unwrap();
    std::os::unix::fs::symlink(&f.backend.sdk, f.backend.root.join("backups")).unwrap();
    assert!(f.plan().remove_data(true).is_err());
    assert!(f.backend.sdk.join("keep").exists());
}
#[test]
fn active_processes_and_pending_marker_block_mutation() {
    assert!(
        check_processes(
            "1 /sbin/launchd\n42 /Applications/Android Hub.app/Contents/MacOS/android-hub\n",
            42,
            43
        )
        .is_ok()
    );
    for name in [
        "android-hub",
        "android-hub-uninstaller",
        "qemu-system-aarch64",
        "qemu-system-aarch64-headless",
    ] {
        assert!(check_processes(&format!("88 /tmp/{name}\n"), 42, 43).is_err());
    }
    let f = Fixture::new();
    fs::write(f.backend.root.join(PENDING), b"interrupted").unwrap();
    assert!(f.backend.lock_raw().is_err());
    assert!(f.backend.initialize().is_err());
    assert!(f.backend.root.join("backups/keep.txt").exists());
}
