use super::*;
use settings::Navigation;

fn snapshot() -> Snapshot {
    Snapshot {
        profiles: vec![
            Profile::standard("hub-draft-a".into(), "A".into(), 5660),
            Profile::standard("hub-draft-b".into(), "B".into(), 5662),
        ],
        runtime: Default::default(),
        android_versions: Default::default(),
    }
}
fn hub() -> Hub {
    let (mut hub, _) = Hub::new();
    let _ = hub.update(Message::Loaded(Ok(snapshot()), Work::Initialize));
    hub.settings_open = true;
    hub
}
fn edit(hub: &mut Hub) {
    let _ = hub.update(Message::Name("My draft".into()));
    assert!(hub.has_unsaved_settings());
}

#[test]
fn save_requires_valid_dirty_stopped_current_settings() {
    let mut hub = hub();
    assert!(!hub.can_save_settings());
    edit(&mut hub);
    assert!(hub.can_save_settings());
    assert_eq!(hub.state.as_ref().unwrap().profiles[0].name, "A");
    let _ = hub.update(Message::Name(" ".into()));
    assert!(settings::name_error(hub.draft.as_ref().unwrap()).is_some());
    let _ = hub.update(Message::Save);
    assert!(!hub.busy);
    assert!(!hub.can_save_settings());
    let _ = hub.update(Message::DiscardSettings);
    assert_eq!(hub.draft.as_ref().unwrap().name, "A");
    assert!(!hub.has_unsaved_settings());
    edit(&mut hub);
    hub.state.as_mut().unwrap().runtime.insert(
        "hub-draft-a".into(),
        android_hub_domain::Runtime {
            pid: Some(42),
            ..Default::default()
        },
    );
    assert!(!hub.can_save_settings());
    let _ = hub.update(Message::Save);
    assert!(!hub.busy); // Does not implicitly stop a running guest.
}

#[test]
fn dirty_navigation_defaults_to_stay_and_discard_uses_latest_saved_profile() {
    let mut hub = hub();
    edit(&mut hub);
    let _ = hub.update(Message::PickSlot(1, "hub-draft-b".into()));
    assert_eq!(
        hub.pending_navigation,
        Some(Navigation::Slot(1, "hub-draft-b".into()))
    );
    let _ = hub.update(Message::ModalKey(iced::keyboard::key::Named::Enter));
    assert!(!hub.modal_open());
    assert!(hub.has_unsaved_settings());
    assert_eq!(hub.selected_avd_id(), Some("hub-draft-a"));
    let _ = hub.update(Message::for_avd("hub-draft-b", Message::Select).unwrap());
    let _ = hub.update(Message::DiscardAndContinue);
    assert_eq!(hub.selected_avd_id(), Some("hub-draft-b"));
    assert!(!hub.has_unsaved_settings());
    assert_eq!(hub.draft.as_ref().unwrap().name, "B");
}

#[test]
fn saving_then_navigation_waits_for_success_and_failure_keeps_draft() {
    for succeed in [true, false] {
        let mut hub = hub();
        edit(&mut hub);
        let expected = hub.draft_base.clone().unwrap();
        let saved = hub.draft.clone().unwrap();
        let _ = hub.update(Message::for_avd("hub-draft-b", Message::Select).unwrap());
        let _ = hub.update(Message::SaveAndContinue);
        assert!(hub.busy && !hub.modal_open());
        assert_eq!(hub.selected_avd_id(), Some("hub-draft-a"));
        let result = if succeed {
            let mut state = snapshot();
            state.profiles[0] = saved.clone();
            Ok(state)
        } else {
            Err("could not save".into())
        };
        let _ = hub.update(Message::Loaded(result, Work::Save(saved, expected)));
        assert!(!hub.busy && hub.saving_navigation.is_none());
        if succeed {
            assert_eq!(hub.selected_avd_id(), Some("hub-draft-b"));
            assert!(!hub.has_unsaved_settings());
        } else {
            assert_eq!(hub.selected_avd_id(), Some("hub-draft-a"));
            assert_eq!(hub.draft.as_ref().unwrap().name, "My draft");
            assert!(hub.has_unsaved_settings() && hub.settings_open);
            assert!(!hub.quit_requested);
        }
    }
}

#[test]
fn polls_preserve_dirty_base_detect_conflicts_and_refresh_clean_drafts() {
    let mut hub = hub();
    let mut external = snapshot();
    external.profiles[0].ram_mb = 4096;
    let _ = hub.update(Message::Loaded(
        Ok(external.clone()),
        Work::Poll(hub.command_epoch),
    ));
    assert_eq!(hub.draft.as_ref().unwrap().ram_mb, 4096);
    assert!(!hub.has_unsaved_settings());
    edit(&mut hub);
    external.profiles[0].cpus = 4;
    let _ = hub.update(Message::Loaded(
        Ok(external.clone()),
        Work::Poll(hub.command_epoch),
    ));
    assert_eq!(hub.draft.as_ref().unwrap().cpus, 2);
    assert_eq!(hub.draft.as_ref().unwrap().name, "My draft");
    assert!(hub.settings_conflict() && !hub.can_save_settings());
    let _ = hub.update(Message::Save);
    assert!(!hub.busy);
    let _ = hub.update(Message::DiscardSettings);
    assert_eq!(hub.draft.as_ref(), Some(&external.profiles[0]));
    assert!(!hub.settings_conflict() && !hub.has_unsaved_settings());
}

#[test]
fn external_removal_keeps_orphan_draft_and_missing_navigation_target_is_rejected() {
    let mut hub = hub();
    edit(&mut hub);
    let _ = hub.update(Message::for_avd("hub-draft-b", Message::Select).unwrap());
    let mut removed = snapshot();
    removed.profiles.clear();
    let _ = hub.update(Message::Loaded(Ok(removed), Work::Poll(hub.command_epoch)));
    assert_eq!(hub.draft.as_ref().unwrap().name, "My draft");
    assert!(hub.settings_conflict());
    let _ = hub.update(Message::DiscardAndContinue);
    assert!(!hub.busy);
    assert!(
        hub.feedback
            .persistent
            .iter()
            .any(|n| n.title == "Máy đích không còn trong danh sách.")
    );
}

#[test]
fn close_create_start_and_quit_all_resolve_dirty_settings_first() {
    for (message, expected) in [
        (Message::ToggleSettings, Navigation::CloseSettings),
        (Message::Create, Navigation::Create),
        (
            Message::for_avd("hub-draft-a", Message::StartDevice).unwrap(),
            Navigation::Start(DeviceId::Emulator(EmulatorId::new("hub-draft-a").unwrap())),
        ),
        (Message::RequestQuit, Navigation::Quit),
    ] {
        let mut hub = hub();
        edit(&mut hub);
        let _ = hub.update(message);
        assert_eq!(hub.pending_navigation, Some(expected));
        assert!(!hub.busy && !hub.quit_requested && hub.settings_open);
        let _ = hub.update(Message::CancelModal);
        assert!(hub.has_unsaved_settings());
        assert!(!hub.quit_requested);
    }
    let mut hub = hub();
    edit(&mut hub);
    let _ = hub.update(Message::RequestQuit);
    let _ = hub.update(Message::DiscardAndContinue);
    assert!(hub.quit_requested && hub.quitting);
}

#[test]
fn failed_save_does_not_quit_and_saved_start_uses_the_new_profile() {
    for quit in [true, false] {
        let mut hub = hub();
        edit(&mut hub);
        let original = hub.draft_base.clone().unwrap();
        let saved = hub.draft.clone().unwrap();
        let _ = hub.update(if quit {
            Message::RequestQuit
        } else {
            Message::for_avd("hub-draft-a", Message::StartDevice).unwrap()
        });
        let _ = hub.update(Message::SaveAndContinue);
        if quit {
            let _ = hub.update(Message::Loaded(
                Err("write failed".into()),
                Work::Save(saved, original),
            ));
            assert!(!hub.quit_requested && !hub.quitting && hub.has_unsaved_settings());
        } else {
            let mut state = snapshot();
            state.profiles[0] = saved.clone();
            let _ = hub.update(Message::Loaded(Ok(state), Work::Save(saved, original)));
            assert_eq!(hub.state.as_ref().unwrap().profiles[0].name, "My draft");
            assert!(!hub.has_unsaved_settings());
            assert!(hub.busy);
            assert_eq!(hub.lifecycle_action.as_ref().unwrap().id, "hub-draft-a");
        }
    }
}

#[test]
fn quit_during_save_supersedes_navigation_and_stay_cancels_both_intents() {
    for stay in [false, true] {
        let mut hub = hub();
        edit(&mut hub);
        let original = hub.draft_base.clone().unwrap();
        let saved = hub.draft.clone().unwrap();
        let _ = hub.update(Message::for_avd("hub-draft-b", Message::Select).unwrap());
        let _ = hub.update(Message::SaveAndContinue);
        let _ = hub.update(Message::RequestQuit);
        assert_eq!(hub.pending_navigation, Some(Navigation::Quit));
        let _ = hub.update(Message::ModalBackTab);
        assert_eq!(hub.modal_focus, 2);
        if stay {
            let _ = hub.update(Message::CancelModal);
        }
        let mut state = snapshot();
        state.profiles[0] = saved.clone();
        let _ = hub.update(Message::Loaded(Ok(state), Work::Save(saved, original)));
        assert_eq!(hub.selected_avd_id(), Some("hub-draft-a"));
        assert_eq!(hub.quit_requested, !stay);
        assert!(hub.pending_navigation.is_none() && hub.saving_navigation.is_none());
    }
}
