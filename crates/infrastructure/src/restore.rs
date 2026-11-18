//! Offline restore transaction and conservative crash recovery.
use crate::{Backend, Snapshot, err};
use android_hub_domain::Profile;
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug)]
pub struct RestoreOutcome {
    pub snapshot: Snapshot,
    pub previous_data: PathBuf,
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum Step {
    Journal,
    OldMoved,
    NewMoved,
    Registry,
    Committed,
}
pub(super) fn sync_dir(path: &Path) -> Result<(), String> {
    fs::File::open(path).map_err(err)?.sync_all().map_err(err)
}
fn directory_identity(path: &Path) -> Result<Value, String> {
    let meta = fs::symlink_metadata(path).map_err(err)?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err("Restore từ chối đường dẫn không phải thư mục thật".into());
    }
    Ok(json!({"dev":meta.dev(),"ino":meta.ino()}))
}
pub(super) fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    let temp = path.with_extension("json.new");
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp)
        .map_err(err)?;
    let result = (|| {
        file.write_all(&serde_json::to_vec_pretty(value).map_err(err)?)
            .map_err(err)?;
        file.sync_all().map_err(err)?;
        fs::rename(&temp, path).map_err(err)?;
        sync_dir(path.parent().ok_or("Thiếu journal parent")?)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
impl Backend {
    fn journal_path(&self, id: &str) -> PathBuf {
        self.runtime().join(format!("restore-pending-{id}.json"))
    }
    fn transaction_paths(&self, id: &str, token: &str) -> (PathBuf, PathBuf) {
        (
            self.avds().join(format!(".restore-{id}-{token}")),
            self.avds().join(format!(".previous-{id}-{token}")),
        )
    }
    fn replace_profile(&self, profile: &Profile) -> Result<(), String> {
        let mut all = self.read_profiles()?;
        let current = all
            .iter_mut()
            .find(|p| p.id == profile.id)
            .ok_or("Máy không còn trong registry")?;
        if current.port != profile.port {
            return Err("Port máy đã thay đổi; dừng recovery để kiểm tra".into());
        }
        *current = profile.clone();
        self.write_profiles(&all)?;
        sync_dir(&self.runtime())
    }
    fn archive_journal(&self, id: &str, token: &str, outcome: &str) -> Result<(), String> {
        let history = self.runtime().join("restore-history");
        fs::create_dir_all(&history).map_err(err)?;
        directory_identity(&history)?;
        let receipt = history.join(format!("{id}-{token}-{outcome}.json"));
        if receipt.exists() {
            return Err("Biên nhận restore đã tồn tại; không ghi đè".into());
        }
        fs::rename(self.journal_path(id), receipt).map_err(err)?;
        let unfinished = self.journal_path(id).with_extension("json.new");
        if unfinished.exists() {
            if !fs::symlink_metadata(&unfinished).map_err(err)?.is_file() {
                return Err("Journal tạm không phải file thường".into());
            }
            fs::remove_file(unfinished).map_err(err)?;
        }
        sync_dir(&history)?;
        sync_dir(&self.runtime())
    }
    fn commit_restore(
        &self,
        before: &Profile,
        after: &Profile,
        token: &str,
        mut checkpoint: impl FnMut(Step) -> Result<(), String>,
    ) -> Result<PathBuf, String> {
        let live = self.data(before);
        let (stage, previous) = self.transaction_paths(&before.id, token);
        if previous.exists() || self.journal_path(&before.id).exists() {
            return Err("Restore transaction đã tồn tại".into());
        }
        let mut journal = json!({"schema":1,"state":"pending","token":token,"before":before,"after":after,
            "original_identity":directory_identity(&live)?,"staged_identity":directory_identity(&stage)?});
        write_json(&self.journal_path(&before.id), &journal)?;
        checkpoint(Step::Journal)?;
        fs::rename(&live, &previous).map_err(err)?;
        sync_dir(&self.avds())?;
        checkpoint(Step::OldMoved)?;
        fs::rename(&stage, &live).map_err(err)?;
        sync_dir(&self.avds())?;
        checkpoint(Step::NewMoved)?;
        self.replace_profile(after)?;
        checkpoint(Step::Registry)?;
        journal["state"] = json!("committed");
        write_json(&self.journal_path(&before.id), &journal)?;
        checkpoint(Step::Committed)?;
        self.archive_journal(&before.id, token, "committed")?;
        Ok(previous)
    }
    fn read_journal(&self, path: &Path) -> Result<(Value, Profile, Profile, String), String> {
        let meta = fs::symlink_metadata(path).map_err(err)?;
        if !meta.is_file() || meta.len() > 64 * 1024 {
            return Err("Journal restore không hợp lệ".into());
        }
        let journal: Value = serde_json::from_slice(&fs::read(path).map_err(err)?).map_err(err)?;
        if journal["schema"] != 1 {
            return Err("Journal restore khác phiên bản".into());
        }
        let before: Profile = serde_json::from_value(journal["before"].clone()).map_err(err)?;
        let after: Profile = serde_json::from_value(journal["after"].clone()).map_err(err)?;
        before.validate()?;
        after.validate()?;
        if before.id != after.id
            || before.port != after.port
            || path != self.journal_path(&before.id)
        {
            return Err("Journal restore sai máy hoặc port".into());
        }
        let token = journal["token"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 40 && s.bytes().all(|b| b.is_ascii_digit()))
            .ok_or("Token restore không hợp lệ")?
            .to_string();
        if !matches!(journal["state"].as_str(), Some("pending" | "committed")) {
            return Err("Trạng thái journal không hợp lệ".into());
        }
        Ok((journal, before, after, token))
    }
    /// Caller owns the Hub lock and has verified the target engine is stopped.
    fn recover_offline(&self, path: &Path) -> Result<(), String> {
        directory_identity(&self.avds())?;
        let (journal, before, after, token) = self.read_journal(path)?;
        let current = self.existing(&before.id)?;
        if current != before && current != after {
            return Err("Registry đã sửa bên ngoài; không tự ghi đè khi recovery".into());
        }
        let live = self.data(&before);
        let (stage, previous) = self.transaction_paths(&before.id, &token);
        if journal["state"] == "committed" {
            if current != after
                || directory_identity(&live)? != journal["staged_identity"]
                || directory_identity(&previous)? != journal["original_identity"]
            {
                return Err(
                    "Restore đã commit nhưng dữ liệu không khớp journal; cần kiểm tra".into(),
                );
            }
            return self.archive_journal(&before.id, &token, "committed");
        }
        if previous.exists() {
            if directory_identity(&previous)? != journal["original_identity"] {
                return Err("Dữ liệu rollback không khớp journal".into());
            }
            if live.exists() {
                if stage.exists() || directory_identity(&live)? != journal["staged_identity"] {
                    return Err("AVD đã thay đổi; không ghi đè khi rollback".into());
                }
                fs::rename(&live, &stage).map_err(err)?;
                sync_dir(&self.avds())?;
            }
            fs::rename(&previous, &live).map_err(err)?;
            sync_dir(&self.avds())?;
        } else if directory_identity(&live)? != journal["original_identity"] {
            return Err("Thiếu dữ liệu gốc để rollback; giữ journal để kiểm tra".into());
        }
        self.replace_profile(&before)?;
        // The original is now in place. Only remove the positively identified
        // staged copy; never guess which directory belongs to this transaction.
        if stage.exists() {
            if directory_identity(&stage)? != journal["staged_identity"] {
                return Err("Staging đã thay đổi; giữ nguyên để kiểm tra".into());
            }
            fs::remove_dir_all(stage).map_err(err)?;
            sync_dir(&self.avds())?;
        }
        self.archive_journal(&before.id, &token, "rolled-back")
    }
    pub(super) fn recover_restores_locked(&self) -> Result<(), String> {
        for entry in fs::read_dir(self.runtime()).map_err(err)? {
            let path = entry.map_err(err)?.path();
            match fs::symlink_metadata(&path) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(err(e)),
                Ok(_) => {}
            }
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            if let Some(id) = name
                .strip_prefix("restore-pending-")
                .and_then(|s| s.strip_suffix(".json.new"))
            {
                if !self.journal_path(id).exists() {
                    let profile = self.existing(id)?;
                    self.ensure_stopped(&profile)?;
                    if !fs::symlink_metadata(&path).map_err(err)?.is_file() {
                        return Err("Journal tạm không phải file thường".into());
                    }
                    fs::remove_file(&path).map_err(err)?;
                    sync_dir(&self.runtime())?;
                }
                continue;
            }
            if !name.starts_with("restore-pending-") || !name.ends_with(".json") {
                continue;
            }
            let (_, before, _, _) = self.read_journal(&path)?;
            self.ensure_stopped(&before)?;
            self.recover_offline(&path)?;
        }
        Ok(())
    }
    pub fn restore(
        &self,
        id: &str,
        backup: &Path,
        expected_manifest: &str,
    ) -> Result<RestoreOutcome, String> {
        let _guard = self.lock()?;
        let before = self.existing(id)?;
        self.ensure_stopped(&before)?;
        if self.backup_manifest_digest(backup)? != expected_manifest {
            return Err(
                "Backup đã thay đổi kể từ lúc chọn; hãy tải lại danh sách và xác nhận lại".into(),
            );
        }
        let after = self.verify_restore_backup(id, backup)?;
        let manifest_before: Value =
            serde_json::from_slice(&fs::read(backup.join("manifest.json")).map_err(err)?)
                .map_err(err)?;
        directory_identity(&self.avds())?;
        crate::storage::ensure_copy_space(&backup.join("payload"), &self.avds())?;
        let token = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(err)?
            .as_nanos()
            .to_string();
        let (stage, _) = self.transaction_paths(id, &token);
        if stage.exists() {
            return Err("Staging restore đã tồn tại".into());
        }
        let prepare = (|| {
            let mut copy = Command::new("/bin/cp");
            copy.arg("-R").arg(backup.join("payload")).arg(&stage);
            self.command(copy, 600)?;
            if json!(self.inventory(&stage)?) != manifest_before["files"] {
                return Err("Staging checksum không khớp".into());
            }
            self.sync_payload(&stage)?;
            if self.verify_restore_backup(id, backup)? != after {
                return Err("Profile backup đã thay đổi trong lúc chuẩn bị restore".into());
            }
            let manifest_after: Value =
                serde_json::from_slice(&fs::read(backup.join("manifest.json")).map_err(err)?)
                    .map_err(err)?;
            if manifest_after != manifest_before
                || self.backup_manifest_digest(backup)? != expected_manifest
            {
                return Err("Manifest backup đã thay đổi trong lúc chuẩn bị restore".into());
            }
            self.ensure_stopped(&before)?;
            Ok(())
        })();
        if let Err(error) = prepare {
            let _ = fs::remove_dir_all(&stage);
            return Err(error);
        }
        match self.commit_restore(&before, &after, &token, |_| Ok(())) {
            Ok(previous_data) => Ok(RestoreOutcome {
                snapshot: self.snapshot().map_err(|e| format!("Restore đã commit nhưng chưa đọc được trạng thái: {e}. Kiểm tra restore-history trước khi thử lại."))?,
                previous_data,
            }),
            Err(error) => {
                if self.journal_path(id).exists() {
                    // Do not turn a committed operation into a rollback on a
                    // receipt/fsync error. Recovery inspects the durable state.
                    match self.ensure_stopped(&before).and_then(|_| self.recover_offline(&self.journal_path(id))) {
                        Ok(()) => Err(format!(
                            "Restore gặp lỗi ({error}); recovery đã hoàn tất. Kiểm tra restore-history trước khi thử lại."
                        )),
                        Err(recovery) => Err(format!(
                            "Restore lỗi: {error}. Recovery đang chờ: {recovery}. Dữ liệu gốc được giữ lại."
                        )),
                    }
                } else {
                    Err(error)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(label: &str) -> (Backend, Profile, Profile, String) {
        let root = std::env::temp_dir().join(format!("hub-restore-{}-{label}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let backend = Backend::new(root).unwrap();
        fs::create_dir_all(backend.avds()).unwrap();
        let before = Profile::standard("hub-restore-test".into(), "Original".into(), 5680);
        let mut after = before.clone();
        after.name = "Restored".into();
        after.ram_mb = 1024;
        backend
            .write_profiles(std::slice::from_ref(&before))
            .unwrap();
        let token = "12345".to_string();
        let (stage, _) = backend.transaction_paths(&before.id, &token);
        fs::create_dir(backend.data(&before)).unwrap();
        fs::write(backend.data(&before).join("value"), b"original").unwrap();
        fs::create_dir(&stage).unwrap();
        fs::write(stage.join("value"), b"restored").unwrap();
        (backend, before, after, token)
    }
    #[test]
    fn every_commit_boundary_recovers_without_losing_original() {
        for (i, step) in [
            Step::Journal,
            Step::OldMoved,
            Step::NewMoved,
            Step::Registry,
            Step::Committed,
        ]
        .into_iter()
        .enumerate()
        {
            let (backend, before, after, token) = fixture(&format!("boundary-{i}"));
            assert!(
                backend
                    .commit_restore(&before, &after, &token, |at| if at == step {
                        Err("simulated process loss".into())
                    } else {
                        Ok(())
                    })
                    .is_err()
            );
            let reopened = Backend::new(backend.root.clone()).unwrap();
            reopened
                .recover_offline(&reopened.journal_path(&before.id))
                .unwrap();
            let (stage, previous) = reopened.transaction_paths(&before.id, &token);
            let committed = step == Step::Committed;
            assert_eq!(
                fs::read(reopened.data(&before).join("value")).unwrap(),
                if committed {
                    b"restored".as_slice()
                } else {
                    b"original".as_slice()
                }
            );
            assert_eq!(
                reopened.existing(&before.id).unwrap(),
                if committed { after } else { before.clone() }
            );
            assert!(!reopened.journal_path(&before.id).exists());
            assert!(!stage.exists());
            if committed {
                assert_eq!(fs::read(previous.join("value")).unwrap(), b"original");
            } else {
                assert!(!previous.exists());
            }
            reopened.recover_restores_locked().unwrap(); // repeated startup is a no-op
            fs::remove_dir_all(backend.root).unwrap();
        }
    }
    #[test]
    fn recovery_can_resume_after_each_directory_move() {
        for point in 0..3 {
            let (backend, before, after, token) = fixture(&format!("rollback-{point}"));
            let _ = backend.commit_restore(&before, &after, &token, |at| {
                if at == Step::Registry {
                    Err("crash".into())
                } else {
                    Ok(())
                }
            });
            let (stage, previous) = backend.transaction_paths(&before.id, &token);
            fs::rename(backend.data(&before), &stage).unwrap();
            if point >= 1 {
                fs::rename(&previous, backend.data(&before)).unwrap();
            }
            if point == 2 {
                fs::remove_dir_all(&stage).unwrap();
            }
            backend
                .recover_offline(&backend.journal_path(&before.id))
                .unwrap();
            assert_eq!(
                fs::read(backend.data(&before).join("value")).unwrap(),
                b"original"
            );
            assert_eq!(backend.existing(&before.id).unwrap(), before);
            fs::remove_dir_all(backend.root).unwrap();
        }
    }
    #[test]
    fn registry_write_failure_keeps_journal_until_recovery_can_finish() {
        let (backend, before, after, token) = fixture("registry-failure");
        let obstacle = backend.runtime().join("hub-profiles.json.new");
        fs::create_dir(&obstacle).unwrap();
        assert!(
            backend
                .commit_restore(&before, &after, &token, |_| Ok(()))
                .is_err()
        );
        assert!(
            backend
                .recover_offline(&backend.journal_path(&before.id))
                .is_err()
        );
        assert!(backend.journal_path(&before.id).exists());
        assert_eq!(
            fs::read(backend.data(&before).join("value")).unwrap(),
            b"original"
        );
        fs::remove_dir(obstacle).unwrap();
        backend
            .recover_offline(&backend.journal_path(&before.id))
            .unwrap();
        assert_eq!(backend.existing(&before.id).unwrap(), before);
        fs::remove_dir_all(backend.root).unwrap();
    }
    #[test]
    fn changed_registry_and_unsafe_journal_are_not_overwritten() {
        let (backend, before, after, token) = fixture("foreign-change");
        let _ = backend.commit_restore(&before, &after, &token, |at| {
            if at == Step::NewMoved {
                Err("crash".into())
            } else {
                Ok(())
            }
        });
        let mut external = before.clone();
        external.name = "External edit".into();
        backend
            .write_profiles(std::slice::from_ref(&external))
            .unwrap();
        assert!(
            backend
                .recover_offline(&backend.journal_path(&before.id))
                .is_err()
        );
        assert_eq!(backend.existing(&before.id).unwrap(), external);
        let path = backend.journal_path(&before.id);
        let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        value["token"] = json!("../../elsewhere");
        write_json(&path, &value).unwrap();
        assert!(backend.read_journal(&path).is_err());
        let (_, previous) = backend.transaction_paths(&before.id, &token);
        assert_eq!(fs::read(previous.join("value")).unwrap(), b"original");
        fs::remove_dir_all(backend.root).unwrap();
    }
    #[test]
    #[ignore = "requires host process inspection and loopback port checks"]
    fn reopening_hub_recovers_pending_transaction_only_when_stopped() {
        let (backend, before, after, token) = fixture("startup");
        let _ = backend.commit_restore(&before, &after, &token, |at| {
            if at == Step::OldMoved {
                Err("crash".into())
            } else {
                Ok(())
            }
        });
        let guard = std::net::TcpListener::bind(("127.0.0.1", before.port)).unwrap();
        let reopened = Backend::new(backend.root.clone()).unwrap();
        assert!(reopened.initialize().is_err());
        assert!(reopened.journal_path(&before.id).exists());
        drop(guard);
        let state = reopened.initialize().unwrap();
        assert_eq!(state.profiles, vec![before.clone()]);
        assert_eq!(
            fs::read(reopened.data(&before).join("value")).unwrap(),
            b"original"
        );
        // An interrupted first journal write cannot have moved the live directory.
        let unfinished = reopened.journal_path(&before.id).with_extension("json.new");
        fs::write(&unfinished, b"partial json").unwrap();
        reopened.initialize().unwrap();
        assert!(!unfinished.exists());
        assert_eq!(
            fs::read(reopened.data(&before).join("value")).unwrap(),
            b"original"
        );
        fs::remove_dir_all(backend.root).unwrap();
    }
    #[test]
    fn foreign_directory_is_preserved_instead_of_forced_rollback() {
        let (backend, before, after, token) = fixture("foreign-directory");
        let _ = backend.commit_restore(&before, &after, &token, |at| {
            if at == Step::NewMoved {
                Err("crash".into())
            } else {
                Ok(())
            }
        });
        let live = backend.data(&before);
        fs::rename(&live, backend.root.join("external-move")).unwrap();
        fs::create_dir(&live).unwrap();
        fs::write(live.join("value"), b"external data").unwrap();
        assert!(
            backend
                .recover_offline(&backend.journal_path(&before.id))
                .is_err()
        );
        assert_eq!(fs::read(live.join("value")).unwrap(), b"external data");
        let (_, previous) = backend.transaction_paths(&before.id, &token);
        assert_eq!(fs::read(previous.join("value")).unwrap(), b"original");
        fs::remove_dir_all(backend.root).unwrap();
    }
}
