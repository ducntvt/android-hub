//! Window bootstrap and subscriptions; no guest state mutations.
use super::{Hub, Message, lifecycle, shortcuts, workspace};
use crate::{appearance, quit};
use android_hub_domain::{DEFAULT_FPS, FPS_OPTIONS};
use iced::Subscription;
use std::time::Duration;

pub(super) fn frame_updates(
    id: String,
    pid: u32,
    fps: u32,
    updates: std::sync::Arc<tokio::sync::Notify>,
) -> impl iced::futures::Stream<Item = Message> {
    iced::futures::stream::unfold((id, updates), move |(id, updates)| async move {
        // Cap UI work; Notify retains only one wakeup while frames accumulate.
        tokio::time::sleep(frame_interval(fps)).await;
        updates.notified().await;
        Some((Message::FrameReady(id.clone(), pid), (id, updates)))
    })
}

pub(super) fn frame_interval(fps: u32) -> Duration {
    let fps = if FPS_OPTIONS.contains(&fps) {
        fps
    } else {
        DEFAULT_FPS
    };
    Duration::from_secs_f64(1. / f64::from(fps))
}

pub(crate) fn run() -> iced::Result {
    quit::install();
    iced::application("Android Hub", Hub::update, Hub::view)
        .theme(|hub: &Hub| appearance::theme(hub.appearance.is_dark(hub.system_dark)))
        .subscription(|hub: &Hub| {
            let boots = hub
                .boot_checks
                .iter()
                .filter(|(_, check)| !check.finished && !hub.quit_requested)
                .filter_map(|(id, check)| {
                    let p = hub
                        .state
                        .as_ref()?
                        .profiles
                        .iter()
                        .find(|p| &p.id == id)?
                        .clone();
                    Some(Subscription::run_with_id(
                        ("boot", id.clone(), check.pid, check.epoch),
                        lifecycle::boot_updates(hub.backend.clone(), p, check.pid, check.epoch),
                    ))
                });
            let displays = hub.displays.iter().map(|(id, display)| {
                let id = id.clone();
                let pid = display.pid;
                // Use saved device settings, never an unsaved inspector draft.
                let fps = hub
                    .state
                    .as_ref()
                    .and_then(|s| s.profiles.iter().find(|p| p.id == id))
                    .map_or(DEFAULT_FPS, |p| p.fps);
                let updates = display.session.updates();
                Subscription::run_with_id(
                    (id.clone(), pid, fps),
                    frame_updates(id, pid, fps, updates),
                )
            });
            Subscription::batch(displays.chain(boots).chain([
                hub.connectivity_subscription(),
                iced::time::every(Duration::from_secs(3)).map(|_| Message::Tick),
                Subscription::run_with_id(
                    "native-quit",
                    iced::futures::stream::unfold(quit::requests(), |requests| async move {
                        requests.notified().await;
                        Some((Message::RequestQuit, requests))
                    }),
                ),
                iced::event::listen_with(|event, status, _| match event {
                    iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                        key,
                        modifiers,
                        ..
                    }) => shortcuts::command(&key, modifiers, status),
                    iced::Event::Window(iced::window::Event::CloseRequested) => {
                        Some(Message::RequestQuit)
                    }
                    iced::Event::Window(iced::window::Event::FileHovered(_)) => {
                        Some(Message::ApkHover(true))
                    }
                    iced::Event::Window(iced::window::Event::FilesHoveredLeft) => {
                        Some(Message::ApkHover(false))
                    }
                    iced::Event::Window(iced::window::Event::FileDropped(path)) => {
                        Some(Message::DropApk(path))
                    }
                    iced::Event::Window(iced::window::Event::Unfocused) => {
                        Some(Message::ReleaseInputs)
                    }
                    _ => None,
                }),
            ]))
        })
        .window(iced::window::Settings {
            exit_on_close_request: false,
            size: workspace::initial_size(std::env::var("ANDROID_HUB_WINDOW_SIZE").ok().as_deref()),
            min_size: Some(iced::Size::new(1000., 640.)),
            ..Default::default()
        })
        .run_with(Hub::new)
}
