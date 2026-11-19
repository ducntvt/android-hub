use super::*;
use crate::images::tests::Fixture;
use serde_json::json;

fn managed() -> Fixture {
    let mut f = Fixture::new();
    let sdk = f.0.root.join("managed-sdks/sdk-1-2-3");
    fs::create_dir_all(sdk.parent().unwrap()).unwrap();
    fs::rename(&f.0.sdk, &sdk).unwrap();
    f.0.sdk = sdk;
    fs::write(f.0.sdk.join(".hub-sdk.json"), json!({"schema":1,"state":"ready","id":"1-2-3","packages":[{"destination":"cmdline-tools/latest"},{"destination":"platform-tools"},{"destination":"emulator"}]}).to_string()).unwrap();
    f
}
fn own(f: &Fixture, version: AndroidVersion) {
    f.image(version);
    let identity = f.0.installed_image_identity(version).unwrap();
    fs::write(f.0.sdk.join(version.directory()).join(".hub-image.json"), json!({"schema":1,"state":"complete","sdk":f.0.sdk.canonicalize().unwrap(),"identity":identity}).to_string()).unwrap();
}
#[test]
fn unused_owned_image_removal_keeps_other_images_and_download_cache() {
    let f = managed();
    let b = &f.0;
    let v = AndroidVersion::Android13;
    own(&f, v);
    own(&f, AndroidVersion::Android16);
    fs::create_dir_all(b.root.join("sdk-downloads")).unwrap();
    fs::write(b.root.join("sdk-downloads/keep.zip"), "cache").unwrap();
    let p = b.prepare_image_removal(v).unwrap();
    assert!(p.allocated_bytes > 0);
    b.remove_android_image(&p).unwrap();
    assert!(!b.sdk.join(v.directory()).exists());
    assert!(b.sdk.join(AndroidVersion::Android16.directory()).is_dir());
    assert!(b.root.join("sdk-downloads/keep.zip").is_file());
    assert!(b.ensure_no_image_removal(v).is_ok());
}
#[test]
fn external_unowned_changed_and_escaped_images_cannot_be_removed() {
    let external = Fixture::new();
    let v = AndroidVersion::Android13;
    own(&external, v);
    assert!(
        external
            .0
            .prepare_image_removal(v)
            .unwrap_err()
            .contains("bên ngoài")
    );
    let f = managed();
    let b = &f.0;
    f.image(v);
    assert!(
        b.prepare_image_removal(v)
            .unwrap_err()
            .contains("biên nhận")
    );
    own(&f, v);
    let p = b.prepare_image_removal(v).unwrap();
    fs::write(b.sdk.join(v.directory()).join("system.img"), "changed").unwrap();
    assert!(b.remove_android_image(&p).is_err());
    own(&f, v);
    let source = b.sdk.join(v.directory());
    let other = b.root.join("outside");
    fs::rename(&source, &other).unwrap();
    std::os::unix::fs::symlink(&other, &source).unwrap();
    assert!(b.prepare_image_removal(v).is_err());
    assert!(other.join("system.img").exists());
}
#[test]
fn references_and_unknown_metadata_block_stale_preview() {
    let f = managed();
    let b = &f.0;
    let v = AndroidVersion::Android13;
    own(&f, v);
    let plan = b.prepare_image_removal(v).unwrap();
    let mut p = f.device(v, "hub-reference", 5664);
    p.image = Some(plan.identity.clone());
    b.write_profiles(&[p.clone()]).unwrap();
    assert!(
        b.remove_android_image(&plan)
            .unwrap_err()
            .contains("tham chiếu")
    );
    assert!(
        b.image_usage(v)
            .references
            .iter()
            .any(|r| r.contains("Device:"))
    );
    b.write_profiles(&[]).unwrap();
    assert!(b.remove_android_image(&plan).is_err()); // retained AVD still protects it
    fs::remove_dir_all(b.data(&p)).unwrap();
    let backup = b.root.join("backups/example");
    fs::create_dir_all(&backup).unwrap();
    fs::write(
        backup.join("manifest.json"),
        json!({"profile":p}).to_string(),
    )
    .unwrap();
    assert!(b.remove_android_image(&plan).is_err());
    fs::write(backup.join("manifest.json"), "corrupt").unwrap();
    assert!(b.remove_android_image(&plan).is_err());
    fs::remove_dir_all(backup).unwrap();
    let archive = b.avds().join(".removed-test/data.avd");
    fs::create_dir_all(&archive).unwrap();
    fs::write(
        archive.join("config.ini"),
        format!(
            "target=android-{}\nimage.sysdir.1={}\n",
            v.api(),
            v.directory()
        ),
    )
    .unwrap();
    assert!(b.remove_android_image(&plan).is_err());
    fs::remove_dir_all(archive.parent().unwrap()).unwrap();
    let mut legacy = p;
    legacy.image = None;
    b.write_profiles(&[legacy]).unwrap();
    assert!(b.remove_android_image(&plan).is_err()); // missing unresolved AVD
}
#[test]
fn interrupted_removal_is_not_ready_and_explicit_retry_reclaims_partial_payload() {
    for checkpoint in 0..3 {
        let f = managed();
        let b = &f.0;
        let v = AndroidVersion::Android14;
        own(&f, v);
        let plan = b.prepare_image_removal(v).unwrap();
        write_json(
            &b.image_removal_journal(v),
            &json!({"schema":1,"plan":plan}),
        )
        .unwrap();
        if checkpoint > 0 {
            fs::rename(b.sdk.join(v.directory()), b.removal_payload(v)).unwrap();
        }
        if checkpoint == 2 {
            fs::remove_file(b.removal_payload(v).join("system.img")).unwrap();
        }
        assert!(b.installed_image_revision(v).is_err());
        assert!(b.available_image_revision(v).is_err());
        assert!(b.ensure_no_image_removal(v).is_err());
        assert!(b.image_usage(v).removal_pending);
        let retry = b.prepare_image_removal(v).unwrap();
        b.remove_android_image(&retry).unwrap();
        assert!(b.ensure_no_image_removal(v).is_ok());
        assert!(!b.sdk.join(v.directory()).exists());
    }
}
#[test]
fn sdk_and_hub_locks_exclude_competing_install_or_create() {
    let f = managed();
    let b = &f.0;
    let v = AndroidVersion::Android15;
    own(&f, v);
    let plan = b.prepare_image_removal(v).unwrap();
    let sdk = image_lock(&b.sdk).unwrap();
    assert!(b.remove_android_image(&plan).is_err());
    drop(sdk);
    let hub = b.lock().unwrap();
    assert!(b.remove_android_image(&plan).is_err());
    drop(hub);
    b.remove_android_image(&plan).unwrap();
}

#[test]
fn pending_transactions_and_archive_receipts_keep_references_without_live_avds() {
    let f = managed();
    let b = &f.0;
    let v = AndroidVersion::Android13;
    own(&f, v);
    let mut profile =
        android_hub_domain::Profile::standard("hub-pending".into(), "Pending machine".into(), 5664);
    profile.image = Some(b.installed_image_identity(v).unwrap());
    let journal = b.root.join("create-pending-hub-pending.json");
    fs::write(
        &journal,
        json!({"profile":profile,"state":"preparing"}).to_string(),
    )
    .unwrap();
    let refs = b.image_references(v).unwrap();
    assert!(refs.iter().any(|r| r.contains("create-pending")));
    fs::write(&journal, "{}").unwrap();
    assert!(b.image_references(v).is_err());
    fs::remove_file(journal).unwrap();
    let archive = b.avds().join(".removed-hub-pending");
    fs::create_dir_all(&archive).unwrap();
    fs::write(
        archive.join("removed.json"),
        json!({"profile":profile}).to_string(),
    )
    .unwrap();
    assert_eq!(
        b.image_references(v).unwrap(),
        vec!["Archive: Pending machine"]
    );
    fs::write(archive.join("removed.json"), "invalid").unwrap();
    assert!(b.image_references(v).is_err());
}

#[test]
fn unpublished_removal_stage_is_discarded_without_touching_image() {
    let f = managed();
    let b = &f.0;
    let v = AndroidVersion::Android13;
    own(&f, v);
    let stage = b.image_removal_journal(v).with_extension("json.new");
    fs::write(&stage, b"partial json").unwrap();
    let plan = b.prepare_image_removal(v).unwrap();
    assert!(!stage.exists());
    assert!(b.sdk.join(v.directory()).join("system.img").is_file());
    b.remove_android_image(&plan).unwrap();
}
