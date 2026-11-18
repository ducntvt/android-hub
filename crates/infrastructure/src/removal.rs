//! Device archival and restoration; permanent deletion is isolated in `purge`.
mod purge;
use crate::{
    Backend, Snapshot, err,
    restore::{sync_dir, write_json},
};
use android_hub_domain::Profile;
pub use purge::{DeletePlan, PurgePlan};
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug)]
pub struct RemovalOutcome {
    pub snapshot: Snapshot,
    pub archive: PathBuf,
}
#[derive(Clone, Debug, PartialEq)]
pub struct RemovedDevice {
    pub profile: Profile,
    pub archive: PathBuf,
    pub unavailable: Option<String>,
}
impl std::fmt::Display for RemovedDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} · {}", self.profile.name, self.profile.id)
    }
}
#[derive(Clone, Debug, Default)]
pub struct RemovedCatalog {
    pub entries: Vec<RemovedDevice>,
    pub skipped: usize,
}
struct Journal {
    value: Value,
    profile: Profile,
    before: Vec<Profile>,
    after: Vec<Profile>,
    archive: PathBuf,
}
#[derive(Clone, Copy, PartialEq, Debug)]
enum Step {
    Journal,
    Data,
    Ini,
    Registry,
    Committed,
}
fn identity(path: &Path, directory: bool) -> Result<Value, String> {
    let meta = fs::symlink_metadata(path).map_err(err)?;
    if meta.file_type().is_symlink()
        || (directory && !meta.is_dir())
        || (!directory && !meta.is_file())
    {
        return Err("Đường dẫn gỡ máy không phải file/thư mục thật".into());
    }
    Ok(json!({"dev":meta.dev(), "ino":meta.ino()}))
}
fn present(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(err(e)),
    }
}
impl Backend {
    fn removal_journal(&self, id: &str) -> PathBuf {
        self.runtime().join(format!("remove-pending-{id}.json"))
    }
    fn removal_archive(&self, id: &str, token: &str) -> PathBuf {
        self.avds().join(format!(".removed-{id}-{token}"))
    }
    fn commit_removal(
        &self,
        profile: &Profile,
        mut checkpoint: impl FnMut(Step) -> Result<(), String>,
    ) -> Result<PathBuf, String> {
        let before = self.read_profiles()?;
        if before.iter().find(|p| p.id == profile.id) != Some(profile) {
            return Err("Máy đã thay đổi; xác nhận lại trước khi gỡ".into());
        }
        let after: Vec<_> = before
            .iter()
            .filter(|p| p.id != profile.id)
            .cloned()
            .collect();
        identity(&self.avds(), true)?;
        let live = self.data(profile);
        let ini = self.avds().join(format!("{}.ini", profile.id));
        let data_identity = identity(&live, true)?;
        let ini_identity = identity(&ini, false)?;
        let token = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(err)?
            .as_nanos()
            .to_string();
        let archive = self.removal_archive(&profile.id, &token);
        fs::create_dir(&archive).map_err(err)?;
        fs::set_permissions(&archive, fs::Permissions::from_mode(0o700)).map_err(err)?;
        let mut journal = json!({"schema":1,"state":"pending","profile":profile,"before":before,"after":after,
            "token":token,"data_identity":data_identity,"ini_identity":ini_identity,"archive_identity":identity(&archive,true)?});
        let path = self.removal_journal(&profile.id);
        if present(&path)? {
            return Err("Gỡ máy trước đó chưa hoàn tất".into());
        }
        write_json(&path, &journal)?;
        checkpoint(Step::Journal)?;
        fs::rename(live, archive.join("data.avd")).map_err(err)?;
        sync_dir(&self.avds())?;
        sync_dir(&archive)?;
        checkpoint(Step::Data)?;
        fs::rename(ini, archive.join("registration.ini")).map_err(err)?;
        sync_dir(&self.avds())?;
        sync_dir(&archive)?;
        checkpoint(Step::Ini)?;
        self.write_profiles(&after)?;
        sync_dir(&self.runtime())?;
        checkpoint(Step::Registry)?;
        journal["state"] = json!("committed");
        write_json(&path, &journal)?;
        checkpoint(Step::Committed)?;
        self.recover_removal_offline(&path)?;
        Ok(archive)
    }
    fn read_removal(&self, path: &Path) -> Result<Journal, String> {
        identity(path, false)?;
        if fs::metadata(path).map_err(err)?.len() > 1024 * 1024 {
            return Err("Journal gỡ máy quá lớn".into());
        }
        let value: Value = serde_json::from_slice(&fs::read(path).map_err(err)?).map_err(err)?;
        if value["schema"] != 1 || ![json!("pending"), json!("committed")].contains(&value["state"])
        {
            return Err("Journal gỡ máy không hợp lệ".into());
        }
        let profile: Profile = serde_json::from_value(value["profile"].clone()).map_err(err)?;
        profile.validate()?;
        let token = value["token"].as_str().ok_or("Thiếu token")?;
        if token.is_empty() || token.len() > 40 || !token.bytes().all(|c| c.is_ascii_digit()) {
            return Err("Token gỡ máy không hợp lệ".into());
        }
        let before: Vec<Profile> = serde_json::from_value(value["before"].clone()).map_err(err)?;
        let after: Vec<Profile> = serde_json::from_value(value["after"].clone()).map_err(err)?;
        crate::validate_registry(&before)?;
        crate::validate_registry(&after)?;
        let undo = value["operation"] == "undo";
        if !value["operation"].is_null() && !undo {
            return Err("Operation journal không hợp lệ".into());
        }
        let expected_after: Vec<_> = if undo {
            if before.iter().any(|p| p.id == profile.id) {
                return Err("Undo journal đã có máy đích".into());
            }
            let mut result = before.clone();
            result.push(profile.clone());
            result
        } else {
            if before.iter().find(|p| p.id == profile.id) != Some(&profile) {
                return Err("Journal thiếu máy đích".into());
            }
            before
                .iter()
                .filter(|p| p.id != profile.id)
                .cloned()
                .collect()
        };
        if after != expected_after {
            return Err("Registry journal không hợp lệ".into());
        }
        let archive = self.removal_archive(&profile.id, token);
        let committed = value["state"] == "committed";
        let name = path.file_name().and_then(|v| v.to_str()).unwrap_or("");
        let undo_rollback = name
            .strip_prefix("undo-rolled-back-")
            .and_then(|v| v.strip_suffix(".json"))
            .is_some_and(|v| {
                !v.is_empty() && v.len() <= 40 && v.bytes().all(|c| c.is_ascii_digit())
            });
        let receipt = path.parent() == Some(archive.as_path())
            && match (undo, committed) {
                (false, true) => matches!(name, "removed.json" | "purging.json"),
                (false, false) => name == "rolled-back.json",
                (true, true) => name == "restored.json",
                (true, false) => undo_rollback,
            };
        if path != self.removal_journal(&profile.id) && !receipt {
            return Err("Journal sai máy hoặc trạng thái biên nhận".into());
        }
        identity(&self.avds(), true)?;
        if identity(&archive, true)? != value["archive_identity"] {
            return Err("Archive đã thay đổi; giữ dữ liệu để kiểm tra".into());
        }
        Ok(Journal {
            value,
            profile,
            before,
            after,
            archive,
        })
    }
    fn recover_removal_offline(&self, path: &Path) -> Result<(), String> {
        let Journal {
            value,
            profile,
            before,
            after,
            archive,
        } = self.read_removal(path)?;
        let current = self.read_profiles()?;
        if current != before && current != after {
            return Err("Registry thay đổi ngoài Hub; không recovery gỡ máy".into());
        }
        let pairs = [
            (
                self.data(&profile),
                archive.join("data.avd"),
                true,
                "data_identity",
            ),
            (
                self.avds().join(format!("{}.ini", profile.id)),
                archive.join("registration.ini"),
                false,
                "ini_identity",
            ),
        ];
        let committed = value["state"] == "committed";
        let undo = value["operation"] == "undo";
        // Validate both before moving either, including partially rolled-back states.
        for (live, stored, directory, key) in &pairs {
            let l = present(live)?;
            let a = present(stored)?;
            if l == a
                || (committed && l != undo)
                || identity(if a { stored } else { live }, *directory)? != value[*key]
            {
                return Err("Dữ liệu gỡ máy không khớp journal; không ghi đè".into());
            }
        }
        if committed {
            if current != after {
                return Err("Registry committed không khớp".into());
            }
        } else {
            for (live, stored, _, _) in &pairs {
                let (source, destination) = if undo { (live, stored) } else { (stored, live) };
                if present(source)? {
                    fs::rename(source, destination).map_err(err)?;
                    sync_dir(&self.avds())?;
                    sync_dir(&archive)?;
                }
            }
            self.write_profiles(&before)?;
            sync_dir(&self.runtime())?;
        }
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(err)?
            .as_nanos();
        let receipt = archive.join(match (undo, committed) {
            (false, true) => "removed.json".into(),
            (false, false) => "rolled-back.json".into(),
            (true, true) => "restored.json".into(),
            (true, false) => format!("undo-rolled-back-{suffix}.json"),
        });
        if present(&receipt)? {
            return Err("Biên nhận gỡ máy đã tồn tại".into());
        }
        let unfinished = path.with_extension("json.new");
        if present(&unfinished)? {
            identity(&unfinished, false)?;
            let kept = archive.join(format!("unfinished-journal-{suffix}.json"));
            if present(&kept)? {
                return Err("Đã có journal tạm trong archive".into());
            }
            fs::rename(unfinished, kept).map_err(err)?;
            sync_dir(&archive)?;
            sync_dir(&self.runtime())?;
        }
        fs::rename(path, receipt).map_err(err)?;
        sync_dir(&archive)?;
        sync_dir(&self.runtime())?;
        Ok(())
    }
    pub(super) fn recover_removals_locked(&self) -> Result<(), String> {
        for entry in fs::read_dir(self.runtime()).map_err(err)? {
            let path = entry.map_err(err)?.path();
            if path
                .file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.starts_with("remove-pending-") && s.ends_with(".json"))
            {
                let profile = self.read_removal(&path)?.profile;
                self.ensure_stopped(&profile)?;
                self.recover_removal_offline(&path)?;
            }
        }
        Ok(())
    }
    fn undo_source(&self, archive: &Path, expected: &Profile) -> Result<Journal, String> {
        let journal = self.read_removal(&archive.join("removed.json"))?;
        if journal.archive != archive
            || journal.profile != *expected
            || present(&archive.join("restored.json"))?
            || present(&archive.join("purging.json"))?
        {
            return Err("Archive đã thay đổi hoặc máy đã được đưa trở lại".into());
        }
        self.ensure_new_avd(expected)?;
        if identity(&archive.join("data.avd"), true)? != journal.value["data_identity"]
            || identity(&archive.join("registration.ini"), false)? != journal.value["ini_identity"]
        {
            return Err("Dữ liệu archive đã thay đổi; không ghi đè".into());
        }
        Ok(journal)
    }
    fn commit_undo_removal(
        &self,
        archive: &Path,
        expected: &Profile,
        mut checkpoint: impl FnMut(Step) -> Result<(), String>,
    ) -> Result<(), String> {
        let source = self.undo_source(archive, expected)?;
        let before = self.read_profiles()?;
        if before
            .iter()
            .any(|p| p.id == expected.id || p.port == expected.port)
        {
            return Err("ID hoặc port đã được máy khác sử dụng; chưa thể đưa máy trở lại".into());
        }
        let mut after = before.clone();
        after.push(expected.clone());
        crate::validate_registry(&after)?;
        let mut journal = source.value;
        journal["operation"] = json!("undo");
        journal["state"] = json!("pending");
        journal["before"] = json!(before);
        journal["after"] = json!(after);
        let path = self.removal_journal(&expected.id);
        if present(&path)? {
            return Err("Có transaction chưa hoàn tất".into());
        }
        write_json(&path, &journal)?;
        checkpoint(Step::Journal)?;
        fs::rename(archive.join("data.avd"), self.data(expected)).map_err(err)?;
        sync_dir(archive)?;
        sync_dir(&self.avds())?;
        checkpoint(Step::Data)?;
        fs::rename(
            archive.join("registration.ini"),
            self.avds().join(format!("{}.ini", expected.id)),
        )
        .map_err(err)?;
        sync_dir(archive)?;
        sync_dir(&self.avds())?;
        checkpoint(Step::Ini)?;
        self.write_profiles(&after)?;
        sync_dir(&self.runtime())?;
        checkpoint(Step::Registry)?;
        journal["state"] = json!("committed");
        write_json(&path, &journal)?;
        checkpoint(Step::Committed)?;
        self.recover_removal_offline(&path)
    }
    pub fn removal_history(&self) -> Result<Vec<String>, String> {
        let mut history = vec![];
        let mut skipped = 0;
        if !self.avds().exists() {
            return Ok(vec![]);
        }
        identity(&self.avds(), true)?;
        for entry in fs::read_dir(self.avds()).map_err(err)? {
            let entry = entry.map_err(err)?;
            if !entry.file_name().to_string_lossy().starts_with(".removed-") {
                continue;
            }
            if identity(&entry.path(), true).is_err() {
                skipped += 1;
                continue;
            }
            for file in fs::read_dir(entry.path()).map_err(err)? {
                let file = file.map_err(err)?;
                let name = file.file_name();
                let name = name.to_string_lossy();
                if !["removed.json", "restored.json", "rolled-back.json"].contains(&name.as_ref())
                    && !name.starts_with("undo-rolled-back-")
                {
                    continue;
                }
                let read = (|| -> Result<_, String> {
                    let record = self.read_removal(&file.path())?;
                    let action = match (
                        record.value["operation"] == "undo",
                        record.value["state"] == "committed",
                    ) {
                        (false, true) => "đã gỡ khỏi danh sách; dữ liệu được giữ",
                        (false, false) => "gỡ máy bị gián đoạn; đã đưa máy về trạng thái trước đó",
                        (true, true) => "đã đưa máy trở lại danh sách",
                        (true, false) => "đưa máy trở lại bị gián đoạn; dữ liệu vẫn trong archive",
                    };
                    let modified = fs::metadata(file.path())
                        .map_err(err)?
                        .modified()
                        .map_err(err)?;
                    Ok((
                        modified,
                        file.path(),
                        format!("{}: {action}", record.profile.name),
                    ))
                })();
                match read {
                    Ok(record) => history.push(record),
                    Err(_) => skipped += 1,
                }
            }
        }
        history.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
        let mut result: Vec<_> = history.into_iter().take(3).map(|v| v.2).collect();
        if skipped > 0 {
            result.push(format!("{skipped} biên nhận/archive không đọc được; cần kiểm tra trước khi kết luận lịch sử đầy đủ."));
        }
        Ok(result)
    }
    pub fn removed_devices(&self) -> Result<RemovedCatalog, String> {
        let mut catalog = RemovedCatalog::default();
        if !present(&self.avds())? {
            return Ok(catalog);
        }
        identity(&self.avds(), true)?;
        let active = self.read_profiles()?;
        for entry in fs::read_dir(self.avds()).map_err(err)? {
            let entry = entry.map_err(err)?;
            if !entry.file_name().to_string_lossy().starts_with(".removed-") {
                continue;
            }
            let archive = entry.path();
            let read = (|| -> Result<Option<RemovedDevice>, String> {
                identity(&archive, true)?;
                if present(&archive.join("restored.json"))?
                    || (!present(&archive.join("removed.json"))?
                        && !present(&archive.join("purging.json"))?)
                {
                    return Ok(None);
                }
                let source =
                    self.read_removal(&archive.join(if present(&archive.join("purging.json"))? {
                        "purging.json"
                    } else {
                        "removed.json"
                    }))?;
                let unavailable = if present(&archive.join("purging.json"))? {
                    Some("Xóa trước đó chưa hoàn tất; chỉ có thể tiếp tục xóa.".into())
                } else if active
                    .iter()
                    .any(|p| p.id == source.profile.id || p.port == source.profile.port)
                {
                    Some("ID hoặc port cũ đang được máy khác sử dụng".into())
                } else {
                    self.undo_source(&archive, &source.profile).err()
                };
                Ok(Some(RemovedDevice {
                    profile: source.profile,
                    archive: archive.clone(),
                    unavailable,
                }))
            })();
            match read {
                Ok(Some(info)) => catalog.entries.push(info),
                Ok(None) => {}
                Err(_) => catalog.skipped += 1,
            }
        }
        catalog.entries.sort_by(|a, b| {
            a.profile
                .name
                .cmp(&b.profile.name)
                .then_with(|| a.archive.cmp(&b.archive))
        });
        Ok(catalog)
    }
    pub fn undo_remove_device(
        &self,
        archive: &Path,
        expected: &Profile,
    ) -> Result<Snapshot, String> {
        expected.validate()?;
        let _guard = self.lock()?;
        self.undo_source(archive, expected)?;
        self.ensure_stopped(expected)?;
        if let Err(error) = self.commit_undo_removal(archive, expected, |_| Ok(())) {
            if self.ensure_stopped(expected).is_ok()
                && present(&self.removal_journal(&expected.id))?
                && let Err(recovery) =
                    self.recover_removal_offline(&self.removal_journal(&expected.id))
            {
                return Err(format!("{error}; recovery: {recovery}"));
            }
            return Err(error);
        }
        self.snapshot()
            .map_err(|e| format!("Đã đưa máy trở lại nhưng chưa refresh được: {e}"))
    }
    pub fn remove_device(&self, expected: &Profile) -> Result<RemovalOutcome, String> {
        expected.validate()?;
        let _guard = self.lock()?;
        if self.existing(&expected.id)? != *expected {
            return Err("Máy đã thay đổi; xác nhận lại".into());
        }
        self.ensure_stopped(expected)?;
        let archive = match self.commit_removal(expected, |_| Ok(())) {
            Ok(path) => path,
            Err(error) => {
                if self.ensure_stopped(expected).is_ok()
                    && self.removal_journal(&expected.id).exists()
                    && let Err(recovery) =
                        self.recover_removal_offline(&self.removal_journal(&expected.id))
                {
                    return Err(format!("{error}; recovery: {recovery}"));
                }
                return Err(error);
            }
        };
        let snapshot = self.snapshot().map_err(|e| {
            format!(
                "Đã gỡ máy, dữ liệu giữ tại {}; chưa refresh được: {e}",
                archive.display()
            )
        })?;
        Ok(RemovalOutcome { snapshot, archive })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn fixture() -> (Backend, Profile) {
        let token = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let sequence = crate::SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "hub-removal-{}-{token}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        let b = Backend {
            sdk: root.join("sdk"),
            root,
        };
        fs::create_dir_all(b.avds()).unwrap();
        let p = Profile::standard("hub-removal-test".into(), "Fixture".into(), 5680);
        b.write_profiles(std::slice::from_ref(&p)).unwrap();
        fs::create_dir(b.data(&p)).unwrap();
        fs::write(b.data(&p).join("userdata.img"), b"original guest bytes").unwrap();
        fs::write(
            b.avds().join(format!("{}.ini", p.id)),
            b"original registration",
        )
        .unwrap();
        (b, p)
    }
    #[test]
    fn interruption_at_every_step_rolls_back_or_preserves_commit() {
        for step in [
            Step::Journal,
            Step::Data,
            Step::Ini,
            Step::Registry,
            Step::Committed,
        ] {
            let (b, p) = fixture();
            assert!(
                b.commit_removal(&p, |at| if at == step {
                    Err("injected interruption".into())
                } else {
                    Ok(())
                })
                .is_err()
            );
            let path = b.removal_journal(&p.id);
            let archive = b.read_removal(&path).unwrap().archive;
            b.recover_removal_offline(&path).unwrap();
            assert!(!path.exists());
            if step == Step::Committed {
                assert!(b.read_profiles().unwrap().is_empty());
                assert!(!b.data(&p).exists());
                assert_eq!(
                    fs::read(archive.join("data.avd/userdata.img")).unwrap(),
                    b"original guest bytes"
                );
                assert!(archive.join("removed.json").is_file());
            } else {
                assert_eq!(b.read_profiles().unwrap(), vec![p.clone()]);
                assert_eq!(
                    fs::read(b.data(&p).join("userdata.img")).unwrap(),
                    b"original guest bytes"
                );
                assert_eq!(
                    fs::read(b.avds().join(format!("{}.ini", p.id))).unwrap(),
                    b"original registration"
                );
                assert!(archive.join("rolled-back.json").is_file());
            }
            fs::remove_dir_all(b.root).unwrap();
        }
    }
    #[test]
    fn foreign_live_directory_prevents_rollback_without_data_loss() {
        let (b, p) = fixture();
        let _ = b.commit_removal(&p, |at| {
            if at == Step::Ini {
                Err("crash".into())
            } else {
                Ok(())
            }
        });
        let path = b.removal_journal(&p.id);
        let archive = b.read_removal(&path).unwrap().archive;
        fs::create_dir(b.data(&p)).unwrap();
        fs::write(b.data(&p).join("foreign"), b"keep").unwrap();
        assert!(b.recover_removal_offline(&path).is_err());
        assert!(path.exists());
        assert_eq!(fs::read(b.data(&p).join("foreign")).unwrap(), b"keep");
        assert_eq!(
            fs::read(archive.join("data.avd/userdata.img")).unwrap(),
            b"original guest bytes"
        );
        fs::remove_dir_all(b.root).unwrap();
    }
    #[test]
    fn recovery_resumes_after_first_rollback_move_and_rejects_registry_edits() {
        let (b, p) = fixture();
        let _ = b.commit_removal(&p, |at| {
            if at == Step::Registry {
                Err("crash".into())
            } else {
                Ok(())
            }
        });
        let path = b.removal_journal(&p.id);
        let archive = b.read_removal(&path).unwrap().archive;
        fs::rename(archive.join("data.avd"), b.data(&p)).unwrap();
        let mut changed = p.clone();
        changed.name = "external change".into();
        b.write_profiles(&[changed]).unwrap();
        assert!(b.recover_removal_offline(&path).is_err());
        b.write_profiles(&[]).unwrap();
        b.recover_removal_offline(&path).unwrap();
        assert_eq!(b.read_profiles().unwrap(), vec![p]);
        fs::remove_dir_all(b.root).unwrap();
    }
    #[test]
    fn undo_each_boundary_preserves_archived_or_restored_data() {
        for step in [
            Step::Journal,
            Step::Data,
            Step::Ini,
            Step::Registry,
            Step::Committed,
        ] {
            let (b, p) = fixture();
            let archive = b.commit_removal(&p, |_| Ok(())).unwrap();
            assert!(
                b.commit_undo_removal(&archive, &p, |at| if at == step {
                    Err("crash".into())
                } else {
                    Ok(())
                })
                .is_err()
            );
            b.recover_removal_offline(&b.removal_journal(&p.id))
                .unwrap();
            if step == Step::Committed {
                assert_eq!(b.read_profiles().unwrap(), vec![p.clone()]);
                assert_eq!(
                    fs::read(b.data(&p).join("userdata.img")).unwrap(),
                    b"original guest bytes"
                );
                assert!(archive.join("restored.json").is_file());
                assert!(b.commit_undo_removal(&archive, &p, |_| Ok(())).is_err());
            } else {
                assert!(b.read_profiles().unwrap().is_empty());
                assert!(!b.data(&p).exists());
                assert_eq!(
                    fs::read(archive.join("data.avd/userdata.img")).unwrap(),
                    b"original guest bytes"
                );
                // A failed undo may be retried without colliding with its receipt.
                b.commit_undo_removal(&archive, &p, |_| Ok(())).unwrap();
                assert_eq!(b.read_profiles().unwrap(), vec![p]);
            }
            fs::remove_dir_all(b.root).unwrap();
        }
    }
    #[test]
    fn undo_rejects_reused_port_and_existing_paths() {
        let (b, p) = fixture();
        let archive = b.commit_removal(&p, |_| Ok(())).unwrap();
        let other = Profile::standard("hub-other".into(), "Other".into(), p.port);
        b.write_profiles(std::slice::from_ref(&other)).unwrap();
        assert!(b.commit_undo_removal(&archive, &p, |_| Ok(())).is_err());
        assert_eq!(b.read_profiles().unwrap(), vec![other]);
        b.write_profiles(&[]).unwrap();
        fs::write(b.avds().join(format!("{}.ini", p.id)), b"foreign").unwrap();
        assert!(b.commit_undo_removal(&archive, &p, |_| Ok(())).is_err());
        assert_eq!(
            fs::read(archive.join("data.avd/userdata.img")).unwrap(),
            b"original guest bytes"
        );
        assert!(!b.removal_journal(&p.id).exists());
        fs::remove_dir_all(b.root).unwrap();
    }
    #[test]
    #[ignore = "requires host process inspection and loopback ports"]
    fn public_remove_undo_and_startup_recovery_respect_busy_port() -> Result<(), String> {
        let (b, p) = fixture();
        let guard = std::net::TcpListener::bind(("127.0.0.1", p.port)).map_err(err)?;
        assert!(b.remove_device(&p).is_err());
        assert!(b.data(&p).exists());
        drop(guard);
        let removed = b.remove_device(&p)?;
        assert!(removed.snapshot.profiles.is_empty());
        let guard = std::net::TcpListener::bind(("127.0.0.1", p.port)).map_err(err)?;
        assert!(b.undo_remove_device(&removed.archive, &p).is_err());
        drop(guard);
        assert_eq!(
            b.undo_remove_device(&removed.archive, &p)?.profiles,
            vec![p.clone()]
        );
        for undo in [false, true] {
            if undo {
                let archive = b.commit_removal(&p, |_| Ok(()))?;
                let _ = b.commit_undo_removal(&archive, &p, |at| {
                    if at == Step::Data {
                        Err("crash".into())
                    } else {
                        Ok(())
                    }
                });
            } else {
                let _ = b.commit_removal(&p, |at| {
                    if at == Step::Data {
                        Err("crash".into())
                    } else {
                        Ok(())
                    }
                });
            }
            let guard = std::net::TcpListener::bind(("127.0.0.1", p.port)).map_err(err)?;
            assert!(b.initialize().is_err());
            assert!(b.removal_journal(&p.id).exists());
            drop(guard);
            let recovered = b.initialize()?;
            assert_eq!(recovered.profiles.is_empty(), undo);
            let expected = if undo {
                "đưa máy trở lại bị gián đoạn"
            } else {
                "gỡ máy bị gián đoạn"
            };
            assert!(
                b.removal_history()?
                    .iter()
                    .any(|event| event.contains(expected))
            );
            assert!(!b.removal_journal(&p.id).exists());
        }
        fs::remove_dir_all(b.root).map_err(err)?;
        Ok(())
    }
    #[test]
    fn catalog_lists_archived_device_with_conflict_and_hides_restored_device() {
        let (b, p) = fixture();
        let archive = b.commit_removal(&p, |_| Ok(())).unwrap();
        let catalog = b.removed_devices().unwrap();
        assert_eq!(catalog.entries.len(), 1);
        assert_eq!(catalog.entries[0].archive, archive);
        assert!(catalog.entries[0].unavailable.is_none());
        let other = Profile::standard("hub-other".into(), "Other".into(), p.port);
        b.write_profiles(&[other]).unwrap();
        assert!(
            b.removed_devices().unwrap().entries[0]
                .unavailable
                .is_some()
        );
        b.write_profiles(&[]).unwrap();
        b.commit_undo_removal(&archive, &p, |_| Ok(())).unwrap();
        assert!(b.removed_devices().unwrap().entries.is_empty());
        std::os::unix::fs::symlink(b.root.join("missing"), b.avds().join(".removed-bad")).unwrap();
        assert_eq!(b.removed_devices().unwrap().skipped, 1);
        fs::remove_dir_all(b.root).unwrap();
    }
    #[test]
    fn history_labels_commit_and_rollback_and_rejects_forged_receipt_state() {
        let (b, p) = fixture();
        let _ = b.commit_removal(&p, |at| {
            if at == Step::Data {
                Err("crash".into())
            } else {
                Ok(())
            }
        });
        b.recover_removal_offline(&b.removal_journal(&p.id))
            .unwrap();
        assert!(
            b.removal_history()
                .unwrap()
                .iter()
                .any(|v| v.contains("gỡ máy bị gián đoạn"))
        );
        let archive = b.commit_removal(&p, |_| Ok(())).unwrap();
        b.commit_undo_removal(&archive, &p, |_| Ok(())).unwrap();
        let history = b.removal_history().unwrap();
        assert!(history.iter().any(|v| v.contains("đã gỡ khỏi")));
        assert!(
            history
                .iter()
                .any(|v| v.contains("đã đưa máy trở lại danh sách"))
        );
        // A committed undo receipt cannot be renamed as a rollback receipt.
        fs::copy(
            archive.join("restored.json"),
            archive.join("undo-rolled-back-1.json"),
        )
        .unwrap();
        assert!(
            b.removal_history()
                .unwrap()
                .iter()
                .any(|v| v.contains("không đọc được"))
        );
        fs::remove_dir_all(b.root).unwrap();
    }
}
