//! Android engine, SDK and persisted-device operations behind a stable facade.
mod device;
pub mod physical;
pub use device::DeviceBackend;
mod sdk;
pub use sdk::{
    ImageInstallResult, ImageInstallSpace, ManagedSdk, ManagedSdkCatalog, ManagedSdkEntry,
    SdkArchive, SdkDownloadControl, SdkExtraction, SdkExtractionControl, SdkInstallControl,
    SdkInstallPackage, SdkLicense, SdkPackage, SdkPackageSummary, SdkPlan, SdkSetupControl,
    SdkSetupStage, SdkToolProbe, VerifiedSdkPackage, verify_sdk_package, verify_sdk_package_set,
};
mod apk;
mod backup;
mod catalog;
mod connectivity;
mod creation;
mod file_lock;
mod images;
mod registry;
pub use connectivity::{CONNECTIVITY_ENDPOINT, ConnectivityReport, ProbeResult};
pub use creation::{CreatedDevice, CreationEnvironment, CreationFailure};
pub use images::{
    AndroidImage, ImageAvailability, ImageRemovalPlan, ImageUsage, android_image_catalog,
};
mod lifecycle;
mod logs;
pub use lifecycle::BootObservation;
#[cfg(test)]
mod metrics_smoke;
pub use catalog::{BackupCatalog, BackupInfo, RestoreReceipt};
mod removal;
pub use removal::{DeletePlan, PurgePlan, RemovalOutcome, RemovedCatalog, RemovedDevice};
mod restore;
mod scroll;
mod shutdown;
mod snapshot;
mod storage;
#[cfg(test)]
mod tests;
use shutdown::stop_registered_progress;
pub use shutdown::{ShutdownProgress, ShutdownStatus};
pub use snapshot::Snapshot;
use snapshot::{android_image_version, parse_processes};
mod uninstall;
mod usage;
pub use restore::RestoreOutcome;
pub use uninstall::{UninstallPlan, run_uninstall_helper};
pub use usage::{DiskUsage, UsageCategory};
pub mod display;
use android_hub_domain::Profile;
use file_lock::FileLock;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::Read,
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
#[derive(Clone, Debug)]
pub struct Backend {
    /// Directory containing registry, preferences, SDK state and device data.
    pub root: PathBuf,
    pub sdk: PathBuf,
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
impl Backend {
    pub fn new(root: PathBuf) -> Result<Self, String> {
        let mut backend = Self::environment_sdk(root)?;
        if std::env::var_os("ANDROID_SDK_ROOT").is_none()
            && let Some(sdk) = backend.saved_sdk()?
        {
            backend.sdk = sdk;
        }
        Ok(backend)
    }
    pub fn environment_sdk(root: PathBuf) -> Result<Self, String> {
        let home = std::env::var_os("HOME").ok_or("HOME không có")?;
        let sdk = std::env::var_os("ANDROID_SDK_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(home).join("Library/Android/sdk"));
        Ok(Self { root, sdk })
    }
    fn runtime(&self) -> PathBuf {
        self.root.to_path_buf()
    }
    fn avds(&self) -> PathBuf {
        self.runtime().join("emulator-avds")
    }
    fn data(&self, p: &Profile) -> PathBuf {
        self.avds().join(format!("{}.avd", p.id))
    }
    fn registry(&self) -> PathBuf {
        self.runtime().join("hub-profiles.json")
    }
    fn lock(&self) -> Result<FileLock, String> {
        let guard = self.lock_raw()?;
        self.recover_removals_locked()?;
        self.recover_restores_locked()?;
        self.recover_creations_locked()?;
        Ok(guard)
    }
    fn lock_raw(&self) -> Result<FileLock, String> {
        if self
            .root
            .join(uninstall::PENDING)
            .try_exists()
            .map_err(err)?
        {
            return Err(
                "Hub đang gỡ cài đặt hoặc lần gỡ trước bị gián đoạn; không tự tiếp tục xóa".into(),
            );
        }
        fs::create_dir_all(self.runtime()).map_err(err)?;
        let f = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.runtime().join("hub.lock"))
            .map_err(err)?;
        FileLock::acquire(f, "Một thao tác Hub khác đang thực hiện")
    }
    fn command(&self, mut cmd: Command, timeout: u64) -> Result<String, String> {
        let seq = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = self
            .runtime()
            .join(format!("command-{}-{seq}.tmp", std::process::id()));
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(err)?;
        cmd.env("LC_ALL", "C")
            .stdin(Stdio::null())
            .stdout(file.try_clone().map_err(err)?)
            .stderr(file);
        let result = (|| {
            let mut child = cmd.spawn().map_err(err)?;
            let start = Instant::now();
            let status = loop {
                if let Some(s) = child.try_wait().map_err(err)? {
                    break s;
                }
                if start.elapsed() > Duration::from_secs(timeout) {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "Lệnh quá thời gian {timeout}s; kiểm tra log trước khi thử lại"
                    ));
                }
                thread::sleep(Duration::from_millis(30));
            };
            let mut text = String::new();
            File::open(&path)
                .map_err(err)?
                .take(2 * 1024 * 1024)
                .read_to_string(&mut text)
                .map_err(err)?;
            if !status.success() {
                return Err(format!(
                    "Lệnh thất bại: {}",
                    text.chars().take(1600).collect::<String>()
                ));
            }
            Ok(text)
        })();
        let _ = fs::remove_file(path);
        result
    }
    fn initialize_registry(&self) -> Result<(), String> {
        let _guard = self.lock()?;
        if !self.registry().exists() {
            self.write_profiles(&[])?;
        }
        self.migrate_profile_images()?;
        Ok(())
    }
    pub fn initialize(&self) -> Result<Snapshot, String> {
        self.initialize_registry()?;
        self.snapshot()
    }
    pub fn snapshot(&self) -> Result<Snapshot, String> {
        let profiles = self.read_profiles()?;
        let mut cmd = Command::new("/bin/ps");
        cmd.args(["-axo", "pid=,%cpu=,rss=,command="]);
        let output = self.command(cmd, 5)?;
        let runtime = parse_processes(&output, &profiles)?;
        let android_versions = profiles
            .iter()
            .filter_map(|profile| {
                if let Some(image) = &profile.image {
                    return Some((
                        profile.id.clone(),
                        format!("{} · API {}", image.version, image.version.api()),
                    ));
                }
                let config = std::fs::read_to_string(self.data(profile).join("config.ini")).ok()?;
                Some((profile.id.clone(), android_image_version(&config)?))
            })
            .collect();
        Ok(Snapshot {
            profiles,
            runtime,
            android_versions,
        })
    }
    fn ensure_stopped(&self, p: &Profile) -> Result<(), String> {
        if self
            .snapshot()?
            .runtime
            .get(&p.id)
            .is_some_and(|r| r.pid.is_some())
        {
            return Err("Máy đang chạy. Hãy dừng trước khi thay đổi.".into());
        }
        for port in [p.port, p.port + 1, p.port + 3000] {
            TcpListener::bind(("127.0.0.1", port)).map_err(|_| {
                format!("Port {port} đang được sử dụng; không thao tác để tránh nhầm máy")
            })?;
        }
        Ok(())
    }
    fn existing(&self, id: &str) -> Result<Profile, String> {
        self.read_profiles()?
            .into_iter()
            .find(|p| p.id == id)
            .ok_or("Không tìm thấy máy trong registry".into())
    }
    fn configure(&self, p: &Profile) -> Result<(), String> {
        p.validate()?;
        let path = self.data(p).join("config.ini");
        // Reading legacy files on macOS can block inside open (e.g. file-provider
        // mediation). Isolate it in a bounded process so the Hub stays usable.
        let mut read = Command::new("/bin/cat");
        read.arg(&path);
        let original = self.command(read, 5)?;
        let mut values: BTreeMap<String, String> = original
            .lines()
            .filter_map(|l| l.split_once('='))
            .map(|(k, v)| (k.trim().into(), v.trim().into()))
            .collect();
        for (k, v) in [
            ("hw.cpu.ncore", p.cpus.to_string()),
            ("hw.ramSize", p.ram_mb.to_string()),
            ("hw.lcd.width", p.width.to_string()),
            ("hw.lcd.height", p.height.to_string()),
            ("hw.lcd.density", "320".into()),
            ("hw.keyboard", "yes".into()),
            ("hw.gpu.enabled", "yes".into()),
            ("hw.gpu.mode", "host".into()),
            ("hw.audioInput", "no".into()),
            ("hw.camera.back", "none".into()),
            ("hw.camera.front", "none".into()),
            ("PlayStore.enabled", "true".into()),
            ("showDeviceFrame", "no".into()),
        ] {
            values.insert(k.into(), v);
        }
        let new = path.with_extension("ini.new");
        fs::write(
            &new,
            values
                .into_iter()
                .map(|(k, v)| format!("{k}={v}\n"))
                .collect::<String>(),
        )
        .map_err(err)?;
        fs::rename(new, path).map_err(err)
    }
    #[cfg(test)]
    pub(crate) fn save(&self, profile: Profile) -> Result<Snapshot, String> {
        self.save_checked(None, profile)
    }
    pub(crate) fn save_if_unchanged(
        &self,
        expected: &Profile,
        profile: Profile,
    ) -> Result<Snapshot, String> {
        self.save_checked(Some(expected), profile)
    }
    fn save_checked(
        &self,
        expected: Option<&Profile>,
        profile: Profile,
    ) -> Result<Snapshot, String> {
        let _guard = self.lock()?;
        profile.validate()?;
        let old = self.existing(&profile.id)?;
        if expected.is_some_and(|expected| expected != &old) {
            return Err("Cấu hình đã thay đổi bên ngoài. Nạp lại cấu hình trước khi lưu.".into());
        }
        if old.port != profile.port {
            return Err("Không được đổi port".into());
        }
        if old.image != profile.image {
            return Err("Không được đổi image Android của máy đã tạo; hãy tạo máy mới".into());
        }
        self.ensure_stopped(&old)?;
        // Registry is the desired configuration; start reapplies it before launch.
        let mut all = self.read_profiles()?;
        let id = profile.id.clone();
        *all.iter_mut()
            .find(|p| p.id == id)
            .ok_or("Máy không tồn tại")? = profile;
        self.write_profiles(&all)?;
        self.snapshot()
    }
    fn ensure_new_avd(&self, profile: &Profile) -> Result<(), String> {
        for path in [
            self.data(profile),
            self.avds().join(format!("{}.ini", profile.id)),
        ] {
            match fs::symlink_metadata(&path) {
                Ok(_) => {
                    return Err(format!(
                        "Đường dẫn máy đã tồn tại; không ghi đè: {}",
                        path.display()
                    ));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(format!(
                        "Không kiểm tra được đường dẫn {}: {error}",
                        path.display()
                    ));
                }
            }
        }
        Ok(())
    }
    fn creation_failure(&self, profile: &Profile, error: &str) -> String {
        format!(
            "Chưa hoàn tất tạo máy {} ({}): {error}\nNếu công cụ đã tạo file dở, chúng được giữ tại {} và {}. Journal sẽ được kiểm tra khi mở lại Hub hoặc thao tác tiếp theo; không tự xóa hoặc ghi đè các file này.",
            profile.name,
            profile.id,
            self.data(profile).display(),
            self.avds().join(format!("{}.ini", profile.id)).display()
        )
    }
    fn launcher(&self, p: &Profile) -> Result<PathBuf, String> {
        // Separate helper identity avoids cached foreground metadata from legacy launchers.
        let app = self
            .runtime()
            .join("launchers")
            .join(format!("Android-{}.app", p.port));
        let contents = app.join("Contents");
        fs::create_dir_all(contents.join("MacOS")).map_err(err)?;
        let info = format!(
            "<?xml version=\"1.0\"?><plist version=\"1.0\"><dict><key>CFBundleIdentifier</key><string>local.androidhub.engine.{}</string><key>CFBundleName</key><string>Android {}</string><key>CFBundleExecutable</key><string>engine</string><key>CFBundlePackageType</key><string>APPL</string><key>NSHighResolutionCapable</key><true/><key>LSUIElement</key><true/></dict></plist>",
            p.id, p.port
        );
        fs::write(contents.join("Info.plist"), info).map_err(err)?;
        let script = format!(
            "#!/bin/bash\nexport ANDROID_HOME={}\nexport ANDROID_SDK_ROOT={}\nexport ANDROID_AVD_HOME={}\nexport ANDROID_USER_HOME={}\nexec {} -avd {} -port {} -memory {} -cores {} -skin {}x{} -gpu host -no-window -grpc-use-token -grpc {} -no-snapshot -no-boot-anim -vsync-rate {} -camera-back none -camera-front none >> {} 2>&1\n",
            quote(&self.sdk.to_string_lossy()),
            quote(&self.sdk.to_string_lossy()),
            quote(&self.avds().to_string_lossy()),
            quote(&self.runtime().join("emulator-user").to_string_lossy()),
            quote(&self.sdk.join("emulator/emulator").to_string_lossy()),
            quote(&p.id),
            p.port,
            p.ram_mb,
            p.cpus,
            p.width,
            p.height,
            p.port + 3000,
            p.fps,
            quote(
                &self
                    .runtime()
                    .join(format!("{}.log", p.id))
                    .to_string_lossy()
            )
        );
        let executable = contents.join("MacOS/engine");
        fs::write(&executable, script).map_err(err)?;
        fs::set_permissions(executable, fs::Permissions::from_mode(0o755)).map_err(err)?;
        Ok(app)
    }
    pub(crate) fn start(&self, id: &str) -> Result<Snapshot, String> {
        let _guard = self.lock()?;
        let p = self.existing(id)?;
        self.ensure_stopped(&p)?;
        self.ensure_sdk(false)?;
        self.verify_profile_image(&p)?;
        self.rotate_engine_log(&p)?;
        // Apply runtime settings using supported emulator flags. Existing AVD
        // configuration need not be opened or rewritten by the desktop app.
        let app = self.launcher(&p)?;
        // Keep the engine in Hub's process lineage. LaunchServices gives each
        // helper a separate privacy identity, which can deny Desktop runtime I/O.
        let mut child = Command::new("/bin/bash")
            .arg(app.join("Contents/MacOS/engine"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(err)?;
        // Reap it without blocking the UI or tying its lifetime to this command.
        thread::spawn(move || {
            let _ = child.wait();
        });
        for _ in 0..30 {
            let state = self.snapshot()?;
            if state.runtime.get(id).is_some_and(|r| r.pid.is_some()) {
                return Ok(state);
            }
            thread::sleep(Duration::from_millis(500));
        }
        Err(format!(
            "Chưa thấy engine khởi động. Xem log {}.log trong runtime",
            p.id
        ))
    }
    /// Stop only engines registered to this data root using the normal identity checks.
    /// Continue after one failure so other guests do not remain running unnecessarily.
    pub fn stop_all(&self) -> Result<(), String> {
        self.stop_all_with_progress(|_| {})
    }
    pub fn stop_all_with_progress(
        &self,
        progress: impl FnMut(ShutdownProgress),
    ) -> Result<(), String> {
        let state = self.snapshot()?;
        let errors = stop_registered_progress(&state, |id| self.stop(id).map(|_| ()), progress);
        let remaining = self.snapshot()?;
        if remaining.runtime.is_empty() {
            return Ok(());
        }
        let names = remaining
            .profiles
            .iter()
            .filter(|p| remaining.runtime.contains_key(&p.id))
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        Err(format!(
            "Android still running: {names}\n{}",
            errors.join("\n")
        ))
    }
    pub(crate) fn stop(&self, id: &str) -> Result<Snapshot, String> {
        self.stop_checked(id, None)
    }
    /// A UI confirmation must not stop a replacement engine that appeared while
    /// the command was queued or waiting for the Hub lock.
    pub(crate) fn stop_session(&self, id: &str, expected_pid: u32) -> Result<Snapshot, String> {
        self.stop_checked(id, Some(expected_pid))
    }
    fn stop_checked(&self, id: &str, expected_pid: Option<u32>) -> Result<Snapshot, String> {
        // An externally started guest may need stopping before journal recovery.
        let _guard = self.lock_raw()?;
        let p = self.existing(id)?;
        let before = self.snapshot()?;
        let pid = before
            .runtime
            .get(id)
            .and_then(|r| r.pid)
            .ok_or("Máy chưa chạy")?;
        if expected_pid.is_some_and(|expected| expected != pid) {
            return Err("Engine đã đổi phiên; hãy xác nhận dừng lại".into());
        }
        let serial = format!("emulator-{}", p.port);
        let mut cmd = Command::new(self.sdk.join("platform-tools/adb"));
        cmd.args(["-s", &serial, "emu", "avd", "name"]);
        let identity = self.command(cmd, 5)?;
        if identity.lines().next().map(str::trim) != Some(p.id.as_str()) {
            return Err("ADB trả về máy khác; đã từ chối dừng".into());
        }
        if self.snapshot()?.runtime.get(id).and_then(|r| r.pid) != Some(pid) {
            return Err("Engine đã thay đổi; thử làm mới danh sách".into());
        }
        let mut cmd = Command::new(self.sdk.join("platform-tools/adb"));
        cmd.args(["-s", &serial, "emu", "kill"]);
        self.command(cmd, 10)?;
        for _ in 0..30 {
            let state = self.snapshot()?;
            if !state.runtime.contains_key(id) {
                return Ok(state);
            }
            thread::sleep(Duration::from_millis(500));
        }
        Err("Đã yêu cầu dừng; engine vẫn đang thoát. Không cưỡng bức dừng.".into())
    }
}
pub fn validate_registry(profiles: &[Profile]) -> Result<(), String> {
    let mut ids = BTreeSet::new();
    let mut ports = BTreeSet::new();
    for p in profiles {
        p.validate()?;
        if !ids.insert(&p.id) || !ports.insert(p.port) {
            return Err("Registry có ID/port trùng".into());
        }
    }
    Ok(())
}
