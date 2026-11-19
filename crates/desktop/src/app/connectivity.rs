use super::*;
use android_hub_infrastructure::ConnectivityReport;
use iced::Subscription;

pub(super) struct Check {
    pub profile: Profile,
    pub pid: u32,
    pub boot: String,
    pub epoch: u64,
    pub started: u64,
    pub result: Option<Result<ConnectivityReport, String>>,
    pub details: bool,
}
impl Hub {
    pub(super) fn connectivity_target(&self) -> Option<(Profile, u32, String)> {
        if self.stale || self.busy || self.quit_requested || !self.settings_open {
            return None;
        }
        let id = self.selected_avd_id()?;
        let state = self.state.as_ref()?;
        let profile = state.profiles.iter().find(|p| p.id == id)?.clone();
        let pid = state.runtime.get(id)?.pid?;
        let check = self.boot_checks.get(id)?;
        let boot = check.result.as_ref()?.as_ref().ok()?;
        (check.pid == pid && boot.completed).then(|| (profile, pid, boot.boot_id.clone()))
    }
    pub(super) fn reconcile_connectivity(&mut self) {
        let valid = self.connectivity.as_ref().is_none_or(|check| {
            self.connectivity_target()
                .is_some_and(|(profile, pid, boot)| {
                    profile.id == check.profile.id && pid == check.pid && boot == check.boot
                })
        });
        if !valid {
            self.connectivity = None;
        }
    }
    pub(super) fn start_connectivity(&mut self) {
        if self
            .connectivity
            .as_ref()
            .is_some_and(|c| c.result.is_none())
        {
            return;
        }
        let Some((profile, pid, boot)) = self.connectivity_target() else {
            return;
        };
        self.connectivity_epoch = self.connectivity_epoch.wrapping_add(1);
        self.connectivity = Some(Check {
            profile,
            pid,
            boot,
            epoch: self.connectivity_epoch,
            started: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            result: None,
            details: false,
        });
    }
    pub(super) fn connectivity_subscription(&self) -> Subscription<Message> {
        let Some(check) = &self.connectivity else {
            return Subscription::none();
        };
        if check.result.is_some() || self.connectivity_target().is_none() {
            return Subscription::none();
        }
        let (backend, profile, pid, boot, epoch) = (
            self.backend.clone(),
            check.profile.clone(),
            check.pid,
            check.boot.clone(),
            check.epoch,
        );
        Subscription::run_with_id(
            ("connectivity", epoch),
            iced::futures::stream::once(async move {
                let result = backend.check_connectivity(&profile, pid, &boot).await;
                Message::ConnectivityFinished(epoch, result)
            }),
        )
    }
}
