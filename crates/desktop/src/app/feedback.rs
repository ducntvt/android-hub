//! Presentation feedback. Diagnostics never become clipboard payloads implicitly.
use super::{Hub, Message, Work};
use crate::i18n::Language;
use iced::keyboard::key::Named;
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Success,
    Info,
    Error,
    Unknown,
}
impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Success => "Thành công",
            Self::Info => "Thông tin",
            Self::Error => "Cần xử lý",
            Self::Unknown => "Chưa xác định kết quả",
        }
    }
    fn persistent(self) -> bool {
        matches!(self, Self::Error | Self::Unknown)
    }
}

#[derive(Debug)]
pub(super) struct Notification {
    pub id: u64,
    pub kind: Kind,
    pub title: &'static str,
    pub subject: Option<String>,
    pub details: Option<String>,
}
impl Notification {
    // Deliberately omit raw diagnostics, device/file names and guest output.
    // Backend strings can contain paths or Android output; copying is an allowlist.
    pub fn safe_summary(&self, language: Language) -> String {
        format!(
            "Android Hub {}\n{}: {}\n{}",
            env!("CARGO_PKG_VERSION"),
            language.tr(self.kind.label()),
            language.tr(self.title),
            language.tr("Bản tóm tắt không chứa đường dẫn hoặc nội dung Android.")
        )
    }
}

#[derive(Default)]
pub(super) struct Feedback {
    next_id: u64,
    pub toast: Option<Notification>,
    pub persistent: VecDeque<Notification>,
}
impl Feedback {
    pub fn push(
        &mut self,
        kind: Kind,
        title: &'static str,
        subject: Option<String>,
        details: Option<String>,
    ) {
        self.publish(kind, title, subject, details, kind.persistent());
    }
    pub fn setup_notice(&mut self, details: String) {
        self.publish(
            Kind::Info,
            "Thông tin thiết lập Hub",
            None,
            Some(details),
            true,
        );
    }
    fn publish(
        &mut self,
        kind: Kind,
        title: &'static str,
        subject: Option<String>,
        details: Option<String>,
        persistent: bool,
    ) {
        if persistent
            && self.persistent.iter().any(|n| {
                n.kind == kind && n.title == title && n.subject == subject && n.details == details
            })
        {
            return; // A failing poll does not accumulate identical alerts.
        }
        self.next_id = self.next_id.wrapping_add(1);
        let item = Notification {
            id: self.next_id,
            kind,
            title,
            subject,
            details,
        };
        if persistent {
            self.persistent.push_back(item);
        } else {
            self.toast = Some(item);
        }
    }
    pub fn find(&self, id: u64) -> Option<&Notification> {
        self.persistent
            .iter()
            .chain(self.toast.iter())
            .find(|n| n.id == id)
    }
    pub fn pin_details(&mut self, id: u64) {
        if self.toast.as_ref().is_some_and(|n| n.id == id) {
            self.persistent.push_front(self.toast.take().unwrap());
        }
    }
    pub fn close_details(&mut self, id: u64) {
        // Transient feedback can be acknowledged by closing its details. Errors
        // keep their alert until explicitly dismissed.
        self.persistent
            .retain(|n| n.id != id || n.kind.persistent());
        self.expire(id);
    }
    pub fn dismiss(&mut self, id: u64) {
        self.persistent.retain(|n| n.id != id);
        self.expire(id);
    }
    pub fn expire(&mut self, id: u64) {
        if self.toast.as_ref().is_some_and(|n| n.id == id) {
            self.toast = None;
        }
    }
}

impl Hub {
    pub(super) fn notify(&mut self, kind: Kind, title: &'static str, details: Option<String>) {
        self.feedback.push(kind, title, None, details);
    }
    pub(super) fn error(&mut self, title: &'static str, error: String) {
        self.notify(Kind::Error, title, Some(error));
    }
    pub(super) fn work_feedback(&mut self, work: &Work, error: Option<&String>) {
        if matches!(work, Work::Poll(_)) && error.is_none() {
            return;
        }
        if matches!(work, Work::Initialize) && error.is_none() {
            return;
        }
        let (success, failure, subject) = match work {
            Work::Initialize | Work::Poll(_) => ("Sẵn sàng", "Không đọc được trạng thái máy", None),
            Work::Start(id) => (
                "Đã mở engine. Android có thể cần thêm thời gian để boot.",
                "Không khởi động được máy",
                self.device_name(id),
            ),
            Work::Stop(id, _) => ("Đã dừng máy", "Chưa dừng được máy", self.device_name(id)),
            Work::Save(p, _) => (
                "Đã lưu. Cấu hình áp dụng khi mở máy.",
                "Không lưu được cấu hình",
                Some(p.name.clone()),
            ),
            Work::InstallApk(p) => (
                "Đã cài APK. Hub không tự mở ứng dụng.",
                "Chưa xác nhận cài APK thành công",
                Some(p.name.clone()),
            ),
            Work::Delete(p) => (
                "Đã xóa vĩnh viễn máy",
                "Chưa xóa hoàn tất. Kiểm tra máy và dữ liệu đã giữ trong Cài đặt Hub.",
                Some(p.profile.name.clone()),
            ),
            Work::Purge(p) => (
                "Đã xóa vĩnh viễn máy",
                "Chưa xóa hoàn tất. Làm mới danh sách để thử lại.",
                Some(p.device.profile.name.clone()),
            ),
            Work::UndoRemove(p) => (
                "Đã đưa máy trở lại. Chọn máy và Khởi động để dùng.",
                "Chưa xác nhận đưa máy trở lại",
                Some(p.profile.name.clone()),
            ),
        };
        let kind = if error.is_some() {
            if matches!(
                work,
                Work::InstallApk(_) | Work::Delete(_) | Work::UndoRemove(_) | Work::Purge(_)
            ) {
                Kind::Unknown
            } else {
                Kind::Error
            }
        } else {
            Kind::Success
        };
        self.feedback.push(
            kind,
            if error.is_some() { failure } else { success },
            subject,
            error.cloned(),
        );
    }
    pub(super) fn device_name(&self, id: &str) -> Option<String> {
        self.state
            .as_ref()?
            .profiles
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.name.clone())
    }
    pub(super) fn modal_open(&self) -> bool {
        self.image_removal.is_some()
            || self.creation.is_some()
            || self.apk_request.is_some()
            || self.pending_stop.is_some()
            || self.pending_apk.is_some()
            || self.pending_restore.is_some()
            || self.pending_delete.is_some()
            || self.pending_undo.is_some()
            || self.pending_purge.is_some()
            || self.detail_id.is_some()
            || self.pending_navigation.is_some()
    }
    pub(super) fn cancel_modal(&mut self) {
        if self.creation.is_some() && self.busy && !self.quit_requested {
            return;
        }
        if self.creation_image_review() {
            self.sdk_review_open = false;
            self.sdk_cache_reset = None;
            self.accepted_licenses.clear();
        }
        self.creation = None;
        self.apk_request = None;
        self.apk_hover = false;
        if self.pending_navigation.is_some() {
            self.saving_navigation = None;
        }
        self.pending_navigation = None;
        self.pending_stop = None;
        self.pending_apk = None;
        self.pending_restore = None;
        self.pending_delete = None;
        self.delete_plan = None;
        self.delete_error = None;
        self.delete_backup = None;
        self.pending_undo = None;
        self.pending_purge = None;
        self.image_removal = None;
        if let Some(id) = self.detail_id.take() {
            self.feedback.close_details(id);
        }
    }
    pub(super) fn modal_primary_action(&self) -> Option<Message> {
        if self.pending_navigation.is_some() {
            return self.can_save_settings().then_some(Message::SaveAndContinue);
        }
        if let Some(id) = self.detail_id {
            return Some(Message::CopyNotification(id));
        }
        if self.busy || self.stale {
            return None;
        }
        if self.image_removal.is_some() {
            Some(Message::ConfirmImageRemoval)
        } else if self.pending_stop.is_some() {
            Some(Message::Stop)
        } else if self.pending_apk.is_some() {
            Some(Message::ConfirmApk)
        } else if self.pending_restore.is_some() && !self.running() {
            Some(Message::ConfirmRestore)
        } else if self.pending_delete.is_some()
            && self.delete_plan.is_some()
            && self.delete_target_current()
        {
            Some(Message::ConfirmDelete)
        } else if self.pending_purge.is_some() {
            Some(Message::ConfirmPurge)
        } else if self.pending_undo.is_some() {
            Some(Message::ConfirmUndoRemove)
        } else {
            None
        }
    }
    pub(super) fn modal_transition(&mut self, was_open: bool) {
        if self.modal_open() && !was_open {
            self.modal_return_focus = self.input_focus.as_ref().and_then(|id| {
                let d = self.displays.get(id)?;
                Some((id.clone(), d.pid, d.session.input_generation()?))
            });
            self.release_inputs();
            self.modal_focus = 0; // Cancel is the explicit initial focus.
        } else if was_open && !self.modal_open() {
            self.release_inputs();
            if let Some((id, pid, generation)) = self.modal_return_focus.take()
                && !self.busy
                && !self.stale
                && self.displays.get(&id).is_some_and(|d| {
                    d.pid == pid
                        && d.session.input_generation() == Some(generation)
                        && d.image.is_some()
                        && d.error.is_none()
                })
            {
                self.input_focus = Some(id);
            }
        }
    }
    pub(super) fn modal_allows(message: &Message) -> bool {
        matches!(
            message,
            Message::ApkPicked(..)
                | Message::ApkInspected(..)
                | Message::ChooseApkTarget(..)
                | Message::EditCreation(..)
                | Message::RefreshCreation
                | Message::CreationEnvironment(..)
                | Message::ConfirmCreate
                | Message::Created(..)
                | Message::CreateSdk
                | Message::FormTab(..)
                | Message::Tick
                | Message::FrameReady(..)
                | Message::ReleaseInputs
                | Message::BootObserved(..)
                | Message::ConnectivityFinished(..)
                | Message::Loaded(..)
                | Message::ReceiptsLoaded(..)
                | Message::RemovedLoaded(..)
                | Message::PurgePrepared(..)
                | Message::DeletePrepared(..)
                | Message::DeleteBackupFinished(..)
                | Message::BackupBeforeDelete
                | Message::BackupsLoaded(..)
                | Message::StorageLoaded(..)
                | Message::ManagedSdksLoaded(..)
                | Message::ImagesLoaded(..)
                | Message::BackupFinished(..)
                | Message::RestoreFinished(..)
                | Message::SdkSetupFinished(..)
                | Message::ImageSetupFinished(..)
                | Message::ImageRemovalPrepared(..)
                | Message::ImageRemovalFinished(..)
                | Message::ConfirmImageRemoval
                | Message::SdkCacheResetFinished(..)
                | Message::ManagedSdkSaved(..)
                | Message::JavaSaved(..)
                | Message::SdkSaved(..)
                | Message::SetLanguage(..)
                | Message::SetAppearance(..)
                | Message::ModalKey(..)
                | Message::ModalBackTab
                | Message::ExpireToast(..)
                | Message::DismissNotification(..)
                | Message::CancelModal
                | Message::SaveAndContinue
                | Message::DiscardAndContinue
                | Message::CopyNotification(..)
                | Message::Stop
                | Message::ConfirmApk
                | Message::ConfirmRestore
                | Message::ConfirmDelete
                | Message::ConfirmUndoRemove
                | Message::ConfirmPurge
        )
    }
    pub(super) fn modal_key(&mut self, key: Named) -> iced::Task<Message> {
        if !self.modal_open() {
            return iced::Task::none();
        }
        match key {
            Named::Escape => self.cancel_modal(),
            Named::Tab => self.modal_focus = (self.modal_focus + 1) % self.modal_action_count(),
            Named::Enter | Named::Space => {
                if self.apk_request.is_some() && self.modal_focus > 0 {
                    if let Some(target) = self.apk_targets().get(self.modal_focus - 1) {
                        return self.update_normal(Message::ChooseApkTarget(target.id.clone()));
                    }
                } else if self.pending_delete.is_some() && self.modal_focus == 1 {
                    return self.update_normal(Message::BackupBeforeDelete);
                } else if self.pending_navigation.is_some() && self.modal_focus == 1 {
                    return self.update_normal(Message::DiscardAndContinue);
                } else if self.modal_focus > 0 {
                    if let Some(action) = self.modal_primary_action() {
                        return self.update_normal(action);
                    }
                } else {
                    self.cancel_modal();
                }
            }
            _ => {}
        }
        iced::Task::none()
    }
}
