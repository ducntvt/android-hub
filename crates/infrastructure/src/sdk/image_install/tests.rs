use super::*;
use crate::{SdkArchive, SdkSetupControl, SdkSetupStage, images::tests::Fixture};
use std::{
    collections::BTreeSet,
    io::{Cursor, Write},
};
use zip::{ZipWriter, write::SimpleFileOptions};

fn cached_image(b: &Backend, version: AndroidVersion) -> SdkInstallPackage {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .unix_permissions(0o600);
    zip.start_file("arm64-v8a/source.properties", options)
        .unwrap();
    write!(zip, "Pkg.Revision=9\nPkg.Dependencies=emulator#35.4.9\nAndroidVersion.ApiLevel={}\nSystemImage.Abi=arm64-v8a\nSystemImage.TagId=google_apis_playstore\nAddon.VendorId=google\n",version.api()).unwrap();
    for file in [
        "system.img",
        "vendor.img",
        "ramdisk.img",
        "kernel-ranchu",
        "encryptionkey.img",
        "data/empty_data_disk",
    ] {
        zip.start_file(format!("arm64-v8a/{file}"), options)
            .unwrap();
        zip.write_all(b"fixture image").unwrap();
    }
    let bytes = zip.finish().unwrap().into_inner();
    let checksum = digest(&bytes);
    let cache = b.runtime().join("sdk-downloads");
    fs::create_dir_all(&cache).unwrap();
    fs::write(cache.join("download.lock"), b"").unwrap();
    fs::write(cache.join(format!("sha256-{checksum}.zip")), &bytes).unwrap();
    SdkInstallPackage {
        kind: SdkPackage::PlayImage(version),
        revision: "9.0.0".into(),
        accepted_license_sha256: "a".repeat(64),
        archive: SdkArchive {
            url:
                "https://dl.google.com/android/repository/sys-img/google_apis_playstore/fixture.zip"
                    .into(),
            bytes: bytes.len() as u64,
            checksum_algorithm: "sha256".into(),
            checksum,
            license_sha256: "a".repeat(64),
        },
    }
}

#[test]
fn cached_image_publishes_atomically_then_reuses_without_downloading_or_modifying_other_devices() {
    let fixture = Fixture::new();
    let b = &fixture.0;
    fixture.image(AndroidVersion::Android16);
    let mut old = fixture.device(AndroidVersion::Android16, "hub-unchanged", 5664);
    old.image = Some(
        b.installed_image_identity(AndroidVersion::Android16)
            .unwrap(),
    );
    b.write_profiles(&[old.clone()]).unwrap();
    let registry = fs::read(b.registry()).unwrap();
    let engine = fs::read(b.sdk.join("emulator/source.properties")).unwrap();
    let version = AndroidVersion::Android13;
    let package = cached_image(b, version);
    let control = SdkSetupControl::default();
    let installed = b.run_image_package(&package, &control).unwrap();
    assert!(!installed.reused);
    assert_eq!(installed.identity.version, version);
    assert_eq!(installed.identity.revision, [9, 0, 0]);
    assert_eq!(control.stage(), SdkSetupStage::Complete);
    assert_eq!(
        b.installed_image_identity(version).unwrap(),
        installed.identity
    );
    assert!(
        b.sdk
            .join(version.directory())
            .join(".hub-image.json")
            .is_file()
    );
    assert!(
        fs::read_dir(b.sdk.join(".hub-image-staging"))
            .unwrap()
            .next()
            .is_none()
    );
    fs::remove_file(
        b.runtime()
            .join("sdk-downloads")
            .join(format!("sha256-{}.zip", package.archive.checksum)),
    )
    .unwrap();
    let control = SdkSetupControl::default();
    let reused = b.run_image_package(&package, &control).unwrap();
    assert!(reused.reused);
    assert_eq!(reused.identity, installed.identity);
    assert_eq!(control.downloaded_bytes(), 0);
    assert_eq!(fs::read(b.registry()).unwrap(), registry);
    assert_eq!(
        fs::read(b.sdk.join("emulator/source.properties")).unwrap(),
        engine
    );
    assert_eq!(
        fs::read(b.data(&old).join("userdata.img")).unwrap(),
        b"keep user data"
    );
}

#[test]
fn consent_cancel_lock_contention_and_bad_checksum_never_publish() {
    let fixture = Fixture::new();
    let b = &fixture.0;
    let version = AndroidVersion::Android13;
    let plan = crate::SdkPlan::for_image(version).unwrap();
    assert!(
        b.prepare_android_image(&plan, &BTreeSet::new(), &SdkSetupControl::default())
            .is_err()
    );
    assert!(!b.sdk.join(".hub-image-install.lock").exists());
    let package = cached_image(b, version);
    let canceled = SdkSetupControl::default();
    canceled.cancel();
    assert!(b.run_image_package(&package, &canceled).is_err());
    assert_eq!(canceled.stage(), SdkSetupStage::Canceled);
    assert!(!b.sdk.join(".hub-image-install.lock").exists());
    let guard = image_lock(&b.sdk).unwrap();
    assert!(
        b.run_image_package(&package, &SdkSetupControl::default())
            .unwrap_err()
            .contains("khác đang chạy")
    );
    drop(guard);
    let cache = b
        .runtime()
        .join("sdk-downloads")
        .join(format!("sha256-{}.zip", package.archive.checksum));
    fs::write(&cache, vec![0; package.archive.bytes as usize]).unwrap();
    assert!(
        b.run_image_package(&package, &SdkSetupControl::default())
            .is_err()
    );
    assert!(!b.sdk.join(version.directory()).exists());
    let package = cached_image(b, version);
    assert!(
        !b.run_image_package(&package, &SdkSetupControl::default())
            .unwrap()
            .reused
    );
}

#[test]
fn missing_referenced_image_is_not_replaced_by_catalog_revision() {
    let fixture = Fixture::new();
    let b = &fixture.0;
    let version = AndroidVersion::Android13;
    let mut profile = fixture.device(version, "hub-old-revision", 5664);
    profile.image = Some(ImageIdentity {
        version,
        revision: [8, 0, 0],
        fingerprint: "b".repeat(64),
    });
    b.write_profiles(&[profile]).unwrap();
    let package = cached_image(b, version);
    assert!(
        b.run_image_package(&package, &SdkSetupControl::default())
            .unwrap_err()
            .contains("tham chiếu")
    );
    assert!(!b.sdk.join(version.directory()).exists());
}

#[test]
fn publication_cannot_replace_even_an_empty_external_destination() {
    let fixture = Fixture::new();
    let source = fixture.0.root.join("source");
    let destination = fixture.0.root.join("destination");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("payload"), b"new").unwrap();
    fs::create_dir(&destination).unwrap();
    assert!(publish(&source, &destination).is_err());
    assert!(source.join("payload").exists());
    assert!(fs::read_dir(destination).unwrap().next().is_none());
}

#[test]
fn space_admission_requires_both_filesystems_and_does_not_claim_reservation() {
    let mut space = ImageInstallSpace {
        download_bytes: 100,
        required_data_bytes: 200,
        available_data_bytes: 200,
        required_sdk_bytes: 300,
        available_sdk_bytes: 300,
    };
    assert!(space.ready());
    space.available_data_bytes = 199;
    assert!(!space.ready());
    space.available_data_bytes = 200;
    space.available_sdk_bytes = 299;
    assert!(!space.ready());
}

pub(super) fn crash_checkpoint(backend: &Backend, point: &str) {
    if std::env::var_os("HUB_IMAGE_CRASH_ROOT").as_deref() == Some(backend.root.as_os_str())
        && std::env::var("HUB_IMAGE_CRASH_POINT").ok().as_deref() == Some(point)
    {
        fs::write(backend.root.join("image-crash-checkpoint"), point).unwrap();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
}

#[test]
#[ignore = "owned image installer crash worker; inert without explicit fixture environment"]
fn crash_image_worker() {
    let Some(root) = std::env::var_os("HUB_IMAGE_CRASH_ROOT") else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    let backend = Backend {
        sdk: root.join("sdk"),
        root,
    };
    let package = cached_image(&backend, AndroidVersion::Android13);
    backend
        .run_image_package(&package, &SdkSetupControl::default())
        .unwrap();
    panic!("expected controlled crash checkpoint");
}

#[test]
#[ignore = "SIGKILLs only two owned disposable image installer workers"]
fn image_sigkill_preserves_publication_boundary_and_retry() {
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    for point in ["before-publish", "after-publish"] {
        let fixture = Fixture::new();
        let b = &fixture.0;
        let mut child = Child(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "sdk::image_install::tests::crash_image_worker",
                    "--ignored",
                    "--nocapture",
                ])
                .env("HUB_IMAGE_CRASH_ROOT", &b.root)
                .env("HUB_IMAGE_CRASH_POINT", point)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(15);
        while !b.root.join("image-crash-checkpoint").exists() {
            assert!(
                child.0.try_wait().unwrap().is_none(),
                "worker exited before checkpoint"
            );
            assert!(Instant::now() < deadline, "checkpoint timeout");
            std::thread::sleep(Duration::from_millis(25));
        }
        child.0.kill().unwrap();
        child.0.wait().unwrap();
        let version = AndroidVersion::Android13;
        assert_eq!(
            b.sdk.join(version.directory()).exists(),
            point == "after-publish"
        );
        let package = cached_image(b, version);
        let outcome = b
            .run_image_package(&package, &SdkSetupControl::default())
            .unwrap();
        assert_eq!(outcome.reused, point == "after-publish");
        assert_eq!(
            b.installed_image_identity(version).unwrap(),
            outcome.identity
        );
    }
}
