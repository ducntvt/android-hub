//! Offline, verified directory backups. No restore or guest mutation here.
use crate::{Backend, err};
use android_hub_domain::Profile;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Component, Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn recorded_location(value: &Value) -> Option<PathBuf> {
    value
        .as_str()
        .map(Path::new)
        .filter(|path| path.is_absolute())
        .and_then(|path| path.canonicalize().ok())
}

pub(super) fn files(root: &Path) -> Result<Vec<PathBuf>, String> {
    fn visit(root: &Path, here: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
        for entry in fs::read_dir(here).map_err(err)? {
            let entry = entry.map_err(err)?;
            let path = entry.path();
            let kind = entry.file_type().map_err(err)?;
            if kind.is_dir() {
                visit(root, &path, out)?;
            } else if kind.is_file() {
                out.push(path.strip_prefix(root).map_err(err)?.to_path_buf());
            } else {
                return Err(format!(
                    "Không sao lưu symlink/socket/file đặc biệt: {}",
                    path.display()
                ));
            }
        }
        Ok(())
    }
    let mut result = vec![];
    visit(root, root, &mut result)?;
    result.sort();
    Ok(result)
}
fn safe_relative(value: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if path.as_os_str().is_empty() || !path.components().all(|c| matches!(c, Component::Normal(_)))
    {
        return Err("Đường dẫn manifest không hợp lệ".into());
    }
    Ok(path)
}
pub(crate) const IMAGE_FILES: [&str; 6] = [
    "source.properties",
    "system.img",
    "vendor.img",
    "ramdisk.img",
    "kernel-ranchu",
    "encryptionkey.img",
];

/// AVD managers may append a slash to the same SDK-relative directory. Preserve
/// manifest spelling for legacy restore comparisons, but hash its normalized path.
pub(crate) fn image_fingerprint(inventory: &Value) -> Result<String, String> {
    let mut normalized = inventory.clone();
    let directory = safe_relative(
        inventory["directory"]
            .as_str()
            .ok_or("Image inventory thiếu directory")?,
    )?;
    normalized["directory"] = json!(directory.components().collect::<PathBuf>());
    Ok(crate::images::digest(
        &serde_json::to_vec(&normalized).map_err(err)?,
    ))
}

impl Backend {
    fn image_directory(&self, avd: &Path) -> Result<PathBuf, String> {
        let mut read = Command::new("/bin/cat");
        read.arg(avd.join("config.ini"));
        let config = self.command(read, 5)?;
        let value = config
            .lines()
            .filter_map(|line| line.split_once('='))
            .find(|(key, _)| key.trim() == "image.sysdir.1")
            .map(|(_, value)| value.trim())
            .ok_or("AVD thiếu image.sysdir.1")?;
        safe_relative(value)
    }
    fn image_identity(&self, avd: &Path) -> Result<Value, String> {
        let relative = self.image_directory(avd)?;
        self.image_inventory(&relative)
    }
    pub(crate) fn image_inventory(&self, relative: &Path) -> Result<Value, String> {
        let image = self.sdk.join(relative);
        self.image_inventory_at(&image, relative)
    }
    pub(crate) fn image_inventory_at(
        &self,
        image: &Path,
        relative: &Path,
    ) -> Result<Value, String> {
        let mut entries = BTreeMap::new();
        for name in IMAGE_FILES {
            let path = image.join(name);
            if !path.is_file() {
                return Err(format!(
                    "SDK image thiếu {name}; chưa thể bảo đảm khôi phục"
                ));
            }
            entries.insert(name, json!({"bytes":path.metadata().map_err(err)?.len(), "sha256":self.file_digest(&path)?}));
        }
        Ok(json!({"directory":relative,"files":entries}))
    }
    /// Read-only compatibility gate. A future restore transaction must recheck
    /// stopped state under the Hub lock before replacing any data.
    pub fn verify_restore_backup(
        &self,
        id: &str,
        directory: &Path,
    ) -> Result<android_hub_domain::Profile, String> {
        self.verify_backup(directory)?;
        let manifest: Value =
            serde_json::from_slice(&fs::read(directory.join("manifest.json")).map_err(err)?)
                .map_err(err)?;
        if manifest["schema"] != 2 {
            return Err("Backup cũ chưa ghi fingerprint image; hãy tạo backup mới trước khi khôi phục tự động".into());
        }
        let mut profile: android_hub_domain::Profile =
            serde_json::from_value(manifest["profile"].clone()).map_err(err)?;
        let current = self.existing(id)?;
        if profile.id != current.id || profile.port != current.port {
            return Err("Backup không thuộc máy được chọn".into());
        }
        // A project rename may retain its old path as a compatibility symlink.
        // Accept that alias only when it still resolves to this exact AVD.
        if recorded_location(&manifest["sdk_root"]) != Some(self.sdk.canonicalize().map_err(err)?)
            || recorded_location(&manifest["avd_path"])
                != Some(self.data(&current).canonicalize().map_err(err)?)
        {
            return Err(
                "Đường dẫn SDK/AVD đã thay đổi; chưa hỗ trợ chuyển backup sang máy/thư mục khác"
                    .into(),
            );
        }
        if self.image_directory(&directory.join("payload"))?
            != self.image_directory(&self.data(&current))?
        {
            return Err("Máy hiện tại dùng image Android khác backup".into());
        }
        if self.image_identity(&directory.join("payload"))? != manifest["image"] {
            return Err("SDK image đã thay đổi hoặc checksum image không khớp backup".into());
        }
        if let Some(expected) = &current.image {
            self.verify_profile_image(&current)?;
            if profile
                .image
                .as_ref()
                .is_some_and(|saved| saved != expected)
            {
                return Err("Image identity backup không khớp máy hiện tại".into());
            }
            // Legacy schema-2 backups reached the same full fingerprint/path gates above.
            profile.image = Some(expected.clone());
        } else if profile.image.is_some() {
            return Err("Cần xác định image máy hiện tại trước khi khôi phục".into());
        }
        Ok(profile)
    }
    pub(super) fn file_digest(&self, path: &Path) -> Result<String, String> {
        let mut cmd = Command::new("/usr/bin/shasum");
        cmd.args(["-a", "256"]).arg(path);
        let output = self.command(cmd, 300)?;
        let digest = output.split_whitespace().next().ok_or("Thiếu SHA-256")?;
        if digest.len() != 64 || !digest.bytes().all(|v| v.is_ascii_hexdigit()) {
            return Err("SHA-256 không hợp lệ".into());
        }
        Ok(digest.to_ascii_lowercase())
    }
    pub(super) fn inventory(&self, root: &Path) -> Result<BTreeMap<String, Value>, String> {
        let mut entries = BTreeMap::new();
        for path in files(root)? {
            let name = path
                .to_str()
                .ok_or("Tên file backup không phải UTF-8")?
                .to_string();
            let source = root.join(&path);
            let size = source.metadata().map_err(err)?.len();
            entries.insert(
                name,
                json!({"bytes":size,"sha256":self.file_digest(&source)?}),
            );
        }
        Ok(entries)
    }
    /// Verify the manifest and every payload file before treating a backup as usable.
    pub fn verify_backup(&self, directory: &Path) -> Result<(), String> {
        // Refuse links before any content is followed by the verifier.
        let listing = files(directory)?;
        if !listing.contains(&PathBuf::from("manifest.json")) {
            return Err("Thiếu manifest backup".into());
        }
        let manifest: Value =
            serde_json::from_slice(&fs::read(directory.join("manifest.json")).map_err(err)?)
                .map_err(err)?;
        if ![json!(1), json!(2)].contains(&manifest["schema"]) || manifest["complete"] != true {
            return Err("Backup chưa hoàn tất hoặc khác phiên bản".into());
        }
        let profile: android_hub_domain::Profile =
            serde_json::from_value(manifest["profile"].clone()).map_err(err)?;
        profile.validate()?;
        let declared = manifest["files"]
            .as_object()
            .ok_or("Thiếu danh sách file backup")?;
        for name in declared.keys() {
            safe_relative(name)?;
        }
        let actual = self.inventory(&directory.join("payload"))?;
        let expected: BTreeMap<String, Value> = declared
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        if actual != expected || actual.is_empty() {
            return Err("Backup thiếu file hoặc checksum không khớp".into());
        }
        // No undeclared siblings outside payload.
        if listing
            .iter()
            .any(|p| p != Path::new("manifest.json") && !p.starts_with("payload"))
        {
            return Err("Backup có file ngoài manifest".into());
        }
        Ok(())
    }
    pub(super) fn sync_payload(&self, root: &Path) -> Result<(), String> {
        for path in files(root)? {
            fs::File::open(root.join(path))
                .map_err(err)?
                .sync_all()
                .map_err(err)?;
        }
        fs::File::open(root).map_err(err)?.sync_all().map_err(err)
    }
    pub(crate) fn backup(&self, id: &str) -> Result<PathBuf, String> {
        let _guard = self.lock()?;
        let profile = self.existing(id)?;
        self.ensure_stopped(&profile)?;
        self.backup_locked(profile)
    }
    pub(super) fn backup_locked(&self, profile: Profile) -> Result<PathBuf, String> {
        let source = self.data(&profile);
        if fs::symlink_metadata(&source)
            .map_err(err)?
            .file_type()
            .is_symlink()
        {
            return Err("Không sao lưu thư mục AVD dạng symlink".into());
        }
        let base = self.runtime().join("backups");
        fs::create_dir_all(&base).map_err(err)?;
        crate::storage::ensure_copy_space(&source, &base)?;
        let image = self.image_identity(&source)?;
        if let Some(expected) = &profile.image
            && (self.image_version_at(&source)? != expected.version
                || self.installed_image_revision(expected.version)? != expected.revision
                || image_fingerprint(&image)? != expected.fingerprint)
        {
            return Err("Image Android đã thay đổi; không tạo backup với identity cũ".into());
        }
        let baseline = self.inventory(&source)?;
        if !baseline.contains_key("config.ini")
            || !baseline.keys().any(|p| p.starts_with("userdata"))
        {
            return Err("AVD thiếu cấu hình hoặc userdata".into());
        }
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(err)?
            .as_nanos();
        let name = format!("{}-{stamp}", profile.id);
        let pending = base.join(format!(".{name}.partial"));
        let complete = base.join(&name);
        fs::create_dir(&pending).map_err(err)?;
        fs::set_permissions(&pending, fs::Permissions::from_mode(0o700)).map_err(err)?;
        let result = (|| {
            let mut copy = Command::new("/bin/cp");
            copy.arg("-R").arg(&source).arg(pending.join("payload"));
            self.command(copy, 600)?;
            let manifest = json!({"schema":2,"complete":true,"profile":profile,
                "created_unix_nanos":stamp.to_string(),"sdk_root":self.sdk.canonicalize().map_err(err)?,
                "avd_path":source.canonicalize().map_err(err)?,"image":image,
                "files":baseline,"scope":"offline AVD data; requires compatible installed SDK image"});
            let mut file = fs::File::create(pending.join("manifest.json")).map_err(err)?;
            file.write_all(&serde_json::to_vec_pretty(&manifest).map_err(err)?)
                .map_err(err)?;
            file.sync_all().map_err(err)?;
            self.verify_backup(&pending)?;
            for path in files(&pending.join("payload"))? {
                fs::File::open(pending.join("payload").join(path))
                    .map_err(err)?
                    .sync_all()
                    .map_err(err)?;
            }
            self.ensure_stopped(&profile)?;
            if self.inventory(&source)? != baseline || self.image_identity(&source)? != image {
                return Err("Dữ liệu máy thay đổi trong lúc backup; đã hủy".into());
            }
            fs::rename(&pending, &complete).map_err(err)?;
            Ok(complete.clone())
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(&pending);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_backup_inherits_verified_binding_but_different_binding_is_rejected() {
        let fixture = crate::images::tests::Fixture::new();
        let b = &fixture.0;
        let version = android_hub_domain::AndroidVersion::Android12;
        fixture.image(version);
        let mut p = fixture.device(version, "hub-image-backup", 5664);
        p.image = Some(b.installed_image_identity(version).unwrap());
        b.write_profiles(&[p.clone()]).unwrap();
        let directory = b.root.join("backup");
        let payload = directory.join("payload");
        fs::create_dir_all(&payload).unwrap();
        for name in ["config.ini", "userdata.img"] {
            fs::copy(b.data(&p).join(name), payload.join(name)).unwrap();
        }
        let mut legacy = p.clone();
        legacy.image = None;
        let mut manifest = json!({"schema":2,"complete":true,"profile":legacy,
            "sdk_root":b.sdk.canonicalize().unwrap(), "avd_path":b.data(&p).canonicalize().unwrap(),
            "image":b.image_identity(&payload).unwrap(), "files":b.inventory(&payload).unwrap()});
        let path = directory.join("manifest.json");
        fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert_eq!(
            b.verify_restore_backup(&p.id, &directory).unwrap().image,
            p.image
        );
        let original = fs::read(b.data(&p).join("userdata.img")).unwrap();
        manifest["profile"]["image"] = serde_json::to_value(&p.image).unwrap();
        manifest["profile"]["image"]["revision"] = json!([99, 0, 0]);
        fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert!(
            b.verify_restore_backup(&p.id, &directory)
                .unwrap_err()
                .contains("identity")
        );
        manifest["profile"] = serde_json::to_value(&legacy).unwrap();
        fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        fs::write(
            b.sdk.join(version.directory()).join("system.img"),
            "changed image",
        )
        .unwrap();
        assert!(b.verify_restore_backup(&p.id, &directory).is_err());
        assert_eq!(fs::read(b.data(&p).join("userdata.img")).unwrap(), original);
        assert_eq!(b.existing(&p.id).unwrap(), p);
    }
    use std::os::unix::fs::symlink;

    #[test]
    #[ignore = "requires host process inspection and loopback port checks"]
    fn backup_then_direct_delete_keeps_verified_backup_and_shared_image() -> Result<(), String> {
        let root = std::env::temp_dir().join(format!(
            "hub-delete-backup-{}-{}",
            std::process::id(),
            crate::SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let mut b = Backend::new(root.clone())?;
        b.sdk = root.join("sdk");
        let image = b.sdk.join("system-images/test");
        fs::create_dir_all(&image).map_err(err)?;
        for name in IMAGE_FILES {
            fs::write(image.join(name), b"test SDK image").map_err(err)?;
        }
        let p = Profile::standard("hub-direct-delete".into(), "Delete fixture".into(), 5668);
        fs::create_dir_all(b.data(&p)).map_err(err)?;
        fs::write(b.avds().join(format!("{}.ini", p.id)), b"registration").map_err(err)?;
        fs::write(b.data(&p).join("userdata.img"), b"guest bytes").map_err(err)?;
        b.write_profiles(std::slice::from_ref(&p))?;
        let plan = b.prepare_delete(&p)?;
        // A failed optional backup must leave the device and registry intact.
        assert!(b.backup_before_delete(&plan).is_err());
        assert!(b.data(&p).join("userdata.img").is_file());
        assert_eq!(b.read_profiles()?, vec![p.clone()]);
        fs::write(
            b.data(&p).join("config.ini"),
            b"image.sysdir.1=system-images/test/\n",
        )
        .map_err(err)?;
        let port = std::net::TcpListener::bind(("127.0.0.1", p.port)).map_err(err)?;
        assert!(b.delete_device(&plan).is_err());
        assert!(b.backup_before_delete(&plan).is_err());
        drop(port);
        let backup = b.backup_before_delete(&plan)?;
        b.verify_backup(&backup)?;
        assert!(b.data(&p).is_dir()); // Backup does not delete.
        assert_eq!(b.read_profiles()?, vec![p.clone()]);
        b.delete_device(&plan)?;
        assert!(!b.data(&p).exists());
        assert!(b.read_profiles()?.is_empty());
        assert!(b.removed_devices()?.entries.is_empty());
        b.verify_backup(&backup)?;
        assert_eq!(
            fs::read(backup.join("payload/userdata.img")).map_err(err)?,
            b"guest bytes"
        );
        assert_eq!(
            fs::read(image.join("system.img")).map_err(err)?,
            b"test SDK image"
        );
        fs::remove_dir_all(root).map_err(err)?;
        Ok(())
    }
    #[test]
    fn rejects_traversal_and_nonregular_files() {
        for path in ["", "/tmp/data", "../data", "data/../../other"] {
            assert!(safe_relative(path).is_err());
        }
        assert!(safe_relative("data/app/file").is_ok());
        let root = std::env::temp_dir().join(format!("hub-backup-links-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        symlink("/tmp", root.join("link")).unwrap();
        assert!(files(&root).is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn project_rename_alias_preserves_backup_identity_but_wrong_target_is_rejected() {
        let root = std::env::temp_dir().join(format!(
            "android-hub-rename-fixture-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let old = root.join("VM");
        let new = root.join("Android Hub");
        let mut backend = Backend {
            root: old.clone(),
            sdk: old.join("SDK"),
        };
        let profile =
            android_hub_domain::Profile::standard("hub-rename".into(), "Android".into(), 5584);
        let source = backend.data(&profile);
        fs::create_dir_all(&source).unwrap();
        backend
            .write_profiles(std::slice::from_ref(&profile))
            .unwrap();
        let image = backend.sdk.join("system-images/test");
        fs::create_dir_all(&image).unwrap();
        for name in IMAGE_FILES {
            fs::write(image.join(name), b"image fixture").unwrap();
        }
        let backup = root.join("backup");
        let payload = backup.join("payload");
        fs::create_dir_all(&payload).unwrap();
        for (name, contents) in [
            ("config.ini", "image.sysdir.1=system-images/test/\n"),
            ("userdata.img", "guest fixture"),
        ] {
            fs::write(source.join(name), contents).unwrap();
            fs::write(payload.join(name), contents).unwrap();
        }
        let manifest = json!({"schema":2,"complete":true,"profile":profile,
            "sdk_root":backend.sdk.canonicalize().unwrap(),
            "avd_path":source.canonicalize().unwrap(),
            "image":backend.image_identity(&source).unwrap(),
            "files":backend.inventory(&payload).unwrap()});
        fs::write(
            backup.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        backend.verify_restore_backup(&profile.id, &backup).unwrap();
        fs::rename(&old, &new).unwrap();
        symlink(&new, &old).unwrap();
        backend.sdk = new.join("SDK");
        backend.root = new;
        assert_eq!(
            backend.verify_restore_backup(&profile.id, &backup).unwrap(),
            profile
        );
        assert_eq!(
            fs::read(source.join("userdata.img")).unwrap(),
            b"guest fixture"
        );
        fs::remove_file(&old).unwrap();
        assert!(backend.verify_restore_backup(&profile.id, &backup).is_err());
        let other = root.join("another-project");
        fs::create_dir_all(other.join("emulator-avds/hub-rename.avd")).unwrap();
        symlink(&other, &old).unwrap();
        assert!(backend.verify_restore_backup(&profile.id, &backup).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn verifier_detects_corruption_missing_and_extra_files() {
        let root = std::env::temp_dir().join(format!("hub-backup-test-{}", std::process::id()));
        let backend = Backend::new(root.clone()).unwrap();
        fs::create_dir_all(backend.runtime()).unwrap();
        let backup = root.join("backup");
        fs::create_dir_all(backup.join("payload")).unwrap();
        let data = backup.join("payload/userdata.img");
        fs::write(&data, b"original guest data").unwrap();
        let profile = android_hub_domain::Profile::standard("hub-test".into(), "Test".into(), 5584);
        let manifest = json!({"schema":1,"complete":true,"profile":profile,"files":backend.inventory(&backup.join("payload")).unwrap()});
        fs::write(
            backup.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        backend.verify_backup(&backup).unwrap();
        fs::write(&data, b"corrupted guest data").unwrap();
        assert!(backend.verify_backup(&backup).is_err());
        fs::remove_file(&data).unwrap();
        assert!(backend.verify_backup(&backup).is_err());
        fs::write(&data, b"original guest data").unwrap();
        fs::write(backup.join("payload/extra"), b"unexpected").unwrap();
        assert!(backend.verify_backup(&backup).is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    #[ignore = "requires host process inspection and loopback port checks"]
    fn offline_backup_publishes_verified_copy_and_rejects_busy_port() -> Result<(), String> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(err)?
            .as_nanos();
        let root = std::env::temp_dir().join(format!("hub-offline-backup-{stamp}"));
        let mut backend = Backend::new(root.clone())?;
        backend.sdk = root.join("sdk");
        let image = backend.sdk.join("system-images/test");
        fs::create_dir_all(&image).map_err(err)?;
        for name in IMAGE_FILES {
            fs::write(image.join(name), b"test SDK image").map_err(err)?;
        }
        fs::create_dir_all(backend.runtime()).map_err(err)?;
        let profile = android_hub_domain::Profile::standard(
            "hub-backup-fixture".into(),
            "Fixture".into(),
            5680,
        );
        backend.write_profiles(std::slice::from_ref(&profile))?;
        let source = backend.data(&profile);
        fs::create_dir_all(&source).map_err(err)?;
        fs::write(
            source.join("config.ini"),
            b"image.sysdir.1=system-images/test/\n",
        )
        .map_err(err)?;
        fs::write(source.join("userdata.img"), b"private guest data fixture").map_err(err)?;
        let guard = std::net::TcpListener::bind(("127.0.0.1", profile.port)).map_err(err)?;
        assert!(backend.backup(&profile.id).is_err());
        assert!(!backend.runtime().join("backups").exists());
        drop(guard);
        // A sparse fixture advertises more logical data than available disk,
        // without filling the host filesystem or copying its contents.
        let sparse = source.join("space-test.img");
        let oversized = fs2::available_space(&source)
            .map_err(err)?
            .checked_add(1)
            .ok_or("Available space overflow")?;
        fs::File::create(&sparse)
            .map_err(err)?
            .set_len(oversized)
            .map_err(err)?;
        let refused = backend.backup(&profile.id).unwrap_err();
        assert!(refused.contains("Không đủ dung lượng"), "{refused}");
        assert_eq!(
            fs::read_dir(backend.runtime().join("backups"))
                .map_err(err)?
                .count(),
            0
        );
        assert_eq!(
            fs::read(source.join("userdata.img")).map_err(err)?,
            b"private guest data fixture"
        );
        fs::remove_file(sparse).map_err(err)?;
        let backup = backend.backup(&profile.id)?;
        backend.verify_backup(&backup)?;
        assert_eq!(
            fs::metadata(&backup).map_err(err)?.permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::read(source.join("userdata.img")).map_err(err)?,
            b"private guest data fixture"
        );
        assert_eq!(
            fs::read(backup.join("payload/userdata.img")).map_err(err)?,
            b"private guest data fixture"
        );
        assert_eq!(
            fs::read_dir(backend.runtime().join("backups"))
                .map_err(err)?
                .count(),
            1
        );
        assert_eq!(
            backend.verify_restore_backup(&profile.id, &backup)?,
            profile
        );
        let catalog = backend.list_backups(&profile.id)?;
        assert_eq!(catalog.entries.len(), 1);
        assert_eq!(catalog.entries[0].path, backup);
        assert_eq!(catalog.skipped, 0);
        let selected_digest = catalog.entries[0].manifest_sha256.clone();
        let manifest_path = backup.join("manifest.json");
        let original_manifest: Value =
            serde_json::from_slice(&fs::read(&manifest_path).map_err(err)?).map_err(err)?;
        for (field, replacement) in [
            ("schema", json!(1)),
            ("sdk_root", json!("/tmp/another-sdk")),
            ("avd_path", json!("/tmp/another-avd")),
            ("image", json!({})),
        ] {
            let mut changed = original_manifest.clone();
            changed[field] = replacement;
            fs::write(&manifest_path, serde_json::to_vec(&changed).map_err(err)?).map_err(err)?;
            assert!(
                backend.verify_restore_backup(&profile.id, &backup).is_err(),
                "accepted changed {field}"
            );
        }
        assert!(
            backend
                .restore(&profile.id, &backup, &selected_digest)
                .unwrap_err()
                .contains("thay đổi")
        );
        assert_eq!(
            fs::read(source.join("userdata.img")).map_err(err)?,
            b"private guest data fixture"
        );
        let mut other_guest = original_manifest.clone();
        other_guest["profile"]["id"] = json!("hub-other");
        fs::write(
            &manifest_path,
            serde_json::to_vec(&other_guest).map_err(err)?,
        )
        .map_err(err)?;
        assert!(backend.verify_restore_backup(&profile.id, &backup).is_err());
        let catalog = backend.list_backups(&profile.id)?;
        assert!(catalog.entries.is_empty());
        assert_eq!(catalog.skipped, 1);
        fs::write(
            &manifest_path,
            serde_json::to_vec(&original_manifest).map_err(err)?,
        )
        .map_err(err)?;
        fs::write(image.join("system.img"), b"changed SDK image").map_err(err)?;
        assert!(backend.verify_restore_backup(&profile.id, &backup).is_err());
        assert_eq!(backend.read_profiles()?, vec![profile]);
        fs::remove_dir_all(root).map_err(err)?;
        Ok(())
    }
}
