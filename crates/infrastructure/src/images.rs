//! Read-only image catalog and verified, per-device SDK bindings.
mod management;
mod references;
use crate::{Backend, SdkArchive, SdkLicense, SdkPackage, err, sdk::package};
use android_hub_domain::{AndroidVersion, ImageIdentity, Profile};
pub use management::{ImageRemovalPlan, ImageUsage};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, os::unix::fs::MetadataExt, path::Path};

#[derive(Clone, Debug)]
pub struct AndroidImage {
    pub version: AndroidVersion,
    pub revision: [u32; 3],
    pub minimum_emulator: [u32; 3],
    pub archive: SdkArchive,
    pub license: SdkLicense,
}

#[derive(Clone, Debug)]
pub struct ImageAvailability {
    pub version: AndroidVersion,
    pub revision: Option<[u32; 3]>,
    pub error: Option<String>,
    pub download_bytes: u64,
    pub usage: ImageUsage,
}

pub fn android_image_catalog() -> Result<Vec<AndroidImage>, String> {
    parse_catalog(include_str!("../resources/android-images.json"))
}

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("Image catalog thiếu {key}"))
}

fn parse_catalog(text: &str) -> Result<Vec<AndroidImage>, String> {
    let value: Value = serde_json::from_str(text).map_err(err)?;
    if value["schema"] != 1 || value["platform"] != "macos-arm64" {
        return Err("Image catalog không hỗ trợ".into());
    }
    let packages = value["packages"].as_array().ok_or("Thiếu image catalog")?;
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for item in packages {
        let version = AndroidVersion::from_package(string(item, "package")?)
            .ok_or("Image không được hỗ trợ")?;
        if !seen.insert(version) || item["destination"] != version.directory() {
            return Err("Image catalog có package trùng hoặc sai destination".into());
        }
        let revision = package::revision(string(item, "revision")?)?;
        let dependencies = item["dependencies"]
            .as_array()
            .ok_or("Thiếu dependency image")?;
        if dependencies.len() != 1 || dependencies[0]["package"] != "emulator" {
            return Err("Image dependency không hỗ trợ".into());
        }
        let minimum = &dependencies[0]["minimum_revision"];
        let minimum_emulator = package::revision(&format!(
            "{}.{}.{}",
            string(minimum, "major")?,
            string(minimum, "minor")?,
            string(minimum, "micro")?
        ))?;
        let license = &item["license"];
        let text = string(license, "text")?.to_owned();
        let sha256 = string(license, "text_sha256")?.to_owned();
        if license["accepted"] != false
            || text.trim().is_empty()
            || digest(text.as_bytes()) != sha256
        {
            return Err("License image không hợp lệ".into());
        }
        let license = SdkLicense {
            id: string(license, "id")?.into(),
            text,
            sha256,
        };
        let archive = SdkArchive {
            url: string(item, "url")?.into(),
            bytes: item["archive_bytes"]
                .as_u64()
                .ok_or("Thiếu archive bytes")?,
            checksum_algorithm: string(&item["published_checksum"], "algorithm")?.into(),
            checksum: string(&item["published_checksum"], "value")?.into(),
            license_sha256: license.sha256.clone(),
        };
        archive.validate(&license.sha256)?;
        if !archive
            .url
            .starts_with("https://dl.google.com/android/repository/sys-img/google_apis_playstore/")
        {
            return Err("Nguồn image không hợp lệ".into());
        }
        result.push(AndroidImage {
            version,
            revision,
            minimum_emulator,
            archive,
            license,
        });
    }
    if seen.len() != AndroidVersion::ALL.len() {
        return Err("Image catalog chưa đủ Android 12–16".into());
    }
    result.sort_by_key(|image| image.version);
    Ok(result)
}

pub(crate) fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

impl Backend {
    /// Presence and validated layout/dependencies, not a repeated full image checksum.
    pub fn android_images(&self) -> Result<Vec<ImageAvailability>, String> {
        let _guard = self.lock()?;
        android_image_catalog()?
            .into_iter()
            .map(|image| {
                let (revision, error) = match self.available_image_revision(image.version) {
                    Ok(revision) => (revision, None),
                    Err(error) => (None, Some(error)),
                };
                Ok(ImageAvailability {
                    version: image.version,
                    revision,
                    error,
                    download_bytes: image.archive.bytes,
                    usage: self.image_usage(image.version),
                })
            })
            .collect()
    }
    pub(crate) fn available_image_revision(
        &self,
        version: AndroidVersion,
    ) -> Result<Option<[u32; 3]>, String> {
        self.ensure_no_image_removal(version)?;
        match fs::symlink_metadata(self.sdk.join(version.directory())) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(err(e)),
            Ok(_) => self.installed_image_revision(version).map(Some),
        }
    }
    pub(crate) fn image_version_at(&self, avd: &Path) -> Result<AndroidVersion, String> {
        let props = package::properties(&avd.join("config.ini"))?;
        let relative = props
            .get("image.sysdir.1")
            .ok_or("AVD thiếu image.sysdir.1")?;
        let version = AndroidVersion::ALL
            .into_iter()
            .find(|v| relative.trim_end_matches('/') == v.directory())
            .ok_or("Image AVD không thuộc Android 12–16 Google Play ARM64")?;
        // Some AVD configs omit this redundant field. The package path remains
        // mandatory; an explicit target must still agree with its API.
        if props
            .get("target")
            .is_some_and(|target| target != &format!("android-{}", version.api()))
            || props.get("abi.type").is_some_and(|abi| abi != "arm64-v8a")
            || props
                .get("tag.id")
                .is_some_and(|tag| tag != "google_apis_playstore")
        {
            return Err("Cấu hình AVD mâu thuẫn với image Android".into());
        }
        Ok(version)
    }

    /// Layout/revision/dependency checks only; suitable for inexpensive readiness.
    pub(crate) fn installed_image_revision(
        &self,
        version: AndroidVersion,
    ) -> Result<[u32; 3], String> {
        self.ensure_no_image_removal(version)?;
        let root = self
            .sdk
            .join(version.directory())
            .canonicalize()
            .map_err(err)?;
        let props = package::properties(&root.join("source.properties"))?;
        let revision = props.get("Pkg.Revision").ok_or("Thiếu revision image")?;
        let verified =
            package::verify_sdk_package_root(&root, SdkPackage::PlayImage(version), revision)?;
        let engine = package::properties(&self.sdk.join("emulator/source.properties"))?;
        let installed =
            package::revision(engine.get("Pkg.Revision").ok_or("Thiếu revision engine")?)?;
        if installed
            < verified
                .minimum_emulator()
                .ok_or("Thiếu dependency engine")?
        {
            return Err(format!(
                "Engine hiện tại chưa đáp ứng {version}; Hub không tự nâng cấp engine"
            ));
        }
        Ok(verified.revision())
    }

    /// Hashing is deliberately outside snapshot polling; used at binding/start/restore gates.
    pub(crate) fn installed_image_identity(
        &self,
        version: AndroidVersion,
    ) -> Result<ImageIdentity, String> {
        let revision = self.installed_image_revision(version)?;
        let stamps = self.image_stamps(version)?;
        let cache_directory = self.runtime().join("image-fingerprints");
        let key = digest(
            self.sdk
                .join(version.directory())
                .canonicalize()
                .map_err(err)?
                .as_os_str()
                .as_encoded_bytes(),
        );
        let cache = cache_directory.join(format!("{key}.json"));
        if fs::symlink_metadata(&cache_directory).is_ok_and(|m| m.is_dir())
            && fs::symlink_metadata(&cache).is_ok_and(|m| m.is_file() && m.len() <= 65536)
            && let Ok(bytes) = fs::read(&cache)
            && let Ok(saved) = serde_json::from_slice::<Value>(&bytes)
            && saved["schema"] == 1
            && saved["stamps"] == stamps
            && let Ok(identity) = serde_json::from_value::<ImageIdentity>(saved["identity"].clone())
            && identity.validate().is_ok()
            && identity.version == version
            && identity.revision == revision
        {
            return Ok(identity);
        }
        let inventory = self.image_inventory(Path::new(version.directory()))?;
        // Recheck metadata after reading the large payload, refusing concurrent revision edits.
        if self.installed_image_revision(version)? != revision
            || self.image_stamps(version)? != stamps
        {
            return Err("Image thay đổi trong khi kiểm tra".into());
        }
        let identity = ImageIdentity {
            version,
            revision,
            fingerprint: crate::backup::image_fingerprint(&inventory)?,
        };
        // This cache is disposable. Failure to persist it must not bypass verification
        // or prevent an otherwise verified device from starting.
        if (cache_directory.is_dir() || fs::create_dir(&cache_directory).is_ok())
            && fs::symlink_metadata(&cache_directory).is_ok_and(|m| m.is_dir())
        {
            let _ = crate::restore::write_json(
                &cache,
                &serde_json::json!({"schema":1,"stamps":stamps,"identity":identity}),
            );
        }
        Ok(identity)
    }

    fn image_stamps(&self, version: AndroidVersion) -> Result<Value, String> {
        let root = self
            .sdk
            .join(version.directory())
            .canonicalize()
            .map_err(err)?;
        let mut stamps = serde_json::Map::new();
        for name in crate::backup::IMAGE_FILES {
            let meta = fs::symlink_metadata(root.join(name)).map_err(err)?;
            if !meta.is_file() {
                return Err("Image chứa file không hợp lệ".into());
            }
            stamps.insert(
                name.into(),
                serde_json::json!([
                    meta.dev(),
                    meta.ino(),
                    meta.len(),
                    meta.mtime(),
                    meta.mtime_nsec(),
                    meta.ctime(),
                    meta.ctime_nsec()
                ]),
            );
        }
        Ok(serde_json::json!({"root":root,"files":stamps}))
    }

    pub(crate) fn verify_profile_image(&self, profile: &Profile) -> Result<(), String> {
        let expected = profile.image.as_ref().ok_or("Chưa xác định được image máy cũ. Kiểm tra SDK/cấu hình AVD rồi mở lại Hub; không đổi image hoặc tạo đè máy.")?;
        if self.image_version_at(&self.data(profile))? != expected.version
            || self.installed_image_identity(expected.version)? != *expected
        {
            return Err("Image Android đã thay đổi so với lúc tạo máy. Khôi phục đúng image/revision; muốn dùng Android khác hãy tạo máy mới.".into());
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt, sync::atomic::Ordering};

    pub(crate) struct Fixture(pub Backend);
    impl Fixture {
        pub(crate) fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "hub-image-{}-{}",
                std::process::id(),
                crate::SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            let backend = Backend {
                sdk: root.join("sdk"),
                root,
            };
            fs::create_dir_all(backend.sdk.join("emulator")).unwrap();
            fs::write(
                backend.sdk.join("emulator/source.properties"),
                "Pkg.Revision=37.2.12\n",
            )
            .unwrap();
            for file in [
                "emulator/emulator",
                "platform-tools/adb",
                "cmdline-tools/latest/bin/avdmanager",
                "emulator/lib/hardware-properties.ini",
            ] {
                let path = backend.sdk.join(file);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(&path, "fixture").unwrap();
                fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
            }
            backend.write_profiles(&[]).unwrap();
            Self(backend)
        }
        pub(crate) fn image(&self, version: AndroidVersion) {
            let root = self.0.sdk.join(version.directory());
            fs::create_dir_all(root.join("data")).unwrap();
            fs::write(root.join("source.properties"), format!("Pkg.Revision=7\nPkg.Dependencies=emulator#35.4.9\nAndroidVersion.ApiLevel={}\nSystemImage.Abi=arm64-v8a\nSystemImage.TagId=google_apis_playstore\nAddon.VendorId=google\n", version.api())).unwrap();
            for file in [
                "system.img",
                "vendor.img",
                "ramdisk.img",
                "kernel-ranchu",
                "encryptionkey.img",
                "data/empty_data_disk",
            ] {
                fs::write(root.join(file), b"image fixture").unwrap();
            }
        }
        pub(crate) fn device(&self, version: AndroidVersion, id: &str, port: u16) -> Profile {
            let profile = Profile::standard(id.into(), id.into(), port);
            let data = self.0.data(&profile);
            fs::create_dir_all(&data).unwrap();
            fs::write(data.join("config.ini"), format!("target=android-{}\nimage.sysdir.1={}/\nabi.type=arm64-v8a\ntag.id=google_apis_playstore\n", version.api(), version.directory())).unwrap();
            fs::write(data.join("userdata.img"), b"keep user data").unwrap();
            profile
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0.root);
        }
    }

    #[test]
    fn reviewed_catalog_rejects_tampered_license_destination_and_unsupported_image() {
        let catalog = android_image_catalog().unwrap();
        assert_eq!(
            catalog.iter().map(|p| p.version.api()).collect::<Vec<_>>(),
            [31, 33, 34, 35, 36]
        );
        assert!(
            catalog
                .iter()
                .all(|p| p.archive.bytes > 0 && p.minimum_emulator <= [37, 2, 12])
        );
        let source: Value =
            serde_json::from_str(include_str!("../resources/android-images.json")).unwrap();
        for key in ["destination", "package"] {
            let mut value = source.clone();
            value["packages"][0][key] = "../../other".into();
            assert!(parse_catalog(&value.to_string()).is_err());
        }
        let mut value = source.clone();
        value["packages"][0]["license"]["accepted"] = true.into();
        assert!(parse_catalog(&value.to_string()).is_err());
        value["packages"][0]["license"]["accepted"] = false.into();
        value["packages"][0]["license"]["text"] = "different terms".into();
        assert!(parse_catalog(&value.to_string()).is_err());
    }

    #[test]
    fn bound_image_backup_accepts_avd_trailing_slash_but_rejects_payload_changes() {
        let fixture = Fixture::new();
        let b = &fixture.0;
        let version = AndroidVersion::Android13;
        fixture.image(version);
        let mut profile = fixture.device(version, "hub-backup-image", 5664);
        profile.image = Some(b.installed_image_identity(version).unwrap());
        b.write_profiles(&[profile.clone()]).unwrap();
        let backup = b.backup_locked(profile.clone()).unwrap();
        assert_eq!(
            b.verify_restore_backup(&profile.id, &backup).unwrap(),
            profile
        );
        fs::write(
            b.sdk.join(version.directory()).join("system.img"),
            b"changed",
        )
        .unwrap();
        assert!(b.backup_locked(profile).unwrap_err().contains("identity"));
    }

    #[test]
    fn missing_other_versions_does_not_block_valid_image_and_changed_payload_is_rejected() {
        let fixture = Fixture::new();
        let b = &fixture.0;
        let version = AndroidVersion::Android16;
        fixture.image(version);
        assert_eq!(b.installed_image_revision(version).unwrap(), [7, 0, 0]);
        assert!(
            b.installed_image_revision(AndroidVersion::Android13)
                .is_err()
        );
        let mut p = fixture.device(version, "hub-image", 5664);
        p.image = Some(b.installed_image_identity(version).unwrap());
        b.verify_profile_image(&p).unwrap();
        fs::write(
            b.sdk.join(version.directory()).join("system.img"),
            b"same revision, modified payload",
        )
        .unwrap();
        assert!(b.verify_profile_image(&p).unwrap_err().contains("thay đổi"));
        fixture.image(version);
        fs::write(
            b.sdk.join("emulator/source.properties"),
            "Pkg.Revision=31.1\n",
        )
        .unwrap();
        assert!(
            b.installed_image_revision(version)
                .unwrap_err()
                .contains("không tự nâng cấp")
        );
    }

    #[test]
    fn missing_avd_target_preserves_verified_image_binding_and_backup() {
        for version in AndroidVersion::ALL {
            let fixture = Fixture::new();
            let b = &fixture.0;
            fixture.image(version);
            let mut profile = fixture.device(version, "hub-no-target", 5664);
            profile.image = Some(b.installed_image_identity(version).unwrap());
            let path = b.data(&profile).join("config.ini");
            let config = format!(
                "image.sysdir.1={}/\nabi.type=arm64-v8a\ntag.id=google_apis_playstore\n",
                version.directory()
            );
            fs::write(&path, &config).unwrap();
            b.verify_profile_image(&profile).unwrap();
            b.write_profiles(&[profile.clone()]).unwrap();
            let backup = b.backup_locked(profile.clone()).unwrap();
            assert_eq!(
                b.verify_restore_backup(&profile.id, &backup).unwrap(),
                profile
            );
            assert_eq!(fs::read_to_string(&path).unwrap(), config);

            for wrong in [
                format!("{config}target=android-99\n"),
                format!("{config}target=\n"),
                config.replace("abi.type=arm64-v8a", "abi.type=x86_64"),
                config.replace("tag.id=google_apis_playstore", "tag.id=google_apis"),
                config.replace(
                    version.directory(),
                    "system-images/android-99/google_apis_playstore/arm64-v8a",
                ),
                config
                    .lines()
                    .filter(|line| !line.starts_with("image.sysdir.1="))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ] {
                fs::write(&path, wrong).unwrap();
                assert!(b.verify_profile_image(&profile).is_err());
            }
            fs::write(&path, &config).unwrap();
            fs::write(
                b.sdk.join(version.directory()).join("system.img"),
                b"changed payload",
            )
            .unwrap();
            assert!(b.verify_profile_image(&profile).is_err());
        }
    }

    #[test]
    fn image_metadata_and_avd_must_agree_with_the_selected_stable_package() {
        let fixture = Fixture::new();
        let version = AndroidVersion::Android12;
        fixture.image(version);
        let b = &fixture.0;
        let profile = fixture.device(version, "hub-image", 5664);
        assert_eq!(b.image_version_at(&b.data(&profile)).unwrap(), version);
        let path = b.data(&profile).join("config.ini");
        let original = fs::read_to_string(&path).unwrap();
        for wrong in [
            original.replace("target=android-31", "target=android-36"),
            original.replace("abi.type=arm64-v8a", "abi.type=x86_64"),
            original.replace("tag.id=google_apis_playstore", "tag.id=google_apis"),
        ] {
            fs::write(&path, wrong).unwrap();
            assert!(b.image_version_at(&b.data(&profile)).is_err());
        }
        let path = b.sdk.join(version.directory()).join("source.properties");
        let original = fs::read_to_string(&path).unwrap();
        fs::write(path, original.replace("ApiLevel=31", "ApiLevel=36")).unwrap();
        assert!(b.installed_image_revision(version).is_err());
    }
}
