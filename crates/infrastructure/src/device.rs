//! Typed dispatch boundary. Reject an incompatible target before any I/O.
use crate::{
    Backend, DeletePlan, PurgePlan, RemovalOutcome, RemovedDevice, RestoreOutcome, Snapshot,
};
use android_hub_domain::{DeviceId, DeviceOperation, EmulatorId, Profile};
use std::path::{Path, PathBuf};

pub struct DeviceBackend<'a> {
    backend: &'a Backend,
    target: DeviceId,
}
impl Backend {
    /// Emulator-only adapter for the existing AVD UI. Physical selection must use
    /// `device` with its typed identity instead of converting a serial here.
    pub fn emulator(&self, id: &str) -> Result<DeviceBackend<'_>, String> {
        Ok(self.device(DeviceId::Emulator(EmulatorId::new(id)?)))
    }
    pub fn device(&self, target: DeviceId) -> DeviceBackend<'_> {
        DeviceBackend {
            backend: self,
            target,
        }
    }
}
impl DeviceBackend<'_> {
    fn avd(&self, operation: DeviceOperation) -> Result<&str, String> {
        self.target
            .capabilities()
            .require(operation)
            .map_err(|e| e.to_string())?;
        match &self.target {
            DeviceId::Emulator(id) => Ok(id.as_str()),
            DeviceId::Physical(_) => Err("An AVD target is required".into()),
        }
    }
    fn profile(&self, operation: DeviceOperation, profile: &Profile) -> Result<(), String> {
        let id = self.avd(operation)?;
        if id != profile.id {
            return Err("Device target does not match the AVD profile".into());
        }
        profile.validate()
    }
    pub fn start(&self) -> Result<Snapshot, String> {
        self.backend
            .start(self.avd(DeviceOperation::StartEmulator)?)
    }
    /// For explicit CLI shutdown of the currently observed owned AVD. UI confirmations
    /// must use `stop` with the PID captured when the dialog was opened.
    pub fn stop_current(&self) -> Result<Snapshot, String> {
        self.backend.stop(self.avd(DeviceOperation::StopEmulator)?)
    }
    pub fn stop(&self, expected_pid: u32) -> Result<Snapshot, String> {
        self.backend
            .stop_session(self.avd(DeviceOperation::StopEmulator)?, expected_pid)
    }
    pub fn save(&self, expected: &Profile, profile: Profile) -> Result<Snapshot, String> {
        self.profile(DeviceOperation::ConfigureAvd, expected)?;
        self.profile(DeviceOperation::ConfigureAvd, &profile)?;
        self.backend.save_if_unchanged(expected, profile)
    }
    pub fn archive(&self, expected: &Profile) -> Result<RemovalOutcome, String> {
        self.profile(DeviceOperation::ArchiveAvd, expected)?;
        self.backend.remove_device(expected)
    }
    pub fn prepare_delete(&self, expected: &Profile) -> Result<DeletePlan, String> {
        self.profile(DeviceOperation::DeleteAvd, expected)?;
        self.backend.prepare_delete(expected)
    }
    pub fn delete(&self, plan: &DeletePlan) -> Result<Snapshot, String> {
        self.profile(DeviceOperation::DeleteAvd, &plan.profile)?;
        self.backend.delete_device(plan)
    }
    pub fn undo_archive(&self, info: &RemovedDevice) -> Result<Snapshot, String> {
        self.profile(DeviceOperation::RestoreAvd, &info.profile)?;
        self.backend
            .undo_remove_device(&info.archive, &info.profile)
    }
    pub fn prepare_purge(&self, info: &RemovedDevice) -> Result<PurgePlan, String> {
        self.profile(DeviceOperation::DeleteAvd, &info.profile)?;
        self.backend.prepare_purge(info)
    }
    pub fn purge(&self, plan: &PurgePlan) -> Result<Snapshot, String> {
        self.profile(DeviceOperation::DeleteAvd, &plan.device.profile)?;
        self.backend.purge_device(plan)
    }
    pub fn backup(&self) -> Result<PathBuf, String> {
        self.backend.backup(self.avd(DeviceOperation::BackupAvd)?)
    }
    pub fn backup_before_delete(&self, plan: &DeletePlan) -> Result<PathBuf, String> {
        self.profile(DeviceOperation::BackupAvd, &plan.profile)?;
        self.backend.backup_before_delete(plan)
    }
    pub fn restore(&self, backup: &Path, manifest: &str) -> Result<RestoreOutcome, String> {
        self.backend
            .restore(self.avd(DeviceOperation::RestoreAvd)?, backup, manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use android_hub_domain::{EmulatorId, PhysicalId};
    fn backend() -> Backend {
        Backend {
            root: std::env::temp_dir().join(format!("hub-us057-no-io-{}", std::process::id())),
            sdk: PathBuf::from("/missing-us057-sdk"),
        }
    }
    #[test]
    fn physical_dispatch_is_rejected_before_creating_registry_or_running_sdk() {
        let backend = backend();
        assert!(!backend.root.exists());
        // A matching raw AVD identifier must still not escape the namespace.
        let target = DeviceId::Physical(PhysicalId::new("hub-1").unwrap());
        let facade = backend.device(target.clone());
        let profile = Profile::standard("hub-1".into(), "QA".into(), 5584);
        let results = [
            facade.start().map(|_| ()),
            facade.stop(1).map(|_| ()),
            facade.save(&profile, profile.clone()).map(|_| ()),
            facade.archive(&profile).map(|_| ()),
            facade.prepare_delete(&profile).map(|_| ()),
            facade.backup().map(|_| ()),
            facade.restore(Path::new("/missing"), "invalid").map(|_| ()),
        ];
        for result in results {
            assert!(result.unwrap_err().contains("unavailable for Physical"));
        }
        assert!(!backend.root.exists());
    }
    #[test]
    fn a_profile_cannot_redirect_an_emulator_target() {
        let backend = backend();
        let target = DeviceId::Emulator(EmulatorId::new("hub-1").unwrap());
        let other = Profile::standard("hub-2".into(), "Other".into(), 5586);
        assert!(
            backend
                .device(target.clone())
                .save(&other, other.clone())
                .unwrap_err()
                .contains("does not match")
        );
        assert!(
            backend
                .device(target.clone())
                .prepare_delete(&other)
                .unwrap_err()
                .contains("does not match")
        );
        assert!(!backend.root.exists());
    }
}
