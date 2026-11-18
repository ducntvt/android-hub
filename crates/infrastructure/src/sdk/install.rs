//! Assemble verified cached archives into a new SDK. Never changes active preferences.
use crate::{
    Backend, SEQUENCE, SdkArchive, SdkExtractionControl, SdkPackage, err,
    restore::{sync_dir, write_json},
    sdk::download::file_size,
    verify_sdk_package, verify_sdk_package_set,
};
#[cfg(test)]
use fs2::FileExt;
use serde_json::json;
use std::{
    collections::BTreeSet,
    fs,
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug)]
pub struct SdkInstallPackage {
    pub kind: SdkPackage,
    pub revision: String,
    pub archive: SdkArchive,
    /// Exact license digest explicitly supplied by the caller; UI acceptance is separate.
    pub accepted_license_sha256: String,
}
#[derive(Clone)]
pub struct SdkInstallControl {
    canceled: Arc<AtomicBool>,
    started: Arc<AtomicBool>,
    completed_packages: Arc<AtomicUsize>,
    extraction: [SdkExtractionControl; 4],
}
impl Default for SdkInstallControl {
    fn default() -> Self {
        Self {
            canceled: Arc::default(),
            started: Arc::default(),
            completed_packages: Arc::default(),
            extraction: std::array::from_fn(|_| SdkExtractionControl::default()),
        }
    }
}
impl SdkInstallControl {
    pub fn cancel(&self) {
        self.canceled.store(true, Ordering::Release);
        for control in &self.extraction {
            control.cancel();
        }
    }
    pub fn completed_packages(&self) -> usize {
        self.completed_packages.load(Ordering::Acquire)
    }
    pub fn written_bytes(&self) -> u64 {
        self.extraction
            .iter()
            .map(SdkExtractionControl::written_bytes)
            .sum()
    }
    fn check(&self) -> Result<(), String> {
        if self.canceled.load(Ordering::Acquire) {
            Err("Đã hủy ghép SDK".into())
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Debug)]
pub struct ManagedSdk {
    pub path: PathBuf,
    pub java_home: PathBuf,
}

fn private_directory(path: &Path) -> Result<(), String> {
    match fs::DirBuilder::new().mode(0o700).create(path) {
        Ok(()) => Ok(()),
        Err(e)
            if e.kind() == std::io::ErrorKind::AlreadyExists
                && fs::symlink_metadata(path).is_ok_and(|m| m.is_dir()) =>
        {
            Ok(())
        }
        Err(e) => Err(err(e)),
    }
}
fn sync_tree(
    path: &Path,
    remaining: &mut usize,
    control: &SdkInstallControl,
) -> Result<(), String> {
    control.check()?;
    *remaining = remaining
        .checked_sub(1)
        .ok_or("SDK vượt giới hạn entries")?;
    let metadata = fs::symlink_metadata(path).map_err(err)?;
    if metadata.is_dir() {
        for entry in fs::read_dir(path).map_err(err)? {
            sync_tree(&entry.map_err(err)?.path(), remaining, control)?;
        }
    } else if !metadata.is_file() || metadata.nlink() != 1 {
        return Err("SDK sau probe chứa link/file đặc biệt".into());
    }
    fs::File::open(path).map_err(err)?.sync_all().map_err(err)
}

impl Backend {
    /// Requires all four archives already downloaded with their reviewed plan/digests.
    /// Returns a new published SDK directory; caller must separately select it.
    pub fn assemble_sdk(
        &self,
        packages: &[SdkInstallPackage],
        java_home: &Path,
        control: &SdkInstallControl,
    ) -> Result<ManagedSdk, String> {
        if control.started.swap(true, Ordering::AcqRel) {
            return Err("Mỗi control chỉ dùng cho một lượt ghép SDK".into());
        }
        control.check()?;
        if packages.len() != 4
            || packages
                .iter()
                .map(|p| p.kind)
                .collect::<BTreeSet<_>>()
                .len()
                != 4
        {
            return Err("Cần đúng bốn package SDK khác nhau".into());
        }
        for package in packages {
            package.archive.validate(&package.accepted_license_sha256)?;
        }
        if !java_home.is_absolute() {
            return Err("Java home phải là đường dẫn tuyệt đối".into());
        }
        fs::create_dir_all(self.runtime()).map_err(err)?;
        let managed = self.runtime().join("managed-sdks");
        private_directory(&managed)?;
        let lock_path = managed.join("install.lock");
        file_size(&lock_path)?;
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .mode(0o600)
            .open(lock_path)
            .map_err(err)?;
        // Probe subprocesses have completed before this guard drops. Explicit
        // unlock also releases transient descriptors inherited by unrelated forks.
        let _lock =
            super::cache_lock::SdkCacheLock::acquire(lock, "Một lượt ghép SDK khác đang chạy")?;
        let id = format!(
            "{}-{}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(err)?
                .as_millis(),
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let stage = managed.join(format!(".staging-{id}"));
        let published = managed.join(format!("sdk-{id}"));
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&stage)
            .map_err(err)?;
        let sdk = stage.join("sdk");
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&sdk)
            .map_err(err)?;
        let mut receipt = json!({"schema":1,"id":id,"state":"assembling","packages":packages.iter().map(|p| json!({"destination":p.kind.destination(),"revision":p.revision,"url":p.archive.url,"archive_bytes":p.archive.bytes,"checksum_algorithm":p.archive.checksum_algorithm,"checksum":p.archive.checksum,"license_sha256":p.accepted_license_sha256})).collect::<Vec<_>>()});
        write_json(&stage.join("assembly.json"), &receipt)?;
        let result: Result<ManagedSdk, String> = (|| {
            let mut verified = Vec::new();
            receipt["extractions"] = json!([]);
            for (index, package) in packages.iter().enumerate() {
                control.check()?;
                let extraction = self.extract_sdk_archive(
                    &package.archive,
                    &package.accepted_license_sha256,
                    &control.extraction[index],
                )?;
                receipt["extractions"].as_array_mut().ok_or("Invalid extraction journal")?.push(json!({"path":extraction.directory,"destination":package.kind.destination()}));
                write_json(&stage.join("assembly.json"), &receipt)?;
                verified.push(verify_sdk_package(
                    &extraction.directory,
                    package.kind,
                    &package.revision,
                )?);
                control
                    .completed_packages
                    .store(index + 1, Ordering::Release);
            }
            verify_sdk_package_set(&verified)?;
            for package in &verified {
                control.check()?;
                let target = sdk.join(package.kind().destination());
                fs::DirBuilder::new()
                    .recursive(true)
                    .mode(0o700)
                    .create(target.parent().ok_or("Thiếu SDK destination parent")?)
                    .map_err(err)?;
                // Fresh private SDK, fixed destinations, same runtime filesystem: no payload copy.
                fs::rename(package.root(), &target).map_err(err)?;
                sync_dir(package.root().parent().ok_or("Thiếu extraction parent")?)?;
                #[cfg(test)]
                if package.kind() == SdkPackage::CommandLineTools {
                    tests::crash_checkpoint(self, "first-package-moved");
                }
            }
            let emulator = verified
                .iter()
                .find(|package| package.kind() == SdkPackage::Emulator)
                .ok_or("Thiếu emulator package")?;
            super::registration::register_emulator(&sdk, emulator.revision())?;
            let candidate = Backend {
                root: self.root.clone(),
                sdk: sdk.clone(),
            };
            receipt["state"] = json!("probing");
            receipt["published_path"] = json!(published);
            write_json(&stage.join("assembly.json"), &receipt)?;
            let probe = candidate.probe_sdk_tools(java_home)?;
            control.check()?;
            receipt["state"] = json!("ready");
            receipt["java_home"] = json!(probe.java_home);
            receipt["java_major"] = json!(probe.java_major);
            receipt["java_arch"] = json!(probe.java_arch);
            receipt["adb_version"] = json!(probe.adb_version);
            receipt["emulator_version"] = json!(probe.emulator_version);
            write_json(&sdk.join(".hub-sdk.json"), &receipt)?;
            sync_tree(&sdk, &mut 500_000, control)?;
            sync_dir(&stage)?;
            control.check()?;
            if fs::symlink_metadata(&published).is_ok() {
                return Err("SDK destination đã tồn tại; không ghi đè".into());
            }
            #[cfg(test)]
            tests::crash_checkpoint(self, "before-publish");
            fs::rename(&sdk, &published).map_err(err)?;
            #[cfg(test)]
            tests::crash_checkpoint(self, "after-publish");
            // The rename is the commit point. Report the published path even if durability sync fails.
            sync_dir(&managed)
                .and_then(|_| sync_dir(&stage))
                .map_err(|e| {
                    format!(
                        "SDK đã publish tại {} nhưng sync chưa đạt: {e}",
                        published.display()
                    )
                })?;
            Ok(ManagedSdk {
                path: published.clone(),
                java_home: probe.java_home,
            })
        })();
        match result {
            Ok(result) => Ok(result),
            Err(error) => Err(format!(
                "{error}; dữ liệu ghép SDK giữ tại {}; lựa chọn SDK hiện tại không đổi",
                stage.display()
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Cursor, Write},
        os::unix::fs::PermissionsExt,
    };
    use zip::{ZipWriter, write::SimpleFileOptions};
    pub(super) fn crash_checkpoint(backend: &Backend, point: &str) {
        if std::env::var_os("HUB_ASSEMBLY_CRASH_ROOT").as_deref() != Some(backend.root.as_os_str())
            || std::env::var("HUB_ASSEMBLY_CRASH_POINT").ok().as_deref() != Some(point)
        {
            return;
        }
        fs::write(backend.root.join("crash-checkpoint"), point).unwrap();
        // The owning test SIGKILLs this worker. Bound the wait if its controller disappears.
        std::thread::sleep(std::time::Duration::from_secs(15));
        panic!("Crash controller did not kill worker at {point}");
    }
    struct OwnedDriver(std::process::Child);
    impl Drop for OwnedDriver {
        fn drop(&mut self) {
            if !matches!(self.0.try_wait(), Ok(Some(_))) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
    }
    struct Fixture {
        backend: Backend,
        java: PathBuf,
        packages: Vec<SdkInstallPackage>,
    }
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "hub-assembly-fixture-{}-{}",
                std::process::id(),
                SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            Self::new_at(root)
        }
        fn new_at(root: PathBuf) -> Self {
            let backend = Backend {
                sdk: root.join("existing-sdk"),
                root: root.clone(),
            };
            let cache = backend.runtime().join("sdk-downloads");
            fs::create_dir_all(&cache).unwrap();
            fs::write(cache.join("download.lock"), b"").unwrap();
            fs::create_dir(&backend.sdk).unwrap();
            fs::write(backend.sdk.join("keep"), b"existing SDK").unwrap();
            fs::write(
                backend.runtime().join("hub-settings.json"),
                b"existing preferences untouched",
            )
            .unwrap();
            let java = root.join("fixture-java");
            fs::create_dir_all(java.join("bin")).unwrap();
            fs::write(java.join("bin/java"), b"#!/bin/sh\ncase \"$*\" in *-version*) printf 'java.specification.version = 21\\nos.arch = aarch64\\n';; *) printf 'id: 1 or \"pixel_5\"\\n';; esac\n").unwrap();
            fs::set_permissions(java.join("bin/java"), fs::Permissions::from_mode(0o700)).unwrap();
            let mut packages = Vec::new();
            for (kind, top, revision, properties, files) in [
                (
                    SdkPackage::CommandLineTools,
                    "cmdline-tools",
                    "23.0",
                    "Pkg.Path=cmdline-tools;23.0\n",
                    vec![
                        "bin/android",
                        "bin/avdmanager",
                        "bin/sdkmanager",
                        "lib/avdmanager-classpath.jar",
                    ],
                ),
                (
                    SdkPackage::PlatformTools,
                    "platform-tools",
                    "37.0.1",
                    "",
                    vec!["adb"],
                ),
                (
                    SdkPackage::Emulator,
                    "emulator",
                    "37.2.13",
                    "Pkg.Path=emulator\n",
                    vec![
                        "emulator",
                        "qemu/darwin-aarch64/qemu-system-aarch64",
                        "lib/hardware-properties.ini",
                    ],
                ),
                (
                    SdkPackage::PlayImage36,
                    "arm64-v8a",
                    "7",
                    "AndroidVersion.ApiLevel=36\nSystemImage.Abi=arm64-v8a\nSystemImage.TagId=google_apis_playstore\nAddon.VendorId=google\nPkg.Dependencies=emulator#35.4.9\n",
                    vec![
                        "system.img",
                        "vendor.img",
                        "ramdisk.img",
                        "kernel-ranchu",
                        "encryptionkey.img",
                        "data/empty_data_disk",
                    ],
                ),
            ] {
                let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
                let options = SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored)
                    .unix_permissions(0o755);
                writer
                    .start_file(format!("{top}/source.properties"), options)
                    .unwrap();
                writer
                    .write_all(format!("Pkg.Revision={revision}\n{properties}").as_bytes())
                    .unwrap();
                for file in files {
                    writer.start_file(format!("{top}/{file}"), options).unwrap();
                    let content = match (kind, file) {
                        (SdkPackage::PlatformTools, "adb") => {
                            "#!/bin/sh\nprintf 'Android Debug Bridge version 1.0.41\\n'\n"
                        }
                        (SdkPackage::Emulator, "emulator") => {
                            "#!/bin/sh\nprintf 'Android emulator version 37.2.13.0\\n'\n"
                        }
                        _ => "fixture payload, not real SDK",
                    };
                    writer.write_all(content.as_bytes()).unwrap();
                }
                let bytes = writer.finish().unwrap().into_inner();
                let temporary = cache.join("fixture.zip");
                fs::write(&temporary, &bytes).unwrap();
                let checksum = backend.file_digest(&temporary).unwrap();
                fs::rename(temporary, cache.join(format!("sha256-{checksum}.zip"))).unwrap();
                packages.push(SdkInstallPackage {
                    kind,
                    revision: revision.into(),
                    archive: SdkArchive {
                        url: "https://dl.google.com/android/repository/fixture.zip".into(),
                        bytes: bytes.len() as u64,
                        checksum_algorithm: "sha256".into(),
                        checksum,
                        license_sha256: "a".repeat(64),
                    },
                    accepted_license_sha256: "a".repeat(64),
                });
            }
            Self {
                backend,
                java,
                packages,
            }
        }
        fn assert_untouched(&self) {
            assert_eq!(
                fs::read(self.backend.sdk.join("keep")).unwrap(),
                b"existing SDK"
            );
            assert_eq!(
                fs::read(self.backend.runtime().join("hub-settings.json")).unwrap(),
                b"existing preferences untouched"
            );
        }
        fn published(&self) -> usize {
            fs::read_dir(self.backend.runtime().join("managed-sdks"))
                .unwrap()
                .filter_map(Result::ok)
                .filter(|e| e.file_name().to_string_lossy().starts_with("sdk-"))
                .count()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.backend.root);
        }
    }
    #[test]
    fn cached_packages_publish_new_sdk_without_selecting_or_copying_user_data() {
        let fixture = Fixture::new();
        let control = SdkInstallControl::default();
        let result = fixture
            .backend
            .assemble_sdk(&fixture.packages, &fixture.java, &control)
            .unwrap();
        assert_eq!(fixture.published(), 1);
        assert_eq!(control.completed_packages(), 4);
        assert!(control.written_bytes() > 0);
        assert_eq!(result.java_home, fixture.java.canonicalize().unwrap());
        assert!(
            result
                .path
                .join("system-images/android-36/google_apis_playstore/arm64-v8a/system.img")
                .is_file()
        );
        let receipt: serde_json::Value =
            serde_json::from_slice(&fs::read(result.path.join(".hub-sdk.json")).unwrap()).unwrap();
        assert_eq!(receipt["state"], "ready");
        assert_eq!(receipt["packages"].as_array().unwrap().len(), 4);
        fixture.assert_untouched();
        assert!(
            fixture
                .backend
                .assemble_sdk(&fixture.packages, &fixture.java, &control)
                .is_err()
        );
    }
    #[test]
    fn failed_probe_does_not_publish_and_retry_uses_cached_archives() {
        let fixture = Fixture::new();
        assert!(
            fixture
                .backend
                .assemble_sdk(
                    &fixture.packages,
                    &fixture.java.join("missing"),
                    &SdkInstallControl::default()
                )
                .is_err()
        );
        assert_eq!(fixture.published(), 0);
        fixture.assert_untouched();
        let result = fixture
            .backend
            .assemble_sdk(
                &fixture.packages,
                &fixture.java,
                &SdkInstallControl::default(),
            )
            .unwrap();
        assert!(result.path.is_dir());
        assert_eq!(fixture.published(), 1);
        fixture.assert_untouched();
    }
    #[test]
    fn catalog_distinguishes_published_incomplete_invalid_and_busy() {
        let fixture = Fixture::new();
        assert!(
            fixture
                .backend
                .managed_sdk_catalog()
                .unwrap()
                .sdks
                .is_empty()
        );
        assert!(!fixture.backend.runtime().join("managed-sdks").exists());
        let result = fixture
            .backend
            .assemble_sdk(
                &fixture.packages,
                &fixture.java,
                &SdkInstallControl::default(),
            )
            .unwrap();
        let managed = fixture.backend.runtime().join("managed-sdks");
        let catalog = fixture.backend.managed_sdk_catalog().unwrap();
        assert_eq!(
            (catalog.sdks.len(), catalog.incomplete, catalog.skipped),
            (1, 0, 0)
        );
        assert_eq!(catalog.sdks[0].path, result.path);
        assert_eq!(catalog.sdks[0].emulator_revision, "37.2.13");
        fs::create_dir(managed.join(".staging-1-2-3")).unwrap();
        std::os::unix::fs::symlink(&result.path, managed.join("sdk-4-5-6")).unwrap();
        let catalog = fixture.backend.managed_sdk_catalog().unwrap();
        assert_eq!(
            (catalog.sdks.len(), catalog.incomplete, catalog.skipped),
            (1, 1, 1)
        );
        let marker = result.path.join(".hub-sdk.json");
        let original = fs::read(&marker).unwrap();
        let mut bad: serde_json::Value = serde_json::from_slice(&original).unwrap();
        bad["id"] = json!("wrong");
        fs::write(&marker, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(
            fixture
                .backend
                .managed_sdk_catalog()
                .unwrap()
                .sdks
                .is_empty()
        );
        fs::write(&marker, original).unwrap();
        fs::remove_file(result.path.join("platform-tools/adb")).unwrap();
        assert!(
            fixture
                .backend
                .managed_sdk_catalog()
                .unwrap()
                .sdks
                .is_empty()
        );
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(managed.join("install.lock"))
            .unwrap();
        lock.try_lock_exclusive().unwrap();
        assert!(
            fixture
                .backend
                .managed_sdk_catalog()
                .unwrap_err()
                .contains("đang được ghép")
        );
        fixture.assert_untouched();
    }
    #[test]
    fn setup_workflow_uses_verified_cache_and_never_selects_result() {
        let fixture = Fixture::new();
        let control = crate::SdkSetupControl::default();
        let result = fixture
            .backend
            .run_sdk_packages(&fixture.packages, &fixture.java, &control)
            .unwrap();
        assert_eq!(control.stage(), crate::SdkSetupStage::Complete);
        assert_eq!(
            control.downloaded_bytes(),
            fixture
                .packages
                .iter()
                .map(|p| p.archive.bytes)
                .sum::<u64>()
        );
        assert!(control.extracted_bytes() > 0);
        assert!(result.path.join(".hub-sdk.json").is_file());
        fixture.assert_untouched();
    }
    #[test]
    fn setup_failure_keeps_cache_and_retry_requires_fresh_control() {
        let fixture = Fixture::new();
        let control = crate::SdkSetupControl::default();
        assert!(
            fixture
                .backend
                .run_sdk_packages(&fixture.packages, &fixture.java.join("missing"), &control)
                .is_err()
        );
        assert_eq!(control.stage(), crate::SdkSetupStage::Failed);
        assert_eq!(control.downloaded_bytes(), 0);
        assert!(!fixture.backend.runtime().join("managed-sdks").exists());
        assert!(
            fixture
                .backend
                .run_sdk_packages(&fixture.packages, &fixture.java, &control)
                .unwrap_err()
                .contains("Mỗi control")
        );
        let fresh = crate::SdkSetupControl::default();
        fixture
            .backend
            .run_sdk_packages(&fixture.packages, &fixture.java, &fresh)
            .unwrap();
        assert_eq!(fresh.stage(), crate::SdkSetupStage::Complete);
        fixture.assert_untouched();
    }
    #[test]
    #[ignore = "process/port gates on disposable SDK fixture; no real SDK or guest mutation"]
    fn select_published_sdk_updates_both_preferences_and_preserves_migration_guard() {
        assert!(std::env::var_os("ANDROID_SDK_ROOT").is_none());
        assert!(Backend::java_override().is_none());
        let fixture = Fixture::new();
        fs::write(fixture.backend.runtime().join("hub-profiles.json"), b"[]").unwrap();
        let first = fixture
            .backend
            .assemble_sdk(
                &fixture.packages,
                &fixture.java,
                &SdkInstallControl::default(),
            )
            .unwrap();
        let (sdk, java) = fixture.backend.configure_managed_sdk(&first.path).unwrap();
        let loaded = Backend::new(fixture.backend.root.clone()).unwrap();
        assert_eq!(loaded.sdk, sdk);
        assert_eq!(loaded.java_home().unwrap(), java);
        let preferences = fs::read(fixture.backend.runtime().join("hub-settings.json")).unwrap();
        fs::rename(
            fixture.java.join("bin/java"),
            fixture.java.join("bin/java-saved"),
        )
        .unwrap();
        assert!(fixture.backend.configure_managed_sdk(&first.path).is_err());
        assert_eq!(
            fs::read(fixture.backend.runtime().join("hub-settings.json")).unwrap(),
            preferences
        );
        fs::rename(
            fixture.java.join("bin/java-saved"),
            fixture.java.join("bin/java"),
        )
        .unwrap();
        let second = fixture
            .backend
            .assemble_sdk(
                &fixture.packages,
                &fixture.java,
                &SdkInstallControl::default(),
            )
            .unwrap();
        fs::create_dir_all(fixture.backend.avds().join(".removed-fixture")).unwrap();
        assert!(
            fixture
                .backend
                .configure_managed_sdk(&second.path)
                .unwrap_err()
                .contains("AVD/archive")
        );
        assert_eq!(
            fs::read(fixture.backend.runtime().join("hub-settings.json")).unwrap(),
            preferences
        );
        assert_eq!(
            fs::read(fixture.backend.sdk.join("keep")).unwrap(),
            b"existing SDK"
        );
    }
    #[test]
    fn cancel_license_mismatch_and_lock_contention_do_not_publish() {
        let mut fixture = Fixture::new();
        let control = SdkInstallControl::default();
        control.cancel();
        assert!(
            fixture
                .backend
                .assemble_sdk(&fixture.packages, &fixture.java, &control)
                .unwrap_err()
                .contains("hủy")
        );
        fixture.packages[0].accepted_license_sha256 = "b".repeat(64);
        assert!(
            fixture
                .backend
                .assemble_sdk(
                    &fixture.packages,
                    &fixture.java,
                    &SdkInstallControl::default()
                )
                .is_err()
        );
        assert!(!fixture.backend.runtime().join("managed-sdks").exists());
        fixture.packages[0].accepted_license_sha256 = "a".repeat(64);
        let managed = fixture.backend.runtime().join("managed-sdks");
        private_directory(&managed).unwrap();
        let lock = fs::OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(managed.join("install.lock"))
            .unwrap();
        lock.try_lock_exclusive().unwrap();
        assert!(
            fixture
                .backend
                .assemble_sdk(
                    &fixture.packages,
                    &fixture.java,
                    &SdkInstallControl::default()
                )
                .unwrap_err()
                .contains("khác đang chạy")
        );
        assert_eq!(fixture.published(), 0);
        fixture.assert_untouched();
    }

    #[test]
    #[ignore = "helper for owned assembly SIGKILL test; no-op without explicit fixture environment"]
    fn crash_assembly_worker() {
        let Some(root) = std::env::var_os("HUB_ASSEMBLY_CRASH_ROOT") else {
            return;
        };
        let fixture = Fixture::new_at(PathBuf::from(root));
        fixture
            .backend
            .assemble_sdk(
                &fixture.packages,
                &fixture.java,
                &SdkInstallControl::default(),
            )
            .unwrap();
        panic!("Worker should have stopped at the requested checkpoint");
    }

    #[test]
    #[ignore = "SIGKILLs only owned fixture workers at package move and publish boundaries"]
    fn assembly_sigkill_preserves_commit_boundary_and_cache_retry() {
        use std::{
            os::unix::process::ExitStatusExt,
            process::{Command, Stdio},
            time::{Duration, Instant},
        };
        for point in ["first-package-moved", "before-publish", "after-publish"] {
            let root = std::env::temp_dir().join(format!(
                "hub-assembly-crash-{}-{}",
                std::process::id(),
                SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            let mut driver = OwnedDriver(
                Command::new(std::env::current_exe().unwrap())
                    .args([
                        "--exact",
                        "sdk::install::tests::crash_assembly_worker",
                        "--ignored",
                        "--nocapture",
                    ])
                    .env("HUB_ASSEMBLY_CRASH_ROOT", &root)
                    .env("HUB_ASSEMBLY_CRASH_POINT", point)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::inherit())
                    .spawn()
                    .unwrap(),
            );
            let deadline = Instant::now() + Duration::from_secs(10);
            while !root.join("crash-checkpoint").exists() {
                assert!(
                    driver.0.try_wait().unwrap().is_none(),
                    "worker exited before {point}"
                );
                assert!(Instant::now() < deadline, "worker did not reach {point}");
                std::thread::sleep(Duration::from_millis(10));
            }
            assert!(driver.0.try_wait().unwrap().is_none());
            let backend = Backend {
                sdk: root.join("existing-sdk"),
                root: root.clone(),
            };
            let managed = backend.runtime().join("managed-sdks");
            let observer = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(managed.join("install.lock"))
                .unwrap();
            assert!(observer.try_lock_exclusive().is_err());
            driver.0.kill().unwrap();
            assert_eq!(driver.0.wait().unwrap().signal(), Some(9));
            observer.try_lock_exclusive().unwrap();
            drop(observer);
            let stage = fs::read_dir(&managed)
                .unwrap()
                .map(Result::unwrap)
                .find(|e| e.file_name().to_string_lossy().starts_with(".staging-"))
                .unwrap()
                .path();
            let receipt: serde_json::Value =
                serde_json::from_slice(&fs::read(stage.join("assembly.json")).unwrap()).unwrap();
            let packages = receipt["packages"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| {
                    let kind = [
                        SdkPackage::CommandLineTools,
                        SdkPackage::PlatformTools,
                        SdkPackage::Emulator,
                        SdkPackage::PlayImage36,
                    ]
                    .into_iter()
                    .find(|k| Some(k.destination()) == p["destination"].as_str())
                    .unwrap();
                    SdkInstallPackage {
                        kind,
                        revision: p["revision"].as_str().unwrap().into(),
                        archive: SdkArchive {
                            url: p["url"].as_str().unwrap().into(),
                            bytes: p["archive_bytes"].as_u64().unwrap(),
                            checksum_algorithm: p["checksum_algorithm"].as_str().unwrap().into(),
                            checksum: p["checksum"].as_str().unwrap().into(),
                            license_sha256: p["license_sha256"].as_str().unwrap().into(),
                        },
                        accepted_license_sha256: p["license_sha256"].as_str().unwrap().into(),
                    }
                })
                .collect();
            let fixture = Fixture {
                backend,
                java: root.join("fixture-java"),
                packages,
            };
            fixture.assert_untouched();
            let committed = usize::from(point == "after-publish");
            assert_eq!(fixture.published(), committed);
            let catalog = fixture.backend.managed_sdk_catalog().unwrap();
            assert_eq!(catalog.sdks.len(), committed);
            assert_eq!(catalog.incomplete, 1 - committed);
            assert_eq!(catalog.skipped, 0);
            if committed == 1 {
                let published = PathBuf::from(receipt["published_path"].as_str().unwrap());
                assert!(!stage.join("sdk").exists());
                let marker: serde_json::Value =
                    serde_json::from_slice(&fs::read(published.join(".hub-sdk.json")).unwrap())
                        .unwrap();
                assert_eq!(marker["state"], "ready");
                let candidate = Backend {
                    root: root.clone(),
                    sdk: published.clone(),
                };
                candidate.probe_sdk_tools(&fixture.java).unwrap();
                assert!(
                    published
                        .join("system-images/android-36/google_apis_playstore/arm64-v8a/system.img")
                        .is_file()
                );
            } else {
                assert!(stage.join("sdk").is_dir());
                if point == "before-publish" {
                    assert!(stage.join("sdk/.hub-sdk.json").is_file());
                }
            }
            // Retry always re-verifies cached archives and creates a new SDK; never reuses partial trees.
            let retried = fixture
                .backend
                .assemble_sdk(
                    &fixture.packages,
                    &fixture.java,
                    &SdkInstallControl::default(),
                )
                .unwrap();
            assert!(retried.path.join(".hub-sdk.json").is_file());
            assert_eq!(fixture.published(), committed + 1);
            fixture.assert_untouched();
            assert!(
                stage.join("assembly.json").is_file(),
                "diagnostic journal preserved"
            );
            println!(
                "SIGKILL {point}: commit boundary, unchanged preferences/SDK, cache retry passed"
            );
        }
    }
}
