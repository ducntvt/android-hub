//! APK selection is read-only until an explicit, session-bound confirmation.
use super::*;
use android_hub_domain::PhysicalSession;
use android_hub_infrastructure::physical::{ConnectionState, TransportKind};
use feedback::Kind;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Session {
    Emulator(u32),
    Physical(PhysicalSession),
}

#[derive(Clone, Debug)]
pub(super) struct Target {
    pub id: DeviceId,
    pub name: String,
    pub session: Session,
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
        let mut targets = Vec::new();
        if !self.stale
            && let Some(state) = &self.state
        {
            targets.extend(state.profiles.iter().filter_map(|p| {
                let pid = state.runtime.get(&p.id)?.pid?;
                let check = self.boot_checks.get(&p.id)?;
                if check.pid != pid
                    || !check.result.as_ref().is_some_and(|result| {
                        result
                            .as_ref()
                            .is_ok_and(|observation| observation.completed)
                    })
                {
                    return None;
                }
                Some(Target {
                    id: DeviceId::Emulator(EmulatorId::new(p.id.clone()).ok()?),
                    name: p.name.clone(),
                    session: Session::Emulator(pid),
                })
            }));
        }
        targets.extend(self.physical.phones.values().filter_map(|phone| {
            if phone.device.state != ConnectionState::Ready
                || phone.device.kind != TransportKind::Usb
            {
                return None;
            }
            let metadata = phone.metadata.as_ref()?.as_ref().ok()?;
            if metadata.emulator {
                return None;
            }
            Some(Target {
                id: DeviceId::Physical(phone.device.id.clone()),
                name: format!("{} · USB", metadata.model),
                session: Session::Physical(phone.session.clone()?),
            })
        }));
        targets
    }
    fn apk_target_current(&self, target: &Target) -> bool {
        self.apk_targets()
            .iter()
            .any(|t| t.id == target.id && t.session == target.session)
    }
    fn apk_request_current(&self, request: &Request) -> bool {
        !self.busy
            && !self.quit_requested
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
    pub(super) fn pick_apk(&mut self, id: DeviceId) -> Task<Message> {
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
            self.notify(
                Kind::Error,
                "Kết nối điện thoại hoặc khởi động máy Android trước khi cài APK",
                None,
            );
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
    pub(super) fn choose_apk_target(&mut self, id: DeviceId) {
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
            target,
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
                && self.apk_target_current(&p.target)
                && !self.busy
                && !self.quit_requested
            {
                if let Session::Physical(token) = p.target.session.clone() {
                    self.command_epoch = self.command_epoch.wrapping_add(1);
                    let epoch = self.command_epoch;
                    self.busy = true;
                    self.release_inputs();
                    self.notice = "Đang cài APK…".into();
                    return Task::perform(
                        async move {
                            let result = android_hub_infrastructure::physical::apk::install(
                                &token, &p.path, p.bytes,
                            )
                            .await;
                            (epoch, p.target, result)
                        },
                        |(epoch, target, result)| {
                            Message::PhysicalApkFinished(epoch, target, result)
                        },
                    );
                }
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
        hub.reconcile_boot_checks();
        for check in hub.boot_checks.values_mut() {
            check.result = Some(Ok(android_hub_infrastructure::BootObservation {
                boot_id: "qa-boot".into(),
                completed: true,
            }));
        }
        hub
    }
    fn connect_phone(hub: &mut Hub) -> Target {
        use android_hub_infrastructure::physical::{PhoneMetadata, parse_devices};
        // Deliberately collide with an AVD's raw ID: destination kinds must stay distinct.
        let tokens = hub.physical.apply(Ok(
            parse_devices("hub-b device usb:1 transport_id:7").unwrap()
        ));
        let token = tokens.into_iter().next().unwrap();
        hub.physical.metadata(
            &token,
            Ok(PhoneMetadata {
                model: "QA Phone".into(),
                android: "15".into(),
                emulator: false,
            }),
        );
        hub.apk_targets()
            .into_iter()
            .find(|t| matches!(t.session, Session::Physical(_)))
            .unwrap()
    }
    #[test]
    fn apk_targets_require_verified_boot_and_keep_device_namespaces_separate() {
        let mut hub = hub();
        let phone = connect_phone(&mut hub);
        assert_eq!(hub.apk_targets().len(), 3);
        assert_eq!(phone.name, "QA Phone · USB");
        hub.boot_checks.get_mut("hub-b").unwrap().result = None;
        assert_eq!(hub.apk_targets().len(), 2);
        hub.stale = true;
        let targets = hub.apk_targets();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].id, phone.id);
    }
    #[test]
    fn phone_confirmation_works_without_avd_state_but_cancel_never_installs() {
        for cancel in [false, true] {
            let mut hub = hub();
            let phone = connect_phone(&mut hub);
            hub.stale = true;
            drop_inspected(&mut hub);
            let _ = hub.update(Message::ChooseApkTarget(phone.id.clone()));
            assert_eq!(hub.pending_apk.as_ref().unwrap().target.id, phone.id);
            if cancel {
                let _ = hub.update(Message::CancelModal);
            }
            // Drop the task without polling: these tests never send an APK.
            let _ = hub.update(Message::ConfirmApk);
            assert_eq!(hub.busy, !cancel);
            assert!(hub.pending_apk.is_none());
        }
    }
    #[test]
    fn phone_reconnect_before_confirmation_cannot_retarget_or_reuse_session() {
        let mut hub = hub();
        let phone = connect_phone(&mut hub);
        drop_inspected(&mut hub);
        let _ = hub.update(Message::ChooseApkTarget(phone.id.clone()));
        hub.physical.apply(Ok(vec![]));
        let new_phone = connect_phone(&mut hub);
        assert_eq!(phone.id, new_phone.id);
        assert_ne!(phone.session, new_phone.session);
        let _ = hub.update(Message::ConfirmApk);
        assert!(!hub.busy && hub.pending_apk.is_none());
        assert_eq!(hub.feedback.persistent.back().unwrap().kind, Kind::Error);
    }
    #[test]
    fn phone_install_result_after_reconnect_is_unknown_even_if_success_arrives() {
        let mut hub = hub();
        let phone = connect_phone(&mut hub);
        drop_inspected(&mut hub);
        let _ = hub.update(Message::ChooseApkTarget(phone.id.clone()));
        let _ = hub.update(Message::ConfirmApk);
        assert!(hub.busy);
        let epoch = hub.command_epoch;
        hub.physical.apply(Ok(vec![]));
        connect_phone(&mut hub);
        let _ = hub.update(Message::PhysicalApkFinished(epoch, phone, Ok(())));
        assert!(!hub.busy);
        assert_eq!(hub.feedback.persistent.back().unwrap().kind, Kind::Unknown);
        assert!(
            hub.feedback
                .toast
                .as_ref()
                .is_none_or(|n| n.kind != Kind::Success)
        );
    }
    #[test]
    fn known_phone_install_rejection_is_not_reported_as_unknown() {
        let mut hub = hub();
        let phone = connect_phone(&mut hub);
        hub.busy = true;
        let _ = hub.update(Message::PhysicalApkFinished(
            hub.command_epoch,
            phone,
            Err(android_hub_infrastructure::physical::apk::InstallError::Permission),
        ));
        assert!(!hub.busy);
        assert_eq!(hub.feedback.persistent.back().unwrap().kind, Kind::Error);
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
        let _ = hub.update(Message::ChooseApkTarget(DeviceId::Emulator(
            EmulatorId::new("hub-b").unwrap(),
        )));
        let pending = hub.pending_apk.as_ref().unwrap();
        assert_eq!(
            (&pending.target.id, &pending.target.session, pending.bytes),
            (
                &DeviceId::Emulator(EmulatorId::new("hub-b").unwrap()),
                &Session::Emulator(42),
                1234
            )
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
        let _ = hub.update(Message::ChooseApkTarget(DeviceId::Emulator(
            EmulatorId::new("hub-b").unwrap(),
        )));
        let _ = hub.update(Message::CancelModal);
        let _ = hub.update(Message::ConfirmApk);
        assert!(!hub.modal_open() && !hub.busy);
    }
    #[test]
    fn confirmation_rejects_restart_selection_and_command_epoch_changes() {
        for change in 0..3 {
            let mut hub = hub();
            drop_inspected(&mut hub);
            let _ = hub.update(Message::ChooseApkTarget(DeviceId::Emulator(
                EmulatorId::new("hub-b").unwrap(),
            )));
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
            .get_mut("hub-b")
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
        assert_eq!(
            hub.pending_apk.as_ref().unwrap().target.id,
            DeviceId::Emulator(EmulatorId::new("hub-b").unwrap())
        );
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
