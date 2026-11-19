//! Unsaved settings and deferred context changes. Never retain/replay guest input.
use super::*;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Navigation {
    Select(DeviceId),
    Slot(usize, String),
    Activate(String),
    Start(DeviceId),
    Create,
    CloseSettings,
    Removed,
    Sdk,
    Storage,
    History,
    Preferences,
    Quit,
}

impl Hub {
    pub(super) fn settings_conflict(&self) -> bool {
        self.draft_base.as_ref().is_some_and(|base| {
            self.state
                .as_ref()
                .and_then(|s| s.profiles.iter().find(|p| p.id == base.id))
                != Some(base)
        })
    }
    pub(super) fn can_save_settings(&self) -> bool {
        self.selected_allows(DeviceOperation::ConfigureAvd)
            && !self.busy
            && !self.stale
            && !self.running()
            && self.has_unsaved_settings()
            && !self.settings_conflict()
            && self.draft.as_ref().is_some_and(|p| p.validate().is_ok())
    }
    pub(super) fn reload_settings(&mut self) {
        self.draft = self
            .state
            .as_ref()
            .and_then(|s| {
                s.profiles
                    .iter()
                    .find(|p| Some(p.id.as_str()) == self.selected_avd_id())
            })
            .cloned();
        self.draft_base = self.draft.clone();
    }
    pub(super) fn refresh_clean_settings(&mut self) {
        if !self.has_unsaved_settings() {
            self.reload_settings();
        }
    }
    pub(super) fn navigation_for(&self, message: &Message) -> Option<Navigation> {
        let other = |id: &String| {
            self.selected_avd_id() != Some(id.as_str())
                && self
                    .state
                    .as_ref()
                    .is_some_and(|s| s.profiles.iter().any(|p| &p.id == id))
        };
        match message {
            Message::Select(id) if self.selected.as_ref() != Some(id) => {
                Some(Navigation::Select(id.clone()))
            }
            Message::PickSlot(slot, id) if *slot < 2 && other(id) => {
                Some(Navigation::Slot(*slot, id.clone()))
            }
            Message::StartDevice(id)
                if id.capabilities().allows(DeviceOperation::StartEmulator) =>
            {
                Some(Navigation::Start(id.clone()))
            }
            Message::DeviceInput(id, epoch, Some(generation), Input::Mouse { down: true, .. })
                if other(id)
                    && *epoch == self.input_epoch
                    && self.displays.get(id).is_some_and(|d| {
                        d.image.is_some()
                            && d.error.is_none()
                            && d.session.input_generation() == Some(*generation)
                    }) =>
            {
                Some(Navigation::Activate(id.clone()))
            }
            Message::Create => Some(Navigation::Create),
            Message::ToggleSettings if self.settings_open => Some(Navigation::CloseSettings),
            Message::OpenRemoved => Some(Navigation::Removed),
            Message::OpenSdk => Some(Navigation::Sdk),
            Message::MeasureStorage => Some(Navigation::Storage),
            Message::OpenHistory => Some(Navigation::History),
            Message::OpenPreferences => Some(Navigation::Preferences),
            Message::RequestQuit => Some(Navigation::Quit),
            _ => None,
        }
    }
    pub(super) fn execute_navigation(&mut self, navigation: Navigation) -> Task<Message> {
        let target = match &navigation {
            Navigation::Slot(_, id) | Navigation::Activate(id) => Some(id.as_str()),
            Navigation::Select(id) | Navigation::Start(id) => {
                id.as_emulator().map(EmulatorId::as_str)
            }
            _ => None,
        };
        if target.is_some_and(|id| {
            !self
                .state
                .as_ref()
                .is_some_and(|s| s.profiles.iter().any(|p| p.id == id))
        }) {
            self.notify(
                feedback::Kind::Error,
                "Máy đích không còn trong danh sách.",
                None,
            );
            return Task::none();
        }
        match navigation {
            Navigation::Select(id) => self.update_normal(Message::Select(id)),
            Navigation::Slot(slot, id) => self.update_normal(Message::PickSlot(slot, id)),
            Navigation::Activate(id) => {
                self.release_inputs();
                self.activate_visible(id);
                self.sync_displays();
                Task::none()
            }
            Navigation::Start(id) => self.update_normal(Message::StartDevice(id)),
            Navigation::Create => self.update_normal(Message::Create),
            Navigation::CloseSettings => {
                self.settings_open = false;
                Task::none()
            }
            Navigation::Removed => self.update_normal(Message::OpenRemoved),
            Navigation::Sdk => self.update_normal(Message::OpenSdk),
            Navigation::Storage => self.update_normal(Message::MeasureStorage),
            Navigation::History => self.update_normal(Message::OpenHistory),
            Navigation::Preferences => self.update_normal(Message::OpenPreferences),
            Navigation::Quit => self.update(Message::RequestQuit),
        }
    }
    pub(super) fn save_settings(&mut self, continue_navigation: bool) -> Task<Message> {
        if !self.can_save_settings() {
            return Task::none();
        }
        self.saving_navigation = if continue_navigation {
            self.pending_navigation.take()
        } else {
            None
        };
        let profile = self.draft.clone().unwrap();
        let expected = self.draft_base.clone().unwrap();
        self.launch(Work::Save(profile, expected))
    }
}

pub(super) fn name_error(profile: &Profile) -> Option<&'static str> {
    (profile.name.trim().is_empty()
        || profile.name.chars().count() > 48
        || profile.name.chars().any(char::is_control))
    .then_some("Tên máy phải có 1–48 ký tự, không có ký tự điều khiển.")
}
