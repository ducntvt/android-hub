//! Discovery is independent of the AVD registry, SDK images and selected device.
use super::*;
use android_hub_domain::{PhysicalId, PhysicalSession, PhysicalSessions};
use android_hub_infrastructure::physical::{
    AdbDevice, AdbDiscovery, ConnectionState, DiscoveryError, PhoneMetadata, TransportKind,
    read_phone_metadata,
};
use iced::Subscription;

#[derive(Clone, Debug)]
pub(super) struct Phone {
    pub device: AdbDevice,
    pub session: Option<PhysicalSession>,
    pub metadata: Option<Result<PhoneMetadata, DiscoveryError>>,
}
#[derive(Default)]
pub(super) struct Discovery {
    pub focused: bool,
    pub input_epoch: u64,
    pub stopping: bool,
    pub completions: Vec<android_hub_infrastructure::physical::stream::Completion>,
    pub preview: Option<Preview>,
    pub preview_epoch: u64,
    pub stream_options: android_hub_infrastructure::physical::stream::Options,
    pub phones: BTreeMap<PhysicalId, Phone>,
    sessions: PhysicalSessions,
    pub error: Option<DiscoveryError>,
    pub epoch: u64,
    pub adb_path: String,
    pub configuring: bool,
    pub setup_result: Option<Result<(), String>>,
}
impl Discovery {
    pub fn new(adb_path: String) -> Self {
        Self {
            adb_path,
            ..Default::default()
        }
    }
    pub(super) fn invalidate(&mut self) {
        self.focused = false;
        self.preview = None;
        for phone in self.phones.values_mut() {
            if let Some(token) = phone.session.take() {
                self.sessions.disconnect(&token);
            }
            phone.device.state = ConnectionState::Disconnected;
        }
    }
    pub fn apply(
        &mut self,
        result: Result<Vec<AdbDevice>, DiscoveryError>,
    ) -> Vec<PhysicalSession> {
        let records = match result {
            Ok(records) => {
                self.error = None;
                records
            }
            Err(error) => {
                self.invalidate();
                self.error = Some(error);
                return vec![];
            }
        };
        let mut reads = vec![];
        for phone in self.phones.values_mut() {
            if !records
                .iter()
                .any(|d| d.id == phone.device.id && d.kind != TransportKind::Emulator)
            {
                if let Some(token) = phone.session.take() {
                    self.sessions.disconnect(&token);
                }
                phone.device.state = ConnectionState::Disconnected;
            }
        }
        for device in records
            .into_iter()
            .filter(|d| d.kind != TransportKind::Emulator)
        {
            let old = self.phones.get(&device.id);
            let same = old.is_some_and(|p| p.device == device);
            if same {
                continue;
            }
            if let Some(token) = old.and_then(|p| p.session.as_ref()) {
                self.sessions.disconnect(token);
            }
            let session =
                if device.kind == TransportKind::Usb && device.state == ConnectionState::Ready {
                    self.sessions
                        .connect(device.id.clone(), device.transport)
                        .ok()
                } else {
                    None
                };
            if let Some(token) = &session {
                reads.push(token.clone());
            }
            self.phones.insert(
                device.id.clone(),
                Phone {
                    device,
                    session,
                    metadata: None,
                },
            );
        }
        // Bound remembered disconnected rows across a long-running workspace.
        while self.phones.len() > 128 {
            let Some(id) = self
                .phones
                .iter()
                .find(|(_, p)| p.device.state == ConnectionState::Disconnected)
                .map(|(id, _)| id.clone())
            else {
                break;
            };
            self.phones.remove(&id);
        }
        reads
    }
    pub fn metadata(
        &mut self,
        token: &PhysicalSession,
        result: Result<PhoneMetadata, DiscoveryError>,
    ) {
        if !self.sessions.is_current(token) {
            return;
        }
        if let Some(phone) = self.phones.get_mut(token.device()) {
            if result.as_ref().is_ok_and(|m| m.emulator) {
                phone.device.kind = TransportKind::Emulator;
                self.sessions.disconnect(token);
                phone.session = None;
            }
            phone.metadata = Some(result);
        }
    }
}
impl Hub {
    pub(super) fn physical_subscription(&self) -> Subscription<Message> {
        if self.quit_requested {
            return Subscription::none();
        }
        let epoch = self.physical.epoch;
        Subscription::run_with_id(
            ("physical-discovery", epoch),
            iced::futures::stream::unfold(AdbDiscovery::default(), move |mut watcher| async move {
                let result = watcher.next().await;
                Some((Message::PhysicalDiscovered(epoch, result), watcher))
            }),
        )
    }
    pub(super) fn physical_update(
        &mut self,
        epoch: u64,
        result: Result<Vec<AdbDevice>, DiscoveryError>,
    ) -> Task<Message> {
        if epoch != self.physical.epoch || self.quit_requested {
            return Task::none();
        }
        let reads = self.physical.apply(result);
        self.reconcile_physical_preview();
        if self.selected.is_none()
            && self.state.as_ref().is_none_or(|s| s.profiles.is_empty())
            && let Some(phone) = self.physical.phones.values().find(|p| {
                p.device.kind == TransportKind::Usb
                    && p.device.state != ConnectionState::Disconnected
            })
        {
            self.select_device_metadata(DeviceId::Physical(phone.device.id.clone()));
        }
        Task::batch(reads.into_iter().map(|token| {
            Task::perform(
                async move {
                    let result = read_phone_metadata(token.transport()).await;
                    (token, result)
                },
                |(token, result)| Message::PhysicalMetadata(token, result),
            )
        }))
    }
    pub(super) fn displayed_phone(&self) -> Option<&Phone> {
        self.physical
            .preview
            .as_ref()
            .and_then(|p| self.physical.phones.get(p.token.device()))
            .or_else(|| self.selected_phone())
    }
    pub(super) fn selected_phone(&self) -> Option<&Phone> {
        let DeviceId::Physical(id) = self.selected.as_ref()? else {
            return None;
        };
        self.physical.phones.get(id)
    }
}
pub(super) struct Preview {
    pub token: PhysicalSession,
    pub session: android_hub_infrastructure::physical::stream::Session,
    pub epoch: u64,
    pub capture: u64,
    pub image: Option<image::Handle>,
    pub size: (u32, u32),
    pub hardware: bool,
    pub error: Option<String>,
}
#[derive(Clone, Debug)]
pub(super) struct FrameReport {
    pub epoch: u64,
    pub capture: u64,
    pub image: Option<image::Handle>,
    pub size: (u32, u32),
    pub hardware: bool,
    pub error: Option<String>,
}
impl Hub {
    pub(super) fn physical_input(&mut self, stream: u64, epoch: u64, capture: u64, input: Input) {
        if self.quit_requested
            || self.modal_open()
            || self.busy
            || epoch != self.physical.input_epoch
        {
            return;
        }
        self.reconcile_physical_preview();
        let Some(preview) = &self.physical.preview else {
            return;
        };
        if preview.epoch != stream
            || preview.capture != capture
            || preview.image.is_none()
            || preview.error.is_some()
        {
            return;
        }
        let pressed = matches!(input, Input::Mouse { down: true, .. });
        if let Err(error) = preview.session.send(capture, input) {
            self.release_inputs();
            self.error("Thao tác chưa hoàn tất", error);
        } else if pressed {
            let target = DeviceId::Physical(preview.token.device().clone());
            if self.selected.as_ref() != Some(&target) {
                self.input_focus = None;
                self.input_epoch = self.input_epoch.wrapping_add(1);
                for display in self.displays.values() {
                    let _ = display.session.send(Input::Mouse {
                        x: 0,
                        y: 0,
                        down: false,
                    });
                }
                self.select_device_metadata(target);
            }
            self.physical.focused = true;
        }
    }
    pub(super) fn start_physical_preview(&mut self) -> Task<Message> {
        if self.quit_requested || self.busy || self.modal_open() || self.physical.preview.is_some()
        {
            return Task::none();
        }
        let Some(phone) = self.selected_phone() else {
            return Task::none();
        };
        if phone.device.state != ConnectionState::Ready
            || phone.device.kind != TransportKind::Usb
            || !phone
                .metadata
                .as_ref()
                .is_some_and(|m| m.as_ref().is_ok_and(|m| !m.emulator))
        {
            return Task::none();
        }
        let Some(token) = phone.session.clone() else {
            return Task::none();
        };
        if !self.physical.sessions.is_current(&token) {
            return Task::none();
        }
        self.physical.focused = false;
        self.physical.preview_epoch += 1;
        self.physical.preview = Some(Preview {
            session: android_hub_infrastructure::physical::stream::Session::open(
                token.clone(),
                self.physical.stream_options,
            ),
            token,
            epoch: self.physical.preview_epoch,
            capture: 0,
            image: None,
            size: (0, 0),
            hardware: false,
            error: None,
        });
        self.physical.completions.retain(|c| !c.finished());
        self.physical
            .completions
            .push(self.physical.preview.as_ref().unwrap().session.completion());
        self.sync_displays();
        Task::none()
    }
    pub(super) fn stop_physical_sessions(&mut self) -> Task<Message> {
        self.physical.invalidate();
        let completions = std::mem::take(&mut self.physical.completions);
        if completions.iter().all(|completion| completion.finished()) {
            self.physical.stopping = false;
            return Task::none();
        }
        self.physical.stopping = true;
        Task::perform(
            async move {
                for completion in completions {
                    completion.wait().await;
                }
            },
            |_| Message::PhysicalStopped,
        )
    }
    pub(super) fn reconcile_physical_preview(&mut self) {
        if self.physical.preview.as_ref().is_some_and(|p| {
            self.quit_requested
                || !self.physical.sessions.is_current(&p.token)
                || (self.selected.as_ref() != Some(&DeviceId::Physical(p.token.device().clone()))
                    && !(self.split && self.selected_avd_id().is_some()))
        }) {
            self.physical.focused = false;
            self.physical.preview = None;
        }
    }
    pub(super) fn physical_frame_subscription(&self) -> Subscription<Message> {
        let Some(preview) = &self.physical.preview else {
            return Subscription::none();
        };
        if self.quit_requested || preview.error.is_some() {
            return Subscription::none();
        }
        let epoch = preview.epoch;
        let frames = preview.session.frames();
        let interval = Duration::from_secs_f64(1. / f64::from(self.physical.stream_options.fps));
        Subscription::run_with_id(
            ("physical-frames", epoch),
            iced::futures::stream::unfold(frames, move |frames| async move {
                tokio::time::sleep(interval).await;
                let source = frames.clone();
                let result = tokio::task::spawn_blocking(move || {
                    let update = source.take();
                    let (image, size) = update
                        .frame
                        .map(|frame| {
                            (
                                Some(image::Handle::from_rgba(
                                    frame.width,
                                    frame.height,
                                    frame.pixels,
                                )),
                                (frame.width, frame.height),
                            )
                        })
                        .unwrap_or((None, (0, 0)));
                    FrameReport {
                        epoch,
                        capture: update.capture,
                        image,
                        size,
                        hardware: update.hardware,
                        error: update.error,
                    }
                })
                .await
                .unwrap_or_else(|e| FrameReport {
                    epoch,
                    capture: 0,
                    image: None,
                    size: (0, 0),
                    hardware: false,
                    error: Some(e.to_string()),
                });
                Some((Message::PhysicalFrame(result), frames))
            }),
        )
    }
    pub(super) fn physical_frame(&mut self, report: FrameReport) {
        self.reconcile_physical_preview();
        let Some(preview) = &mut self.physical.preview else {
            return;
        };
        if preview.epoch != report.epoch {
            return;
        }
        if preview.capture != report.capture || report.error.is_some() {
            self.physical.focused = false;
            self.physical.input_epoch = self.physical.input_epoch.wrapping_add(1);
            preview.session.release_inputs();
            preview.image = None;
        }
        if report.error.is_some() {
            preview.session.stop();
        }
        preview.capture = report.capture;
        preview.error = report.error;
        preview.hardware = report.hardware;
        if let Some(image) = report.image {
            preview.image = Some(image);
            preview.size = report.size;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use android_hub_infrastructure::physical::parse_devices;
    fn rows() -> Vec<AdbDevice> {
        parse_devices("phone device usb:1 model:Phone transport_id:7").unwrap()
    }
    #[test]
    fn stale_stream_frames_and_previous_capture_cannot_survive_selection_change() {
        let (mut hub, _) = Hub::new();
        let token = hub.physical.apply(Ok(rows())).pop().unwrap();
        hub.select_device_metadata(DeviceId::Physical(token.device().clone()));
        // Invalid options fail before any ADB access; this is a state-only session.
        let session = android_hub_infrastructure::physical::stream::Session::open(
            token.clone(),
            android_hub_infrastructure::physical::stream::Options {
                fps: 0,
                max_size: 0,
            },
        );
        hub.physical.preview = Some(Preview {
            token,
            session,
            epoch: 7,
            capture: 1,
            image: None,
            size: (0, 0),
            hardware: false,
            error: None,
        });
        let report = |epoch, capture| FrameReport {
            epoch,
            capture,
            image: Some(image::Handle::from_rgba(1, 1, vec![0; 4])),
            size: (1, 1),
            hardware: true,
            error: None,
        };
        hub.physical_frame(report(6, 1));
        assert!(hub.physical.preview.as_ref().unwrap().image.is_none());
        hub.physical_frame(report(7, 1));
        assert!(hub.physical.preview.as_ref().unwrap().image.is_some());
        hub.physical_frame(FrameReport {
            epoch: 7,
            capture: 2,
            image: None,
            size: (0, 0),
            hardware: true,
            error: None,
        });
        assert!(hub.physical.preview.as_ref().unwrap().image.is_none());
        hub.split = true;
        hub.select_metadata("hub-other".into());
        assert!(
            hub.physical.preview.is_some(),
            "mixed slots retain the phone session"
        );
        hub.physical.focused = true;
        let previous_epoch = hub.physical.input_epoch;
        hub.release_inputs();
        assert!(!hub.physical.focused);
        assert_ne!(hub.physical.input_epoch, previous_epoch);
        hub.split = false;
        hub.reconcile_physical_preview();
        assert!(hub.physical.preview.is_none());
        hub.physical_frame(report(7, 2));
        assert!(hub.physical.preview.is_none());
    }
    #[test]
    fn multiple_usb_devices_keep_metadata_and_disconnects_separate() {
        let mut d = Discovery::default();
        let rows =
            parse_devices("one device usb:1 transport_id:1\ntwo device usb:2 transport_id:2")
                .unwrap();
        let tokens = d.apply(Ok(rows));
        assert_eq!(tokens.len(), 2);
        for (index, token) in tokens.iter().enumerate() {
            d.metadata(
                token,
                Ok(PhoneMetadata {
                    model: format!("Phone {index}"),
                    android: "15".into(),
                    emulator: false,
                }),
            );
        }
        d.apply(Ok(parse_devices("two device usb:2 transport_id:2").unwrap()));
        assert_eq!(
            d.phones[tokens[0].device()].device.state,
            ConnectionState::Disconnected
        );
        assert_eq!(
            d.phones[tokens[1].device()]
                .metadata
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .model,
            "Phone 1"
        );
    }
    #[test]
    fn phone_only_workspace_selects_usb_without_creating_an_avd_draft() {
        let (mut hub, _) = Hub::new();
        hub.state = Some(Snapshot {
            profiles: vec![],
            runtime: BTreeMap::new(),
            android_versions: BTreeMap::new(),
        });
        let epoch = hub.physical.epoch;
        let _ = hub.physical_update(epoch, Ok(rows()));
        assert!(matches!(hub.selected, Some(DeviceId::Physical(_))));
        assert!(hub.draft.is_none());
        assert!(hub.selected_avd_id().is_none());
        let _ = hub.view();
        let before = hub.selected.clone();
        let _ = hub.physical_update(epoch + 1, Ok(vec![]));
        assert_eq!(hub.selected, before);
        assert_eq!(
            hub.selected_phone().unwrap().device.state,
            ConnectionState::Ready
        );
    }
    #[test]
    fn repeated_updates_cache_metadata_but_server_restart_invalidates_tokens() {
        let mut d = Discovery::default();
        let token = d.apply(Ok(rows())).pop().unwrap();
        assert!(d.apply(Ok(rows())).is_empty());
        d.apply(Err(DiscoveryError::Unavailable));
        let next = d.apply(Ok(rows())).pop().unwrap();
        assert_ne!(token, next);
        d.metadata(&token, Err(DiscoveryError::Protocol));
        assert!(d.phones[&token.device().clone()].metadata.is_none());
        d.metadata(
            &next,
            Ok(PhoneMetadata {
                model: "Phone".into(),
                android: "15".into(),
                emulator: false,
            }),
        );
        assert!(
            d.phones[&next.device().clone()]
                .metadata
                .as_ref()
                .unwrap()
                .is_ok()
        );
    }
    #[test]
    fn unplug_and_authorization_changes_invalidate_pending_reads() {
        let mut d = Discovery::default();
        let token = d.apply(Ok(rows())).pop().unwrap();
        d.apply(Ok(vec![]));
        d.metadata(&token, Err(DiscoveryError::Timeout));
        assert_eq!(
            d.phones[token.device()].device.state,
            ConnectionState::Disconnected
        );
        assert!(d.phones[token.device()].metadata.is_none());
        let unauthorized = parse_devices("phone unauthorized usb:1 transport_id:8").unwrap();
        assert!(d.apply(Ok(unauthorized)).is_empty());
        assert_eq!(d.apply(Ok(rows())).len(), 1);
    }
}
