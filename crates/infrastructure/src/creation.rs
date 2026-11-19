//! Creation intent survives Hub exits without deleting partially created AVDs.
use crate::{
    Backend, err,
    restore::{sync_dir, write_json},
    validate_registry,
};
use android_hub_domain::{AndroidVersion, CreateRequest, ImageIdentity, Profile};
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, MetadataExt},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::Ordering,
    time::{SystemTime, UNIX_EPOCH},
};

/// Component presence, not a guarantee that SDK processes will execute successfully.
#[derive(Clone, Debug)]
pub struct CreationEnvironment {
    pub components: Result<(), String>,
    /// None means absent; an invalid existing image is an error, never a download target.
    pub image: Result<Option<[u32; 3]>, String>,
    pub available_bytes: Result<u64, String>,
    pub minimum_bytes: u64,
}
impl CreationEnvironment {
    pub fn ready(&self) -> bool {
        self.prerequisites_ready() && self.image.as_ref().is_ok_and(Option::is_some)
    }
    pub fn downloadable(&self) -> bool {
        self.prerequisites_ready() && matches!(self.image, Ok(None))
    }
    fn prerequisites_ready(&self) -> bool {
        self.components.is_ok()
            && self
                .available_bytes
                .as_ref()
                .is_ok_and(|n| *n >= self.minimum_bytes)
    }
}

/// Registry publication succeeded. Never retry creation just because refresh failed.
#[derive(Clone, Debug)]
pub struct CreatedDevice {
    pub profile: Profile,
    pub snapshot: Result<crate::Snapshot, String>,
    pub warning: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CreationFailure {
    pub message: String,
    /// Journal publication was attempted; recovery may register this device later.
    pub retained_profile: Option<Box<Profile>>,
    pub outcome_unknown: bool,
}
impl From<String> for CreationFailure {
    fn from(message: String) -> Self {
        Self {
            message,
            retained_profile: None,
            outcome_unknown: false,
        }
    }
}

fn available_at_destination(destination: &Path) -> Result<u64, String> {
    // Before AVD_HOME exists, inspect the containing filesystem without creating it.
    for path in destination.ancestors() {
        match fs::metadata(path) {
            Ok(meta) if meta.is_dir() => return fs2::available_space(path).map_err(err),
            Ok(_) => return Err("Đường dẫn dữ liệu không phải thư mục".into()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(err(e)),
        }
    }
    Err("Không xác định được filesystem chứa dữ liệu máy".into())
}

fn identity(path: &Path, directory: bool) -> Result<Value, String> {
    let meta = fs::symlink_metadata(path).map_err(err)?;
    if meta.file_type().is_symlink()
        || (directory && !meta.is_dir())
        || (!directory && !meta.is_file())
    {
        return Err(format!(
            "Đường dẫn tạo máy không hợp lệ: {}",
            path.display()
        ));
    }
    Ok(json!({"dev":meta.dev(), "ino":meta.ino()}))
}
impl Backend {
    /// No writes, SDK execution, license acceptance or installation.
    pub fn creation_environment(&self) -> CreationEnvironment {
        self.creation_environment_for(AndroidVersion::default())
    }
    pub fn creation_environment_for(&self, version: AndroidVersion) -> CreationEnvironment {
        CreationEnvironment {
            components: self.ensure_creation_tools(),
            image: self.available_image_revision(version),
            available_bytes: available_at_destination(&self.avds()),
            minimum_bytes: crate::storage::CREATE_MINIMUM,
        }
    }

    /// Compatibility for existing callers; desktop forms use `create_with`.
    pub fn create(&self) -> Result<crate::Snapshot, String> {
        let outcome = self.create_requested(None, None).map_err(|e| e.message)?;
        if let Some(warning) = outcome.warning {
            return Err(format!(
                "Đã tạo máy {} ({}). {warning}. Không tạo lại; làm mới hoặc mở lại Hub.",
                outcome.profile.name, outcome.profile.id
            ));
        }
        outcome.snapshot.map_err(|error| format!(
            "Đã tạo máy {} ({}), nhưng chưa đọc lại được trạng thái: {error}. Không cần tạo lại; hãy làm mới hoặc mở lại Hub.",
            outcome.profile.name, outcome.profile.id
        ))
    }

    pub fn create_with(&self, request: CreateRequest) -> Result<CreatedDevice, CreationFailure> {
        self.create_requested(Some(request), None)
    }

    /// A download-and-create transaction must use exactly the image it just verified.
    pub fn create_with_image(
        &self,
        request: CreateRequest,
        expected: ImageIdentity,
    ) -> Result<CreatedDevice, CreationFailure> {
        expected.validate()?;
        self.create_requested(Some(request), Some(expected))
    }

    fn create_requested(
        &self,
        request: Option<CreateRequest>,
        expected: Option<ImageIdentity>,
    ) -> Result<CreatedDevice, CreationFailure> {
        if let Some(request) = &request {
            request.validate()?; // Invalid input must not create even a lock or runtime directory.
        }
        let _guard = self.lock()?;
        let mut all = self.read_profiles()?;
        let request = request.unwrap_or_else(|| CreateRequest::suggested(&all));
        let mut p = allocate_profile(&request, &all)?;
        self.ensure_creation_tools()?;
        p.image = Some(self.installed_image_identity(request.android_version)?);
        if expected.is_some() && p.image != expected {
            return Err(
                "Image đã thay đổi sau khi tải. Kiểm tra lại trước khi tạo máy."
                    .to_owned()
                    .into(),
            );
        }
        self.ensure_new_avd(&p)?;
        self.ensure_stopped(&p)?;
        // avdmanager otherwise falls back to ANDROID_USER_HOME/avd.
        fs::create_dir_all(self.avds()).map_err(err)?;
        crate::storage::ensure_creation_space(&self.avds())?;
        fs::create_dir_all(self.runtime().join("emulator-user")).map_err(err)?;
        let mut cmd = Command::new(self.sdk.join("cmdline-tools/latest/bin/avdmanager"));
        cmd.env("JAVA_HOME", self.java_home()?)
            .env("ANDROID_HOME", &self.sdk)
            .env("ANDROID_SDK_ROOT", &self.sdk)
            .env("ANDROID_AVD_HOME", self.avds())
            .env("ANDROID_USER_HOME", self.runtime().join("emulator-user"))
            .args(["create", "avd", "--name", &p.id, "--path"])
            .arg(self.data(&p))
            .args([
                "--package",
                &request.android_version.package(),
                "--device",
                "pixel_5",
            ]);
        let mut journal = self
            .begin_creation(&p, &all)
            .map_err(|error| CreationFailure {
                message: self.creation_failure(&p, &error),
                retained_profile: Some(Box::new(p.clone())),
                outcome_unknown: false,
            })?;
        let prepare = (|| -> Result<(), String> {
            self.command(cmd, 120)?;
            self.configure(&p)?;
            self.verify_profile_image(&p)?;
            self.prepare_creation(&p, &mut journal)?;
            all.push(p.clone());
            self.write_profiles(&all)
        })();
        if let Err(error) = prepare {
            return Err(CreationFailure {
                message: self.creation_failure(&p, &error),
                retained_profile: Some(Box::new(p)),
                outcome_unknown: false,
            });
        }
        // The registry now owns this ID, even if receipt publication or ps fails.
        Ok(self.registered_creation(p))
    }

    fn registered_creation(&self, profile: Profile) -> CreatedDevice {
        let warning = self
            .finish_creation(&profile, "registered")
            .err()
            .map(|error| {
                format!("Chưa hoàn tất biên nhận tạo máy: {error}. Journal được giữ để recovery")
            });
        CreatedDevice {
            profile,
            snapshot: self.snapshot(),
            warning,
        }
    }

    /// Read retained receipts, without validating or touching mutable guest data.
    pub fn creation_history(&self) -> Result<Vec<String>, String> {
        let root = self.runtime().join("creation-history");
        match fs::symlink_metadata(&root) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(err(e)),
            Ok(_) => {
                identity(&root, true)?;
            }
        }
        let mut events = Vec::new();
        let mut skipped = 0;
        for entry in fs::read_dir(&root).map_err(err)? {
            let path = entry.map_err(err)?.path();
            let read = (|| -> Result<_, String> {
                identity(&path, false)?;
                let meta = fs::metadata(&path).map_err(err)?;
                if meta.len() > 1024 * 1024 {
                    return Err("Biên nhận quá lớn".into());
                }
                let value: Value =
                    serde_json::from_slice(&fs::read(&path).map_err(err)?).map_err(err)?;
                let profile: Profile =
                    serde_json::from_value(value["profile"].clone()).map_err(err)?;
                profile.validate()?;
                let mut before: Vec<Profile> =
                    serde_json::from_value(value["before"].clone()).map_err(err)?;
                validate_registry(&before)?;
                before.push(profile.clone());
                validate_registry(&before)?;
                let (suffix, action) = match value["state"].as_str() {
                    Some("preparing") => (
                        "incomplete",
                        "tạo bị gián đoạn; file tạo dở được giữ, chưa đăng ký máy",
                    ),
                    Some("ready") if value["fingerprint"].is_object() => {
                        ("registered", "đã hoàn tất đăng ký máy")
                    }
                    _ => return Err("Trạng thái biên nhận không hợp lệ".into()),
                };
                if suffix == "registered" {
                    let fingerprint = &value["fingerprint"];
                    for key in ["data", "ini", "config"] {
                        if fingerprint[key]["dev"].as_u64().is_none()
                            || fingerprint[key]["ino"].as_u64().is_none()
                        {
                            return Err("Fingerprint metadata không hợp lệ".into());
                        }
                    }
                    for key in ["ini_sha256", "config_sha256"] {
                        if !fingerprint[key].as_str().is_some_and(|hash| {
                            hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit())
                        }) {
                            return Err("Fingerprint SHA-256 không hợp lệ".into());
                        }
                    }
                }
                if value["schema"] != 1
                    || path != root.join(format!("{}-{suffix}.json", profile.id))
                {
                    return Err("Tên/schema biên nhận không hợp lệ".into());
                }
                Ok((
                    meta.modified().map_err(err)?,
                    path.clone(),
                    format!("{} ({}): {action}", profile.name, profile.id),
                ))
            })();
            match read {
                Ok(event) => events.push(event),
                Err(_) => skipped += 1,
            }
        }
        events.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
        let mut result: Vec<_> = events.into_iter().take(3).map(|e| e.2).collect();
        if skipped > 0 {
            result.push(format!(
                "{skipped} biên nhận tạo máy không đọc được; lịch sử có thể chưa đầy đủ."
            ));
        }
        Ok(result)
    }
    fn creation_path(&self, profile: &Profile) -> PathBuf {
        self.runtime()
            .join(format!("create-pending-{}.json", profile.id))
    }
    pub(super) fn begin_creation(
        &self,
        profile: &Profile,
        before: &[Profile],
    ) -> Result<Value, String> {
        let path = self.creation_path(profile);
        if fs::symlink_metadata(&path).is_ok() {
            return Err("Journal tạo máy đã tồn tại".into());
        }
        let journal = json!({"schema":1,"state":"preparing","profile":profile,"before":before});
        write_json(&path, &journal)?;
        Ok(journal)
    }
    fn creation_fingerprint(&self, profile: &Profile) -> Result<Value, String> {
        let data = self.data(profile);
        let ini = self.avds().join(format!("{}.ini", profile.id));
        let config = data.join("config.ini");
        let dir = identity(&data, true)?;
        let ini_id = identity(&ini, false)?;
        let config_id = identity(&config, false)?;
        Ok(json!({"data":dir,"ini":ini_id,"config":config_id,
            "ini_sha256":self.file_digest(&ini)?,"config_sha256":self.file_digest(&config)?}))
    }
    pub(super) fn prepare_creation(
        &self,
        profile: &Profile,
        journal: &mut Value,
    ) -> Result<(), String> {
        journal["fingerprint"] = self.creation_fingerprint(profile)?;
        // Sync small metadata before publishing readiness. Guest payload durability
        // after host power loss is not asserted by this process-crash protocol.
        fs::File::open(self.data(profile).join("config.ini"))
            .map_err(err)?
            .sync_all()
            .map_err(err)?;
        fs::File::open(self.avds().join(format!("{}.ini", profile.id)))
            .map_err(err)?
            .sync_all()
            .map_err(err)?;
        sync_dir(&self.data(profile))?;
        sync_dir(&self.avds())?;
        journal["state"] = json!("ready");
        write_json(&self.creation_path(profile), journal)
    }
    pub(super) fn finish_creation(&self, profile: &Profile, outcome: &str) -> Result<(), String> {
        let history = self.runtime().join("creation-history");
        match fs::DirBuilder::new().mode(0o700).create(&history) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(err(error)),
        }
        identity(&history, true)?;
        let target = history.join(format!("{}-{outcome}.json", profile.id));
        if fs::symlink_metadata(&target).is_ok() {
            return Err("Biên nhận tạo máy đã tồn tại; giữ journal để kiểm tra".into());
        }
        fs::rename(self.creation_path(profile), target).map_err(err)?;
        sync_dir(&history)?;
        sync_dir(&self.runtime())
    }
    // The caller owns the Hub lock. Preparing intents only retain a receipt;
    // their external avdmanager may still be alive, so never move its files.
    pub(super) fn recover_creations_locked(&self) -> Result<(), String> {
        let mut paths = Vec::new();
        for entry in fs::read_dir(self.runtime()).map_err(err)? {
            let path = entry.map_err(err)?.path();
            let name = path.file_name().unwrap().to_string_lossy();
            if name.starts_with("create-pending-") && name.ends_with(".json") {
                paths.push(path);
            }
        }
        paths.sort();
        for path in paths {
            self.recover_creation(&path, true)?;
        }
        Ok(())
    }
    fn recover_creation(&self, path: &Path, check_process: bool) -> Result<(), String> {
        identity(path, false)?;
        if fs::metadata(path).map_err(err)?.len() > 1024 * 1024 {
            return Err("Journal tạo máy quá lớn".into());
        }
        let journal: Value = serde_json::from_slice(&fs::read(path).map_err(err)?).map_err(err)?;
        let profile: Profile = serde_json::from_value(journal["profile"].clone()).map_err(err)?;
        profile.validate()?;
        if journal["schema"] != 1 || path != self.creation_path(&profile) {
            return Err("Journal tạo máy không hợp lệ".into());
        }
        let before: Vec<Profile> =
            serde_json::from_value(journal["before"].clone()).map_err(err)?;
        validate_registry(&before)?;
        let mut after = before.clone();
        after.push(profile.clone());
        validate_registry(&after)?;
        let current = self.read_profiles()?;
        match journal["state"].as_str() {
            Some("preparing") if current == before => self.finish_creation(&profile, "incomplete"),
            Some("ready") if current == before || current == after => {
                if check_process {
                    // Include the not-yet-registered profile in process identity checks.
                    let mut cmd = Command::new("/bin/ps");
                    cmd.args(["-axo", "pid=,%cpu=,rss=,command="]);
                    if !crate::parse_processes(
                        &self.command(cmd, 5)?,
                        std::slice::from_ref(&profile),
                    )?
                    .is_empty()
                    {
                        return Err("Máy tạo dở đang chạy; dừng máy trước khi recovery".into());
                    }
                    self.ensure_stopped(&profile)?;
                }
                if journal["fingerprint"] != self.creation_fingerprint(&profile)? {
                    return Err("Dữ liệu máy tạo dở đã thay đổi; giữ nguyên để kiểm tra".into());
                }
                if current == before {
                    self.write_profiles(&after)?;
                }
                self.finish_creation(&profile, "registered")
            }
            _ => Err("Registry hoặc journal tạo máy đã thay đổi; không tự ghi đè".into()),
        }
    }
}

fn allocate_profile(request: &CreateRequest, all: &[Profile]) -> Result<Profile, String> {
    request.validate()?;
    if all
        .iter()
        .any(|p| p.name.trim().eq_ignore_ascii_case(request.name.trim()))
    {
        return Err("Tên máy đã có trong danh sách. Chọn tên khác trước khi tạo.".into());
    }
    let port = (5584..=5680)
        .step_by(2)
        .find(|port| !all.iter().any(|p| p.port == *port))
        .ok_or("Đã hết port dành cho Hub")?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(err)?
        .as_millis();
    let sequence = crate::SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let profile = request.profile(format!("hub-{stamp}-{sequence}"), port)?;
    if all.iter().any(|p| p.id == profile.id) {
        return Err("ID máy đã tồn tại; không ghi đè".into());
    }
    Ok(profile)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn environment_observation_and_invalid_request_do_not_create_runtime() {
        let root = std::env::temp_dir().join(format!(
            "hub-create-no-write-{}-{}",
            std::process::id(),
            crate::SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        assert!(!root.exists());
        let b = Backend {
            sdk: root.join("missing-sdk"),
            root,
        };
        let environment = b.creation_environment();
        assert!(environment.components.is_err());
        assert!(environment.available_bytes.is_ok());
        assert_eq!(environment.minimum_bytes, 12 * 1024 * 1024 * 1024);
        assert!(!environment.ready());
        let mut request = CreateRequest::suggested(&[]);
        request.name = "\n".into();
        let error = b.create_with(request).unwrap_err();
        assert!(error.retained_profile.is_none());
        assert!(!b.root.exists());
    }

    #[test]
    fn allocation_owns_identity_and_rechecks_name_and_port_exhaustion() {
        let mut request = CreateRequest::suggested(&[]);
        request.name = "My game".into();
        request.ram_mb = 4096;
        let existing = Profile::standard("hub-existing".into(), "Other".into(), 5584);
        let created = allocate_profile(&request, std::slice::from_ref(&existing)).unwrap();
        assert_eq!(created.port, 5586);
        assert_eq!(created.ram_mb, 4096);
        assert!(created.id.starts_with("hub-"));
        assert_ne!(created.id, existing.id);
        let another = allocate_profile(&request, &[]).unwrap();
        assert_ne!(created.id, another.id);
        request.name = " other ".into();
        assert!(
            allocate_profile(&request, &[existing])
                .unwrap_err()
                .contains("Tên máy")
        );
        request.name = "Unique".into();
        let full: Vec<_> = (5584..=5680)
            .step_by(2)
            .map(|port| Profile::standard(format!("hub-{port}"), format!("Machine {port}"), port))
            .collect();
        assert!(
            allocate_profile(&request, &full)
                .unwrap_err()
                .contains("port")
        );
    }

    #[test]
    fn creation_rechecks_downloaded_identity_before_any_avd_or_journal() {
        use std::os::unix::fs::PermissionsExt;
        let fixture = crate::images::tests::Fixture::new();
        let b = &fixture.0;
        let version = AndroidVersion::Android13;
        fixture.image(version);
        let java = b.root.join("java");
        fs::create_dir_all(java.join("bin")).unwrap();
        fs::write(java.join("bin/java"), b"fixture").unwrap();
        fs::set_permissions(java.join("bin/java"), fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(
            b.root.join("hub-settings.json"),
            serde_json::to_vec(&json!({"schema":1,"sdk":b.sdk,"java_home":java})).unwrap(),
        )
        .unwrap();
        let expected = b.installed_image_identity(version).unwrap();
        let mut request = CreateRequest::suggested(&[]);
        request.android_version = version;
        assert!(b.creation_environment_for(version).ready());
        assert!(
            b.creation_environment_for(AndroidVersion::Android12)
                .downloadable()
        );
        fs::write(
            b.sdk.join(version.directory()).join("system.img"),
            b"changed image",
        )
        .unwrap();
        let result = b.create_with_image(request, expected).unwrap_err();
        assert!(
            result.message.contains("Image đã thay đổi"),
            "{}",
            result.message
        );
        assert!(result.retained_profile.is_none() && !result.outcome_unknown);
        assert!(b.read_profiles().unwrap().is_empty() && !b.avds().exists());
        assert!(!fs::read_dir(b.runtime()).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("create-pending-")
        }));
    }

    #[test]
    fn registered_result_survives_receipt_and_snapshot_failures() {
        let (b, p) = fixture();
        let mut journal = b.begin_creation(&p, &[]).unwrap();
        data(&b, &p);
        b.prepare_creation(&p, &mut journal).unwrap();
        b.write_profiles(std::slice::from_ref(&p)).unwrap();
        // External interference after publication: keep the committed registry
        // aside and block receipt publication. Neither error may discard the ID.
        fs::rename(b.registry(), b.root.join("committed-registry.json")).unwrap();
        fs::write(b.runtime().join("creation-history"), "blocked").unwrap();
        let outcome = b.registered_creation(p.clone());
        assert_eq!(outcome.profile, p);
        assert!(outcome.snapshot.is_err());
        assert!(outcome.warning.unwrap().contains("Journal"));
        assert!(b.creation_path(&p).is_file());
        assert!(b.data(&p).join("config.ini").is_file());
        let committed: Value =
            serde_json::from_slice(&fs::read(b.root.join("committed-registry.json")).unwrap())
                .unwrap();
        assert_eq!(committed["schema"], 1);
        let committed: Vec<Profile> =
            serde_json::from_value(committed["profiles"].clone()).unwrap();
        assert_eq!(committed, vec![p]);
        fs::remove_dir_all(b.root).unwrap();
    }

    #[test]
    fn custom_profile_survives_interrupted_ready_registration() {
        let (b, _) = fixture();
        let mut request = CreateRequest::suggested(&[]);
        request.name = "Landscape game".into();
        request.ram_mb = 4096;
        request.cpus = 4;
        request.width = 1280;
        request.height = 720;
        request.fps = 60;
        let p = allocate_profile(&request, &[]).unwrap();
        let mut journal = b.begin_creation(&p, &[]).unwrap();
        data(&b, &p);
        b.configure(&p).unwrap();
        let config = fs::read_to_string(b.data(&p).join("config.ini")).unwrap();
        for expected in [
            "hw.ramSize=4096",
            "hw.cpu.ncore=4",
            "hw.lcd.width=1280",
            "hw.lcd.height=720",
        ] {
            assert!(config.contains(expected));
        }
        b.prepare_creation(&p, &mut journal).unwrap();
        b.recover_creation(&b.creation_path(&p), false).unwrap();
        assert_eq!(b.read_profiles().unwrap(), vec![p.clone()]);
        b.recover_creations_locked().unwrap();
        assert_eq!(b.read_profiles().unwrap(), vec![p]);
        fs::remove_dir_all(b.root).unwrap();
    }

    fn fixture() -> (Backend, Profile) {
        let root = std::env::temp_dir().join(format!(
            "hub-create-recovery-{}-{}",
            std::process::id(),
            crate::SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let backend = Backend {
            sdk: root.join("sdk"),
            root,
        };
        fs::create_dir_all(backend.avds()).unwrap();
        backend.write_profiles(&[]).unwrap();
        let profile = Profile::standard("hub-fixture".into(), "Fixture".into(), 5680);
        (backend, profile)
    }
    fn data(backend: &Backend, profile: &Profile) {
        fs::create_dir(backend.data(profile)).unwrap();
        fs::write(
            backend.data(profile).join("config.ini"),
            "hw.ramSize=1024\n",
        )
        .unwrap();
        fs::write(
            backend.avds().join(format!("{}.ini", profile.id)),
            "path=fixture\n",
        )
        .unwrap();
    }
    // A separate test process holds the actual filesystem lock and is killed
    // by its parent after publishing a checkpoint. No production fault hook.
    #[test]
    #[ignore = "worker for creation_process_kill_recovery only"]
    fn creation_crash_worker() {
        let Some(root) = std::env::var_os("HUB_CREATION_CRASH_ROOT") else {
            return;
        };
        let root = PathBuf::from(root);
        let stage = std::env::var("HUB_CREATION_CRASH_STAGE").unwrap();
        let b = Backend {
            sdk: root.join("sdk"),
            root,
        };
        let p = Profile::standard("hub-fixture".into(), "Fixture".into(), 5680);
        let _guard = b.lock_raw().unwrap();
        let mut journal = b.begin_creation(&p, &[]).unwrap();
        if stage != "intent" {
            data(&b, &p);
        }
        if stage == "ready_temp" {
            // Interrupted atomic publication must not be interpreted as ready.
            fs::write(
                b.creation_path(&p).with_extension("json.new"),
                br#"{"state":"ready","fingerprint":"#,
            )
            .unwrap();
        }
        if ["ready", "registry", "archived"].contains(&stage.as_str()) {
            b.prepare_creation(&p, &mut journal).unwrap();
        }
        if ["registry", "archived"].contains(&stage.as_str()) {
            b.write_profiles(std::slice::from_ref(&p)).unwrap();
        }
        if stage == "archived" {
            b.finish_creation(&p, "registered").unwrap();
        }
        fs::write(b.root.join("checkpoint"), stage).unwrap();
        loop {
            std::thread::park();
        }
    }

    #[test]
    #[ignore = "kills an owned fixture child; requires process inspection and loopback ports"]
    fn creation_process_kill_recovery() {
        use std::{
            os::unix::process::ExitStatusExt,
            process::Stdio,
            time::{Duration, Instant},
        };
        struct Worker(std::process::Child);
        impl Drop for Worker {
            fn drop(&mut self) {
                // A failed assertion must not leave a parked child behind.
                if !matches!(self.0.try_wait(), Ok(Some(_))) {
                    let _ = self.0.kill();
                    let _ = self.0.wait();
                }
            }
        }
        for stage in [
            "intent",
            "partial",
            "ready_temp",
            "ready",
            "registry",
            "archived",
        ] {
            let (b, p) = fixture();
            let child = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "creation::tests::creation_crash_worker",
                    "--ignored",
                    "--nocapture",
                ])
                .env("HUB_CREATION_CRASH_ROOT", &b.root)
                .env("HUB_CREATION_CRASH_STAGE", stage)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap();
            let mut child = Worker(child);
            let deadline = Instant::now() + Duration::from_secs(15);
            while !b.root.join("checkpoint").exists() {
                if let Some(status) = child.0.try_wait().unwrap() {
                    panic!("Worker exited before {stage}: {status}");
                }
                if Instant::now() >= deadline {
                    child.0.kill().unwrap();
                    child.0.wait().unwrap();
                    panic!("Worker did not reach {stage}");
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            assert_eq!(
                fs::read_to_string(b.root.join("checkpoint")).unwrap(),
                stage
            );
            // Verify we really interrupt an active lock holder, not a finished worker.
            assert!(b.lock_raw().is_err());
            child.0.kill().unwrap();
            assert_eq!(child.0.wait().unwrap().signal(), Some(9));
            let expected = if ["ready", "registry", "archived"].contains(&stage) {
                vec![p.clone()]
            } else {
                vec![]
            };
            assert_eq!(b.initialize().unwrap().profiles, expected, "{stage}");
            assert_eq!(b.initialize().unwrap().profiles, expected, "repeat {stage}");
            assert!(!b.creation_path(&p).exists());
            let outcome = if expected.is_empty() {
                "incomplete"
            } else {
                "registered"
            };
            assert!(
                b.runtime()
                    .join(format!("creation-history/{}-{outcome}.json", p.id))
                    .is_file()
            );
            if stage != "intent" {
                assert_eq!(
                    fs::read_to_string(b.data(&p).join("config.ini")).unwrap(),
                    "hw.ramSize=1024\n"
                );
            }
            if stage == "ready_temp" {
                assert!(b.creation_path(&p).with_extension("json.new").is_file());
            }
            println!("SIGKILL {stage}: recovery and second initialize passed; payload retained");
            fs::remove_dir_all(b.root).unwrap();
        }
    }

    #[test]
    #[ignore = "requires host process inspection and loopback ports"]
    fn startup_creation_recovery_waits_for_busy_port() {
        let (b, p) = fixture();
        let mut journal = b.begin_creation(&p, &[]).unwrap();
        data(&b, &p);
        b.prepare_creation(&p, &mut journal).unwrap();
        let listener = std::net::TcpListener::bind(("127.0.0.1", p.port)).unwrap();
        assert!(b.initialize().unwrap_err().contains("Port"));
        assert!(b.read_profiles().unwrap().is_empty());
        assert!(b.creation_path(&p).exists());
        drop(listener);
        assert_eq!(b.initialize().unwrap().profiles, vec![p]);
        assert_eq!(b.initialize().unwrap().profiles.len(), 1);
        fs::remove_dir_all(b.root).unwrap();
    }
    #[test]
    fn creation_history_validates_receipt_and_survives_guest_changes() {
        let (b, p) = fixture();
        assert!(b.creation_history().unwrap().is_empty());
        let mut journal = b.begin_creation(&p, &[]).unwrap();
        data(&b, &p);
        b.prepare_creation(&p, &mut journal).unwrap();
        b.finish_creation(&p, "registered").unwrap();
        fs::write(b.data(&p).join("config.ini"), "normal later edit").unwrap();
        let history = b.creation_history().unwrap();
        assert_eq!(history.len(), 1);
        assert!(history[0].contains("đã hoàn tất đăng ký"));
        let receipt = b
            .runtime()
            .join("creation-history/hub-fixture-registered.json");
        journal["state"] = json!("preparing");
        fs::write(&receipt, serde_json::to_vec(&journal).unwrap()).unwrap();
        assert!(b.creation_history().unwrap()[0].contains("không đọc được"));
        fs::rename(
            &receipt,
            b.runtime()
                .join("creation-history/hub-fixture-incomplete.json"),
        )
        .unwrap();
        assert!(b.creation_history().unwrap()[0].contains("chưa đăng ký"));
        std::os::unix::fs::symlink(b.registry(), &receipt).unwrap();
        assert_eq!(b.creation_history().unwrap().len(), 2);
        fs::remove_dir_all(b.root).unwrap();
    }
    #[test]
    fn history_symlink_cannot_redirect_journal() {
        let (b, p) = fixture();
        b.begin_creation(&p, &[]).unwrap();
        let outside = b.root.join("outside");
        fs::create_dir(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, b.runtime().join("creation-history")).unwrap();
        assert!(b.recover_creation(&b.creation_path(&p), false).is_err());
        assert!(b.creation_path(&p).exists());
        assert_eq!(fs::read_dir(outside).unwrap().count(), 0);
        fs::remove_dir_all(b.root).unwrap();
    }
    #[test]
    fn interrupted_preparation_preserves_partial_data_without_registering_it() {
        let (b, p) = fixture();
        b.begin_creation(&p, &[]).unwrap();
        data(&b, &p);
        b.recover_creation(&b.creation_path(&p), false).unwrap();
        assert!(b.read_profiles().unwrap().is_empty());
        assert!(b.data(&p).join("config.ini").exists());
        assert!(
            b.runtime()
                .join("creation-history/hub-fixture-incomplete.json")
                .exists()
        );
        fs::remove_dir_all(b.root).unwrap();
    }
    #[test]
    fn ready_and_registry_commit_boundaries_recover_idempotently() {
        for committed in [false, true] {
            let (b, p) = fixture();
            let mut j = b.begin_creation(&p, &[]).unwrap();
            data(&b, &p);
            b.prepare_creation(&p, &mut j).unwrap();
            if committed {
                b.write_profiles(std::slice::from_ref(&p)).unwrap();
            }
            b.recover_creation(&b.creation_path(&p), false).unwrap();
            assert_eq!(b.read_profiles().unwrap(), vec![p]);
            b.recover_creations_locked().unwrap();
            fs::remove_dir_all(b.root).unwrap();
        }
    }
    #[test]
    fn modified_data_or_registry_blocks_registration_and_keeps_journal() {
        for change_registry in [false, true] {
            let (b, p) = fixture();
            let mut j = b.begin_creation(&p, &[]).unwrap();
            data(&b, &p);
            b.prepare_creation(&p, &mut j).unwrap();
            if change_registry {
                b.write_profiles(&[Profile::standard("hub-other".into(), "Other".into(), 5584)])
                    .unwrap();
            } else {
                fs::write(b.data(&p).join("config.ini"), "changed").unwrap();
            }
            assert!(b.recover_creation(&b.creation_path(&p), false).is_err());
            assert!(b.creation_path(&p).exists());
            assert!(b.data(&p).exists());
            fs::remove_dir_all(b.root).unwrap();
        }
    }
}
