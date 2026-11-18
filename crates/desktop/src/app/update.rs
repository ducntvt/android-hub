//! State transitions and asynchronous command dispatch.
use super::*;
use feedback::Kind;

impl Hub {
    pub(super) fn update(&mut self, message: Message) -> Task<Message> {
        // Reuse the existing low-frequency refresh; never spawn a theme helper.
        if matches!(message, Message::Tick | Message::SetAppearance(_))
            && let Some(dark) = appearance::system_dark()
        {
            self.system_dark = dark;
        }
        if !self.quit_requested
            && !self.quitting
            && self.has_unsaved_settings()
            && (!self.busy || matches!(message, Message::RequestQuit))
            && (!self.modal_open() || matches!(message, Message::RequestQuit))
            && let Some(navigation) = self.navigation_for(&message)
        {
            let was_open = self.modal_open();
            self.cancel_modal();
            self.pending_navigation = Some(navigation);
            self.modal_focus = 0;
            self.modal_transition(was_open);
            return Task::none();
        }
        match message {
            Message::RequestQuit => {
                if self.quitting {
                    return Task::none();
                }
                self.quit_requested = true;
                self.connectivity = None;
                self.cancel_modal();
                self.modal_return_focus = None;
                self.quit_error = None;
                self.release_inputs();
                self.displays.clear();
                if let Some(control) = &self.sdk_setup {
                    control.cancel();
                }
            }
            Message::CancelQuit if !self.quitting => {
                self.quit_requested = false;
                self.quit_error = None;
                self.polling = false;
                self.stale = true;
                self.command_epoch = self.command_epoch.wrapping_add(1);
                return Task::none();
            }
            Message::ShutdownFinished(result) if self.quitting => {
                self.quitting = false;
                match result {
                    Ok(()) => return iced::exit(),
                    Err(error) => self.quit_error = Some(error),
                }
                return Task::none();
            }
            Message::ShutdownProgress(event) if self.quitting => {
                if let Some(previous) = self.shutdown_progress.iter_mut().find(|p| p.id == event.id)
                {
                    *previous = event;
                } else {
                    self.shutdown_progress.push(event);
                }
                return Task::none();
            }
            _ if self.quitting => return Task::none(),
            _ => {
                // Finish existing work, but do not accept new operations while closing.
                if self.quit_requested
                    && !matches!(
                        message,
                        Message::Loaded(..)
                            | Message::DeletePrepared(..)
                            | Message::DeleteBackupFinished(..)
                            | Message::Created(..)
                            | Message::BackupFinished(..)
                            | Message::RestoreFinished(..)
                            | Message::SdkSetupFinished(..)
                            | Message::ImageSetupFinished(..)
                            | Message::ImageRemovalPrepared(..)
                            | Message::ImageRemovalFinished(..)
                            | Message::SdkCacheResetFinished(..)
                            | Message::ManagedSdkSaved(..)
                            | Message::JavaSaved(..)
                            | Message::SdkSaved(..)
                    )
                {
                    return Task::none();
                }
                let was_modal = self.modal_open();
                let creation_sdk_action = self.creation_image_review()
                    && matches!(
                        message,
                        Message::StartSdkSetup
                            | Message::CancelSdkSetup
                            | Message::CloseSdkReview
                            | Message::ReviewLicense(_)
                            | Message::AcceptLicense(..)
                            | Message::AskSdkCacheReset(_)
                            | Message::CancelSdkCacheReset
                            | Message::ConfirmSdkCacheReset
                    );
                if was_modal && !Self::modal_allows(&message) && !creation_sdk_action {
                    return Task::none();
                }
                let previous_toast = self.feedback.toast.as_ref().map(|n| n.id);
                let task = self.update_normal(message);
                self.reconcile_connectivity();
                self.modal_transition(was_modal);
                let timer = match self.feedback.toast.as_ref().map(|n| n.id) {
                    Some(id) if Some(id) != previous_toast => Task::perform(
                        async move {
                            tokio::time::sleep(Duration::from_secs(6)).await;
                            id
                        },
                        Message::ExpireToast,
                    ),
                    _ => Task::none(),
                };
                return Task::batch([task, timer, self.begin_shutdown_if_ready()]);
            }
        }
        self.begin_shutdown_if_ready()
    }
    fn begin_shutdown_if_ready(&mut self) -> Task<Message> {
        if !self.quit_requested || self.busy || self.quitting || self.quit_error.is_some() {
            return Task::none();
        }
        self.quitting = true;
        self.displays.clear();
        self.boot_checks.clear();
        self.shutdown_progress.clear();
        let backend = self.backend.clone();
        Task::run(lifecycle::shutdown_updates(backend), |message| message)
    }
    pub(super) fn update_normal(&mut self, message: Message) -> Task<Message> {
        if let Message::SetLanguage(language) = message {
            self.language = language;
            if let Err(error) = language.save(&self.backend.root) {
                self.error("Không lưu được ngôn ngữ", error.to_string());
            }
            return Task::none();
        }

        match message {
            Message::CheckConnectivity => self.start_connectivity(),
            Message::CancelConnectivity => self.connectivity = None,
            Message::ToggleConnectivityDetails => {
                if let Some(check) = &mut self.connectivity {
                    check.details = !check.details;
                }
            }
            Message::CopyConnectivity => {
                self.reconcile_connectivity();
                if let Some(Ok(report)) = self.connectivity.as_ref().and_then(|c| c.result.as_ref())
                {
                    return iced::clipboard::write(report.details());
                }
            }
            Message::ConnectivityFinished(epoch, result) => {
                self.reconcile_connectivity();
                if let Some(check) = &mut self.connectivity
                    && check.epoch == epoch
                    && check.result.is_none()
                    && (result.as_ref().is_err()
                        || result
                            .as_ref()
                            .is_ok_and(|report| report.boot_id == check.boot))
                {
                    check.result = Some(result);
                }
            }
            Message::SetAppearance(preference) => {
                self.appearance = preference;
                if let Err(error) = preference.save(&self.backend.root) {
                    self.error("Không lưu được giao diện", error.to_string());
                }
            }
            Message::OpenPreferences if !self.busy => {
                self.release_inputs();
                self.preferences_open = true;
                self.settings_open = false;
                self.history_open = false;
                self.sdk_open = false;
                self.storage_open = false;
                self.removed_open = false;
                self.clear_backup_selection();
            }
            Message::ClosePreferences => self.preferences_open = false,
            Message::AskRestore | Message::AskDelete(_) if self.has_unsaved_settings() => {
                self.notify(
                    Kind::Error,
                    "Lưu hoặc hủy thay đổi trước khi thao tác dữ liệu.",
                    None,
                );
            }
            Message::DiscardSettings if !self.busy => self.reload_settings(),
            Message::SaveAndContinue if self.pending_navigation.is_some() => {
                return self.save_settings(true);
            }
            Message::DiscardAndContinue if !self.busy => {
                if let Some(navigation) = self.pending_navigation.take() {
                    self.reload_settings();
                    return self.execute_navigation(navigation);
                }
            }
            Message::ModalKey(key) => return self.modal_key(key),
            Message::ModalBackTab if self.modal_open() => {
                let count = self.modal_action_count();
                self.modal_focus = (self.modal_focus + count - 1) % count;
            }
            Message::DismissNotification(id) => {
                self.feedback.dismiss(id);
                if self.detail_id == Some(id) {
                    self.detail_id = None;
                }
            }
            Message::ExpireToast(id) => {
                // Details remain readable until the user closes them.
                if self.detail_id != Some(id) {
                    self.feedback.expire(id);
                }
            }
            Message::ShowNotificationDetails(id) if self.feedback.find(id).is_some() => {
                self.feedback.pin_details(id);
                self.detail_id = Some(id);
            }
            Message::CancelModal => self.cancel_modal(),
            Message::CopyNotification(id) => {
                if let Some(n) = self.feedback.find(id) {
                    return iced::clipboard::write(n.safe_summary(self.language));
                }
            }
            Message::OpenHistory => {
                self.preferences_open = false;
                self.history_open = true;
                self.history_unread = false;
                self.release_inputs();
            }
            Message::CloseHistory => self.history_open = false,
            Message::BootObserved(id, pid, epoch, finished, result) => {
                if let Some(check) = self.boot_checks.get_mut(&id)
                    && check.pid == pid
                    && check.epoch == epoch
                {
                    check.result = Some(result);
                    check.finished = finished;
                }
            }
            Message::RetryBoot(id) => {
                if let Some(check) = self.boot_checks.get_mut(&id) {
                    check.retry();
                }
            }
            Message::OpenRemoved if !self.busy && !self.stale && !self.removed_loading => {
                self.preferences_open = false;
                self.history_open = false;
                self.clear_backup_selection();
                self.pending_apk = None;
                self.pending_stop = None;
                self.settings_open = false;
                self.storage_open = false;
                self.sdk_open = false;
                self.removed_open = true;
                self.removed_loading = true;
                let epoch = self.removed_epoch;
                let backend = self.backend.clone();
                return Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || backend.removed_devices())
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    move |result| Message::RemovedLoaded(epoch, result),
                );
            }
            Message::CloseRemoved => {
                self.removed_open = false;
                self.clear_backup_selection();
            }
            Message::RemovedLoaded(epoch, result)
                if epoch == self.removed_epoch && self.removed_open =>
            {
                self.removed_loading = false;
                match result {
                    Ok(catalog) => self.removed = catalog,
                    Err(error) => self.removed_error = Some(error),
                }
            }
            Message::PickRemoved(info) if !self.busy && !self.removed_loading => {
                if self.removed.entries.contains(&info) {
                    self.chosen_removed = Some(info);
                    self.pending_undo = None;
                    self.pending_purge = None;
                }
            }
            Message::ToggleDeviceMenu(id) if !self.busy => {
                self.device_menu = if self.device_menu.as_ref() == Some(&id) {
                    None
                } else {
                    Some(id)
                };
            }
            Message::AskDelete(id) if !self.busy && !self.stale => return self.ask_delete(id),
            Message::DeletePrepared(epoch, result) => self.delete_prepared(epoch, result),
            Message::BackupBeforeDelete if !self.busy && !self.stale => {
                return self.backup_before_delete();
            }
            Message::DeleteBackupFinished(epoch, result) => {
                self.delete_backup_finished(epoch, result)
            }
            Message::ConfirmDelete if !self.busy && !self.stale => return self.confirm_delete(),
            Message::AskPurge
                if !self.busy && !self.stale && !self.removed_loading && self.removed_open =>
            {
                if let Some(info) = self.chosen_removed.clone()
                    && self.removed.entries.contains(&info)
                {
                    self.pending_undo = None;
                    self.pending_purge = None;
                    self.removed_loading = true;
                    let epoch = self.removed_epoch;
                    let backend = self.backend.clone();
                    return Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || backend.prepare_purge(&info))
                                .await
                                .unwrap_or_else(|e| Err(e.to_string()))
                        },
                        move |result| Message::PurgePrepared(epoch, result),
                    );
                }
            }
            Message::PurgePrepared(epoch, result)
                if epoch == self.removed_epoch && self.removed_open =>
            {
                self.removed_loading = false;
                match result {
                    Ok(plan)
                        if !self.busy
                            && !self.stale
                            && !self.modal_open()
                            && self.chosen_removed.as_ref() == Some(&plan.device)
                            && self.removed.entries.contains(&plan.device) =>
                    {
                        self.pending_purge = Some(plan);
                    }
                    Ok(_) => {}
                    Err(error) => self.error("Không thể chuẩn bị xóa máy", error),
                }
            }
            Message::ConfirmPurge if !self.busy && !self.stale && self.removed_open => {
                if let Some(plan) = self.pending_purge.take()
                    && self.chosen_removed.as_ref() == Some(&plan.device)
                    && self.removed.entries.contains(&plan.device)
                {
                    self.storage = None;
                    return self.launch(Work::Purge(plan));
                }
            }
            Message::AskUndoRemove if !self.busy && !self.stale && !self.removed_loading => {
                if let Some(info) = self.chosen_removed.clone()
                    && info.unavailable.is_none()
                    && self.removed.entries.contains(&info)
                {
                    self.pending_delete = None;
                    self.pending_restore = None;
                    self.pending_apk = None;
                    self.pending_stop = None;
                    self.pending_undo = Some(info);
                }
            }
            Message::ConfirmUndoRemove if !self.busy && !self.stale => {
                if let Some(info) = self.pending_undo.take()
                    && self.chosen_removed.as_ref() == Some(&info)
                    && self.removed.entries.contains(&info)
                {
                    return self.launch(Work::UndoRemove(info));
                }
            }
            Message::LoadBackups if !self.busy && !self.stale && !self.catalog_loading => {
                if let Some(id) = self.selected.clone() {
                    self.clear_backup_selection();
                    self.catalog_loading = true;
                    let epoch = self.catalog_epoch;
                    let backend = self.backend.clone();
                    return Task::perform(
                        async move {
                            let returned = id.clone();
                            let result =
                                tokio::task::spawn_blocking(move || backend.list_backups(&id))
                                    .await
                                    .unwrap_or_else(|e| Err(e.to_string()));
                            (returned, epoch, result)
                        },
                        |(id, epoch, result)| Message::BackupsLoaded(id, epoch, result),
                    );
                }
            }
            Message::BackupsLoaded(id, epoch, result)
                if self.selected.as_ref() == Some(&id) && epoch == self.catalog_epoch =>
            {
                self.catalog_loading = false;
                self.catalog_loaded = true;
                match result {
                    Ok(catalog) => {
                        self.backups = catalog;
                    }
                    Err(error) => self.catalog_error = Some(error),
                }
            }
            Message::PickBackup(info) if !self.busy => {
                if self.backups.entries.contains(&info) {
                    self.chosen_backup = Some(info);
                    self.pending_restore = None;
                }
            }
            Message::AskRestore if !self.busy && !self.stale && !self.running() => {
                if let Some(backup) = self.chosen_backup.clone()
                    && self.backups.entries.contains(&backup)
                    && let Some(profile) = self.state.as_ref().and_then(|s| {
                        s.profiles
                            .iter()
                            .find(|p| Some(&p.id) == self.selected.as_ref())
                    })
                    && profile.id == backup.profile_id
                {
                    self.pending_delete = None;
                    self.pending_undo = None;
                    self.pending_apk = None;
                    self.pending_stop = None;
                    self.pending_restore = Some(PendingRestore {
                        profile: profile.clone(),
                        backup,
                    });
                }
            }
            Message::ConfirmRestore if !self.busy && !self.stale => {
                if let Some(pending) = self.pending_restore.take() {
                    let current = self
                        .state
                        .as_ref()
                        .and_then(|s| s.profiles.iter().find(|p| p.id == pending.profile.id));
                    if current != Some(&pending.profile)
                        || self.selected.as_ref() != Some(&pending.profile.id)
                        || self.running()
                    {
                        self.notify(
                            Kind::Error,
                            "Máy đã thay đổi; hãy chọn và xác nhận backup lại",
                            None,
                        );
                    } else {
                        self.command_epoch = self.command_epoch.wrapping_add(1);
                        self.busy = true;
                        self.clear_backup_selection();
                        self.notice = "Đang kiểm tra và khôi phục dữ liệu…".into();
                        let backend = self.backend.clone();
                        return Task::perform(
                            async move {
                                tokio::task::spawn_blocking(move || {
                                    backend.restore(
                                        &pending.profile.id,
                                        &pending.backup.path,
                                        &pending.backup.manifest_sha256,
                                    )
                                })
                                .await
                                .unwrap_or_else(|e| Err(e.to_string()))
                            },
                            Message::RestoreFinished,
                        );
                    }
                }
            }
            Message::RestoreFinished(result) => {
                self.busy = false;
                match result {
                    Ok(outcome) => {
                        self.state = Some(outcome.snapshot);
                        self.stale = false;
                        self.refresh_draft = false;
                        if let Some(id) = self.selected.clone() {
                            self.draft = None;
                            self.choose(id);
                        }
                        self.notify(
                            Kind::Success,
                            "Đã khôi phục dữ liệu. Bản cũ vẫn được giữ.",
                            Some(outcome.previous_data.display().to_string()),
                        );
                    }
                    Err(error) => {
                        self.notify(
                            Kind::Unknown,
                            "Chưa xác nhận khôi phục thành công",
                            Some(error),
                        );
                        self.stale = true;
                        self.refresh_draft = true;
                    }
                }
                return self.receipt_task();
            }
            Message::ReceiptsLoaded(epoch, restore, removal, creation)
                if epoch == self.receipt_epoch =>
            {
                let mut changed = false;
                match restore {
                    Ok(receipts) => {
                        changed |= self
                            .receipts
                            .iter()
                            .map(|r| (&r.path, r.token, r.rolled_back, &r.profile_name))
                            .ne(receipts
                                .iter()
                                .map(|r| (&r.path, r.token, r.rolled_back, &r.profile_name)));
                        self.receipts = receipts;
                    }
                    Err(error) => {
                        self.error("Không đọc được lịch sử restore", error);
                    }
                }
                match removal {
                    Ok(history) => {
                        changed |= self.removal_history != history;
                        self.removal_history = history;
                    }
                    Err(error) => {
                        self.error("Không đọc được lịch sử lưu trữ máy", error);
                    }
                }
                match creation {
                    Ok(history) => {
                        changed |= self.creation_history != history;
                        self.creation_history = history;
                    }
                    Err(error) => {
                        self.error("Không đọc được lịch sử tạo máy", error);
                    }
                }
                if changed
                    && (!self.receipts.is_empty()
                        || !self.removal_history.is_empty()
                        || !self.creation_history.is_empty())
                {
                    self.history_unread = !self.history_open;
                }
            }
            Message::DismissReceipts => {
                self.receipt_epoch = self.receipt_epoch.wrapping_add(1);
                self.history_unread = false;
                self.history_open = false;
            }
            Message::Backup if !self.busy && !self.stale && !self.running() => {
                if let Some(id) = self.selected.clone() {
                    self.command_epoch = self.command_epoch.wrapping_add(1);
                    self.busy = true;
                    self.clear_backup_selection();
                    self.pending_apk = None;
                    self.notice = "Đang sao lưu và kiểm tra dữ liệu…".into();
                    let backend = self.backend.clone();
                    return Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || backend.backup(&id))
                                .await
                                .unwrap_or_else(|e| Err(format!("Backup thất bại: {e}")))
                        },
                        Message::BackupFinished,
                    );
                }
            }
            Message::BackupFinished(result) => {
                self.busy = false;
                match result {
                    Ok(path) => self.notify(
                        Kind::Success,
                        "Đã sao lưu dữ liệu",
                        Some(path.display().to_string()),
                    ),
                    Err(error) => self.error("Không sao lưu được dữ liệu", error),
                }
            }

            Message::ApkHover(hover) => self.apk_hover = hover && !self.busy,
            Message::PickApk(id) if !self.busy && !self.stale => return self.pick_apk(id),
            Message::DropApk(path) if !self.busy => return self.dropped_apk(path),
            Message::ApkPicked(token, path) => return self.picked_apk(token, path),
            Message::ApkInspected(token, result) => self.inspected_apk(token, result),
            Message::ChooseApkTarget(id) if !self.busy && !self.stale => self.choose_apk_target(id),
            Message::ConfirmApk if !self.busy && !self.stale => return self.confirm_apk(),

            message @ (Message::OpenSdk
            | Message::StartSdkSetup
            | Message::AskSdkCacheReset(..)
            | Message::CancelSdkCacheReset
            | Message::ConfirmSdkCacheReset
            | Message::SdkCacheResetFinished(..)
            | Message::CancelSdkSetup
            | Message::SdkSetupFinished(..)
            | Message::ImageSetupFinished(..)
            | Message::OpenSdkReview
            | Message::OpenImageReview(..)
            | Message::AskImageRemoval(..)
            | Message::ImageRemovalPrepared(..)
            | Message::ConfirmImageRemoval
            | Message::ImageRemovalFinished(..)
            | Message::RefreshImages
            | Message::ImagesLoaded(..)
            | Message::CloseSdkReview
            | Message::ReviewLicense(..)
            | Message::AcceptLicense(..)
            | Message::CloseSdk
            | Message::RefreshManagedSdks
            | Message::ManagedSdksLoaded(..)
            | Message::UseManagedSdk(..)
            | Message::ManagedSdkSaved(..)
            | Message::ToggleSdkDetails
            | Message::SdkPath(..)
            | Message::JavaPath(..)
            | Message::SaveJava
            | Message::JavaSaved(..)
            | Message::SaveSdk
            | Message::SdkSaved(..)) => return self.update_sdk(message),
            Message::ToggleStorageDetails => self.storage_details = !self.storage_details,
            Message::ToggleBackupDetails => self.backup_details = !self.backup_details,
            Message::ToggleArchiveDetails => self.archive_details = !self.archive_details,
            Message::MeasureStorage if !self.busy && !self.stale && !self.storage_loading => {
                self.preferences_open = false;
                self.history_open = false;
                self.sdk_open = false;
                self.storage_open = true;
                self.pending_apk = None;
                self.pending_stop = None;
                self.settings_open = false;
                self.removed_open = false;
                self.clear_backup_selection();
                self.storage_loading = true;
                self.storage_error = None;
                self.storage = None;
                let epoch = self.command_epoch;
                let backend = self.backend.clone();
                return Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || backend.disk_usage())
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    move |result| Message::StorageLoaded(epoch, result),
                );
            }
            Message::CloseStorage => self.storage_open = false,
            Message::StorageLoaded(epoch, result) => {
                self.storage_loading = false;
                if epoch == self.command_epoch {
                    match result {
                        Ok(usage) => self.storage = Some(usage),
                        Err(error) => self.storage_error = Some(error),
                    }
                } else {
                    self.storage = None;
                }
            }
            Message::Search(value) => {
                self.device_menu = None;
                self.search = value;
            }
            Message::FocusSearch => {
                self.release_inputs();
                self.sidebar_collapsed = false;
                return iced::widget::text_input::focus("device-search");
            }
            Message::ToggleSettings => {
                self.preferences_open = false;
                self.history_open = false;
                self.sdk_open = false;
                self.storage_open = false;
                self.settings_open = !self.settings_open;
                if self.removed_open {
                    self.clear_backup_selection();
                }
                self.removed_open = false;
            }
            Message::Zoom(value) if value.is_finite() => {
                self.release_inputs();
                self.zoom = value.clamp(0.5, 2.);
            }
            Message::ToggleSidebar => {
                self.release_inputs();
                self.sidebar_collapsed = !self.sidebar_collapsed;
            }
            Message::PickSlot(slot, id) if !self.busy && slot < 2 => {
                if self
                    .state
                    .as_ref()
                    .is_some_and(|s| s.profiles.iter().any(|p| p.id == id))
                {
                    self.release_inputs();
                    self.slots.assign(slot, id.clone());
                    self.select_metadata(id);
                    self.sync_displays();
                }
            }
            Message::StartDevice(id) if !self.busy && !self.stale => {
                if self.has_unsaved_settings() {
                    self.notify(Kind::Error, "Lưu cấu hình trước khi khởi động máy.", None);
                    return Task::none();
                }
                if self.state.as_ref().is_some_and(|s| {
                    s.profiles.iter().any(|p| p.id == id)
                        && s.runtime.get(&id).is_none_or(|r| r.pid.is_none())
                }) {
                    self.release_inputs();
                    self.activate_visible(id.clone());
                    return self.launch(Work::Start(id));
                }
            }
            Message::ReleaseInputs => self.release_inputs(),
            Message::SetSplit(split) => {
                if self.split != split {
                    return self.update_normal(Message::SplitView);
                }
            }
            Message::SplitView => {
                self.release_inputs();
                self.split = !self.split;
                if self.split {
                    let other = 1 - self.slots.active;
                    if self.slots.ids[other].is_none()
                        && let Some(state) = &self.state
                        && let Some(profile) = state
                            .profiles
                            .iter()
                            .filter(|p| self.selected.as_ref() != Some(&p.id))
                            .max_by_key(|p| {
                                state.runtime.get(&p.id).is_some_and(|r| r.pid.is_some())
                            })
                    {
                        self.slots.ids[other] = Some(profile.id.clone());
                    }
                }
                self.sync_displays();
            }
            Message::FrameReady(id, pid) => {
                let mut input_feedback = None;
                if let Some(d) = self.displays.get_mut(&id).filter(|d| d.pid == pid) {
                    let (frame, error, feedback) = d.session.take_frame();
                    let generation = d.session.input_generation();
                    if (generation != d.input_generation || (error.is_some() && d.error.is_none()))
                        && let Some(check) = self.boot_checks.get_mut(&id)
                    {
                        check.retry();
                    }
                    d.input_generation = generation;
                    if let Some(feedback) = feedback {
                        input_feedback = Some(feedback);
                    }
                    d.error = error;
                    if d.error.is_some() {
                        d.image = None;
                        if self.input_focus.as_ref() == Some(&id) {
                            self.input_focus = None;
                        }
                    }
                    if let Some(frame) = frame {
                        d.size = (frame.width, frame.height);
                        d.rotation = frame.rotation;
                        d.image = Some(image::Handle::from_rgba(
                            frame.width,
                            frame.height,
                            frame.pixels,
                        ));
                    }
                }
                if input_feedback.is_some() {
                    self.feedback.push(
                        Kind::Success,
                        "Đã chép. Nhấn giữ trong Android để dán.",
                        self.device_name(&id),
                        None,
                    );
                }
            }
            Message::SendClipboard(id) if !self.busy && !self.stale => {
                if let Some(display) = self.displays.get(&id)
                    && display.image.is_some()
                    && display.error.is_none()
                {
                    let pid = display.pid;
                    let epoch = self.input_epoch;
                    let Some(generation) = display.session.input_generation() else {
                        self.notify(
                            Kind::Error,
                            "Màn hình đang kết nối lại; hãy thử lại sau",
                            None,
                        );
                        return Task::none();
                    };
                    return iced::clipboard::read().map(move |text| {
                        Message::ClipboardRead(id.clone(), pid, epoch, generation, text)
                    });
                }
            }
            Message::ClipboardRead(id, pid, epoch, generation, text) => {
                if !self.busy
                    && !self.stale
                    && epoch == self.input_epoch
                    && let Some(display) = self.displays.get(&id)
                    && display.pid == pid
                    && display.image.is_some()
                    && display.error.is_none()
                {
                    match text {
                        Some(text) => match display
                            .session
                            .send_in_generation(Input::Clipboard(text), generation)
                        {
                            Ok(()) => {
                                self.feedback.push(
                                    Kind::Info,
                                    "Đang gửi văn bản tới Android…",
                                    self.device_name(&id),
                                    None,
                                );
                            }
                            Err(error) => {
                                self.feedback.push(
                                    Kind::Error,
                                    "Không gửi được văn bản tới Android",
                                    self.device_name(&id),
                                    Some(error),
                                );
                            }
                        },
                        None => {
                            self.feedback.push(
                                Kind::Info,
                                "Clipboard không có văn bản đọc được",
                                self.device_name(&id),
                                None,
                            );
                        }
                    }
                }
            }
            Message::DeviceInput(id, epoch, generation, input) => {
                if epoch != self.input_epoch || generation.is_none() {
                    return Task::none();
                }
                if matches!(input, Input::Mouse { down: true, .. })
                    && self.busy
                    && self.selected.as_ref() != Some(&id)
                {
                    return Task::none();
                }
                if epoch == self.input_epoch
                    && let Some(generation) = generation
                    && let Some(d) = self.displays.get(&id)
                    && d.image.is_some()
                    && d.error.is_none()
                {
                    let pressed = matches!(input, Input::Mouse { down: true, .. });
                    match d.session.send_in_generation(input, generation) {
                        Ok(()) if pressed => {
                            self.input_focus = Some(id.clone());
                            if !self.busy && self.selected.as_ref() != Some(&id) {
                                self.activate_visible(id);
                            }
                        }
                        Ok(()) => {}
                        Err(error) => self.error("Thao tác chưa hoàn tất", error),
                    }
                }
            }
            Message::Tick if !self.busy && !self.polling => {
                self.polling = true;
                return Self::task(self.backend.clone(), Work::Poll(self.command_epoch));
            }
            Message::Loaded(result, work) => {
                let poll = matches!(work, Work::Poll(_));
                if poll {
                    self.polling = false;
                    // A poll started before a lifecycle/config command must never
                    // overwrite its result, including after that command completes.
                    if matches!(work, Work::Poll(epoch) if epoch != self.command_epoch) {
                        return Task::none();
                    }
                } else {
                    self.busy = false;
                }
                let succeeded = result.is_ok();
                self.work_feedback(&work, result.as_ref().err());
                if let Work::Start(id) | Work::Stop(id, _) = &work {
                    self.lifecycle_action = None;
                    if let Err(error) = &result {
                        self.lifecycle_errors.insert(id.clone(), error.clone());
                    } else {
                        self.lifecycle_errors.remove(id);
                    }
                }
                match result {
                    Ok(state) => {
                        self.stale = false;
                        for id in self.boot_checks.keys() {
                            if state.runtime.get(id).and_then(|r| r.pid).is_none()
                                && state.profiles.iter().any(|p| &p.id == id)
                                && !matches!(&work, Work::Stop(stopped, _) if stopped == id)
                            {
                                self.lifecycle_errors.entry(id.clone()).or_insert_with(|| {
                                    "Engine không còn chạy; chưa xác định nguyên nhân.".into()
                                });
                            }
                        }
                        self.state = Some(state);
                        self.refresh_clean_settings();
                        self.reconcile_boot_checks();
                        if self.refresh_draft {
                            self.refresh_draft = false;
                            if let Some(id) = self.selected.clone() {
                                self.draft = None;
                                self.choose(id);
                            }
                        }
                        self.sync_displays();
                        if matches!(work, Work::Initialize)
                            && let Some(note) = &self.location_notice
                        {
                            self.feedback.setup_notice(note.clone());
                        }
                        if !self.has_unsaved_settings()
                            && self.selected.as_ref().is_some_and(|id| {
                                !self
                                    .state
                                    .as_ref()
                                    .unwrap()
                                    .profiles
                                    .iter()
                                    .any(|p| &p.id == id)
                            })
                        {
                            self.selected = None;
                            self.draft = None;
                            self.clear_backup_selection();
                            self.release_inputs();
                        }
                        if matches!(work, Work::UndoRemove(_)) || self.selected.is_none() {
                            if let Some(p) = self.state.as_ref().and_then(|s| {
                                if matches!(work, Work::UndoRemove(_)) {
                                    s.profiles.last()
                                } else {
                                    s.profiles.first()
                                }
                            }) {
                                self.choose(p.id.clone());
                            }
                        } else if matches!(work, Work::Save(..))
                            && let Some(id) = self.selected.clone()
                        {
                            self.draft = None;
                            self.choose(id);
                        }
                    }
                    Err(_) => {
                        if poll
                            || matches!(
                                work,
                                Work::Initialize | Work::Delete(_) | Work::UndoRemove(_)
                            )
                        {
                            self.stale = true;
                        }
                    }
                }
                if matches!(work, Work::Save(..)) {
                    if succeeded {
                        // A later Quit supersedes earlier navigation. Consume both intents.
                        let navigation = self
                            .pending_navigation
                            .take()
                            .or(self.saving_navigation.take());
                        if let Some(navigation) = navigation {
                            return self.execute_navigation(navigation);
                        }
                    } else {
                        self.saving_navigation = None;
                        self.pending_navigation = None;
                        self.settings_open = true;
                        self.history_open = false;
                        self.stale = true;
                    }
                }
                if matches!(work, Work::Purge(_)) {
                    // Refresh even after a partial deletion: its marker keeps it retryable.
                    let epoch = self.removed_epoch;
                    let backend = self.backend.clone();
                    self.removed_open = true;
                    self.removed_loading = true;
                    return Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || backend.removed_devices())
                                .await
                                .unwrap_or_else(|e| Err(e.to_string()))
                        },
                        move |result| Message::RemovedLoaded(epoch, result),
                    );
                }
                if matches!(
                    work,
                    Work::Initialize | Work::Delete(_) | Work::UndoRemove(_)
                ) {
                    return self.receipt_task();
                }
            }
            Message::Select(id) if !self.busy => {
                self.choose(id);
            }
            Message::Create if !self.busy && !self.stale => return self.open_creation(),
            Message::RefreshCreation if !self.busy => return self.probe_creation(),
            Message::CreationEnvironment(epoch, environment) if epoch == self.creation_epoch => {
                if let Some(editor) = &mut self.creation {
                    editor.environment = Some(environment);
                }
            }
            Message::EditCreation(edit) if !self.busy => {
                if let Some(editor) = &mut self.creation {
                    if editor.image_review {
                        return Task::none();
                    }
                    let version_changed = matches!(edit, creation::Edit::Version(_));
                    editor.edit(edit);
                    if version_changed {
                        return self.probe_creation();
                    }
                }
            }
            Message::ConfirmCreate => return self.submit_creation(),
            Message::Created(epoch, start, result) if self.creation_in_flight == Some(epoch) => {
                return self.creation_finished(start, result);
            }
            Message::CreateSdk if !self.busy && self.creation.is_some() => {
                self.cancel_modal();
                return self.update_normal(Message::OpenSdk);
            }
            Message::FormTab(backwards) if self.creation.is_some() => {
                return shortcuts::focus(backwards, true);
            }
            Message::FocusNext(backwards) => {
                self.release_inputs();
                return shortcuts::focus(backwards, false);
            }
            Message::AskStop if !self.busy && !self.stale && self.running() => {
                self.pending_delete = None;
                self.pending_undo = None;
                self.pending_restore = None;
                self.pending_apk = None;
                if let Some(id) = &self.selected
                    && let Some(state) = &self.state
                    && let Some(profile) = state.profiles.iter().find(|p| &p.id == id)
                    && let Some(pid) = state.runtime.get(id).and_then(|r| r.pid)
                {
                    self.pending_stop = Some(PendingStop {
                        id: id.clone(),
                        name: profile.name.clone(),
                        pid,
                    });
                }
            }
            Message::Stop if !self.busy && !self.stale => {
                if let Some(pending) = self.pending_stop.take() {
                    let current_pid = self
                        .state
                        .as_ref()
                        .and_then(|s| s.runtime.get(&pending.id))
                        .and_then(|r| r.pid);
                    if self.selected.as_ref() == Some(&pending.id)
                        && current_pid == Some(pending.pid)
                    {
                        return self.launch(Work::Stop(pending.id, pending.pid));
                    }
                    self.notify(Kind::Error, "Máy đã đổi phiên; hãy xác nhận lại.", None);
                }
            }
            Message::Save => return self.save_settings(false),
            Message::Name(v) if !self.busy && !self.stale && !self.running() => {
                if let Some(p) = &mut self.draft {
                    p.name = v;
                }
            }
            Message::Ram(v) if !self.busy && !self.stale && !self.running() => {
                if let Some(p) = &mut self.draft {
                    p.ram_mb = v;
                }
            }
            Message::Cpus(v) if !self.busy && !self.stale && !self.running() => {
                if let Some(p) = &mut self.draft {
                    p.cpus = v;
                }
            }
            Message::Fps(v) if !self.busy && !self.stale && !self.running() => {
                if let Some(p) = &mut self.draft {
                    p.fps = v;
                }
            }
            Message::Orientation(v) if !self.busy && !self.stale && !self.running() => {
                if let Some(p) = &mut self.draft {
                    (p.width, p.height) = p.dimensions_for_orientation(v == Orientation::Landscape);
                }
            }
            Message::Resolution(v) if !self.busy && !self.stale && !self.running() => {
                if let Some(p) = &mut self.draft {
                    let landscape = p.width > p.height;
                    (p.width, p.height) = v.dimensions(landscape);
                }
            }
            _ => {}
        }
        Task::none()
    }
}
