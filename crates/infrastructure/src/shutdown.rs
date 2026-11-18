//! Aggregate per-device shutdown attempts without hiding partial failures.
use crate::Snapshot;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShutdownStatus {
    Waiting,
    Stopping,
    Stopped,
    Failed(String),
}
#[derive(Clone, Debug)]
pub struct ShutdownProgress {
    pub id: String,
    pub name: String,
    pub status: ShutdownStatus,
}
pub(super) fn stop_registered_progress(
    state: &Snapshot,
    mut stop: impl FnMut(&str) -> Result<(), String>,
    mut progress: impl FnMut(ShutdownProgress),
) -> Vec<String> {
    for p in &state.profiles {
        if state.runtime.get(&p.id).is_some_and(|r| r.pid.is_some()) {
            progress(ShutdownProgress {
                id: p.id.clone(),
                name: p.name.clone(),
                status: ShutdownStatus::Waiting,
            });
        }
    }
    state
        .profiles
        .iter()
        .filter(|p| state.runtime.get(&p.id).is_some_and(|r| r.pid.is_some()))
        .filter_map(|p| {
            progress(ShutdownProgress {
                id: p.id.clone(),
                name: p.name.clone(),
                status: ShutdownStatus::Stopping,
            });
            let result = stop(&p.id);
            progress(ShutdownProgress {
                id: p.id.clone(),
                name: p.name.clone(),
                status: match &result {
                    Ok(()) => ShutdownStatus::Stopped,
                    Err(error) => ShutdownStatus::Failed(error.clone()),
                },
            });
            result.err().map(|error| format!("{}: {error}", p.name))
        })
        .collect()
}

#[cfg(test)]
mod shutdown_tests {
    use super::*;
    use android_hub_domain::{Profile, Runtime};
    use std::collections::BTreeMap;
    #[test]
    fn shutdown_attempts_all_registered_running_engines_despite_errors() {
        let profiles = ["a", "b", "stopped"]
            .into_iter()
            .enumerate()
            .map(|(i, name)| {
                Profile::standard(format!("hub-{name}"), name.into(), 5580 + i as u16 * 2)
            })
            .collect();
        let runtime = ["hub-a", "hub-b", "foreign-engine"]
            .into_iter()
            .map(|id| {
                (
                    id.into(),
                    Runtime {
                        pid: Some(42),
                        ..Default::default()
                    },
                )
            })
            .collect();
        let state = Snapshot {
            profiles,
            runtime,
            android_versions: BTreeMap::new(),
        };
        let mut attempted = vec![];
        let mut progress = vec![];
        let errors = stop_registered_progress(
            &state,
            |id| {
                attempted.push(id.to_owned());
                if id == "hub-a" {
                    Err("identity mismatch".into())
                } else {
                    Ok(())
                }
            },
            |event| progress.push(event),
        );
        assert_eq!(progress.len(), 6);
        assert_eq!(progress[0].status, ShutdownStatus::Waiting);
        assert!(matches!(progress[3].status, ShutdownStatus::Failed(_)));
        assert_eq!(progress[5].status, ShutdownStatus::Stopped);
        assert_eq!(attempted, ["hub-a", "hub-b"]);
        assert_eq!(errors, ["a: identity mismatch"]);
    }
}
