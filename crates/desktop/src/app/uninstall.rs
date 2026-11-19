//! Confirmed uninstall UI state; destructive work stays in infrastructure.
use super::*;
use android_hub_infrastructure::UninstallPlan;

#[derive(Default)]
pub(super) struct Dialog {
    pub plan: Option<UninstallPlan>,
    pub keep_backups: bool,
    pub confirmation: String,
    pub confirmed: bool,
    pub error: Option<String>,
}
impl Dialog {
    pub fn can_confirm(&self) -> bool {
        self.plan.is_some()
            && self.confirmation == "DELETE"
            && !self.confirmed
            && self.error.is_none()
    }
}
impl Hub {
    pub(super) fn open_uninstall(&mut self) -> Task<Message> {
        self.uninstall = Some(Dialog::default());
        self.busy = true;
        let backend = self.backend.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || backend.prepare_uninstall())
                    .await
                    .unwrap_or_else(|e| Err(e.to_string()))
            },
            Message::UninstallPrepared,
        )
    }
    pub(super) fn confirm_uninstall(&mut self) -> Task<Message> {
        if self.busy || self.stale || !self.uninstall.as_ref().is_some_and(Dialog::can_confirm) {
            return Task::none();
        }
        self.uninstall.as_mut().unwrap().confirmed = true;
        self.quit_requested = true;
        self.quit_error = None;
        self.connectivity = None;
        self.modal_return_focus = None;
        self.release_inputs();
        self.displays.clear();
        // update() begins the existing, observed engine-shutdown flow.
        Task::none()
    }
    pub(super) fn handoff_uninstall(&mut self) -> Task<Message> {
        let dialog = self.uninstall.as_ref().unwrap();
        let plan = dialog.plan.clone().unwrap();
        let keep = dialog.keep_backups;
        let backend = self.backend.clone();
        self.busy = true;
        self.quitting = true;
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || backend.launch_uninstall(plan, keep))
                    .await
                    .unwrap_or_else(|e| Err(e.to_string()))
            },
            Message::UninstallHandoff,
        )
    }
    pub(super) fn uninstall_failed(&mut self, error: String) {
        self.busy = false;
        self.quitting = false;
        self.quit_requested = false;
        self.quit_error = None;
        if let Some(dialog) = &mut self.uninstall {
            dialog.confirmed = false;
            dialog.confirmation.clear();
            dialog.error = Some(error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn dialog() -> Dialog {
        let plan: UninstallPlan = serde_json::from_value(serde_json::json!({
            "app":{"path":"/Applications/Android Hub.app","allocated_bytes":100},
            "data":[],"backups":"/fixture/backups","root":"/fixture","sdk":"/external",
            "root_identity":{"device":1,"inode":2},"app_identity":{"device":1,"inode":3},"entries":[]
        })).unwrap();
        Dialog {
            plan: Some(plan),
            ..Default::default()
        }
    }
    #[test]
    fn exact_confirmation_is_required_and_retention_change_clears_it() {
        let mut dialog = dialog();
        for text in ["", "delete", " DELETE", "DELETE "] {
            dialog.confirmation = text.into();
            assert!(!dialog.can_confirm());
        }
        dialog.confirmation = "DELETE".into();
        assert!(dialog.can_confirm());
        let (mut hub, _) = Hub::new();
        hub.busy = false;
        hub.stale = false;
        hub.uninstall = Some(dialog);
        let _ = hub.update(Message::UninstallKeepBackups(true));
        assert!(!hub.uninstall.as_ref().unwrap().can_confirm());
        assert!(hub.uninstall.as_ref().unwrap().keep_backups);
        let _ = hub.update(Message::CancelModal);
        assert!(hub.uninstall.is_none() && !hub.quit_requested);
    }
    #[test]
    fn confirmed_uninstall_routes_shutdown_errors_back_to_the_dialog() {
        let (mut hub, _) = Hub::new();
        hub.busy = false;
        hub.stale = false;
        let mut dialog = dialog();
        dialog.confirmation = "DELETE".into();
        hub.uninstall = Some(dialog);
        let _ = hub.update(Message::ConfirmUninstall);
        assert!(hub.quitting && hub.quit_requested);
        let _ = hub.update(Message::CancelModal);
        assert!(hub.uninstall.as_ref().unwrap().confirmed);
        let _ = hub.update(Message::ShutdownFinished(Err(
            "fixture shutdown failed".into()
        )));
        assert!(!hub.quitting && !hub.quit_requested && !hub.busy);
        assert!(hub.uninstall.as_ref().unwrap().error.is_some());
        assert!(!hub.uninstall.as_ref().unwrap().can_confirm());
    }
    #[test]
    fn busy_or_missing_preview_cannot_start_uninstall_and_cancelled_preview_stays_closed() {
        let (mut hub, _) = Hub::new();
        hub.uninstall = Some(dialog());
        hub.uninstall.as_mut().unwrap().confirmation = "DELETE".into();
        let _ = hub.confirm_uninstall();
        assert!(!hub.quit_requested);
        hub.cancel_modal();
        let _ = hub.update(Message::UninstallPrepared(Ok(dialog().plan.unwrap())));
        assert!(hub.uninstall.is_none() && !hub.busy);
        hub.uninstall = Some(Dialog {
            confirmation: "DELETE".into(),
            ..Default::default()
        });
        assert!(!hub.uninstall.as_ref().unwrap().can_confirm());
    }
    #[test]
    fn uninstall_modal_blocks_settings_writes_and_renders_both_languages() {
        let (mut hub, _) = Hub::new();
        hub.uninstall = Some(dialog());
        assert!(hub.modal_open());
        assert!(!Hub::modal_allows(&Message::SaveJava));
        assert!(!Hub::modal_allows(
            &Message::for_avd("hub-fixture", Message::StartDevice).unwrap()
        ));
        for lang in [Language::Vi, Language::En] {
            hub.language = lang;
            let _ = hub.view();
            assert_ne!(
                Language::En.tr("Gỡ cài đặt và xóa dữ liệu"),
                "Gỡ cài đặt và xóa dữ liệu"
            );
        }
    }
}
