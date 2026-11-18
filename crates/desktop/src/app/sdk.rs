//! SDK selection, consent, setup and result handling after shared update guards.
use super::*;
use feedback::Kind;

impl Hub {
    pub(super) fn update_sdk(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AskImageRemoval(version)
                if !self.busy && self.sdk_open && !self.images_loading =>
            {
                self.busy = true;
                self.command_epoch = self.command_epoch.wrapping_add(1);
                let epoch = self.command_epoch;
                let backend = self.backend.clone();
                self.notice = "Đang kiểm tra image trước khi gỡ…".into();
                return Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || backend.prepare_image_removal(version))
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    move |result| Message::ImageRemovalPrepared(epoch, result),
                );
            }
            Message::ImageRemovalPrepared(epoch, result)
                if epoch == self.command_epoch && self.busy =>
            {
                self.busy = false;
                self.notice = "Sẵn sàng".into();
                if !self.quit_requested {
                    match result {
                        Ok(plan) => self.image_removal = Some(plan),
                        Err(error) => self.error("Không thể gỡ image Android", error),
                    }
                }
            }
            Message::ConfirmImageRemoval if !self.busy && !self.stale => {
                let Some(plan) = self.image_removal.take() else {
                    return Task::none();
                };
                self.busy = true;
                self.command_epoch = self.command_epoch.wrapping_add(1);
                let epoch = self.command_epoch;
                let backend = self.backend.clone();
                self.notice = "Đang gỡ image Android…".into();
                return Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || backend.remove_android_image(&plan))
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    move |result| Message::ImageRemovalFinished(epoch, result),
                );
            }
            Message::ImageRemovalFinished(epoch, result)
                if epoch == self.command_epoch && self.busy =>
            {
                self.busy = false;
                self.notice = "Sẵn sàng".into();
                match result {
                    Ok(()) => self.notify(Kind::Success, "Đã gỡ image Android", None),
                    Err(error) => self.error("Gỡ image chưa hoàn tất", error),
                }
                return self.update(Message::RefreshImages);
            }

            Message::RefreshImages if !self.busy && !self.images_loading => {
                self.images_loading = true;
                self.images_epoch = self.images_epoch.wrapping_add(1);
                let epoch = self.images_epoch;
                let backend = self.backend.clone();
                return Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || backend.android_images())
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    move |result| Message::ImagesLoaded(epoch, result),
                );
            }
            Message::ImagesLoaded(epoch, result) if epoch == self.images_epoch => {
                self.images_loading = false;
                self.images = result;
            }
            Message::OpenImageReview(version)
                if !self.busy && (self.sdk_open || self.creation_image_review()) =>
            {
                self.sdk_review_open = true;
                self.sdk_setup_result = None;
                self.sdk_cache_reset = None;
                self.review_license = 0;
                self.accepted_licenses.clear();
                match android_hub_infrastructure::SdkPlan::for_image(version) {
                    Ok(plan) => {
                        self.sdk_plan = Some(plan);
                        self.sdk_plan_error = None;
                    }
                    Err(error) => {
                        self.sdk_plan = None;
                        self.sdk_plan_error = Some(error);
                    }
                }
            }
            Message::ImageSetupFinished(epoch, result)
                if epoch == self.command_epoch && self.sdk_setup.is_some() =>
            {
                self.sdk_setup = None;
                self.busy = false;
                match result {
                    Ok(installed) => {
                        if self.creation_image_review() {
                            let editor = self.creation.as_mut().unwrap();
                            let create = editor.create_after_image && !self.quit_requested;
                            editor.image_review = false;
                            editor.create_after_image = false;
                            self.sdk_review_open = false;
                            self.accepted_licenses.clear();
                            self.sdk_setup_result = None;
                            if create {
                                if editor.request.android_version != installed.identity.version {
                                    editor.error =
                                        Some("Image trả về không khớp phiên bản đã chọn".into());
                                    return self.probe_creation();
                                }
                                return self.launch_creation(Some(installed.identity));
                            }
                            self.notify(
                                Kind::Info,
                                "Image đã cài; chưa tạo máy. Có thể dùng lại khi tạo máy mới.",
                                None,
                            );
                            return self.probe_creation();
                        }
                        self.accepted_licenses.clear();
                        self.sdk_review_open = false;
                        let title = if installed.reused {
                            "Image đã sẵn sàng"
                        } else {
                            "Đã cài image Android"
                        };
                        self.sdk_setup_result = Some(format!(
                            "{}: {}",
                            self.language.tr(title),
                            installed.identity.version
                        ));
                        self.notify(Kind::Success, title, None);
                    }
                    Err(error) => {
                        self.error("Không cài được image Android", error.clone());
                        self.sdk_setup_result = Some(error);
                    }
                }
                return self.update(Message::RefreshImages);
            }
            Message::OpenSdk if !self.busy => {
                self.preferences_open = false;
                self.history_open = false;
                self.sdk_open = true;
                self.sdk_details = false;
                self.sdk_review_open = false;
                self.sdk_cache_reset = None;
                self.accepted_licenses.clear();
                match self.backend.java_home() {
                    Ok(path) => self.java_draft = path.to_string_lossy().into_owned(),
                    Err(error) => self.error("Thao tác chưa hoàn tất", error),
                }
                self.sdk_draft = self.backend.sdk.to_string_lossy().into_owned();
                self.settings_open = false;
                self.storage_open = false;
                self.removed_open = false;
                self.pending_apk = None;
                self.pending_stop = None;
                self.clear_backup_selection();
                self.managed_loading = false;
                return Task::batch([
                    self.update(Message::RefreshManagedSdks),
                    self.update(Message::RefreshImages),
                ]);
            }
            Message::StartSdkSetup
                if !self.busy
                    && (self.sdk_open || self.creation_image_review())
                    && self.sdk_review_open
                    && self.sdk_cache_reset.is_none() =>
            {
                let Some(plan) = self.sdk_plan.clone() else {
                    return Task::none();
                };
                if let Err(error) = plan.approved_packages(&self.accepted_licenses) {
                    self.sdk_setup_result = Some(error);
                    return Task::none();
                }
                let java = PathBuf::from(self.java_draft.trim());
                if plan.image_version().is_none() && !java.is_absolute() {
                    self.sdk_setup_result =
                        Some("Nhập Java home tuyệt đối trước khi bắt đầu".into());
                    return Task::none();
                }
                let accepted = self.accepted_licenses.clone();
                let control = android_hub_infrastructure::SdkSetupControl::default();
                self.sdk_setup = Some(control.clone());
                self.sdk_setup_result = None;
                self.busy = true;
                self.command_epoch = self.command_epoch.wrapping_add(1);
                self.notice = if plan.image_version().is_some() {
                    "Đang cài image Android…"
                } else {
                    "Đang chuẩn bị SDK mới…"
                }
                .into();
                let backend = self.backend.clone();
                if plan.image_version().is_some() {
                    if let Some(editor) = self.creation.as_mut().filter(|e| e.image_review) {
                        if Some(editor.request.android_version) != plan.image_version() {
                            self.busy = false;
                            self.sdk_setup = None;
                            return Task::none();
                        }
                        editor.create_after_image = true;
                    }
                    let epoch = self.command_epoch;
                    return Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                backend.prepare_android_image(&plan, &accepted, &control)
                            })
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                        },
                        move |result| Message::ImageSetupFinished(epoch, result),
                    );
                }
                return Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            backend.prepare_sdk(&plan, &accepted, &java, &control)
                        })
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    Message::SdkSetupFinished,
                );
            }
            Message::AskSdkCacheReset(index)
                if !self.busy
                    && (self.sdk_open || self.creation_image_review())
                    && self.sdk_review_open =>
            {
                if self
                    .sdk_plan
                    .as_ref()
                    .is_some_and(|plan| plan.archive(index).is_some())
                {
                    self.sdk_cache_reset = Some(index);
                }
            }
            Message::CancelSdkCacheReset if !self.busy => self.sdk_cache_reset = None,
            Message::ConfirmSdkCacheReset
                if !self.busy
                    && (self.sdk_open || self.creation_image_review())
                    && self.sdk_review_open =>
            {
                if let Some(archive) = self
                    .sdk_cache_reset
                    .take()
                    .and_then(|index| self.sdk_plan.as_ref().and_then(|plan| plan.archive(index)))
                    .cloned()
                {
                    self.busy = true;
                    self.command_epoch = self.command_epoch.wrapping_add(1);
                    self.sdk_setup_result = Some("Đang chuyển cache cũ sang thư mục riêng…".into());
                    let backend = self.backend.clone();
                    return Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                backend.quarantine_sdk_archive(&archive)
                            })
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                        },
                        Message::SdkCacheResetFinished,
                    );
                }
            }
            Message::SdkCacheResetFinished(result) => {
                self.busy = false;
                match &result {
                    Ok(_) => self.notify(Kind::Success, "Đã giữ cache SDK cũ", None),
                    Err(error) => self.error("Không chuyển được cache SDK", error.clone()),
                }
                let message = match result {
                    Ok(path) => format!(
                        "Đã giữ cache cũ tại {}. Bấm Tải và chuẩn bị SDK để tải lại gói này.",
                        path.display()
                    ),
                    Err(error) => error,
                };
                self.sdk_setup_result = Some(message);
            }
            Message::CancelSdkSetup => {
                if let Some(editor) = &mut self.creation {
                    editor.create_after_image = false;
                }
                if let Some(control) = &self.sdk_setup {
                    control.cancel();
                    self.sdk_setup_result =
                        Some("Đang hủy; chờ thao tác hiện tại kết thúc…".into());
                }
            }
            Message::SdkSetupFinished(result) => {
                self.sdk_setup = None;
                self.busy = false;
                match result {
                    Ok(sdk) => {
                        self.accepted_licenses.clear();
                        self.sdk_review_open = false;
                        self.notice = format!(
                            "Đã chuẩn bị SDK tại {}. Chọn Kiểm tra và sử dụng trong danh sách khi bạn muốn đổi SDK.",
                            sdk.path.display()
                        );
                        self.sdk_setup_result = Some(self.notice.clone());
                        self.notify(Kind::Success, "Đã chuẩn bị SDK", None);
                        return self.update(Message::RefreshManagedSdks);
                    }
                    Err(error) => {
                        self.error("Không chuẩn bị được SDK", error.clone());
                        self.sdk_setup_result = Some(error);
                    }
                }
            }
            Message::OpenSdkReview if !self.busy && self.sdk_open => {
                self.sdk_review_open = true;
                self.sdk_setup_result = None;
                self.sdk_cache_reset = None;
                self.review_license = 0;
                self.accepted_licenses.clear();
                match android_hub_infrastructure::SdkPlan::common_tools() {
                    Ok(plan) => {
                        self.sdk_plan = Some(plan);
                        self.sdk_plan_error = None;
                    }
                    Err(error) => {
                        self.sdk_plan = None;
                        self.sdk_plan_error = Some(error);
                    }
                }
            }
            Message::CloseSdkReview if !self.busy => {
                if let Some(editor) = self.creation.as_mut().filter(|e| e.image_review) {
                    editor.image_review = false;
                    editor.create_after_image = false;
                    self.sdk_review_open = false;
                    self.sdk_cache_reset = None;
                    self.accepted_licenses.clear();
                    self.sdk_setup_result = None;
                    return self.probe_creation();
                }
                self.sdk_review_open = false;
                self.sdk_cache_reset = None;
                self.accepted_licenses.clear();
            }
            Message::ReviewLicense(index)
                if !self.busy
                    && (self.sdk_open || self.creation_image_review())
                    && self.sdk_review_open =>
            {
                if self
                    .sdk_plan
                    .as_ref()
                    .is_some_and(|plan| index < plan.licenses().len())
                {
                    self.review_license = index;
                }
            }
            Message::AcceptLicense(index, accepted)
                if !self.busy
                    && (self.sdk_open || self.creation_image_review())
                    && self.sdk_review_open
                    && index == self.review_license =>
            {
                if let Some(license) = self
                    .sdk_plan
                    .as_ref()
                    .and_then(|plan| plan.licenses().get(index))
                {
                    if accepted {
                        self.accepted_licenses.insert(license.sha256.clone());
                    } else {
                        self.accepted_licenses.remove(&license.sha256);
                    }
                }
            }
            Message::CloseSdk if self.sdk_setup.is_none() => {
                self.sdk_review_open = false;
                self.sdk_cache_reset = None;
                self.accepted_licenses.clear();
                self.sdk_open = false;
                self.managed_epoch = self.managed_epoch.wrapping_add(1);
                self.managed_loading = false;
            }
            Message::RefreshManagedSdks if !self.busy && !self.managed_loading => {
                self.managed_epoch = self.managed_epoch.wrapping_add(1);
                let epoch = self.managed_epoch;
                self.managed_loading = true;
                self.managed_error = None;
                let backend = self.backend.clone();
                return Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || backend.managed_sdk_catalog())
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    move |result| Message::ManagedSdksLoaded(epoch, result),
                );
            }
            Message::ManagedSdksLoaded(epoch, result)
                if epoch == self.managed_epoch && self.sdk_open =>
            {
                self.managed_loading = false;
                match result {
                    Ok(catalog) => {
                        self.managed_sdks = catalog;
                        self.managed_error = None;
                    }
                    Err(error) => {
                        self.managed_sdks = Default::default();
                        self.managed_error = Some(error);
                    }
                }
            }
            Message::UseManagedSdk(path) if !self.busy && !self.managed_loading => {
                self.command_epoch = self.command_epoch.wrapping_add(1);
                self.managed_epoch = self.managed_epoch.wrapping_add(1);
                self.clear_backup_selection();
                self.busy = true;
                self.notice = "Đang kiểm tra lại SDK và Java trước khi sử dụng…".into();
                let backend = self.backend.clone();
                return Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || backend.configure_managed_sdk(&path))
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    Message::ManagedSdkSaved,
                );
            }
            Message::ManagedSdkSaved(result) => {
                self.busy = false;
                match result {
                    Ok((sdk, java)) => {
                        self.backend.sdk = sdk;
                        self.sdk_draft = self.backend.sdk.to_string_lossy().into_owned();
                        self.java_draft = java.to_string_lossy().into_owned();
                        self.location_notice =
                            Some("Đã chọn SDK và Java; lựa chọn được lưu cho lần mở sau.".into());
                        self.images_loading = false;
                        self.images = Ok(vec![]);
                        let images = self.update(Message::RefreshImages);
                        self.busy = true;
                        return Task::batch([
                            images,
                            Self::task(self.backend.clone(), Work::Initialize),
                        ]);
                    }
                    Err(error) => self.error("Thao tác chưa hoàn tất", error),
                }
            }
            Message::ToggleSdkDetails if self.sdk_open => self.sdk_details = !self.sdk_details,
            Message::SdkPath(path) if !self.busy => self.sdk_draft = path,
            Message::JavaPath(path) if !self.busy => self.java_draft = path,
            Message::SaveJava if !self.busy => {
                self.command_epoch = self.command_epoch.wrapping_add(1);
                self.clear_backup_selection();
                self.busy = true;
                self.notice = "Đang kiểm tra Java và các công cụ SDK…".into();
                let backend = self.backend.clone();
                let path = PathBuf::from(self.java_draft.trim());
                return Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || backend.configure_java(&path))
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    Message::JavaSaved,
                );
            }
            Message::JavaSaved(result) => {
                self.busy = false;
                match result {
                    Ok(path) => {
                        self.java_draft = path.to_string_lossy().into_owned();
                        self.location_notice = Some(
                            "Đã lưu Java; Hub dùng lựa chọn này khi tạo máy Android mới.".into(),
                        );
                        self.busy = true;
                        return Self::task(self.backend.clone(), Work::Initialize);
                    }
                    Err(error) => self.error("Thao tác chưa hoàn tất", error),
                }
            }
            Message::SaveSdk if !self.busy => {
                self.command_epoch = self.command_epoch.wrapping_add(1);
                self.clear_backup_selection();
                self.busy = true;
                self.notice = "Đang kiểm tra và lưu SDK…".into();
                let backend = self.backend.clone();
                let path = PathBuf::from(self.sdk_draft.trim());
                return Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || backend.configure_sdk(&path))
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    Message::SdkSaved,
                );
            }
            Message::SdkSaved(result) => {
                self.busy = false;
                match result {
                    Ok(sdk) => {
                        self.backend.sdk = sdk;
                        self.sdk_draft = self.backend.sdk.to_string_lossy().into_owned();
                        self.location_notice = Some(
                            "Đã lưu SDK; lựa chọn này áp dụng cho những lần mở Hub tiếp theo."
                                .into(),
                        );
                        self.images_loading = false;
                        self.images = Ok(vec![]);
                        let images = self.update(Message::RefreshImages);
                        self.busy = true;
                        return Task::batch([
                            images,
                            Self::task(self.backend.clone(), Work::Initialize),
                        ]);
                    }
                    Err(error) => self.error("Thao tác chưa hoàn tất", error),
                }
            }
            _ => {}
        }
        Task::none()
    }
}
