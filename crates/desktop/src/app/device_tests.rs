//! Physical selection cannot fall through to an AVD with the same raw ID.
use super::*;
use android_hub_domain::{PhysicalId, Runtime};

fn hub_with_phone_selected() -> Hub {
    let (mut hub, _) = Hub::new();
    let state = Snapshot {
        profiles: vec![Profile::standard(
            "hub-same".into(),
            "Emulator".into(),
            5584,
        )],
        runtime: BTreeMap::from([(
            "hub-same".into(),
            Runtime {
                pid: Some(123),
                ..Default::default()
            },
        )]),
        android_versions: BTreeMap::new(),
    };
    // No Initialize task is polled; this state-only fixture never starts a guest.
    hub.state = Some(state);
    hub.stale = false;
    hub.busy = false;
    hub.select_metadata("hub-same".into());
    assert!(hub.draft.is_some());
    let _ = hub.update(Message::Select(phone()));
    hub
}
fn phone() -> DeviceId {
    DeviceId::Physical(PhysicalId::new("hub-same").unwrap())
}
#[test]
fn physical_selection_does_not_resolve_to_avd_draft_or_pid() {
    let hub = hub_with_phone_selected();
    assert_eq!(hub.selected, Some(phone()));
    assert!(hub.selected_avd_id().is_none());
    assert!(hub.draft.is_none() && hub.draft_base.is_none());
    assert!(!hub.running());
    assert!(!hub.can_save_settings());
    for op in [
        DeviceOperation::StartEmulator,
        DeviceOperation::StopEmulator,
        DeviceOperation::ConfigureAvd,
        DeviceOperation::ArchiveAvd,
        DeviceOperation::DeleteAvd,
        DeviceOperation::BackupAvd,
        DeviceOperation::RestoreAvd,
    ] {
        assert!(!hub.selected_allows(op));
    }
    let _ = hub.view();
}
#[test]
fn physical_start_delete_stop_backup_restore_events_do_not_dispatch() {
    let mut hub = hub_with_phone_selected();
    let epoch = hub.command_epoch;
    for event in [
        Message::StartDevice(phone()),
        Message::AskDelete(phone()),
        Message::AskStop,
        Message::Stop,
        Message::Backup,
        Message::AskRestore,
        Message::ConfirmRestore,
        Message::LoadBackups,
        Message::ToggleSettings,
    ] {
        let _ = hub.update(event);
        assert!(!hub.busy);
        assert_eq!(hub.command_epoch, epoch);
        assert!(
            hub.pending_delete.is_none()
                && hub.pending_stop.is_none()
                && hub.pending_restore.is_none()
        );
    }
    assert!(!hub.settings_open && !hub.catalog_loading);
}
#[test]
fn stale_avd_draft_cannot_enable_physical_settings_or_save() {
    let mut hub = hub_with_phone_selected();
    let base = hub.state.as_ref().unwrap().profiles[0].clone();
    hub.draft_base = Some(base.clone());
    let mut changed = base;
    changed.name = "Old draft".into();
    hub.draft = Some(changed.clone());
    for event in [
        Message::Name("wrong".into()),
        Message::Ram(4096),
        Message::Cpus(4),
        Message::Fps(120),
        Message::Orientation(Orientation::Landscape),
        Message::Resolution(Resolution::Large),
        Message::Save,
    ] {
        let _ = hub.update(event);
        assert_eq!(hub.draft.as_ref(), Some(&changed));
        assert!(!hub.can_save_settings() && !hub.busy);
    }
}
#[test]
fn stale_avd_catalog_cannot_update_a_physical_selection() {
    let mut hub = hub_with_phone_selected();
    let _ = hub.update(Message::BackupsLoaded(
        "hub-same".into(),
        hub.catalog_epoch,
        Err("old AVD error".into()),
    ));
    assert!(hub.catalog_error.is_none());
    assert_eq!(hub.selected, Some(phone()));
    hub.reload_settings();
    assert!(hub.draft.is_none());
}
