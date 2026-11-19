//! Versioned registry with metadata-only legacy image migration under the Hub lock.
use crate::{Backend, err, restore::sync_dir, validate_registry};
use android_hub_domain::Profile;
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, io::Write, path::Path};

// Callers hold the Hub lock. A leftover staging file cannot replace published data.
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temp = path.with_extension("json.new");
    match fs::symlink_metadata(&temp) {
        Ok(meta) if meta.is_file() => fs::remove_file(&temp).map_err(err)?,
        Ok(_) => return Err("Registry staging không phải file thường".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
        Err(e) => return Err(err(e)),
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(err)?;
    file.write_all(bytes).map_err(err)?;
    file.sync_all().map_err(err)?;
    fs::rename(temp, path).map_err(err)?;
    sync_dir(path.parent().ok_or("Registry thiếu thư mục cha")?)
}

fn decode(bytes: &[u8]) -> Result<(bool, Vec<Profile>), String> {
    let value: Value = serde_json::from_slice(bytes).map_err(err)?;
    let legacy = value.is_array();
    let profiles = if legacy {
        value
    } else if value["schema"] == 1 && value.as_object().is_some_and(|v| v.len() == 2) {
        value["profiles"].clone()
    } else {
        return Err("Schema registry chưa được hỗ trợ; không ghi đè dữ liệu".into());
    };
    let profiles: Vec<Profile> = serde_json::from_value(profiles).map_err(err)?;
    validate_registry(&profiles)?;
    Ok((legacy, profiles))
}

impl Backend {
    pub(crate) fn read_profiles(&self) -> Result<Vec<Profile>, String> {
        let meta = fs::symlink_metadata(self.registry()).map_err(err)?;
        if !meta.is_file() || meta.len() > 4 * 1024 * 1024 {
            return Err("Registry không phải file thường hoặc quá lớn".into());
        }
        decode(&fs::read(self.registry()).map_err(err)?).map(|(_, profiles)| profiles)
    }

    pub(crate) fn write_profiles(&self, profiles: &[Profile]) -> Result<(), String> {
        validate_registry(profiles)?;
        if self.registry().exists() {
            let original = fs::read(self.registry()).map_err(err)?;
            if decode(&original)?.0 {
                self.preserve_legacy_registry(&original)?;
            }
        }
        atomic_write(
            &self.registry(),
            &serde_json::to_vec_pretty(&json!({"schema":1,"profiles":profiles})).map_err(err)?,
        )
    }

    fn preserve_legacy_registry(&self, original: &[u8]) -> Result<(), String> {
        let backup = self.runtime().join("hub-profiles.pre-image-schema.json");
        match fs::symlink_metadata(&backup) {
            Ok(meta) if meta.is_file() && fs::read(&backup).map_err(err)? == original => Ok(()),
            Ok(_) => Err(
                "Bản sao registry trước migration không khớp; giữ nguyên dữ liệu để kiểm tra"
                    .into(),
            ),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => atomic_write(&backup, original),
            Err(e) => Err(err(e)),
        }
    }

    pub(crate) fn migrate_profile_images(&self) -> Result<(), String> {
        let original = fs::read(self.registry()).map_err(err)?;
        let (legacy, mut profiles) = decode(&original)?;
        // Preserve the byte-exact original before any metadata is published.
        if legacy {
            self.preserve_legacy_registry(&original)?;
        }
        let mut changed = legacy;
        let mut verified = BTreeMap::new();
        for profile in &mut profiles {
            if profile.image.is_some() {
                continue;
            }
            // Failure is local to this device. Snapshot/start expose unresolved identity.
            let Ok(version) = self.image_version_at(&self.data(profile)) else {
                continue;
            };
            let identity = verified
                .entry(version)
                .or_insert_with(|| self.installed_image_identity(version));
            if let Ok(identity) = identity {
                profile.image = Some(identity.clone());
                changed = true;
            }
        }
        if changed {
            self.write_profiles(&profiles)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::images::tests::Fixture;
    use android_hub_domain::{AndroidVersion, ImageIdentity};

    #[test]
    fn device_namespaces_do_not_change_legacy_or_schema_one_profile_storage() {
        let raw = serde_json::json!({"id":"hub-existing", "name":"Existing", "port":5584,
            "ram_mb":3072, "cpus":2, "width":720, "height":1280, "fps":30});
        let legacy = serde_json::to_vec(&serde_json::json!([raw.clone()])).unwrap();
        let schema = serde_json::to_vec(&serde_json::json!({"schema":1,"profiles":[raw]})).unwrap();
        let (old, a) = decode(&legacy).unwrap();
        let (new, b) = decode(&schema).unwrap();
        assert!(old && !new);
        assert_eq!(a, b);
        assert_eq!(a[0].id, "hub-existing");
        let encoded = serde_json::to_value(&a[0]).unwrap();
        assert_eq!(encoded.as_object().unwrap().len(), 9);
        assert!(encoded.get("device_kind").is_none());
        assert!(encoded.get("serial").is_none());
        assert!(encoded.get("transport").is_none());
        assert_eq!(encoded["id"], "hub-existing");
    }

    #[test]
    fn migration_is_local_idempotent_and_keeps_original_registry_and_guest_bytes() {
        let fixture = Fixture::new();
        let b = &fixture.0;
        let version = AndroidVersion::Android12;
        fixture.image(version);
        let good = fixture.device(version, "hub-good", 5664);
        let bad = fixture.device(AndroidVersion::Android13, "hub-missing", 5666);
        let conflicting = fixture.device(version, "hub-conflict", 5668);
        fs::write(b.data(&conflicting).join("config.ini"), "target=android-36\nimage.sysdir.1=system-images/android-31/google_apis_playstore/arm64-v8a/\n").unwrap();
        let all = [good.clone(), bad, conflicting];
        let original = serde_json::to_vec(&all).unwrap();
        fs::write(b.registry(), &original).unwrap();
        let config = fs::read(b.data(&good).join("config.ini")).unwrap();
        // Both an interrupted backup write and an interrupted registry write are retryable.
        fs::write(
            b.runtime().join("hub-profiles.pre-image-schema.json.new"),
            b"partial",
        )
        .unwrap();
        fs::write(b.runtime().join("hub-profiles.json.new"), b"partial").unwrap();
        b.migrate_profile_images().unwrap();
        let migrated = b.read_profiles().unwrap();
        assert_eq!(migrated[0].image.as_ref().unwrap().version, version);
        assert!(migrated[1].image.is_none() && migrated[2].image.is_none());
        assert_eq!(
            fs::read(b.runtime().join("hub-profiles.pre-image-schema.json")).unwrap(),
            original
        );
        assert_eq!(fs::read(b.data(&good).join("config.ini")).unwrap(), config);
        assert_eq!(
            fs::read(b.data(&good).join("userdata.img")).unwrap(),
            b"keep user data"
        );
        let once = fs::read(b.registry()).unwrap();
        b.migrate_profile_images().unwrap();
        assert_eq!(fs::read(b.registry()).unwrap(), once);
        fixture.image(AndroidVersion::Android13);
        b.migrate_profile_images().unwrap();
        assert_eq!(
            b.read_profiles().unwrap()[1]
                .image
                .as_ref()
                .unwrap()
                .version,
            AndroidVersion::Android13
        );
        assert_eq!(b.read_profiles().unwrap()[0], migrated[0]);
    }

    #[test]
    fn backend_refuses_binding_mutation_and_unknown_registry_schema_without_writing() {
        let fixture = Fixture::new();
        let b = &fixture.0;
        let mut profile = Profile::standard("hub-binding".into(), "Game".into(), 5664);
        profile.image = Some(ImageIdentity {
            version: AndroidVersion::Android12,
            revision: [9, 0, 0],
            fingerprint: "a".repeat(64),
        });
        b.write_profiles(&[profile.clone()]).unwrap();
        let original = fs::read(b.registry()).unwrap();
        for image in [
            None,
            Some(ImageIdentity {
                version: AndroidVersion::Android16,
                revision: [7, 0, 0],
                fingerprint: "b".repeat(64),
            }),
        ] {
            let mut changed = profile.clone();
            changed.image = image;
            assert!(
                b.save(changed)
                    .unwrap_err()
                    .contains("Không được đổi image")
            );
            assert_eq!(fs::read(b.registry()).unwrap(), original);
        }
        fs::write(b.registry(), br#"{"schema":99,"profiles":[]}"#).unwrap();
        assert!(b.read_profiles().is_err());
        assert!(b.write_profiles(&[]).is_err());
        assert_eq!(
            fs::read(b.registry()).unwrap(),
            br#"{"schema":99,"profiles":[]}"#
        );
    }
}
