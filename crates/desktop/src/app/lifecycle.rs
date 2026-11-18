use super::*;
use android_hub_infrastructure::BootObservation;
use iced::futures::StreamExt;
use std::time::Instant;

pub(super) struct BootCheck {
    pub pid: u32,
    pub epoch: u64,
    pub since: Instant,
    pub result: Option<Result<BootObservation, String>>,
    pub finished: bool,
}
impl BootCheck {
    fn new(pid: u32) -> Self {
        Self {
            pid,
            epoch: 0,
            since: Instant::now(),
            result: None,
            finished: false,
        }
    }
    pub fn retry(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        self.since = Instant::now();
        self.result = None;
        self.finished = false;
    }
}
pub(super) struct Action {
    pub id: String,
    pub starting: bool,
    pub since: Instant,
}

impl Hub {
    pub(super) fn reconcile_boot_checks(&mut self) {
        let Some(state) = &self.state else {
            return;
        };
        self.boot_checks
            .retain(|id, check| state.runtime.get(id).and_then(|r| r.pid) == Some(check.pid));
        for (id, runtime) in &state.runtime {
            if let Some(pid) = runtime.pid {
                self.boot_checks
                    .entry(id.clone())
                    .or_insert_with(|| BootCheck::new(pid));
            }
        }
    }
    pub(super) fn device_phase(&self, id: &str) -> String {
        self.device_attention(id)
            .unwrap_or_else(|| self.language.tr("Sẵn sàng điều khiển").into())
    }

    /// The workspace only needs status text when the device is not ready.
    pub(super) fn device_attention(&self, id: &str) -> Option<String> {
        let lang = self.language;
        if let Some(action) = &self.lifecycle_action
            && action.id == id
        {
            return Some(format!(
                "{} · {}s",
                lang.tr(if action.starting {
                    "Đang mở engine…"
                } else {
                    "Đang dừng máy…"
                }),
                action.since.elapsed().as_secs()
            ));
        }
        if self.lifecycle_errors.contains_key(id) {
            return Some(lang.tr("Cần kiểm tra máy").into());
        }
        let Some(check) = self.boot_checks.get(id) else {
            return Some(
                lang.tr(if self.stale {
                    "Đang xác minh trạng thái thiết bị"
                } else {
                    "Đã dừng"
                })
                .into(),
            );
        };
        // A reconnect rechecks boot identity. That probe must not hide the
        // display failure that triggered it, even while ADB is unavailable.
        if self.displays.get(id).is_some_and(|d| d.error.is_some()) {
            return Some(lang.tr("Mất kết nối màn hình · Đang kết nối lại…").into());
        }
        let elapsed = check.since.elapsed().as_secs();
        if check.result.is_none() {
            return Some(format!(
                "{} · {}s",
                lang.tr("Đang kiểm tra Android…"),
                elapsed
            ));
        }
        if let Some(Err(_)) = &check.result {
            return Some(format!(
                "{} · {}s",
                lang.tr("Chưa xác minh được Android"),
                elapsed
            ));
        }
        if !check
            .result
            .as_ref()
            .is_some_and(|r| r.as_ref().is_ok_and(|v| v.completed))
        {
            return Some(format!(
                "{} · {}s",
                lang.tr("Android đang khởi động…"),
                elapsed
            ));
        }
        match self.displays.get(id) {
            Some(d) if d.image.is_some() && d.session.input_generation().is_some() => None,
            Some(_) => Some(lang.tr("Đang kết nối màn hình…").into()),
            None => Some(lang.tr("Android đã khởi động").into()),
        }
    }
}

/// Subscription cancellation drops observe_boot and kills its current child.
/// No readiness polling remains after success or the two-minute observation cap.
pub(super) fn boot_updates(
    backend: Backend,
    profile: Profile,
    pid: u32,
    epoch: u64,
) -> impl iced::futures::Stream<Item = Message> {
    iced::futures::stream::unfold(
        (backend, profile, Instant::now(), false, false),
        move |(backend, profile, since, delay, finished)| async move {
            if finished {
                return None;
            }
            if delay {
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            let result = backend.observe_boot(&profile, pid).await;
            let complete = result.as_ref().is_ok_and(|v| v.completed);
            let expired = since.elapsed() >= Duration::from_secs(120);
            let result = if expired && !complete {
                Err("Android chưa sẵn sàng sau thời gian chờ; có thể kiểm tra lại.".into())
            } else {
                result
            };
            let message =
                Message::BootObserved(profile.id.clone(), pid, epoch, complete || expired, result);
            Some((
                message,
                (backend, profile, since, true, complete || expired),
            ))
        },
    )
}

pub(super) fn shutdown_updates(backend: Backend) -> impl iced::futures::Stream<Item = Message> {
    iced::futures::stream::once(async move {
        let (events, receiver) = tokio::sync::mpsc::unbounded_channel();
        let finished = events.clone();
        tokio::spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                backend.stop_all_with_progress(|event| {
                    let _ = events.send(Message::ShutdownProgress(event));
                })
            })
            .await
            .map_err(|error| format!("Shutdown worker: {error}"))
            .and_then(|result| result);
            let _ = finished.send(Message::ShutdownFinished(result));
        });
        receiver
    })
    .flat_map(|receiver| {
        iced::futures::stream::unfold(receiver, |mut receiver| async move {
            receiver.recv().await.map(|message| (message, receiver))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn observation() -> BootObservation {
        BootObservation {
            boot_id: "12345678-1234-1234-1234-123456789abc".into(),
            completed: true,
        }
    }
    #[test]
    fn late_readiness_cannot_cross_pid_or_retry_epoch() {
        let (mut hub, _) = Hub::new();
        hub.language = Language::Vi;
        hub.boot_checks
            .insert("hub-test".into(), BootCheck::new(42));
        let _ = hub.update(Message::BootObserved(
            "hub-test".into(),
            41,
            0,
            true,
            Ok(observation()),
        ));
        assert!(hub.boot_checks["hub-test"].result.is_none());
        let _ = hub.update(Message::RetryBoot("hub-test".into()));
        let _ = hub.update(Message::BootObserved(
            "hub-test".into(),
            42,
            0,
            true,
            Ok(observation()),
        ));
        assert!(hub.boot_checks["hub-test"].result.is_none());
        let _ = hub.update(Message::BootObserved(
            "hub-test".into(),
            42,
            1,
            true,
            Ok(observation()),
        ));
        assert!(hub.boot_checks["hub-test"].finished);
        assert_eq!(hub.device_phase("hub-test"), "Android đã khởi động");
        hub.boot_checks.clear();
        let _ = hub.update(Message::BootObserved(
            "hub-test".into(),
            42,
            1,
            true,
            Ok(observation()),
        ));
        assert!(hub.boot_checks.is_empty());
    }
    #[test]
    fn failed_probe_is_not_ready_and_retry_clears_it() {
        let (mut hub, _) = Hub::new();
        hub.language = Language::Vi;
        hub.boot_checks
            .insert("hub-test".into(), BootCheck::new(42));
        let _ = hub.update(Message::BootObserved(
            "hub-test".into(),
            42,
            0,
            true,
            Err("timeout".into()),
        ));
        assert!(
            hub.device_phase("hub-test")
                .starts_with("Chưa xác minh được Android")
        );
        let _ = hub.update(Message::RetryBoot("hub-test".into()));
        assert!(!hub.boot_checks["hub-test"].finished);
        assert!(hub.boot_checks["hub-test"].result.is_none());
    }
    #[test]
    fn display_failure_stays_visible_during_boot_revalidation() {
        let (mut hub, _) = Hub::new();
        hub.language = Language::Vi;
        let profile = Profile::standard("hub-test".into(), "QA".into(), 5584);
        // No registry or guest exists here; the session cannot connect to a
        // user's engine. Only presentation state is exercised by this test.
        let backend = Backend {
            root: PathBuf::from("/nonexistent/android-hub-display-label-test"),
            sdk: PathBuf::from("/nonexistent/android-sdk"),
        };
        hub.displays.insert(
            profile.id.clone(),
            DisplayState {
                pid: 42,
                session: Session::open(backend, profile, 42),
                image: None,
                size: (480, 854),
                rotation: 0,
                error: Some("Input quá thời gian".into()),
                input_generation: None,
            },
        );
        hub.boot_checks
            .insert("hub-test".into(), BootCheck::new(42));
        for result in [
            None,
            Some(Err("ADB unavailable".into())),
            Some(Ok(observation())),
        ] {
            hub.boot_checks.get_mut("hub-test").unwrap().result = result;
            assert_eq!(
                hub.device_phase("hub-test"),
                "Mất kết nối màn hình · Đang kết nối lại…"
            );
        }
        hub.language = Language::En;
        assert_eq!(
            hub.device_phase("hub-test"),
            "Display disconnected · Reconnecting…"
        );
        hub.displays.get_mut("hub-test").unwrap().error = None;
        assert_eq!(hub.device_phase("hub-test"), "Connecting display…");
    }
    #[test]
    fn shutdown_progress_retains_other_devices_after_a_failure() {
        use android_hub_infrastructure::{ShutdownProgress, ShutdownStatus};
        let (mut hub, _) = Hub::new();
        hub.language = Language::Vi;
        hub.quitting = true;
        hub.quit_requested = true;
        for (id, status) in [
            ("a", ShutdownStatus::Waiting),
            ("b", ShutdownStatus::Waiting),
            ("a", ShutdownStatus::Failed("refused".into())),
            ("b", ShutdownStatus::Stopped),
        ] {
            let _ = hub.update(Message::ShutdownProgress(ShutdownProgress {
                id: id.into(),
                name: id.into(),
                status,
            }));
        }
        let _ = hub.update(Message::ShutdownFinished(Err("a remains running".into())));
        assert!(hub.quit_requested && !hub.quitting);
        assert_eq!(hub.shutdown_progress.len(), 2);
        assert!(matches!(
            hub.shutdown_progress[0].status,
            ShutdownStatus::Failed(_)
        ));
        assert_eq!(hub.shutdown_progress[1].status, ShutdownStatus::Stopped);
    }
}
