//! Direct device deletion: prepare, optional backup, then explicit confirmation.
use super::*;

impl Hub {
    pub(super) fn delete_target_current(&self) -> bool {
        self.pending_delete.as_ref().is_some_and(|p| {
            self.delete_plan
                .as_ref()
                .is_none_or(|plan| plan.profile == *p)
                && self.state.as_ref().is_some_and(|s| {
                    s.profiles.iter().find(|v| v.id == p.id) == Some(p)
                        && s.runtime.get(&p.id).is_none_or(|r| r.pid.is_none())
                })
        })
    }

    pub(super) fn ask_delete(&mut self, target: DeviceId) -> Task<Message> {
        let Some(id) = target.as_emulator() else {
            return Task::none();
        };
        let id = id.as_str().to_owned();
        let Some(profile) = self
            .state
            .as_ref()
            .and_then(|s| s.profiles.iter().find(|p| p.id == id))
            .cloned()
        else {
            return Task::none();
        };
        if self
            .state
            .as_ref()
            .and_then(|s| s.runtime.get(&id))
            .is_some_and(|r| r.pid.is_some())
        {
            self.error("Dừng máy trước khi xóa", profile.name);
            return Task::none();
        }
        self.clear_backup_selection();
        self.pending_delete = Some(profile.clone());
        self.delete_epoch = self.delete_epoch.wrapping_add(1);
        self.command_epoch = self.command_epoch.wrapping_add(1);
        self.busy = true;
        self.notice = "Đang kiểm tra dung lượng…".into();
        let epoch = self.delete_epoch;
        let backend = self.backend.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    backend.emulator(&profile.id)?.prepare_delete(&profile)
                })
                .await
                .unwrap_or_else(|e| Err(e.to_string()))
            },
            move |result| Message::DeletePrepared(epoch, result),
        )
    }

    pub(super) fn delete_prepared(&mut self, epoch: u64, result: Result<DeletePlan, String>) {
        if epoch != self.delete_epoch {
            return;
        }
        self.busy = false;
        self.notice = "Sẵn sàng".into();
        if self.pending_delete.is_none() || self.quit_requested {
            return;
        }
        match result {
            Ok(plan)
                if self.pending_delete.as_ref() == Some(&plan.profile)
                    && self.delete_target_current() =>
            {
                self.delete_plan = Some(plan)
            }
            Ok(_) => {
                self.delete_error = Some(
                    self.language
                        .tr("Máy đã thay đổi; đóng hộp thoại và thử lại.")
                        .into(),
                )
            }
            Err(error) => self.delete_error = Some(error),
        }
    }

    pub(super) fn confirm_delete(&mut self) -> Task<Message> {
        if self.pending_delete.is_none() {
            return Task::none();
        }
        if !self.delete_target_current() {
            self.cancel_modal();
            self.notify(
                feedback::Kind::Error,
                "Máy đã thay đổi; hãy xác nhận xóa lại",
                None,
            );
            return Task::none();
        }
        let Some(plan) = self.delete_plan.take() else {
            return Task::none();
        };
        self.storage = None;
        self.launch(Work::Delete(plan))
    }

    pub(super) fn backup_before_delete(&mut self) -> Task<Message> {
        if !self.delete_target_current() || self.delete_backup.is_some() {
            return Task::none();
        }
        let Some(plan) = self.delete_plan.clone() else {
            return Task::none();
        };
        self.busy = true;
        self.delete_error = None;
        self.command_epoch = self.command_epoch.wrapping_add(1);
        self.notice = "Đang sao lưu và kiểm tra dữ liệu…".into();
        let epoch = self.delete_epoch;
        let backend = self.backend.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    backend
                        .emulator(&plan.profile.id)?
                        .backup_before_delete(&plan)
                })
                .await
                .unwrap_or_else(|e| Err(e.to_string()))
            },
            move |result| Message::DeleteBackupFinished(epoch, result),
        )
    }

    pub(super) fn delete_backup_finished(&mut self, epoch: u64, result: Result<PathBuf, String>) {
        if epoch != self.delete_epoch {
            return;
        }
        self.busy = false;
        self.notice = "Sẵn sàng".into();
        if self.pending_delete.is_none() || self.quit_requested {
            return;
        }
        match result {
            Ok(path) => self.delete_backup = Some(path),
            Err(error) => self.delete_error = Some(error),
        }
        // A backup never authorizes deletion, including after a failure.
        self.modal_focus = 0;
    }
}
