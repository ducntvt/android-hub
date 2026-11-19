//! Application state and initialization; feature handlers live in child modules.
mod apk;
mod connectivity;
mod creation;
mod deletion;
mod feedback;
mod lifecycle;
mod message;
mod physical;
mod runtime;
mod sdk;
use message::Message;
pub(crate) use runtime::run;
#[cfg(test)]
mod device_tests;
mod screen;
mod settings;
#[cfg(test)]
mod settings_tests;
mod shortcuts;
#[cfg(test)]
mod tests;
mod ui;
mod uninstall;
mod update;
mod workspace;

use crate::{appearance, data_root, i18n::Language};
use android_hub_domain::{DeviceId, DeviceOperation, EmulatorId, Profile};
use android_hub_infrastructure::display::{Input, Session};
use android_hub_infrastructure::{
    Backend, BackupCatalog, BackupInfo, DeletePlan, DiskUsage, PurgePlan, RemovedCatalog,
    RemovedDevice, RestoreReceipt, Snapshot,
};
use iced::widget::image;
use iced::{Element, Task};
use std::collections::BTreeMap;
use std::{path::PathBuf, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Resolution {
    Small,
    Medium,
    Large,
}
impl Resolution {
    fn dimensions(self, landscape: bool) -> (u32, u32) {
        let (short, long) = match self {
            Self::Small => (480, 854),
            Self::Medium => (720, 1280),
            Self::Large => (1080, 1920),
        };
        if landscape {
            (long, short)
        } else {
            (short, long)
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Orientation {
    Portrait,
    Landscape,
}
impl std::fmt::Display for Orientation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Portrait => "Dọc",
            Self::Landscape => "Ngang",
        })
    }
}
#[derive(Clone, Debug)]
enum Work {
    Initialize,
    Poll(u64),
    Start(String),
    Stop(String, u32),
    Save(Profile, Profile),
    InstallApk(PendingApk),
    Delete(DeletePlan),
    UndoRemove(RemovedDevice),
    Purge(PurgePlan),
}
#[derive(Clone, Debug)]
struct PendingApk {
    id: String,
    name: String,
    pid: u32,
    path: PathBuf,
    bytes: u64,
    epoch: u64,
    selection: Option<DeviceId>,
}
#[derive(Clone, Debug)]
struct PendingRestore {
    profile: Profile,
    backup: BackupInfo,
}
#[derive(Clone, Debug)]
struct PendingStop {
    id: String,
    name: String,
    pid: u32,
}
struct DisplayState {
    pid: u32,
    session: Session,
    image: Option<image::Handle>,
    size: (u32, u32),
    rotation: u8,
    error: Option<String>,
    input_generation: Option<u64>,
}
struct Hub {
    physical: physical::Discovery,
    uninstall: Option<uninstall::Dialog>,
    connectivity: Option<connectivity::Check>,
    connectivity_epoch: u64,
    quit_requested: bool,
    quitting: bool,
    quit_error: Option<String>,
    shutdown_progress: Vec<android_hub_infrastructure::ShutdownProgress>,
    boot_checks: BTreeMap<String, lifecycle::BootCheck>,
    lifecycle_action: Option<lifecycle::Action>,
    lifecycle_errors: BTreeMap<String, String>,
    language: Language,
    appearance: appearance::Preference,
    system_dark: bool,
    preferences_open: bool,
    displays: BTreeMap<String, DisplayState>,
    split: bool,
    sidebar_collapsed: bool,
    slots: workspace::Slots,
    search: String,
    settings_open: bool,
    creation: Option<creation::Editor>,
    creation_epoch: u64,
    creation_in_flight: Option<u64>,
    sdk_open: bool,
    sdk_details: bool,
    sdk_draft: String,
    java_draft: String,
    managed_sdks: android_hub_infrastructure::ManagedSdkCatalog,
    managed_loading: bool,
    managed_epoch: u64,
    managed_error: Option<String>,
    sdk_review_open: bool,
    sdk_setup: Option<android_hub_infrastructure::SdkSetupControl>,
    sdk_setup_result: Option<String>,
    images: Result<Vec<android_hub_infrastructure::ImageAvailability>, String>,
    images_loading: bool,
    images_epoch: u64,
    image_removal: Option<android_hub_infrastructure::ImageRemovalPlan>,
    sdk_cache_reset: Option<usize>,
    sdk_plan: Option<android_hub_infrastructure::SdkPlan>,
    sdk_plan_error: Option<String>,
    review_license: usize,
    accepted_licenses: std::collections::BTreeSet<String>,
    zoom: f32,
    input_focus: Option<String>,
    input_epoch: u64,
    backend: Backend,
    state: Option<Snapshot>,
    selected: Option<DeviceId>,
    draft: Option<Profile>,
    draft_base: Option<Profile>,
    pending_navigation: Option<settings::Navigation>,
    saving_navigation: Option<settings::Navigation>,
    busy: bool,
    polling: bool,
    command_epoch: u64,
    stale: bool,
    pending_stop: Option<PendingStop>,
    pending_apk: Option<PendingApk>,
    apk_request: Option<apk::Request>,
    apk_epoch: u64,
    apk_hover: bool,
    feedback: feedback::Feedback,
    detail_id: Option<u64>,
    modal_focus: usize,
    modal_return_focus: Option<(String, u32, u64)>,
    history_open: bool,
    history_unread: bool,
    backups: BackupCatalog,
    catalog_loading: bool,
    catalog_loaded: bool,
    catalog_error: Option<String>,
    backup_details: bool,
    catalog_epoch: u64,
    chosen_backup: Option<BackupInfo>,
    pending_restore: Option<PendingRestore>,
    receipts: Vec<RestoreReceipt>,
    removal_history: Vec<String>,
    creation_history: Vec<String>,
    receipt_epoch: u64,
    refresh_draft: bool,
    removed_open: bool,
    removed_loading: bool,
    removed_error: Option<String>,
    archive_details: bool,
    removed_epoch: u64,
    removed: RemovedCatalog,
    chosen_removed: Option<RemovedDevice>,
    device_menu: Option<String>,
    delete_epoch: u64,
    delete_plan: Option<DeletePlan>,
    delete_error: Option<String>,
    delete_backup: Option<PathBuf>,
    pending_delete: Option<Profile>,
    pending_undo: Option<RemovedDevice>,
    pending_purge: Option<PurgePlan>,
    storage_open: bool,
    storage_loading: bool,
    storage_error: Option<String>,
    storage_details: bool,
    storage: Option<DiskUsage>,
    location_notice: Option<String>,
    notice: String,
}
impl Hub {
    fn new() -> (Self, Task<Message>) {
        let home = PathBuf::from(std::env::var_os("HOME").expect("macOS home directory"));
        let root = data_root::resolve(
            std::env::var_os("ANDROID_HUB_ROOT").map(PathBuf::from),
            &home,
        );
        let mut location_notice = None;
        let backend = Backend::new(root.clone()).unwrap_or_else(|error| {
            location_notice = Some(format!(
                "Không đọc được cấu hình SDK: {error}. Mở Thiết lập SDK để sửa."
            ));
            Backend::environment_sdk(root).expect("macOS home directory")
        });
        let task = Self::task(backend.clone(), Work::Initialize);
        (
            Self {
                physical: physical::Discovery::new(
                    backend
                        .physical_adb_path()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                ),
                uninstall: None,
                connectivity: None,
                connectivity_epoch: 0,
                quit_requested: false,
                quitting: false,
                quit_error: None,
                shutdown_progress: vec![],
                boot_checks: BTreeMap::new(),
                lifecycle_action: None,
                lifecycle_errors: BTreeMap::new(),
                language: Language::load(&backend.root),
                appearance: appearance::Preference::load(&backend.root),
                system_dark: appearance::system_dark().unwrap_or(false),
                preferences_open: false,
                displays: BTreeMap::new(),
                split: false,
                sidebar_collapsed: false,
                slots: Default::default(),
                search: String::new(),
                settings_open: false,
                creation: None,
                creation_epoch: 0,
                creation_in_flight: None,
                sdk_open: false,
                sdk_details: false,
                managed_sdks: Default::default(),
                managed_loading: false,
                managed_epoch: 0,
                managed_error: None,
                sdk_review_open: false,
                sdk_setup: None,
                sdk_setup_result: None,
                images: Ok(vec![]),
                images_loading: false,
                images_epoch: 0,
                image_removal: None,
                sdk_cache_reset: None,
                sdk_plan: None,
                sdk_plan_error: None,
                review_license: 0,
                accepted_licenses: Default::default(),
                sdk_draft: backend.sdk.to_string_lossy().into_owned(),
                java_draft: backend
                    .java_home()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                zoom: 1.0,
                input_focus: None,
                input_epoch: 0,
                backend,
                state: None,
                selected: None,
                draft: None,
                draft_base: None,
                pending_navigation: None,
                saving_navigation: None,
                busy: true,
                polling: false,
                command_epoch: 0,
                stale: true,
                pending_stop: None,
                pending_apk: None,
                apk_request: None,
                apk_epoch: 0,
                apk_hover: false,
                feedback: Default::default(),
                detail_id: None,
                modal_focus: 0,
                modal_return_focus: None,
                history_open: false,
                history_unread: false,
                backups: BackupCatalog::default(),
                catalog_loading: false,
                catalog_loaded: false,
                catalog_error: None,
                backup_details: false,
                catalog_epoch: 0,
                chosen_backup: None,
                pending_restore: None,
                receipts: vec![],
                removal_history: vec![],
                creation_history: vec![],
                receipt_epoch: 0,
                refresh_draft: false,
                removed_open: false,
                removed_loading: false,
                removed_error: None,
                archive_details: false,
                removed_epoch: 0,
                removed: RemovedCatalog::default(),
                chosen_removed: None,
                device_menu: None,
                delete_epoch: 0,
                delete_plan: None,
                delete_error: None,
                delete_backup: None,
                pending_delete: None,
                pending_undo: None,
                pending_purge: None,
                storage_open: false,
                storage_loading: false,
                storage_error: None,
                storage_details: false,
                storage: None,
                location_notice,
                notice: "Đang đọc danh sách máy…".into(),
            },
            task,
        )
    }
    fn task(backend: Backend, work: Work) -> Task<Message> {
        let returned = work.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || match work {
                    Work::Initialize => backend.initialize(),
                    Work::Poll(_) => backend.snapshot(),
                    Work::Start(id) => backend.emulator(&id)?.start(),
                    Work::Stop(id, pid) => backend.emulator(&id)?.stop(pid),
                    Work::Save(p, expected) => backend.emulator(&expected.id)?.save(&expected, p),
                    Work::InstallApk(p) => backend.install_apk(&p.id, p.pid, &p.path),
                    Work::Delete(plan) => backend.emulator(&plan.profile.id)?.delete(&plan),
                    Work::Purge(plan) => backend.emulator(&plan.device.profile.id)?.purge(&plan),
                    Work::UndoRemove(info) => {
                        backend.emulator(&info.profile.id)?.undo_archive(&info)
                    }
                })
                .await
                .unwrap_or_else(|e| Err(format!("Tác vụ nền thất bại: {e}")))
            },
            move |result| Message::Loaded(result, returned.clone()),
        )
    }
    fn clear_backup_selection(&mut self) {
        self.pending_delete = None;
        self.delete_plan = None;
        self.delete_error = None;
        self.delete_backup = None;
        self.device_menu = None;
        self.pending_undo = None;
        self.pending_purge = None;
        self.removed_epoch = self.removed_epoch.wrapping_add(1);
        self.removed_loading = false;
        self.removed_error = None;
        self.archive_details = false;
        self.removed = RemovedCatalog::default();
        self.chosen_removed = None;
        self.catalog_epoch = self.catalog_epoch.wrapping_add(1);
        self.catalog_loading = false;
        self.catalog_loaded = false;
        self.catalog_error = None;
        self.backup_details = false;
        self.backups = BackupCatalog::default();
        self.chosen_backup = None;
        self.pending_restore = None;
    }
    fn receipt_task(&mut self) -> Task<Message> {
        self.receipt_epoch = self.receipt_epoch.wrapping_add(1);
        let epoch = self.receipt_epoch;
        let backend = self.backend.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    (
                        backend.restore_receipts(),
                        backend.removal_history(),
                        backend.creation_history(),
                    )
                })
                .await
                .unwrap_or_else(|e| {
                    let error = e.to_string();
                    (Err(error.clone()), Err(error.clone()), Err(error))
                })
            },
            move |(restore, removal, creation)| {
                Message::ReceiptsLoaded(epoch, restore, removal, creation)
            },
        )
    }
    fn choose(&mut self, id: String) {
        if let Ok(id) = EmulatorId::new(id) {
            self.choose_device(DeviceId::Emulator(id));
        }
    }
    fn choose_device(&mut self, target: DeviceId) {
        self.release_inputs();
        if let Some(id) = target.as_emulator() {
            self.slots.select(id.as_str().to_owned());
        }
        self.select_device_metadata(target);
        self.sync_displays();
    }
    // A successful pointer-down already belongs to the current input epoch.
    // Updating its inspector must not invalidate the matching drag/release.
    fn selected_avd_id(&self) -> Option<&str> {
        match self.selected.as_ref()? {
            DeviceId::Emulator(id) => Some(id.as_str()),
            DeviceId::Physical(_) => None,
        }
    }
    fn selected_allows(&self, operation: DeviceOperation) -> bool {
        self.selected
            .as_ref()
            .is_some_and(|id| id.capabilities().allows(operation))
    }
    fn select_metadata(&mut self, id: String) {
        let Ok(id) = EmulatorId::new(id) else {
            return;
        };
        self.select_device_metadata(DeviceId::Emulator(id));
    }
    fn select_device_metadata(&mut self, target: DeviceId) {
        let preserve_draft = self.selected.as_ref() == Some(&target) && self.draft.is_some();
        let id = match &target {
            DeviceId::Emulator(id) => Some(id.as_str()),
            DeviceId::Physical(_) => None,
        };
        self.removed_open = false;
        self.clear_backup_selection();
        self.pending_apk = None;
        self.apk_request = None;
        self.pending_delete = None;
        self.delete_plan = None;
        self.delete_error = None;
        self.delete_backup = None;
        self.device_menu = None;
        self.pending_undo = None;
        self.pending_purge = None;
        if !preserve_draft {
            self.draft = self
                .state
                .as_ref()
                .and_then(|s| s.profiles.iter().find(|p| Some(p.id.as_str()) == id))
                .cloned();
            self.draft_base = self.draft.clone();
        }
        self.selected = Some(target);
        self.reconcile_physical_preview();
        self.pending_stop = None;
    }
    fn has_unsaved_settings(&self) -> bool {
        self.draft.is_some() && self.draft != self.draft_base
    }
    fn activate_visible(&mut self, id: String) {
        if let Some(slot) = self
            .slots
            .ids
            .iter()
            .position(|value| value.as_ref() == Some(&id))
        {
            self.slots.active = slot;
        }
        self.select_metadata(id);
    }
    fn release_inputs(&mut self) {
        self.input_focus = None;
        self.input_epoch = self.input_epoch.wrapping_add(1);
        for d in self.displays.values() {
            let _ = d.session.send(Input::Mouse {
                x: 0,
                y: 0,
                down: false,
            });
        }
    }
    fn running(&self) -> bool {
        self.selected_avd_id()
            .and_then(|id| self.state.as_ref()?.runtime.get(id))
            .is_some_and(|r| r.pid.is_some())
    }
    fn launch(&mut self, work: Work) -> Task<Message> {
        if let Work::Start(id) | Work::Stop(id, _) = &work {
            self.lifecycle_errors.remove(id);
            self.lifecycle_action = Some(lifecycle::Action {
                id: id.clone(),
                starting: matches!(work, Work::Start(_)),
                since: std::time::Instant::now(),
            });
        }
        self.command_epoch = self.command_epoch.wrapping_add(1);
        self.busy = true;
        self.clear_backup_selection();
        self.pending_apk = None;
        self.pending_stop = None;
        self.notice = match &work {
            Work::Delete(_) => "Đang xóa vĩnh viễn máy…",
            Work::UndoRemove(_) => "Đang đưa máy trở lại…",
            Work::Purge(_) => "Đang xóa vĩnh viễn máy…",
            Work::Start(_) => "Đang mở Android trong Hub…",
            Work::Stop(..) => "Đang dừng máy…",
            Work::Save(..) => "Đang lưu cấu hình…",
            Work::InstallApk(_) => "Đang cài APK…",
            _ => "Đang tải…",
        }
        .into();
        Self::task(self.backend.clone(), work)
    }
    fn sync_displays(&mut self) {
        if self.quit_requested {
            self.displays.clear();
            return;
        }
        if let Some(state) = &self.state {
            self.slots.retain(&state.profiles);
        }
        let visible = if self.split {
            self.slots.ids.iter().flatten().cloned().collect::<Vec<_>>()
        } else {
            self.selected_avd_id()
                .map(str::to_owned)
                .into_iter()
                .collect()
        };
        let desired: Vec<_> = self
            .state
            .as_ref()
            .map(|s| {
                s.profiles
                    .iter()
                    .filter(|p| visible.contains(&p.id))
                    .filter_map(|p| {
                        s.runtime
                            .get(&p.id)
                            .and_then(|r| r.pid)
                            .map(|pid| (p.clone(), pid))
                    })
                    .take(2)
                    .collect()
            })
            .unwrap_or_default();
        self.displays
            .retain(|id, d| desired.iter().any(|(p, pid)| &p.id == id && *pid == d.pid));
        for (p, pid) in desired {
            self.displays
                .entry(p.id.clone())
                .or_insert_with(|| DisplayState {
                    pid,
                    session: Session::open(self.backend.clone(), p, pid),
                    image: None,
                    size: (480, 854),
                    rotation: 0,
                    error: None,
                    input_generation: None,
                });
        }
    }
    fn view(&self) -> Element<'_, Message> {
        ui::view(self)
    }
}
