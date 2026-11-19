//! APK selection is read-only until an explicit, session-bound confirmation.
use super::*;
use feedback::Kind;

#[derive(Clone, Debug)]
pub(super) struct Target {
    pub id: String,
    pub name: String,
    pub pid: u32,
}
#[derive(Clone, Debug)]
pub(super) struct File {
    pub path: PathBuf,
    pub bytes: u64,
}
#[derive(Clone, Debug)]
pub(super) enum Stage {
    Picking(Target),
    Inspecting(Option<Target>),
    Choosing(File),
}
#[derive(Clone, Debug)]
pub(super) struct Request {
    pub token: u64,
    pub epoch: u64,
    pub selection: Option<DeviceId>,
    pub stage: Stage,
}

pub(super) fn inspect(path: PathBuf) -> Result<File, String> {
    if !path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("apk"))
    {
        return Err("Chỉ hỗ trợ một file .apk; chưa hỗ trợ XAPK/APK split".into());
    }
    let path = path
        .canonicalize()
        .map_err(|e| format!("Không mở được APK: {e}"))?;
    let meta = path
        .metadata()
        .map_err(|e| format!("Không đọc được APK: {e}"))?;
    if !meta.is_file() || meta.len() == 0 {
        return Err("APK phải là file thường, không rỗng".into());
    }
    Ok(File {
        path,
        bytes: meta.len(),
    })
}

impl Hub {
    pub(super) fn apk_targets(&self) -> Vec<Target> {
        if self.stale {
            return vec![];
        }
        self.state
            .as_ref()
            .map(|s| {
                s.profiles
                    .iter()
                    .filter_map(|p| {
                        Some(Target {
                            id: p.id.clone(),
                            name: p.name.clone(),
                            pid: s.runtime.get(&p.id)?.pid?,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
    fn apk_target_current(&self, target: &Target) -> bool {
        self.apk_targets()
            .iter()
            .any(|t| t.id == target.id && t.pid == target.pid)
    }
    fn apk_request_current(&self, request: &Request) -> bool {
        !self.busy
            && !self.stale
            && request.epoch == self.command_epoch
            && request.selection == self.selected
    }
    fn begin_apk(&mut self, stage: Stage) -> u64 {
        self.cancel_modal();
        self.apk_hover = false;
        self.apk_epoch = self.apk_epoch.wrapping_add(1);
        self.apk_request = Some(Request {
            token: self.apk_epoch,
            epoch: self.command_epoch,
            selection: self.selected.clone(),
            stage,
        });
        self.apk_epoch
    }
    pub(super) fn pick_apk(&mut self, id: String) -> Task<Message> {
        let Some(target) = self.apk_targets().into_iter().find(|t| t.id == id) else {
            return Task::none();
        };
        let token = self.begin_apk(Stage::Picking(target));
        #[cfg(target_os = "macos")]
        {
            // Construct on the Iced/AppKit thread; await without blocking its event loop.
            let future = rfd::AsyncFileDialog::new()
                .set_title(self.language.tr("Chọn một file APK"))
                .add_filter("Android APK", &["apk"])
                .pick_file();
            Task::perform(
                async move { future.await.map(|file| file.path().to_path_buf()) },
                move |path| Message::ApkPicked(token, path),
            )
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = token;
            self.apk_request = None;
            self.notify(Kind::Error, "Chọn file APK hiện chỉ hỗ trợ macOS", None);
            Task::none()
        }
    }
    pub(super) fn dropped_apk(&mut self, path: PathBuf) -> Task<Message> {
        self.apk_hover = false;
        if self.apk_targets().is_empty() {
            self.notify(Kind::Error, "Khởi động một máy trước khi cài APK", None);
            return Task::none();
        }
        let token = self.begin_apk(Stage::Inspecting(None));
        Self::inspect_apk(token, path)
    }
    pub(super) fn picked_apk(&mut self, token: u64, path: Option<PathBuf>) -> Task<Message> {
        let Some(request) = self.apk_request.as_ref().filter(|r| r.token == token) else {
            return Task::none();
        };
        let Stage::Picking(target) = &request.stage else {
            return Task::none();
        };
        let target = target.clone();
        if !self.apk_request_current(request) || !self.apk_target_current(&target) {
            self.apk_request = None;
            self.notify(
                Kind::Error,
                "Thiết bị đã thay đổi; chọn lại APK để xác nhận",
                None,
            );
            return Task::none();
        }
        let Some(path) = path else {
            self.apk_request = None;
            return Task::none();
        };
        self.apk_request.as_mut().unwrap().stage = Stage::Inspecting(Some(target));
        Self::inspect_apk(token, path)
    }
    fn inspect_apk(token: u64, path: PathBuf) -> Task<Message> {
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || inspect(path))
                    .await
                    .unwrap_or_else(|e| Err(e.to_string()))
            },
            move |result| Message::ApkInspected(token, result),
        )
    }
    pub(super) fn inspected_apk(&mut self, token: u64, result: Result<File, String>) {
        let Some(request) = self.apk_request.as_ref().filter(|r| r.token == token) else {
            return;
        };
        let Stage::Inspecting(target) = &request.stage else {
            return;
        };
        let target = target.clone();
        if !self.apk_request_current(request)
            || target.as_ref().is_some_and(|t| !self.apk_target_current(t))
        {
            self.apk_request = None;
            self.notify(
                Kind::Error,
                "Thiết bị đã thay đổi; chọn lại APK để xác nhận",
                None,
            );
            return;
        }
        match result {
            Err(error) => {
                self.apk_request = None;
                self.error("Không đọc được file APK", error);
            }
            Ok(file) => {
                self.apk_request.as_mut().unwrap().stage = Stage::Choosing(file);
                if let Some(target) = target {
                    self.choose_apk_target(target.id);
                }
            }
        }
    }
    pub(super) fn choose_apk_target(&mut self, id: String) {
        let Some(request) = self.apk_request.as_ref() else {
            return;
        };
        let Stage::Choosing(file) = &request.stage else {
            return;
        };
        if !self.apk_request_current(request) {
            return;
        }
        let Some(target) = self.apk_targets().into_iter().find(|t| t.id == id) else {
            return;
        };
        self.pending_apk = Some(PendingApk {
            id: target.id,
            name: target.name,
            pid: target.pid,
            path: file.path.clone(),
            bytes: file.bytes,
            epoch: request.epoch,
            selection: request.selection.clone(),
        });
        self.apk_request = None;
        self.modal_focus = 0;
    }
    pub(super) fn confirm_apk(&mut self) -> Task<Message> {
        if let Some(p) = self.pending_apk.take() {
            if p.epoch == self.command_epoch
                && p.selection == self.selected
                && self
                    .apk_targets()
                    .iter()
                    .any(|t| t.id == p.id && t.pid == p.pid)
            {
                return self.launch(Work::InstallApk(p));
            }
            self.notify(
                Kind::Error,
                "Thiết bị đã thay đổi; chọn lại APK để xác nhận",
                None,
            );
        }
        Task::none()
    }
    pub(super) fn modal_action_count(&self) -> usize {
        if self.pending_navigation.is_some() || self.pending_delete.is_some() {
            3
        } else if let Some(request) = &self.apk_request {
            if matches!(request.stage, Stage::Choosing(_)) {
                1 + self.apk_targets().len()
            } else {
                1
            }
        } else {
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hub() -> Hub {
        let (mut hub, _) = Hub::new();
        let state = Snapshot {
            android_versions: BTreeMap::new(),
            runtime: BTreeMap::new(),
            profiles: vec![
                Profile::standard("hub-a".into(), "QA A".into(), 5584),
                Profile::standard("hub-b".into(), "QA B".into(), 5586),
            ],
        };
        let _ = hub.update(Message::Loaded(Ok(state), Work::Initialize));
        // Observations only: never launch an engine or native picker in unit tests.
        for (id, pid) in [("hub-a", 41), ("hub-b", 42)] {
            hub.state.as_mut().unwrap().runtime.insert(
                id.into(),
                android_hub_domain::Runtime {
                    pid: Some(pid),
                    cpu_percent: None,
                    rss_kib: None,
                },
            );
        }
        hub
    }
    fn file() -> File {
        File {
            path: PathBuf::from("/tmp/fixture.apk"),
            bytes: 1234,
        }
    }
    fn drop_inspected(hub: &mut Hub) {
        let _ = hub.update(Message::DropApk(file().path));
        let token = hub.apk_request.as_ref().unwrap().token;
        let _ = hub.update(Message::ApkInspected(token, Ok(file())));
    }
    #[test]
    fn drop_requires_destination_then_confirmation_and_does_not_accept_extra_files() {
        let mut hub = hub();
        drop_inspected(&mut hub);
        assert!(hub.pending_apk.is_none() && !hub.busy);
        let token = hub.apk_request.as_ref().unwrap().token;
        let _ = hub.update(Message::DropApk(PathBuf::from("/tmp/other.apk")));
        assert_eq!(hub.apk_request.as_ref().unwrap().token, token);
        let _ = hub.update(Message::ChooseApkTarget("hub-b".into()));
        let pending = hub.pending_apk.as_ref().unwrap();
        assert_eq!(
            (&*pending.id, pending.pid, pending.bytes),
            ("hub-b", 42, 1234)
        );
        assert_eq!(
            pending.selection,
            Some(DeviceId::Emulator(EmulatorId::new("hub-a").unwrap()))
        );
        let _ = hub.update(Message::DropApk(PathBuf::from("/tmp/other.apk")));
        assert_eq!(hub.pending_apk.as_ref().unwrap().path, file().path);
        let _ = hub.update(Message::ConfirmApk); // task is not polled; no install runs
        assert!(hub.busy && hub.pending_apk.is_none());
        let epoch = hub.command_epoch;
        let _ = hub.update(Message::ConfirmApk);
        assert_eq!(hub.command_epoch, epoch);
    }
    #[test]
    fn cancel_never_installs_and_late_file_results_do_not_reopen_modal() {
        let mut hub = hub();
        let _ = hub.update(Message::DropApk(file().path));
        let token = hub.apk_request.as_ref().unwrap().token;
        let _ = hub.update(Message::CancelModal);
        let _ = hub.update(Message::ApkInspected(token, Ok(file())));
        assert!(!hub.modal_open() && !hub.busy);
        drop_inspected(&mut hub);
        let _ = hub.update(Message::ChooseApkTarget("hub-b".into()));
        let _ = hub.update(Message::CancelModal);
        let _ = hub.update(Message::ConfirmApk);
        assert!(!hub.modal_open() && !hub.busy);
    }
    #[test]
    fn confirmation_rejects_restart_selection_and_command_epoch_changes() {
        for change in 0..3 {
            let mut hub = hub();
            drop_inspected(&mut hub);
            let _ = hub.update(Message::ChooseApkTarget("hub-b".into()));
            match change {
                0 => {
                    hub.state
                        .as_mut()
                        .unwrap()
                        .runtime
                        .get_mut("hub-b")
                        .unwrap()
                        .pid = Some(43)
                }
                1 => {
                    hub.selected = Some(DeviceId::Physical(
                        android_hub_domain::PhysicalId::new("hub-b").unwrap(),
                    ))
                }
                _ => hub.command_epoch += 1,
            }
            let _ = hub.update(Message::ConfirmApk);
            assert!(!hub.busy && hub.pending_apk.is_none());
            assert!(!hub.feedback.persistent.is_empty());
        }
    }
    #[test]
    fn picker_cancel_restart_and_stale_token_are_read_only() {
        let mut hub = hub();
        let target = hub.apk_targets().pop().unwrap();
        let token = hub.begin_apk(Stage::Picking(target.clone()));
        let _ = hub.update(Message::ApkPicked(token, None));
        assert!(!hub.modal_open() && !hub.busy);
        let next = hub.begin_apk(Stage::Picking(target.clone()));
        let _ = hub.update(Message::ApkPicked(token, Some(file().path)));
        assert!(matches!(
            hub.apk_request.as_ref().unwrap().stage,
            Stage::Picking(_)
        ));
        hub.state
            .as_mut()
            .unwrap()
            .runtime
            .get_mut(&target.id)
            .unwrap()
            .pid = Some(101);
        let _ = hub.update(Message::ApkPicked(next, Some(file().path)));
        assert!(!hub.modal_open() && !hub.busy);
        let token = hub.begin_apk(Stage::Inspecting(Some(target)));
        let _ = hub.update(Message::ApkInspected(token, Ok(file())));
        assert!(!hub.modal_open() && !hub.busy);
    }
    #[test]
    fn drop_keyboard_defaults_to_cancel_and_explicit_target_follows_focus() {
        use iced::keyboard::key::Named;
        let mut hub = hub();
        drop_inspected(&mut hub);
        let _ = hub.update(Message::ModalKey(Named::Enter));
        assert!(!hub.modal_open() && !hub.busy);
        drop_inspected(&mut hub);
        let _ = hub.update(Message::ModalBackTab);
        assert_eq!(hub.modal_focus, 2);
        let _ = hub.update(Message::ModalKey(Named::Enter));
        assert_eq!(hub.pending_apk.as_ref().unwrap().id, "hub-b");
        assert_eq!(hub.modal_focus, 0); // confirmation defaults to Cancel again
        let _ = hub.update(Message::ModalKey(Named::Enter));
        assert!(!hub.modal_open() && !hub.busy);
    }
    #[test]
    fn file_inspection_rejects_missing_empty_directory_and_unsupported_files() {
        let dir =
            std::env::temp_dir().join(format!("android-hub-apk-preview-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("fixture.APK");
        assert!(inspect(path.clone()).is_err());
        std::fs::write(&path, []).unwrap();
        assert!(inspect(path.clone()).is_err());
        std::fs::write(&path, b"fake APK; package manager validates the content").unwrap();
        let metadata = inspect(path).unwrap();
        assert!(metadata.path.is_absolute());
        assert_eq!(metadata.bytes, 47);
        assert!(inspect(dir.clone()).is_err());
        assert!(inspect(dir.join("split.xapk")).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
