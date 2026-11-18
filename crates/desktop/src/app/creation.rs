//! Form lifetime and a single explicit creation submission.
use super::*;
use android_hub_domain::CreateRequest;
use android_hub_infrastructure::{CreatedDevice, CreationEnvironment, CreationFailure};

pub(super) struct Editor {
    pub request: CreateRequest,
    pub advanced: bool,
    pub start_after: bool,
    pub environment: Option<CreationEnvironment>,
    pub error: Option<String>,
    pub retained: Option<Profile>,
    pub retry_blocked: bool,
}

#[derive(Clone, Debug)]
pub(super) enum Edit {
    Name(String),
    Ram(u32),
    Cpus(u32),
    Fps(u32),
    Resolution(Resolution),
    Orientation(Orientation),
    Advanced,
    StartAfter(bool),
}
impl Editor {
    pub fn edit(&mut self, edit: Edit) {
        if self.retry_blocked {
            return;
        }
        match edit {
            Edit::Name(value) => self.request.name = value,
            Edit::Ram(value) => self.request.ram_mb = value,
            Edit::Cpus(value) => self.request.cpus = value,
            Edit::Fps(value) => self.request.fps = value,
            Edit::Advanced => self.advanced = !self.advanced,
            Edit::StartAfter(value) => self.start_after = value,
            Edit::Resolution(value) => {
                (self.request.width, self.request.height) =
                    value.dimensions(self.request.width > self.request.height);
            }
            Edit::Orientation(value) => {
                let short = self.request.width.min(self.request.height);
                let long = self.request.width.max(self.request.height);
                (self.request.width, self.request.height) = if value == Orientation::Landscape {
                    (long, short)
                } else {
                    (short, long)
                };
            }
        }
    }
}
impl Hub {
    pub(super) fn open_creation(&mut self) -> Task<Message> {
        self.cancel_modal();
        self.creation = Some(Editor {
            request: CreateRequest::suggested(
                self.state
                    .as_ref()
                    .map(|s| s.profiles.as_slice())
                    .unwrap_or(&[]),
            ),
            advanced: false,
            start_after: false,
            environment: None,
            error: None,
            retained: None,
            retry_blocked: false,
        });
        Task::batch([
            self.probe_creation(),
            // Enter the dialog immediately and clear the previous shell focus.
            iced::widget::text_input::focus("creation-name"),
        ])
    }
    pub(super) fn probe_creation(&mut self) -> Task<Message> {
        let Some(editor) = &mut self.creation else {
            return Task::none();
        };
        editor.environment = None;
        self.creation_epoch = self.creation_epoch.wrapping_add(1);
        let epoch = self.creation_epoch;
        let backend = self.backend.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || backend.creation_environment())
                    .await
                    .unwrap_or_else(|e| CreationEnvironment {
                        components: Err(e.to_string()),
                        available_bytes: Err(e.to_string()),
                        minimum_bytes: 12 * 1024 * 1024 * 1024,
                    })
            },
            move |result| Message::CreationEnvironment(epoch, result),
        )
    }
    pub(super) fn creation_validation(&self) -> Option<String> {
        let editor = self.creation.as_ref()?;
        if let Err(error) = editor.request.validate() {
            return Some(error);
        }
        self.state
            .as_ref()
            .is_some_and(|s| {
                s.profiles.iter().any(|p| {
                    p.name
                        .trim()
                        .eq_ignore_ascii_case(editor.request.name.trim())
                })
            })
            .then(|| "Tên máy đã có trong danh sách. Chọn tên khác trước khi tạo.".into())
    }
    pub(super) fn can_create(&self) -> bool {
        !self.busy
            && !self.stale
            && self.creation_validation().is_none()
            && self.creation.as_ref().is_some_and(|e| {
                !e.retry_blocked
                    && e.environment
                        .as_ref()
                        .is_some_and(CreationEnvironment::ready)
            })
    }
    pub(super) fn submit_creation(&mut self) -> Task<Message> {
        if !self.can_create() {
            return Task::none();
        }
        let editor = self.creation.as_mut().unwrap();
        editor.error = None;
        let request = editor.request.clone();
        let start = editor.start_after;
        let epoch = self.creation_epoch;
        self.creation_in_flight = Some(epoch);
        self.busy = true;
        self.command_epoch = self.command_epoch.wrapping_add(1);
        self.notice = "Đang tạo máy mới…".into();
        let backend = self.backend.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || backend.create_with(request)).await
                .unwrap_or_else(|e| Err(CreationFailure {
                    message: format!("Tác vụ tạo máy kết thúc không rõ kết quả: {e}. Kiểm tra lịch sử trước khi tạo lại."),
                    retained_profile: None,
                    outcome_unknown: true,
                }))
            },
            move |result| Message::Created(epoch, start, result),
        )
    }
    pub(super) fn creation_finished(
        &mut self,
        start: bool,
        result: Result<CreatedDevice, CreationFailure>,
    ) -> Task<Message> {
        self.creation_in_flight = None;
        self.busy = false;
        match result {
            Err(error) => {
                if let Some(editor) = &mut self.creation {
                    editor.error = Some(error.message.clone());
                    editor.retry_blocked =
                        error.outcome_unknown || error.retained_profile.is_some();
                    editor.retained = error.retained_profile;
                    editor.environment = None; // Recheck prerequisites before an explicit retry.
                }
                self.stale = true;
                self.error("Chưa tạo được máy", error.message);
                self.receipt_task()
            }
            Ok(outcome) => {
                let id = outcome.profile.id.clone();
                self.creation = None;
                let snapshot_ok = outcome.snapshot.is_ok();
                let refresh = self.update_normal(Message::Loaded(
                    outcome.snapshot,
                    Work::Poll(self.command_epoch),
                ));
                self.feedback.push(
                    feedback::Kind::Success,
                    "Đã tạo máy Android độc lập",
                    Some(outcome.profile.name),
                    None,
                );
                if let Some(warning) = outcome.warning {
                    self.error("Máy đã được tạo. Không cần tạo lại.", warning);
                }
                if snapshot_ok {
                    self.choose(id.clone());
                } else {
                    self.notify(
                        feedback::Kind::Error,
                        "Máy đã được tạo. Làm mới trạng thái trước khi khởi động.",
                        None,
                    );
                }
                let receipts = self.receipt_task();
                let launch = if start && snapshot_ok && !self.quit_requested {
                    self.launch(Work::Start(id))
                } else {
                    Task::none()
                };
                Task::batch([refresh, receipts, launch])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> Snapshot {
        Snapshot {
            profiles: vec![Profile::standard(
                "hub-old".into(),
                "Android 1".into(),
                5660,
            )],
            runtime: Default::default(),
            android_versions: Default::default(),
        }
    }
    fn hub() -> Hub {
        let (mut hub, _) = Hub::new();
        let _ = hub.update(Message::Loaded(Ok(state()), Work::Initialize));
        hub
    }
    fn ready(hub: &mut Hub) {
        let _ = hub.update(Message::CreationEnvironment(
            hub.creation_epoch,
            CreationEnvironment {
                components: Ok(()),
                available_bytes: Ok(50 * 1024 * 1024 * 1024),
                minimum_bytes: 12 * 1024 * 1024 * 1024,
            },
        ));
    }
    fn submit(hub: &mut Hub, start: bool) -> u64 {
        let _ = hub.update(Message::Create);
        ready(hub);
        let _ = hub.update(Message::EditCreation(Edit::StartAfter(start)));
        let _ = hub.update(Message::ConfirmCreate);
        assert!(hub.busy);
        hub.creation_epoch
    }
    fn outcome() -> CreatedDevice {
        let p = Profile::standard("hub-new".into(), "Android 2".into(), 5662);
        let mut s = state();
        // Explicit ID must win even when the created profile is not last.
        s.profiles.insert(0, p.clone());
        CreatedDevice {
            profile: p,
            snapshot: Ok(s),
            warning: None,
        }
    }

    #[test]
    fn form_opens_without_submission_and_cancel_has_no_creation_side_effects() {
        let mut hub = hub();
        let before = hub.state.as_ref().unwrap().profiles.clone();
        let command = hub.command_epoch;
        let _ = hub.update(Message::Create);
        let form = hub.creation.as_ref().unwrap();
        assert_eq!(form.request.name, "Android 2");
        assert!(!form.start_after && !form.advanced);
        assert!(!hub.busy && !hub.can_create());
        let _ = hub.update(Message::EditCreation(Edit::Name("New game".into())));
        let _ = hub.update(Message::CancelModal);
        assert!(hub.creation.is_none() && hub.creation_in_flight.is_none());
        assert_eq!(hub.command_epoch, command);
        assert_eq!(hub.state.as_ref().unwrap().profiles, before);
    }

    #[test]
    fn creation_gates_validation_prerequisites_and_stale_environment_results() {
        let mut hub = hub();
        let _ = hub.update(Message::Create);
        let old = hub.creation_epoch;
        let _ = hub.update(Message::RefreshCreation);
        let _ = hub.update(Message::CreationEnvironment(
            old,
            CreationEnvironment {
                components: Ok(()),
                available_bytes: Ok(u64::MAX),
                minimum_bytes: 1,
            },
        ));
        assert!(hub.creation.as_ref().unwrap().environment.is_none());
        ready(&mut hub);
        assert!(hub.can_create());
        for name in [" ", "Android 1"] {
            let _ = hub.update(Message::EditCreation(Edit::Name(name.into())));
            let _ = hub.update(Message::ConfirmCreate);
            assert!(!hub.busy && !hub.can_create());
        }
        let _ = hub.update(Message::EditCreation(Edit::Name("Game".into())));
        hub.creation
            .as_mut()
            .unwrap()
            .environment
            .as_mut()
            .unwrap()
            .available_bytes = Ok(0);
        assert!(!hub.can_create());
        ready(&mut hub);
        hub.creation
            .as_mut()
            .unwrap()
            .environment
            .as_mut()
            .unwrap()
            .components = Err("missing Java".into());
        assert!(!hub.can_create());
    }

    #[test]
    fn submission_is_single_and_starts_only_the_returned_id() {
        let mut hub = hub();
        let epoch = submit(&mut hub, true);
        let command = hub.command_epoch;
        let _ = hub.update(Message::ConfirmCreate);
        let _ = hub.update(Message::CancelModal);
        let _ = hub.update(Message::EditCreation(Edit::Name("changed".into())));
        assert_eq!(hub.command_epoch, command);
        assert_eq!(hub.creation.as_ref().unwrap().request.name, "Android 2");
        let _ = hub.update(Message::Created(epoch, true, Ok(outcome())));
        assert!(hub.creation.is_none());
        assert_eq!(hub.selected.as_deref(), Some("hub-new"));
        assert_eq!(hub.lifecycle_action.as_ref().unwrap().id, "hub-new");
        let _ = hub.update(Message::Loaded(
            Err("start failed".into()),
            Work::Start("hub-new".into()),
        ));
        assert!(hub.creation.is_none() && !hub.busy);
        assert_eq!(hub.state.as_ref().unwrap().profiles.len(), 2);
        let command = hub.command_epoch;
        let _ = hub.update(Message::ConfirmCreate);
        let _ = hub.update(Message::Created(epoch, true, Ok(outcome())));
        assert_eq!(hub.command_epoch, command);
        assert!(!hub.busy); // Duplicate/stale completions cannot start another operation.
    }

    #[test]
    fn retained_or_unknown_failure_keeps_input_but_blocks_retry() {
        for unknown in [false, true] {
            let mut hub = hub();
            let epoch = submit(&mut hub, false);
            let retained = (!unknown).then(|| outcome().profile);
            let _ = hub.update(Message::Created(
                epoch,
                false,
                Err(CreationFailure {
                    message: "Interrupted create; inspect retained data".into(),
                    retained_profile: retained,
                    outcome_unknown: unknown,
                }),
            ));
            assert_eq!(hub.creation.as_ref().unwrap().request.name, "Android 2");
            assert!(hub.creation.as_ref().unwrap().error.is_some());
            let _ = hub.update(Message::Loaded(Ok(state()), Work::Poll(hub.command_epoch)));
            ready(&mut hub);
            let _ = hub.update(Message::ConfirmCreate);
            assert!(!hub.busy && !hub.can_create());
        }
    }

    #[test]
    fn committed_snapshot_failure_never_reopens_form_or_starts_guest() {
        let mut hub = hub();
        let epoch = submit(&mut hub, true);
        let mut result = outcome();
        result.snapshot = Err("ps failed".into());
        let _ = hub.update(Message::Created(epoch, true, Ok(result)));
        assert!(hub.creation.is_none() && hub.stale && !hub.busy);
        assert!(hub.lifecycle_action.is_none());
        assert!(
            hub.feedback
                .persistent
                .iter()
                .any(|n| n.title.contains("Máy đã được tạo"))
        );
    }

    #[test]
    fn quit_waits_for_creation_and_skips_optional_start() {
        let mut hub = hub();
        let epoch = submit(&mut hub, true);
        let _ = hub.update(Message::RequestQuit);
        assert!(hub.quit_requested && !hub.quitting && hub.busy);
        let _ = hub.update(Message::Created(epoch, true, Ok(outcome())));
        assert!(hub.quitting && !hub.busy);
        assert!(hub.lifecycle_action.is_none());
        assert_eq!(hub.state.as_ref().unwrap().profiles.len(), 2);
    }
}
