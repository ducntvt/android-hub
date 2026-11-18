use super::*;
use android_hub_domain::{DEFAULT_FPS, FPS_OPTIONS};
use iced::futures::StreamExt;

#[test]
fn high_fps_choices_edit_the_draft_without_changing_the_saved_device() {
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::Loaded(Ok(stopped_snapshot("A")), Work::Initialize));
    for fps in [90, 120] {
        let _ = hub.update(Message::Fps(fps));
        assert_eq!(hub.draft.as_ref().unwrap().fps, fps);
        assert!(hub.can_save_settings());
        assert_eq!(hub.state.as_ref().unwrap().profiles[0].fps, DEFAULT_FPS);
        assert!(hub.draft.as_ref().unwrap().validate().is_ok());
    }
    let _ = hub.update(Message::DiscardSettings);
    assert_eq!(hub.draft.as_ref().unwrap().fps, DEFAULT_FPS);
}

#[test]
fn delete_menu_targets_its_row_and_cancellation_ignores_late_preparation() {
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::Loaded(
        Ok(three_stopped_devices()),
        Work::Initialize,
    ));
    let original = hub.selected.clone();
    let target = hub.state.as_ref().unwrap().profiles[1].id.clone();
    let _ = hub.update(Message::ToggleDeviceMenu(target.clone()));
    assert_eq!(hub.device_menu.as_ref(), Some(&target));
    let _ = hub.update(Message::AskDelete(target.clone()));
    assert_eq!(hub.pending_delete.as_ref().unwrap().id, target);
    assert_eq!(hub.selected, original);
    assert!(hub.device_menu.is_none());
    assert!(hub.busy);
    let epoch = hub.delete_epoch;
    let _ = hub.update(Message::CancelModal);
    let _ = hub.update(Message::DeletePrepared(epoch, Err("late".into())));
    assert!(!hub.busy && !hub.modal_open());
    assert!(hub.delete_plan.is_none() && hub.delete_error.is_none());
}

#[test]
fn backup_completion_never_confirms_deletion_and_keyboard_order_matches_buttons() {
    use iced::keyboard::key::Named;
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::Loaded(Ok(stopped_snapshot("A")), Work::Initialize));
    hub.pending_delete = hub.draft.clone();
    hub.busy = true;
    let _ = hub.update(Message::DeleteBackupFinished(
        hub.delete_epoch,
        Ok(PathBuf::from("/tmp/example-backup")),
    ));
    assert!(!hub.busy && hub.pending_delete.is_some());
    assert!(hub.delete_backup.is_some());
    assert_eq!(hub.modal_action_count(), 3);
    assert_eq!(hub.modal_focus, 0);
    let _ = hub.update(Message::ModalKey(Named::Tab));
    assert_eq!(hub.modal_focus, 1);
    let _ = hub.update(Message::ModalKey(Named::Enter)); // Backup already completed.
    assert!(!hub.busy && hub.pending_delete.is_some());
    let _ = hub.update(Message::ModalKey(Named::Tab));
    assert_eq!(hub.modal_focus, 2);
    let _ = hub.update(Message::ModalKey(Named::Escape));
    assert!(!hub.modal_open());
    assert_eq!(hub.state.as_ref().unwrap().profiles.len(), 1);
}

#[test]
fn permanent_delete_requires_a_prepared_confirmation_and_ignores_closed_panel_results() {
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::Loaded(Ok(stopped_snapshot("A")), Work::Initialize));
    hub.removed_open = true;
    let _ = hub.update(Message::ConfirmPurge);
    assert!(!hub.busy);
    assert!(!hub.modal_open());
    let epoch = hub.removed_epoch;
    hub.removed_loading = true;
    let _ = hub.update(Message::CloseRemoved);
    let _ = hub.update(Message::PurgePrepared(epoch, Err("late result".into())));
    assert!(!hub.removed_open && !hub.removed_loading);
    assert!(hub.pending_purge.is_none());
    assert!(
        !hub.feedback
            .persistent
            .iter()
            .any(|item| item.title == "Không thể chuẩn bị xóa máy")
    );
}

#[test]
fn notifications_keep_errors_across_poll_and_expire_only_the_matching_toast() {
    use feedback::Kind;
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::Loaded(Ok(stopped_snapshot("A")), Work::Initialize));
    let _ = hub.update(Message::Loaded(
        Err("read failed".into()),
        Work::Poll(hub.command_epoch),
    ));
    let error_id = hub.feedback.persistent.front().unwrap().id;
    let _ = hub.update(Message::Loaded(
        Err("read failed".into()),
        Work::Poll(hub.command_epoch),
    ));
    assert_eq!(hub.feedback.persistent.len(), 1);
    let _ = hub.update(Message::Loaded(
        Ok(stopped_snapshot("A")),
        Work::Poll(hub.command_epoch),
    ));
    assert_eq!(hub.feedback.persistent.front().unwrap().id, error_id);
    hub.notify(Kind::Success, "Đã sao lưu dữ liệu", None);
    let old = hub.feedback.toast.as_ref().unwrap().id;
    hub.notify(Kind::Info, "Clipboard không có văn bản đọc được", None);
    let new = hub.feedback.toast.as_ref().unwrap().id;
    let _ = hub.update(Message::ExpireToast(old));
    let _ = hub.update(Message::ExpireToast(error_id));
    assert_eq!(hub.feedback.toast.as_ref().unwrap().id, new);
    assert_eq!(hub.feedback.persistent.len(), 1);
    let _ = hub.update(Message::DismissNotification(error_id));
    assert!(hub.feedback.persistent.is_empty());
    let _ = hub.update(Message::ExpireToast(new));
    assert!(hub.feedback.toast.is_none());
}

#[test]
fn open_toast_details_survive_new_feedback_and_late_expiry() {
    let (mut hub, _) = Hub::new();
    hub.notify(
        feedback::Kind::Success,
        "Đã sao lưu dữ liệu",
        Some("/tmp/backup".into()),
    );
    let id = hub.feedback.toast.as_ref().unwrap().id;
    let _ = hub.update(Message::ShowNotificationDetails(id));
    hub.notify(
        feedback::Kind::Info,
        "Clipboard không có văn bản đọc được",
        None,
    );
    let new_id = hub.feedback.toast.as_ref().unwrap().id;
    let _ = hub.update(Message::ExpireToast(id));
    assert!(hub.feedback.find(id).is_some() && hub.modal_open());
    let _ = hub.update(Message::CancelModal);
    assert!(!hub.modal_open());
    assert!(hub.feedback.find(id).is_none());
    assert_eq!(hub.feedback.toast.as_ref().unwrap().id, new_id);
}

#[test]
fn uncertain_apk_result_stays_visible_and_copied_error_excludes_guest_data() {
    let (mut hub, _) = Hub::new();
    let apk = PendingApk {
        id: "hub-test".into(),
        name: "private-device-name".into(),
        pid: 42,
        path: PathBuf::from("/private/user/downloads/secret.apk"),
        bytes: 123,
        epoch: 0,
        selection: None,
    };
    let _ = hub.update(Message::Loaded(
        Err("guest-account-content /private/user".into()),
        Work::InstallApk(apk),
    ));
    let n = hub.feedback.persistent.back().unwrap();
    assert_eq!(n.kind, feedback::Kind::Unknown);
    let id = n.id;
    for lang in [Language::Vi, Language::En] {
        let copied = n.safe_summary(lang);
        for sensitive in [
            "private-device-name",
            "guest-account-content",
            "/private/user",
            "secret.apk",
        ] {
            assert!(!copied.contains(sensitive));
        }
        assert!(copied.contains(lang.tr(n.title)));
    }
    let _ = hub.update(Message::ExpireToast(id));
    assert!(hub.feedback.find(id).is_some());
    let _ = hub.update(Message::ShowNotificationDetails(id));
    assert!(hub.modal_open());
    let _ = hub.update(Message::ModalKey(iced::keyboard::key::Named::Escape));
    assert!(!hub.modal_open());
    assert!(hub.feedback.find(id).is_some());
}

#[test]
fn modal_releases_inputs_blocks_background_commands_and_defaults_to_cancel() {
    use iced::keyboard::key::Named;
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::Loaded(
        Ok(three_stopped_devices()),
        Work::Initialize,
    ));
    hub.input_focus = Some("hub-test".into());
    let epoch = hub.input_epoch;
    let _ = hub.update(Message::AskDelete("hub-test".into()));
    let _ = hub.update(Message::DeletePrepared(
        hub.delete_epoch,
        Err("fixture has no disk data".into()),
    ));
    assert!(hub.modal_open());
    assert!(hub.input_focus.is_none());
    assert!(hub.input_epoch > epoch);
    for message in [
        Message::StartDevice("second".into()),
        Message::Select("second".into()),
        Message::Search("changed".into()),
        Message::Name("changed".into()),
        Message::Create,
        Message::DropApk(PathBuf::from("/tmp/unwanted.apk")),
        Message::SendClipboard("hub-test".into()),
        Message::ClipboardRead("hub-test".into(), 42, epoch, 1, Some("secret".into())),
        Message::DeviceInput(
            "second".into(),
            hub.input_epoch,
            Some(1),
            Input::Key("Enter".into()),
        ),
    ] {
        let _ = hub.update(message);
        assert!(!hub.busy);
        assert!(hub.modal_open());
        assert_eq!(hub.selected.as_deref(), Some("hub-test"));
        assert_eq!(hub.draft.as_ref().unwrap().name, "A");
        assert!(hub.search.is_empty());
        assert!(hub.input_focus.is_none());
        assert!(hub.pending_apk.is_none());
    }
    let _ = hub.update(Message::ModalKey(Named::Enter));
    assert!(!hub.busy && !hub.modal_open()); // Enter initially selects Cancel.
    let _ = hub.update(Message::AskDelete("hub-test".into()));
    let _ = hub.update(Message::DeletePrepared(
        hub.delete_epoch,
        Err("fixture has no disk data".into()),
    ));
    let _ = hub.update(Message::ModalKey(Named::Tab));
    assert_eq!(hub.modal_focus, 1);
    let _ = hub.update(Message::ModalKey(Named::Escape));
    assert!(!hub.busy && !hub.modal_open());
}

#[test]
fn stop_requires_confirmation_and_rejects_a_replacement_pid() {
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::Loaded(Ok(stopped_snapshot("A")), Work::Initialize));
    hub.state.as_mut().unwrap().runtime.insert(
        "hub-test".into(),
        android_hub_domain::Runtime {
            pid: Some(42),
            cpu_percent: None,
            rss_kib: None,
        },
    );
    let _ = hub.update(Message::Stop);
    assert!(!hub.busy);
    let _ = hub.update(Message::AskStop);
    assert_eq!(hub.pending_stop.as_ref().unwrap().pid, 42);
    hub.state
        .as_mut()
        .unwrap()
        .runtime
        .get_mut("hub-test")
        .unwrap()
        .pid = Some(43);
    let _ = hub.update(Message::Stop);
    assert!(!hub.busy && !hub.modal_open());
    assert_eq!(
        hub.feedback.persistent.back().unwrap().title,
        "Máy đã đổi phiên; hãy xác nhận lại."
    );
}

#[test]
fn details_modal_does_not_discard_inflight_results_or_history() {
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::Loaded(Ok(stopped_snapshot("A")), Work::Initialize));
    hub.error("Thao tác chưa hoàn tất", "details".into());
    let id = hub.feedback.persistent.back().unwrap().id;
    hub.busy = true;
    let _ = hub.update(Message::ShowNotificationDetails(id));
    let _ = hub.update(Message::BackupFinished(Ok(PathBuf::from(
        "/tmp/test-backup",
    ))));
    assert!(!hub.busy && hub.modal_open());
    let _ = hub.update(Message::ReceiptsLoaded(
        hub.receipt_epoch,
        Ok(vec![]),
        Ok(vec!["archive event".into()]),
        Ok(vec![]),
    ));
    assert!(hub.history_unread);
    let _ = hub.update(Message::CancelModal);
    let _ = hub.update(Message::OpenHistory);
    assert!(hub.history_open && !hub.history_unread);
    let _ = hub.update(Message::DismissReceipts);
    assert!(!hub.history_open);
    assert_eq!(hub.removal_history, ["archive event"]);
}

fn three_stopped_devices() -> Snapshot {
    let mut state = stopped_snapshot("A");
    state
        .profiles
        .push(Profile::standard("second".into(), "B".into(), 5586));
    state
        .profiles
        .push(Profile::standard("third".into(), "C".into(), 5588));
    state
}

#[test]
fn explicit_pair_keeps_stopped_slots_and_survives_poll_and_single_mode() {
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::Loaded(
        Ok(three_stopped_devices()),
        Work::Initialize,
    ));
    let _ = hub.update(Message::SetSplit(true));
    let _ = hub.update(Message::PickSlot(0, "third".into()));
    let _ = hub.update(Message::PickSlot(1, "second".into()));
    let pair = [Some("third".into()), Some("second".into())];
    assert_eq!(hub.slots.ids, pair);
    assert!(hub.displays.is_empty()); // selecting a stopped slot never boots it
    let _ = hub.update(Message::Loaded(
        Ok(three_stopped_devices()),
        Work::Poll(hub.command_epoch),
    ));
    assert_eq!(hub.slots.ids, pair);
    let _ = hub.update(Message::ToggleSidebar);
    let _ = hub.update(Message::SetSplit(false));
    assert_eq!(hub.selected.as_deref(), Some("second"));
    let _ = hub.update(Message::SetSplit(true));
    assert_eq!(hub.slots.ids, pair);
    let _ = hub.update(Message::PickSlot(0, "second".into()));
    assert_eq!(hub.slots.ids, [Some("second".into()), Some("third".into())]);
    let mut state = three_stopped_devices();
    state.profiles.retain(|p| p.id != "third");
    let _ = hub.update(Message::Loaded(Ok(state), Work::Poll(hub.command_epoch)));
    assert_eq!(hub.slots.ids, [Some("second".into()), None]);
}

#[test]
fn canvas_selection_keeps_gesture_epoch_and_cancels_old_action_target() {
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::Loaded(
        Ok(three_stopped_devices()),
        Work::Initialize,
    ));
    let _ = hub.update(Message::PickSlot(1, "second".into()));
    hub.pending_stop = Some(PendingStop {
        id: "second".into(),
        name: "B".into(),
        pid: 42,
    });
    hub.pending_delete = hub.draft.clone();
    let epoch = hub.input_epoch;
    let catalog_epoch = hub.catalog_epoch;
    hub.activate_visible("hub-test".into());
    assert_eq!(hub.input_epoch, epoch);
    assert_eq!(hub.slots.active, 0);
    assert_eq!(hub.selected.as_deref(), Some("hub-test"));
    assert_eq!(hub.draft.as_ref().unwrap().id, "hub-test");
    assert!(hub.catalog_epoch > catalog_epoch);
    assert!(hub.pending_stop.is_none() && hub.pending_delete.is_none());
}

#[test]
fn workspace_navigation_and_start_preserve_unsaved_settings() {
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::Loaded(
        Ok(three_stopped_devices()),
        Work::Initialize,
    ));
    let _ = hub.update(Message::Ram(4096));
    for message in [
        Message::Select("hub-test".into()),
        Message::PickSlot(1, "hub-test".into()),
        Message::PickSlot(0, "second".into()),
        Message::StartDevice("hub-test".into()),
    ] {
        let _ = hub.update(message);
        assert_eq!(hub.draft.as_ref().unwrap().ram_mb, 4096);
        assert_eq!(hub.selected.as_deref(), Some("hub-test"));
        assert!(!hub.busy);
    }
    let notice = hub.notice.clone();
    let _ = hub.update(Message::DeviceInput(
        "second".into(),
        hub.input_epoch.wrapping_sub(1),
        Some(0),
        Input::Mouse {
            x: 0,
            y: 0,
            down: true,
        },
    ));
    assert_eq!(hub.notice, notice);
}

#[test]
fn authoritative_refresh_reloads_configuration_after_restore() {
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::Loaded(Ok(stopped_snapshot("A")), Work::Initialize));
    let mut restored = stopped_snapshot("Restored A");
    restored.profiles[0].ram_mb = 4096;
    hub.refresh_draft = true;
    let _ = hub.update(Message::Loaded(Ok(restored), Work::Poll(hub.command_epoch)));
    let draft = hub.draft.unwrap();
    assert_eq!(draft.name, "Restored A");
    assert_eq!(draft.ram_mb, 4096);
}

#[test]
fn quit_waits_for_inflight_work_and_blocks_new_commands() {
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::RequestQuit);
    assert!(hub.quit_requested && hub.busy && !hub.quitting);
    let _ = hub.update(Message::StartDevice("hub-test".into()));
    assert!(!hub.quitting);
    let _ = hub.update(Message::Loaded(Ok(stopped_snapshot("A")), Work::Initialize));
    assert!(hub.quitting && !hub.busy);
    let _ = hub.update(Message::RequestQuit);
    assert!(hub.quitting);
    let _ = hub.update(Message::CancelQuit);
    assert!(hub.quit_requested && hub.quitting);
}

#[test]
fn shutdown_failure_keeps_hub_open_for_retry_or_return() {
    let (mut hub, _) = Hub::new();
    hub.busy = false;
    let _ = hub.update(Message::RequestQuit);
    let _ = hub.update(Message::ShutdownFinished(Err("ADB unavailable".into())));
    assert!(hub.quit_requested && !hub.quitting);
    assert_eq!(hub.quit_error.as_deref(), Some("ADB unavailable"));
    let _ = hub.update(Message::Tick);
    assert!(!hub.quitting);
    let _ = hub.update(Message::RequestQuit);
    assert!(hub.quitting && hub.quit_error.is_none());
    let _ = hub.update(Message::ShutdownFinished(Err("still running".into())));
    let _ = hub.update(Message::CancelQuit);
    assert!(!hub.quit_requested && !hub.quitting && hub.quit_error.is_none());
}

#[test]
fn quitting_waits_for_backup_and_sdk_reinitialization() {
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::RequestQuit);
    let _ = hub.update(Message::SdkSaved(Ok(PathBuf::from("/fixture/sdk"))));
    assert!(hub.busy && !hub.quitting);
    let _ = hub.update(Message::Loaded(
        Err("SDK unavailable".into()),
        Work::Initialize,
    ));
    assert!(hub.quitting);
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::RequestQuit);
    let _ = hub.update(Message::BackupFinished(Err("copy failed".into())));
    assert!(hub.quitting && !hub.busy);
}

fn stopped_snapshot(name: &str) -> Snapshot {
    Snapshot {
        android_versions: BTreeMap::new(),
        profiles: vec![Profile::standard("hub-test".into(), name.into(), 5584)],
        runtime: BTreeMap::new(),
    }
}

#[test]
fn connectivity_is_explicit_cancellable_and_rejects_late_sessions() {
    use android_hub_infrastructure::{BootObservation, ConnectivityReport, ProbeResult};
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::Loaded(Ok(stopped_snapshot("A")), Work::Initialize));
    hub.settings_open = true;
    let _ = hub.update(Message::CheckConnectivity);
    assert!(hub.connectivity.is_none()); // stopped guest
    let boot = "12345678-1234-1234-1234-123456789abc".to_owned();
    hub.state.as_mut().unwrap().runtime.insert(
        "hub-test".into(),
        android_hub_domain::Runtime {
            pid: Some(123),
            cpu_percent: None,
            rss_kib: None,
        },
    );
    hub.boot_checks.insert(
        "hub-test".into(),
        lifecycle::BootCheck {
            pid: 123,
            epoch: 0,
            since: std::time::Instant::now(),
            finished: true,
            result: Some(Ok(BootObservation {
                boot_id: boot.clone(),
                completed: true,
            })),
        },
    );
    assert!(hub.connectivity.is_none()); // no automatic probe on readiness
    let _ = hub.update(Message::CheckConnectivity);
    let first = hub.connectivity.as_ref().unwrap().epoch;
    assert!(!hub.busy); // input/lifecycle mutation lock is not used
    let _ = hub.update(Message::CancelConnectivity);
    let _ = hub.update(Message::ConnectivityFinished(first, Err("late".into())));
    assert!(hub.connectivity.is_none());
    let _ = hub.update(Message::CheckConnectivity);
    let second = hub.connectivity.as_ref().unwrap().epoch;
    assert_ne!(first, second);
    let _ = hub.update(Message::ConnectivityFinished(first, Err("stale".into())));
    assert!(hub.connectivity.as_ref().unwrap().result.is_none());
    let report = ConnectivityReport {
        measured_seconds: 10,
        boot_id: boot,
        interface: ProbeResult::Passed,
        dns: ProbeResult::Passed,
        https: ProbeResult::TlsFailed,
    };
    let _ = hub.update(Message::ConnectivityFinished(second, Ok(report)));
    assert!(hub.connectivity.as_ref().unwrap().result.is_some());
    let _ = hub.update(Message::ToggleSettings);
    assert!(hub.connectivity.is_none()); // closing view drops the subscription
    hub.settings_open = true;
    let _ = hub.update(Message::CheckConnectivity);
    let third = hub.connectivity.as_ref().unwrap().epoch;
    hub.state
        .as_mut()
        .unwrap()
        .runtime
        .get_mut("hub-test")
        .unwrap()
        .pid = Some(124);
    let _ = hub.update(Message::ConnectivityFinished(third, Err("old pid".into())));
    assert!(hub.connectivity.is_none());
}

#[test]
fn sdk_selection_result_changes_backend_only_after_success() {
    let (mut hub, _) = Hub::new();
    let old = hub.backend.sdk.clone();
    let _ = hub.update(Message::SdkSaved(Err("invalid SDK".into())));
    assert_eq!(hub.backend.sdk, old);
    assert!(!hub.busy);
    assert_eq!(
        hub.feedback.persistent.back().unwrap().details.as_deref(),
        Some("invalid SDK")
    );
    let new = PathBuf::from("/fixture/SDK with spaces");
    let _ = hub.update(Message::SdkSaved(Ok(new.clone())));
    assert_eq!(hub.backend.sdk, new);
    assert_eq!(hub.sdk_draft, "/fixture/SDK with spaces");
    assert!(hub.busy); // Refresh must finish before any next command.
}
#[test]
fn java_selection_result_preserves_sdk_and_refreshes_after_success() {
    let (mut hub, _) = Hub::new();
    let sdk = hub.backend.sdk.clone();
    hub.java_draft = "/fixture/invalid".into();
    let _ = hub.update(Message::JavaSaved(Err("Java probe failed".into())));
    assert!(!hub.busy);
    assert_eq!(
        hub.feedback.persistent.back().unwrap().details.as_deref(),
        Some("Java probe failed")
    );
    assert_eq!(hub.java_draft, "/fixture/invalid");
    let _ = hub.update(Message::JavaSaved(Ok(PathBuf::from(
        "/fixture/JDK/Contents/Home",
    ))));
    assert_eq!(hub.backend.sdk, sdk);
    assert_eq!(hub.java_draft, "/fixture/JDK/Contents/Home");
    assert!(hub.busy);
}
#[test]
fn managed_catalog_ignores_old_responses_and_selection_updates_both_drafts() {
    let (mut hub, _) = Hub::new();
    hub.sdk_open = true;
    hub.managed_epoch = 2;
    hub.managed_loading = true;
    let _ = hub.update(Message::ManagedSdksLoaded(1, Err("stale".into())));
    assert!(hub.managed_error.is_none());
    assert!(hub.managed_loading);
    let _ = hub.update(Message::CloseSdk);
    let _ = hub.update(Message::ManagedSdksLoaded(2, Err("dismissed".into())));
    assert!(hub.managed_error.is_none());
    let sdk = hub.backend.sdk.clone();
    let _ = hub.update(Message::ManagedSdkSaved(Err("probe failed".into())));
    assert_eq!(hub.backend.sdk, sdk);
    assert!(!hub.busy);
    let _ = hub.update(Message::ManagedSdkSaved(Ok((
        PathBuf::from("/fixture/managed-sdk"),
        PathBuf::from("/fixture/java"),
    ))));
    assert_eq!(hub.sdk_draft, "/fixture/managed-sdk");
    assert_eq!(hub.java_draft, "/fixture/java");
    assert!(hub.busy);
}
#[test]
fn sdk_review_never_preaccepts_and_clears_consent_on_close_or_reopen() {
    let (mut hub, _) = Hub::new();
    hub.busy = false; // Initial refresh has completed before user interaction.
    hub.sdk_open = true;
    let _ = hub.update(Message::OpenSdkReview);
    assert!(hub.sdk_plan_error.is_none());
    assert!(hub.accepted_licenses.is_empty());
    let _ = hub.update(Message::AcceptLicense(1, true)); // not the displayed license
    assert!(hub.accepted_licenses.is_empty());
    let _ = hub.update(Message::AcceptLicense(0, true));
    assert_eq!(hub.accepted_licenses.len(), 1);
    let _ = hub.update(Message::ReviewLicense(1));
    let _ = hub.update(Message::AcceptLicense(1, true));
    assert!(
        hub.sdk_plan
            .as_ref()
            .unwrap()
            .approved_packages(&hub.accepted_licenses)
            .is_ok()
    );
    let _ = hub.update(Message::CloseSdkReview);
    assert!(hub.accepted_licenses.is_empty());
    let _ = hub.update(Message::AcceptLicense(1, true));
    assert!(hub.accepted_licenses.is_empty());
    let _ = hub.update(Message::OpenSdkReview);
    assert!(hub.accepted_licenses.is_empty());
    assert!(!hub.busy); // Review creates no download/install work.
}
#[test]
fn sdk_setup_ui_requires_consent_and_handles_retry_and_completion_without_selecting() {
    let (mut hub, _) = Hub::new();
    hub.busy = false;
    hub.sdk_open = true;
    let original_sdk = hub.backend.sdk.clone();
    let _ = hub.update(Message::OpenSdkReview);
    let _ = hub.update(Message::StartSdkSetup);
    assert!(hub.sdk_setup.is_none());
    assert!(!hub.busy);
    hub.accepted_licenses = hub
        .sdk_plan
        .as_ref()
        .unwrap()
        .licenses()
        .iter()
        .map(|license| license.sha256.clone())
        .collect();
    hub.java_draft = "/fixture/java".into();
    // Dropped Task is intentionally never polled: this UI test cannot download anything.
    let _ = hub.update(Message::StartSdkSetup);
    assert!(hub.busy);
    assert!(hub.sdk_setup.is_some());
    let _ = hub.update(Message::CloseSdk);
    assert!(hub.sdk_open); // Keep cancellation reachable during setup.
    let _ = hub.update(Message::CancelSdkSetup);
    assert!(hub.sdk_setup_result.as_ref().unwrap().contains("hủy"));
    let _ = hub.update(Message::SdkSetupFinished(Err("canceled fixture".into())));
    assert!(!hub.busy);
    assert!(hub.sdk_setup.is_none());
    assert_eq!(hub.accepted_licenses.len(), 2); // retry same reviewed plan
    let _ = hub.update(Message::StartSdkSetup);
    assert!(hub.busy);
    let _ = hub.update(Message::SdkSetupFinished(Ok(
        android_hub_infrastructure::ManagedSdk {
            path: PathBuf::from("/fixture/new-sdk"),
            java_home: PathBuf::from("/fixture/java"),
        },
    )));
    assert_eq!(hub.backend.sdk, original_sdk);
    assert!(hub.accepted_licenses.is_empty());
    assert!(!hub.sdk_review_open);
    assert!(hub.managed_loading);
}
#[test]
fn sdk_cache_reset_requires_confirmation_and_does_not_start_download() {
    let (mut hub, _) = Hub::new();
    hub.busy = false;
    hub.sdk_open = true;
    let _ = hub.update(Message::OpenSdkReview);
    let _ = hub.update(Message::AskSdkCacheReset(0));
    assert_eq!(hub.sdk_cache_reset, Some(0));
    assert!(!hub.busy);
    let _ = hub.update(Message::CancelSdkCacheReset);
    assert_eq!(hub.sdk_cache_reset, None);
    let _ = hub.update(Message::ConfirmSdkCacheReset);
    assert!(!hub.busy);
    let _ = hub.update(Message::AskSdkCacheReset(0));
    let _ = hub.update(Message::ConfirmSdkCacheReset); // dropped future; no actual file changes
    assert!(hub.busy);
    assert!(hub.sdk_setup.is_none());
    assert_eq!(hub.sdk_cache_reset, None);
    let _ = hub.update(Message::SdkCacheResetFinished(Ok(PathBuf::from(
        "/fixture/quarantine",
    ))));
    assert!(!hub.busy);
    assert!(
        hub.sdk_setup_result
            .as_ref()
            .unwrap()
            .contains("quarantine")
    );
    assert!(hub.accepted_licenses.is_empty());
    assert!(hub.sdk_setup.is_none());
}
#[test]
fn stale_or_dismissed_history_cannot_replace_newer_results() {
    let (mut hub, _) = Hub::new();
    let _ = hub.receipt_task();
    let old = hub.receipt_epoch;
    let _ = hub.receipt_task();
    let _ = hub.update(Message::ReceiptsLoaded(
        hub.receipt_epoch,
        Ok(vec![]),
        Ok(vec!["new".into()]),
        Ok(vec!["created".into()]),
    ));
    let _ = hub.update(Message::ReceiptsLoaded(
        old,
        Ok(vec![]),
        Ok(vec!["old".into()]),
        Ok(vec!["stale creation".into()]),
    ));
    assert_eq!(hub.removal_history, vec!["new"]);
    assert_eq!(hub.creation_history, vec!["created"]);
    let old = hub.receipt_epoch;
    let _ = hub.update(Message::DismissReceipts);
    let _ = hub.update(Message::ReceiptsLoaded(
        old,
        Ok(vec![]),
        Ok(vec!["old".into()]),
        Ok(vec!["stale creation".into()]),
    ));
    assert_eq!(hub.removal_history, vec!["new"]);
    assert_eq!(hub.creation_history, vec!["created"]);
    assert!(!hub.history_unread);
}
#[test]
fn deletion_confirmation_rejects_changed_saved_profile() {
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::Loaded(Ok(stopped_snapshot("A")), Work::Initialize));
    let _ = hub.update(Message::AskDelete("hub-test".into()));
    let _ = hub.update(Message::DeletePrepared(
        hub.delete_epoch,
        Err("fixture has no disk data".into()),
    ));
    assert!(hub.pending_delete.is_some());
    assert!(!hub.busy);
    hub.state.as_mut().unwrap().profiles[0].name = "Changed externally".into();
    let _ = hub.update(Message::ConfirmDelete);
    assert!(!hub.busy);
    assert!(hub.pending_delete.is_none());
    assert!(
        hub.feedback
            .persistent
            .back()
            .unwrap()
            .title
            .contains("Máy đã thay đổi")
    );
    let _ = hub.update(Message::AskDelete("hub-test".into()));
    let _ = hub.update(Message::DeletePrepared(
        hub.delete_epoch,
        Err("fixture has no disk data".into()),
    ));
    hub.choose("hub-other".into());
    assert!(hub.pending_delete.is_none());
}
#[test]
fn last_removed_device_clears_selection_and_archive_panel_can_reload() {
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::Loaded(Ok(stopped_snapshot("A")), Work::Initialize));
    let _ = hub.update(Message::Loaded(
        Ok(Snapshot {
            android_versions: BTreeMap::new(),
            profiles: vec![],
            runtime: BTreeMap::new(),
        }),
        Work::Initialize,
    ));
    assert!(hub.selected.is_none());
    assert!(hub.draft.is_none());
    let _ = hub.update(Message::OpenRemoved);
    let old = hub.removed_epoch;
    assert!(hub.removed_loading);
    let _ = hub.update(Message::ToggleSettings);
    assert!(!hub.removed_loading);
    let _ = hub.update(Message::RemovedLoaded(old, Err("old error".into())));
    assert!(!hub.notice.contains("old error"));
    let _ = hub.update(Message::OpenRemoved);
    assert!(hub.removed_open && hub.removed_loading);
}
#[test]
fn delayed_clipboard_does_not_change_notice_after_selection_changes() {
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::Loaded(Ok(stopped_snapshot("A")), Work::Initialize));
    let epoch = hub.input_epoch;
    hub.choose("hub-test".into());
    let notice = hub.notice.clone();
    let _ = hub.update(Message::ClipboardRead(
        "hub-test".into(),
        42,
        epoch,
        0,
        Some("test".into()),
    ));
    assert_eq!(hub.notice, notice);
    assert!(!hub.busy);
}
#[test]
fn polls_and_save_keep_nonfirst_selection_and_unsaved_orientation() {
    let (mut hub, _) = Hub::new();
    let snapshot = || Snapshot {
        android_versions: BTreeMap::new(),
        profiles: vec![
            Profile::standard("first".into(), "First".into(), 5584),
            Profile::standard("third".into(), "Third".into(), 5588),
        ],
        runtime: BTreeMap::new(),
    };
    let _ = hub.update(Message::Loaded(Ok(snapshot()), Work::Initialize));
    let _ = hub.update(Message::Select("third".into()));
    let _ = hub.update(Message::Orientation(Orientation::Landscape));
    let _ = hub.update(Message::Resolution(Resolution::Small));
    for _ in 0..3 {
        let _ = hub.update(Message::Loaded(
            Ok(snapshot()),
            Work::Poll(hub.command_epoch),
        ));
        assert_eq!(hub.selected.as_deref(), Some("third"));
        let draft = hub.draft.as_ref().unwrap();
        assert_eq!((draft.width, draft.height), (854, 480));
    }
    let saved = hub.draft.clone().unwrap();
    let mut refreshed = snapshot();
    refreshed.profiles[1] = saved.clone();
    let _ = hub.update(Message::Loaded(
        Ok(refreshed),
        Work::Save(saved, hub.draft_base.clone().unwrap()),
    ));
    assert_eq!(hub.selected.as_deref(), Some("third"));
    assert_eq!(hub.draft.as_ref().unwrap().width, 854);
}

#[test]
fn resolution_keeps_orientation_without_mutating_saved_profile() {
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::Loaded(Ok(stopped_snapshot("A")), Work::Initialize));
    let _ = hub.update(Message::Orientation(Orientation::Landscape));
    let _ = hub.update(Message::Resolution(Resolution::Small));
    let draft = hub.draft.as_ref().unwrap();
    assert_eq!((draft.width, draft.height), (854, 480));
    assert_eq!(hub.state.as_ref().unwrap().profiles[0].width, 720);
    hub.stale = true;
    let _ = hub.update(Message::Orientation(Orientation::Portrait));
    assert_eq!(hub.draft.as_ref().unwrap().width, 854);
    hub.stale = false;
    let _ = hub.update(Message::Orientation(Orientation::Portrait));
    assert_eq!(hub.draft.as_ref().unwrap().width, 480);
}
#[test]
fn late_poll_cannot_overwrite_saved_configuration() {
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::Loaded(
        Ok(stopped_snapshot("Before")),
        Work::Initialize,
    ));
    let _ = hub.update(Message::Tick);
    let old_epoch = hub.command_epoch;
    let mut edited = hub.draft.clone().unwrap();
    edited.name = "After".into();
    let _ = hub.launch(Work::Save(edited.clone(), hub.draft_base.clone().unwrap()));
    let _ = hub.update(Message::Loaded(
        Ok(stopped_snapshot("After")),
        Work::Save(edited, hub.draft_base.clone().unwrap()),
    ));
    let notice = hub.notice.clone();
    let _ = hub.update(Message::Loaded(
        Ok(stopped_snapshot("Before")),
        Work::Poll(old_epoch),
    ));
    assert_eq!(hub.state.as_ref().unwrap().profiles[0].name, "After");
    assert_eq!(hub.draft.as_ref().unwrap().name, "After");
    assert_eq!(hub.notice, notice);
    assert!(!hub.polling);
    assert!(!hub.stale);
}

#[test]
fn stale_poll_error_does_not_interrupt_command_and_fresh_poll_recovers() {
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::Loaded(
        Ok(stopped_snapshot("Before")),
        Work::Initialize,
    ));
    let old_epoch = hub.command_epoch;
    let _ = hub.update(Message::Tick);
    let edited = hub.draft.clone().unwrap();
    let _ = hub.launch(Work::Save(edited.clone(), hub.draft_base.clone().unwrap()));
    let notice = hub.notice.clone();
    let _ = hub.update(Message::Loaded(
        Err("old poll error".into()),
        Work::Poll(old_epoch),
    ));
    assert!(hub.busy);
    assert!(!hub.stale);
    assert!(!hub.polling);
    assert_eq!(hub.notice, notice);
    let _ = hub.update(Message::Loaded(
        Ok(stopped_snapshot("Before")),
        Work::Save(edited, hub.draft_base.clone().unwrap()),
    ));
    let _ = hub.update(Message::Tick);
    let _ = hub.update(Message::Loaded(
        Ok(stopped_snapshot("External rename")),
        Work::Poll(hub.command_epoch),
    ));
    assert_eq!(
        hub.state.as_ref().unwrap().profiles[0].name,
        "External rename"
    );
    assert!(!hub.polling);
}

#[test]
fn restore_requires_confirmation_and_rejects_changed_profile() {
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::Loaded(Ok(stopped_snapshot("A")), Work::Initialize));
    let info = BackupInfo {
        profile_name: "Backup source".into(),
        path: PathBuf::from("/tmp/backup"),
        profile_id: "hub-test".into(),
        created_seconds: 0,
        bytes: 42,
        manifest_sha256: "digest".into(),
    };
    hub.backups.entries.push(info.clone());
    let _ = hub.update(Message::PickBackup(info));
    assert!(hub.pending_restore.is_none());
    assert!(!hub.busy);
    let _ = hub.update(Message::AskRestore);
    assert!(hub.pending_restore.is_some());
    assert!(!hub.busy);
    hub.state.as_mut().unwrap().profiles[0].ram_mb = 4096;
    let _ = hub.update(Message::ConfirmRestore);
    assert!(!hub.busy);
    assert!(hub.pending_restore.is_none());
    assert!(
        hub.feedback
            .persistent
            .back()
            .unwrap()
            .title
            .contains("Máy đã thay đổi")
    );
}

#[test]
fn old_catalog_results_cannot_cross_selection_or_reload() {
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::Loaded(Ok(stopped_snapshot("A")), Work::Initialize));
    let _ = hub.update(Message::LoadBackups);
    let epoch = hub.catalog_epoch;
    hub.choose("hub-test".into());
    let _ = hub.update(Message::LoadBackups);
    let _ = hub.update(Message::BackupsLoaded(
        "hub-test".into(),
        epoch,
        Err("stale catalog error".into()),
    ));
    assert!(hub.catalog_loading);
    assert!(!hub.notice.contains("stale catalog error"));
    let _ = hub.update(Message::BackupsLoaded(
        "hub-test".into(),
        hub.catalog_epoch,
        Ok(BackupCatalog::default()),
    ));
    assert!(!hub.catalog_loading);
}

#[tokio::test]
async fn frame_notifications_survive_early_arrival_and_coalesce() {
    for fps in FPS_OPTIONS {
        let updates = std::sync::Arc::new(tokio::sync::Notify::new());
        // Producer can finish frames before Iced installs its subscription.
        for _ in 0..100 {
            updates.notify_one();
        }
        let stream = runtime::frame_updates("test".into(), 42, fps, updates.clone());
        iced::futures::pin_mut!(stream);
        let started = tokio::time::Instant::now();
        assert!(matches!(stream.next().await, Some(Message::FrameReady(id, 42)) if id == "test"));
        assert!(started.elapsed() >= runtime::frame_interval(fps));
        // A slow UI receives one wakeup, not a backlog or recurring idle ticks.
        assert!(
            tokio::time::timeout(Duration::from_millis(80), stream.next())
                .await
                .is_err()
        );
        updates.notify_one();
        assert!(
            tokio::time::timeout(Duration::from_secs(1), stream.next())
                .await
                .unwrap()
                .is_some()
        );
    }
}

#[test]
fn appearance_and_preferences_keep_guest_identity_and_protect_unsaved_settings() {
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::Loaded(Ok(stopped_snapshot("A")), Work::Initialize));
    let root = std::env::temp_dir().join(format!("hub-theme-update-{}", std::process::id()));
    hub.backend.root = root.clone();
    let before_profile = hub.state.as_ref().unwrap().profiles.clone();
    let before_epoch = hub.command_epoch;
    let before_input_epoch = hub.input_epoch;
    for preference in appearance::Preference::ALL {
        let _ = hub.update(Message::SetAppearance(preference));
        assert_eq!(hub.appearance, preference);
        assert_eq!(hub.command_epoch, before_epoch);
        assert_eq!(hub.input_epoch, before_input_epoch);
        assert_eq!(hub.state.as_ref().unwrap().profiles, before_profile);
        assert!(!hub.busy);
    }
    hub.settings_open = true;
    hub.draft.as_mut().unwrap().name = "Unsaved name".into();
    let _ = hub.update(Message::OpenPreferences);
    assert!(!hub.preferences_open);
    assert_eq!(
        hub.pending_navigation,
        Some(settings::Navigation::Preferences)
    );
    // Hub shortcuts do not navigate through a modal or discard its input.
    let _ = hub.update(Message::FocusSearch);
    assert!(hub.pending_navigation.is_some());
    let _ = hub.update(Message::DiscardAndContinue);
    assert!(hub.preferences_open && !hub.settings_open);
    assert_eq!(hub.draft.as_ref().unwrap().name, before_profile[0].name);
    let _ = hub.update(Message::OpenHistory);
    assert!(!hub.preferences_open && hub.history_open);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn tab_traversal_releases_android_focus_and_modal_blocks_background_traversal() {
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::Loaded(Ok(stopped_snapshot("A")), Work::Initialize));
    hub.input_focus = hub.selected.clone();
    let epoch = hub.input_epoch;
    let _ = hub.update(Message::FocusNext(false));
    assert!(hub.input_focus.is_none());
    assert_ne!(hub.input_epoch, epoch);
    hub.detail_id = Some(1);
    let epoch = hub.input_epoch;
    let _ = hub.update(Message::FocusNext(true));
    assert_eq!(hub.input_epoch, epoch);
}

#[test]
fn data_catalog_failures_are_not_successful_empty_lists_and_retries_clear_errors() {
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::Loaded(Ok(stopped_snapshot("A")), Work::Initialize));
    let id = hub.selected.clone().unwrap();
    let _ = hub.update(Message::LoadBackups);
    let epoch = hub.catalog_epoch;
    let _ = hub.update(Message::BackupsLoaded(
        id.clone(),
        epoch,
        Err("catalog unavailable".into()),
    ));
    assert!(hub.catalog_loaded && !hub.catalog_loading);
    assert_eq!(hub.catalog_error.as_deref(), Some("catalog unavailable"));
    let _ = hub.update(Message::LoadBackups);
    assert!(hub.catalog_loading && !hub.catalog_loaded && hub.catalog_error.is_none());
    let _ = hub.update(Message::BackupsLoaded(
        id.clone(),
        epoch,
        Err("old failure".into()),
    ));
    assert!(hub.catalog_loading && hub.catalog_error.is_none());
    let _ = hub.update(Message::BackupsLoaded(
        id,
        hub.catalog_epoch,
        Ok(BackupCatalog::default()),
    ));
    assert!(hub.catalog_loaded && hub.catalog_error.is_none() && hub.backups.entries.is_empty());
    let _ = hub.update(Message::OpenRemoved);
    let _ = hub.update(Message::RemovedLoaded(
        hub.removed_epoch,
        Err("archive unavailable".into()),
    ));
    assert_eq!(hub.removed_error.as_deref(), Some("archive unavailable"));
    let _ = hub.update(Message::OpenRemoved);
    assert!(hub.removed_loading && hub.removed_error.is_none());
    let _ = hub.update(Message::MeasureStorage);
    let _ = hub.update(Message::StorageLoaded(
        hub.command_epoch,
        Err("scan incomplete".into()),
    ));
    assert!(hub.storage.is_none());
    assert_eq!(hub.storage_error.as_deref(), Some("scan incomplete"));
    let _ = hub.update(Message::MeasureStorage);
    assert!(hub.storage_loading && hub.storage_error.is_none() && hub.storage.is_none());
}
