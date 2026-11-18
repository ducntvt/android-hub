//! Explicit, resumable deletion of one validated archive. Never follows symlinks.
use super::*;
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, PartialEq)]
pub struct PurgePlan {
    pub device: RemovedDevice,
    /// Allocated size, not a promise of reclaimed bytes on copy-on-write filesystems.
    pub allocated_bytes: u64,
    identity: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeletePlan {
    pub profile: Profile,
    pub allocated_bytes: u64,
    data_identity: Value,
    ini_identity: Value,
}

fn allocated_size(path: &Path) -> Result<u64, String> {
    let device = fs::symlink_metadata(path).map_err(err)?.dev();
    let mut pending = vec![path.to_path_buf()];
    let mut seen = HashSet::new();
    let mut bytes = 0u64;
    let started = Instant::now();
    let mut count = 0;
    while let Some(path) = pending.pop() {
        count += 1;
        if count > 100_000 || started.elapsed() > Duration::from_secs(10) {
            return Err("Archive quá lớn để kiểm tra an toàn; chưa xóa dữ liệu".into());
        }
        let meta = fs::symlink_metadata(&path).map_err(err)?;
        if meta.dev() != device || !(meta.is_file() || meta.is_dir() || meta.is_symlink()) {
            return Err("Archive chứa mount hoặc loại file không hỗ trợ; chưa xóa dữ liệu".into());
        }
        if seen.insert((meta.dev(), meta.ino())) {
            bytes = bytes
                .checked_add(meta.blocks().saturating_mul(512))
                .ok_or("Dung lượng archive vượt giới hạn")?;
        }
        if meta.is_dir() {
            for entry in fs::read_dir(path).map_err(err)? {
                if count + pending.len() >= 100_000 {
                    return Err("Archive quá lớn để kiểm tra an toàn; chưa xóa dữ liệu".into());
                }
                pending.push(entry.map_err(err)?.path());
            }
        }
    }
    Ok(bytes)
}

impl Backend {
    fn prepare_delete_offline(&self, profile: &Profile) -> Result<DeletePlan, String> {
        profile.validate()?;
        if self.existing(&profile.id)? != *profile {
            return Err("Máy đã thay đổi; hãy xác nhận xóa lại".into());
        }
        identity(&self.avds(), true)?;
        let data = self.data(profile);
        let ini = self.avds().join(format!("{}.ini", profile.id));
        Ok(DeletePlan {
            profile: profile.clone(),
            data_identity: identity(&data, true)?,
            ini_identity: identity(&ini, false)?,
            allocated_bytes: allocated_size(&data)?.saturating_add(allocated_size(&ini)?),
        })
    }

    fn validate_delete(&self, plan: &DeletePlan) -> Result<(), String> {
        let current = self.prepare_delete_offline(&plan.profile)?;
        if current.data_identity != plan.data_identity || current.ini_identity != plan.ini_identity
        {
            return Err("Dữ liệu máy đã thay đổi; hãy xác nhận xóa lại".into());
        }
        Ok(())
    }

    pub fn prepare_delete(&self, profile: &Profile) -> Result<DeletePlan, String> {
        profile.validate()?;
        let _guard = self.lock()?;
        self.ensure_stopped(profile)?;
        self.prepare_delete_offline(profile)
    }

    pub fn backup_before_delete(&self, plan: &DeletePlan) -> Result<PathBuf, String> {
        plan.profile.validate()?;
        let _guard = self.lock()?;
        self.ensure_stopped(&plan.profile)?;
        self.validate_delete(plan)?;
        self.backup_locked(plan.profile.clone())
    }

    pub fn delete_device(&self, plan: &DeletePlan) -> Result<Snapshot, String> {
        plan.profile.validate()?;
        let _guard = self.lock()?;
        self.ensure_stopped(&plan.profile)?;
        self.delete_offline(plan)?;
        self.snapshot()
            .map_err(|e| format!("Đã xóa máy nhưng chưa refresh được: {e}"))
    }

    fn delete_offline(&self, plan: &DeletePlan) -> Result<(), String> {
        self.validate_delete(plan)?;
        // Reuse the journalled registry/data transition internally. The user
        // confirms deletion once; there is no separate archive step in the UI.
        let archive = match self.commit_removal(&plan.profile, |_| Ok(())) {
            Ok(archive) => archive,
            Err(error) => {
                let pending = self.removal_journal(&plan.profile.id);
                if present(&pending)?
                    && let Err(recovery) = self.recover_removal_offline(&pending)
                {
                    return Err(format!("{error}; recovery: {recovery}"));
                }
                return Err(error);
            }
        };
        let info = RemovedDevice {
            profile: plan.profile.clone(),
            archive,
            unavailable: None,
        };
        let purge = self.prepare_purge_offline(&info)?;
        self.purge_offline(&purge, || Ok(()))
            .map_err(|e| format!("{e}. Mở Máy đã lưu trữ để tiếp tục xóa."))
    }

    fn purge_source(&self, info: &RemovedDevice) -> Result<Journal, String> {
        // Check the parent before reading any supplied archive path.
        if info.archive.parent() != Some(self.avds().as_path()) {
            return Err("Archive nằm ngoài thư mục thiết bị của Hub".into());
        }
        identity(&self.avds(), true)?;
        identity(&info.archive, true)?;
        let resuming = present(&info.archive.join("purging.json"))?;
        let journal = self.read_removal(&info.archive.join(if resuming {
            "purging.json"
        } else {
            "removed.json"
        }))?;
        if journal.profile != info.profile
            || journal.archive != info.archive
            || journal.value["operation"] == "undo"
            || journal.value["state"] != "committed"
            || present(&info.archive.join("restored.json"))?
            || present(&self.removal_journal(&info.profile.id))?
            || self
                .read_profiles()?
                .iter()
                .any(|p| p.id == info.profile.id)
            || present(&self.data(&info.profile))?
            || present(&self.avds().join(format!("{}.ini", info.profile.id)))?
        {
            return Err("Máy đã thay đổi hoặc đã được đưa trở lại; không xóa".into());
        }
        for (name, directory, key) in [
            ("data.avd", true, "data_identity"),
            ("registration.ini", false, "ini_identity"),
        ] {
            let path = info.archive.join(name);
            if present(&path)? {
                if identity(&path, directory)? != journal.value[key] {
                    return Err("Dữ liệu archive đã thay đổi; không xóa".into());
                }
            } else if !resuming {
                return Err("Archive thiếu dữ liệu; không xóa".into());
            }
        }
        Ok(journal)
    }

    pub fn prepare_purge(&self, info: &RemovedDevice) -> Result<PurgePlan, String> {
        info.profile.validate()?;
        let _guard = self.lock()?;
        self.ensure_stopped(&info.profile)?;
        self.prepare_purge_offline(info)
    }

    fn prepare_purge_offline(&self, info: &RemovedDevice) -> Result<PurgePlan, String> {
        let journal = self.purge_source(info)?;
        Ok(PurgePlan {
            device: info.clone(),
            allocated_bytes: allocated_size(&info.archive)?,
            identity: journal.value,
        })
    }

    pub fn purge_device(&self, plan: &PurgePlan) -> Result<Snapshot, String> {
        plan.device.profile.validate()?;
        let _guard = self.lock()?;
        self.ensure_stopped(&plan.device.profile)?;
        self.purge_offline(plan, || Ok(()))?;
        self.snapshot()
            .map_err(|e| format!("Đã xóa máy nhưng chưa refresh được: {e}"))
    }

    fn purge_offline(
        &self,
        plan: &PurgePlan,
        checkpoint: impl FnOnce() -> Result<(), String>,
    ) -> Result<(), String> {
        let source = self.purge_source(&plan.device)?;
        if source.value != plan.identity {
            return Err("Archive đã thay đổi; hãy xác nhận xóa lại".into());
        }
        let archive = &plan.device.archive;
        allocated_size(archive)?;
        let marker = archive.join("purging.json");
        if !present(&marker)? {
            // A crash while writing the marker may leave its staging file.
            // The archive has been validated and deletion explicitly confirmed;
            // unlink only this file (never follow a possible symlink).
            let staging = marker.with_extension("json.new");
            if present(&staging)? {
                fs::remove_file(staging).map_err(err)?;
            }
            write_json(&marker, &source.value)?;
            sync_dir(archive)?;
        }
        // Keep the receipt until all payloads are gone, so failures remain visible
        // in the archive list and can be retried after another explicit confirmation.
        for entry in fs::read_dir(archive).map_err(err)? {
            let entry = entry.map_err(err)?;
            if matches!(
                entry.file_name().to_str(),
                Some("removed.json" | "purging.json")
            ) {
                continue;
            }
            let path = entry.path();
            if fs::symlink_metadata(&path).map_err(err)?.is_dir() {
                fs::remove_dir_all(path).map_err(err)?;
            } else {
                fs::remove_file(path).map_err(err)?;
            }
        }
        sync_dir(archive)?;
        checkpoint()?;
        let receipt = archive.join("removed.json");
        if present(&receipt)? {
            fs::remove_file(receipt).map_err(err)?;
        }
        fs::remove_file(marker).map_err(err)?;
        fs::remove_dir(archive).map_err(err)?;
        sync_dir(&self.avds())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn direct_delete_preparation_is_read_only_and_confirmation_removes_registry_and_data() {
        let (b, profile) = super::super::tests::fixture();
        let other = Profile::standard("hub-keep".into(), "Keep".into(), 5678);
        b.write_profiles(&[profile.clone(), other.clone()]).unwrap();
        fs::create_dir(b.data(&other)).unwrap();
        fs::write(b.data(&other).join("userdata.img"), b"keep").unwrap();
        let plan = b.prepare_delete_offline(&profile).unwrap();
        assert!(plan.allocated_bytes > 0);
        assert!(b.data(&profile).join("userdata.img").is_file());
        assert_eq!(b.read_profiles().unwrap().len(), 2);
        b.delete_offline(&plan).unwrap();
        assert_eq!(b.read_profiles().unwrap(), vec![other.clone()]);
        assert!(!b.data(&profile).exists());
        assert!(b.removed_devices().unwrap().entries.is_empty());
        assert_eq!(
            fs::read(b.data(&other).join("userdata.img")).unwrap(),
            b"keep"
        );
        assert!(b.delete_offline(&plan).is_err());
        fs::remove_dir_all(b.root).unwrap();
    }

    #[test]
    fn direct_delete_rejects_changed_profile_and_replaced_data() {
        let (b, profile) = super::super::tests::fixture();
        let plan = b.prepare_delete_offline(&profile).unwrap();
        let mut changed = profile.clone();
        changed.name = "Changed".into();
        b.write_profiles(&[changed]).unwrap();
        assert!(b.delete_offline(&plan).is_err());
        b.write_profiles(std::slice::from_ref(&profile)).unwrap();
        let saved = b.root.join("saved.avd");
        fs::rename(b.data(&profile), &saved).unwrap();
        fs::create_dir(b.data(&profile)).unwrap();
        fs::write(b.data(&profile).join("foreign"), b"keep").unwrap();
        assert!(b.delete_offline(&plan).is_err());
        assert!(saved.join("userdata.img").is_file());
        assert_eq!(fs::read(b.data(&profile).join("foreign")).unwrap(), b"keep");
        fs::remove_dir_all(b.root).unwrap();
    }

    fn archived() -> (Backend, RemovedDevice) {
        let (backend, profile) = super::super::tests::fixture();
        backend.commit_removal(&profile, |_| Ok(())).unwrap();
        let info = backend.removed_devices().unwrap().entries.remove(0);
        (backend, info)
    }

    #[test]
    fn purge_only_selected_archive_and_never_follows_nested_symlinks() {
        let (b, info) = archived();
        let keep = [
            b.sdk.join("system.img"),
            b.runtime().join("backups/save.img"),
            b.avds().join("other.avd/userdata.img"),
        ];
        for path in &keep {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"keep these bytes").unwrap();
        }
        symlink(&b.sdk, info.archive.join("data.avd/shared-image")).unwrap();
        let plan = b.prepare_purge_offline(&info).unwrap();
        assert!(plan.allocated_bytes > 0);
        assert!(info.archive.is_dir()); // Preparing is read-only.
        b.purge_offline(&plan, || Ok(())).unwrap();
        assert!(!info.archive.exists());
        assert!(b.removed_devices().unwrap().entries.is_empty());
        for path in &keep {
            assert_eq!(fs::read(path).unwrap(), b"keep these bytes");
        }
        assert!(b.purge_offline(&plan, || Ok(())).is_err());
        fs::remove_dir_all(b.root).unwrap();
    }

    #[test]
    fn interrupted_purge_remains_listed_cannot_restore_and_can_retry() {
        let (b, info) = archived();
        fs::write(info.archive.join("purging.json.new"), b"interrupted write").unwrap();
        let plan = b.prepare_purge_offline(&info).unwrap();
        assert!(
            b.purge_offline(&plan, || Err("interrupted".into()))
                .is_err()
        );
        let catalog = b.removed_devices().unwrap();
        assert_eq!(catalog.entries.len(), 1);
        assert!(catalog.entries[0].unavailable.is_some());
        assert!(
            b.commit_undo_removal(&info.archive, &info.profile, |_| Ok(()))
                .is_err()
        );
        // Also recover the final phase after the original receipt was removed.
        fs::remove_file(info.archive.join("removed.json")).unwrap();
        let info = b.removed_devices().unwrap().entries.remove(0);
        let retry = b.prepare_purge_offline(&info).unwrap();
        b.purge_offline(&retry, || Ok(())).unwrap();
        assert!(!info.archive.exists());
        fs::remove_dir_all(b.root).unwrap();
    }

    #[test]
    fn restored_device_invalidates_previous_delete_confirmation() {
        let (b, info) = archived();
        let plan = b.prepare_purge_offline(&info).unwrap();
        b.commit_undo_removal(&info.archive, &info.profile, |_| Ok(()))
            .unwrap();
        assert!(b.purge_offline(&plan, || Ok(())).is_err());
        assert_eq!(
            fs::read(b.data(&info.profile).join("userdata.img")).unwrap(),
            b"original guest bytes"
        );
        fs::remove_dir_all(b.root).unwrap();
    }

    #[test]
    fn changed_payload_or_archive_link_is_rejected_without_deleting() {
        let (b, info) = archived();
        let plan = b.prepare_purge_offline(&info).unwrap();
        fs::rename(
            info.archive.join("data.avd"),
            info.archive.join("saved.avd"),
        )
        .unwrap();
        fs::create_dir(info.archive.join("data.avd")).unwrap();
        fs::write(info.archive.join("data.avd/foreign"), b"keep").unwrap();
        assert!(b.purge_offline(&plan, || Ok(())).is_err());
        assert!(info.archive.join("saved.avd/userdata.img").is_file());
        let original = info.archive.with_extension("saved");
        fs::rename(&info.archive, &original).unwrap();
        symlink(&original, &info.archive).unwrap();
        assert!(b.prepare_purge_offline(&info).is_err());
        assert_eq!(
            fs::read(original.join("data.avd/foreign")).unwrap(),
            b"keep"
        );
        fs::remove_dir_all(b.root).unwrap();
    }

    #[test]
    fn foreign_path_active_id_pending_transaction_and_modified_receipt_are_rejected() {
        let (b, info) = archived();
        let plan = b.prepare_purge_offline(&info).unwrap();
        let mut outside = info.clone();
        outside.archive = b.root.clone();
        assert!(b.prepare_purge_offline(&outside).is_err());
        b.write_profiles(std::slice::from_ref(&info.profile))
            .unwrap();
        assert!(b.purge_offline(&plan, || Ok(())).is_err());
        b.write_profiles(&[]).unwrap();
        fs::write(b.removal_journal(&info.profile.id), b"pending").unwrap();
        assert!(b.purge_offline(&plan, || Ok(())).is_err());
        fs::remove_file(b.removal_journal(&info.profile.id)).unwrap();
        let receipt = info.archive.join("removed.json");
        let mut value: Value = serde_json::from_slice(&fs::read(&receipt).unwrap()).unwrap();
        value["extra"] = json!("changed since confirmation");
        write_json(&receipt, &value).unwrap();
        assert!(b.purge_offline(&plan, || Ok(())).is_err());
        assert!(info.archive.join("data.avd/userdata.img").is_file());
        fs::remove_dir_all(b.root).unwrap();
    }

    #[test]
    #[ignore = "requires host process inspection and loopback ports"]
    fn public_purge_respects_busy_port_and_lock() {
        let (b, info) = archived();
        let plan = b.prepare_purge(&info).unwrap();
        let port = std::net::TcpListener::bind(("127.0.0.1", info.profile.port)).unwrap();
        assert!(b.purge_device(&plan).is_err());
        assert!(info.archive.join("data.avd/userdata.img").is_file());
        drop(port);
        let lock = b.lock().unwrap();
        assert!(b.purge_device(&plan).is_err());
        drop(lock);
        b.purge_device(&plan).unwrap();
        assert!(!info.archive.exists());
        fs::remove_dir_all(b.root).unwrap();
    }
}
