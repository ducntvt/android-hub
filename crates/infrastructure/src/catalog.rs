//! Lightweight backup/history discovery for UI; full verification happens on restore.
use crate::{Backend, err};
use android_hub_domain::Profile;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackupInfo {
    pub path: PathBuf,
    pub profile_id: String,
    pub profile_name: String,
    pub created_seconds: u64,
    pub bytes: u64,
    pub manifest_sha256: String,
}
#[derive(Clone, Debug, Default)]
pub struct BackupCatalog {
    pub entries: Vec<BackupInfo>,
    pub skipped: usize,
}
#[derive(Clone, Debug)]
pub struct RestoreReceipt {
    pub path: PathBuf,
    pub profile_name: String,
    pub rolled_back: bool,
    pub token: u128,
}
fn manifest(path: &Path) -> Result<Value, String> {
    let meta = fs::symlink_metadata(path).map_err(err)?;
    if !meta.is_file() || meta.len() > 2 * 1024 * 1024 {
        return Err("Manifest không phải file thường hoặc quá lớn".into());
    }
    serde_json::from_slice(&fs::read(path).map_err(err)?).map_err(err)
}
impl Backend {
    pub fn backup_manifest_digest(&self, path: &Path) -> Result<String, String> {
        manifest(&path.join("manifest.json"))?;
        self.file_digest(&path.join("manifest.json"))
    }
    pub fn list_backups(&self, id: &str) -> Result<BackupCatalog, String> {
        let profile = self.existing(id)?;
        let base = self.runtime().join("backups");
        let mut result = BackupCatalog::default();
        let metadata = match fs::symlink_metadata(&base) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(result),
            Err(error) => return Err(err(error)),
        };
        if metadata.file_type().is_symlink() {
            return Err("Không đọc backup qua symlink".into());
        }
        for entry in fs::read_dir(base).map_err(err)? {
            let entry = entry.map_err(err)?;
            let path = entry.path();
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if !name.starts_with(&format!("{id}-")) {
                continue;
            }
            let read = (|| -> Result<BackupInfo, String> {
                if !entry.file_type().map_err(err)?.is_dir() {
                    return Err("Không phải thư mục backup".into());
                }
                let digest = self.backup_manifest_digest(&path)?;
                let value = manifest(&path.join("manifest.json"))?;
                let saved: Profile =
                    serde_json::from_value(value["profile"].clone()).map_err(err)?;
                saved.validate()?;
                if value["schema"] != 2
                    || value["complete"] != true
                    || saved.id != profile.id
                    || saved.port != profile.port
                {
                    return Err("Backup cũ/không phù hợp".into());
                }
                let nanos = value["created_unix_nanos"]
                    .as_str()
                    .and_then(|s| s.parse::<u128>().ok())
                    .ok_or("Thiếu thời gian backup")?;
                let created_seconds = u64::try_from(nanos / 1_000_000_000).map_err(err)?;
                let mut bytes = 0u64;
                for file in value["files"]
                    .as_object()
                    .ok_or("Thiếu danh sách file")?
                    .values()
                {
                    bytes = bytes
                        .checked_add(
                            file["bytes"]
                                .as_u64()
                                .ok_or("Kích thước file không hợp lệ")?,
                        )
                        .ok_or("Backup quá lớn")?;
                }
                if self.backup_manifest_digest(&path)? != digest {
                    return Err("Backup thay đổi trong khi đọc danh sách".into());
                }
                Ok(BackupInfo {
                    profile_id: id.into(),
                    profile_name: saved.name,
                    path: path.clone(),
                    created_seconds,
                    bytes,
                    manifest_sha256: digest,
                })
            })();
            match read {
                Ok(info) => result.entries.push(info),
                Err(_) => result.skipped += 1,
            }
        }
        result.entries.sort_by(|a, b| {
            b.created_seconds
                .cmp(&a.created_seconds)
                .then_with(|| b.path.cmp(&a.path))
        });
        Ok(result)
    }
    pub fn restore_receipts(&self) -> Result<Vec<RestoreReceipt>, String> {
        let base = self.runtime().join("restore-history");
        let mut result = vec![];
        if !base.exists() {
            return Ok(result);
        }
        if fs::symlink_metadata(&base)
            .map_err(err)?
            .file_type()
            .is_symlink()
        {
            return Err("Không đọc lịch sử qua symlink".into());
        }
        for entry in fs::read_dir(base).map_err(err)? {
            let path = entry.map_err(err)?.path();
            let value = manifest(&path)?;
            let profile: Profile = serde_json::from_value(value["before"].clone()).map_err(err)?;
            profile.validate()?;
            let token = value["token"]
                .as_str()
                .and_then(|s| s.parse::<u128>().ok())
                .ok_or("Lịch sử restore không hợp lệ")?;
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            let committed = format!("{}-{token}-committed.json", profile.id);
            let rollback = format!("{}-{token}-rolled-back.json", profile.id);
            if value["schema"] != 1 || (name != committed && name != rollback) {
                continue;
            }
            result.push(RestoreReceipt {
                profile_name: profile.name,
                rolled_back: name == rollback,
                token,
                path,
            });
        }
        result.sort_by_key(|r| std::cmp::Reverse(r.token));
        result.truncate(3);
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_reports_metadata_without_claiming_payload_verification() {
        let root = std::env::temp_dir().join(format!(
            "hub-catalog-{}-{}",
            std::process::id(),
            crate::SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let backend = Backend::new(root.clone()).unwrap();
        fs::create_dir_all(backend.runtime()).unwrap();
        let profile = Profile::standard("hub-catalog".into(), "Original name".into(), 5680);
        backend
            .write_profiles(std::slice::from_ref(&profile))
            .unwrap();
        assert!(
            backend
                .list_backups(&profile.id)
                .unwrap()
                .entries
                .is_empty()
        );
        let base = backend.runtime().join("backups");
        let good = base.join("hub-catalog-1");
        fs::create_dir_all(&good).unwrap();
        let value = serde_json::json!({"schema":2,"complete":true,"profile":profile,
            "created_unix_nanos":"1794900000000000000", "files":{"userdata.img":{"bytes":1234}}});
        fs::write(
            good.join("manifest.json"),
            serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
        // No payload exists: discovery must not advertise verified/compatible data.
        fs::create_dir_all(base.join("hub-catalog-missing")).unwrap();
        let invalid = base.join("hub-catalog-invalid");
        fs::create_dir_all(&invalid).unwrap();
        fs::write(invalid.join("manifest.json"), b"invalid").unwrap();
        let catalog = backend.list_backups(&profile.id).unwrap();
        assert_eq!(catalog.skipped, 2);
        assert_eq!(catalog.entries.len(), 1);
        assert_eq!(catalog.entries[0].profile_name, "Original name");
        assert_eq!(catalog.entries[0].created_seconds, 1794900000);
        assert_eq!(catalog.entries[0].bytes, 1234);
        assert!(backend.verify_backup(&good).is_err());
        fs::remove_dir_all(&base).unwrap();
        fs::write(&base, b"not a directory").unwrap();
        assert!(backend.list_backups(&profile.id).is_err());
        fs::remove_file(&base).unwrap();
        std::os::unix::fs::symlink(root.join("missing"), &base).unwrap();
        assert!(backend.list_backups(&profile.id).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
