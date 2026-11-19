//! Device identity and policy. Physical identities never represent AVD paths.
use std::collections::BTreeMap;
use std::fmt;
use std::num::NonZeroU64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceKind {
    Emulator,
    Physical,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EmulatorId(String);
impl EmulatorId {
    pub fn new(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        crate::validate_profile_id(&value)?;
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An opaque ADB identity, deliberately not a filesystem path or a Profile.
/// No serialization: physical discovery metadata is session-only for now.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysicalId(String);
impl PhysicalId {
    pub fn new(serial: impl Into<String>) -> Result<Self, String> {
        let serial = serial.into();
        if serial.is_empty()
            || serial.len() > 256
            || serial.chars().any(|c| c.is_whitespace() || c.is_control())
            || serial.contains(['/', '\\'])
            || serial == "."
            || serial == ".."
        {
            return Err("Invalid physical device identity".into());
        }
        Ok(Self(serial))
    }
    /// Only pass as one argument to ADB after validating the current session.
    pub fn adb_serial(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeviceId {
    Emulator(EmulatorId),
    Physical(PhysicalId),
}
impl DeviceId {
    pub fn as_emulator(&self) -> Option<&EmulatorId> {
        match self {
            Self::Emulator(id) => Some(id),
            Self::Physical(_) => None,
        }
    }

    pub fn kind(&self) -> DeviceKind {
        match self {
            Self::Emulator(_) => DeviceKind::Emulator,
            Self::Physical(_) => DeviceKind::Physical,
        }
    }
    pub fn capabilities(&self) -> DeviceCapabilities {
        DeviceCapabilities(self.kind())
    }
}
impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Emulator(id) => write!(f, "emulator:{}", id.as_str()),
            Self::Physical(id) => write!(f, "physical:{}", id.adb_serial()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceOperation {
    StartEmulator,
    StopEmulator,
    ConfigureAvd,
    ArchiveAvd,
    DeleteAvd,
    BackupAvd,
    RestoreAvd,
    OpenDisplay,
    SendInput,
    InstallApk,
    WriteClipboard,
    DisconnectPhysical,
}

/// Static operation policy, not proof of authorization/readiness on a transport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeviceCapabilities(DeviceKind);
impl DeviceCapabilities {
    pub fn allows(self, operation: DeviceOperation) -> bool {
        use DeviceOperation::*;
        match operation {
            StartEmulator | StopEmulator | ConfigureAvd | ArchiveAvd | DeleteAvd | BackupAvd
            | RestoreAvd => self.0 == DeviceKind::Emulator,
            DisconnectPhysical => self.0 == DeviceKind::Physical,
            OpenDisplay | SendInput | InstallApk | WriteClipboard => true,
        }
    }
    pub fn require(self, operation: DeviceOperation) -> Result<(), UnsupportedOperation> {
        if self.allows(operation) {
            Ok(())
        } else {
            Err(UnsupportedOperation {
                kind: self.0,
                operation,
            })
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnsupportedOperation {
    pub kind: DeviceKind,
    pub operation: DeviceOperation,
}
impl fmt::Display for UnsupportedOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Operation {:?} is unavailable for {:?} devices",
            self.operation, self.kind
        )
    }
}
impl std::error::Error for UnsupportedOperation {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhysicalSession {
    device: PhysicalId,
    transport: NonZeroU64,
    generation: NonZeroU64,
}
impl PhysicalSession {
    pub fn device(&self) -> &PhysicalId {
        &self.device
    }
    pub fn transport(&self) -> NonZeroU64 {
        self.transport
    }
    pub fn generation(&self) -> NonZeroU64 {
        self.generation
    }
}

/// One owner per discovery lifetime. Reconnect always allocates a new generation,
/// including a reused serial/transport. Never recreate this owner on refresh.
#[derive(Default, Debug)]
pub struct PhysicalSessions {
    last_generation: u64,
    active: BTreeMap<PhysicalId, PhysicalSession>,
}
impl PhysicalSessions {
    pub fn connect(
        &mut self,
        device: PhysicalId,
        transport: NonZeroU64,
    ) -> Result<PhysicalSession, String> {
        let next = self
            .last_generation
            .checked_add(1)
            .ok_or("Connection generation exhausted")?;
        let session = PhysicalSession {
            device: device.clone(),
            transport,
            generation: NonZeroU64::new(next).unwrap(),
        };
        self.last_generation = next;
        self.active
            .retain(|_, current| current.transport != transport);
        self.active.insert(device, session.clone());
        Ok(session)
    }
    pub fn is_current(&self, session: &PhysicalSession) -> bool {
        self.active.get(&session.device) == Some(session)
    }
    /// A stale disconnect/result cannot remove a replacement connection.
    pub fn disconnect(&mut self, session: &PhysicalSession) -> bool {
        if !self.is_current(session) {
            return false;
        }
        self.active.remove(&session.device);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn namespaces_do_not_collide_even_when_raw_ids_match() {
        let a = DeviceId::Emulator(EmulatorId::new("hub-1").unwrap());
        let b = DeviceId::Physical(PhysicalId::new("hub-1").unwrap());
        assert_ne!(a, b);
        assert_ne!(a.to_string(), b.to_string());
        let map = BTreeMap::from([(a, 1), (b, 2)]);
        assert_eq!(map.len(), 2);
    }
    #[test]
    fn identity_rejects_paths_and_malformed_serials() {
        for value in [
            "",
            ".",
            "..",
            "../hub-1",
            "/tmp/phone",
            "a\\b",
            "a b",
            "a\n",
        ] {
            assert!(PhysicalId::new(value).is_err(), "{value:?}");
        }
        assert!(EmulatorId::new("physical:hub-1").is_err());
        assert!(PhysicalId::new("RFCRA1E91RH").is_ok());
    }
    #[test]
    fn physical_cannot_receive_any_avd_operation() {
        let phone = DeviceId::Physical(PhysicalId::new("hub-1").unwrap());
        let avd = DeviceId::Emulator(EmulatorId::new("hub-1").unwrap());
        use DeviceOperation::*;
        for op in [
            StartEmulator,
            StopEmulator,
            ConfigureAvd,
            ArchiveAvd,
            DeleteAvd,
            BackupAvd,
            RestoreAvd,
        ] {
            assert!(phone.capabilities().require(op).is_err());
            assert!(avd.capabilities().require(op).is_ok());
        }
        for op in [
            OpenDisplay,
            SendInput,
            InstallApk,
            WriteClipboard,
            DisconnectPhysical,
        ] {
            assert!(phone.capabilities().allows(op));
        }
    }
    #[test]
    fn late_results_and_disconnects_cannot_replace_new_session() {
        let mut sessions = PhysicalSessions::default();
        let id = PhysicalId::new("phone").unwrap();
        let transport = NonZeroU64::new(7).unwrap();
        let first = sessions.connect(id.clone(), transport).unwrap();
        let second = sessions.connect(id.clone(), transport).unwrap();
        assert!(!sessions.is_current(&first));
        assert!(!sessions.disconnect(&first));
        assert!(sessions.is_current(&second));
        assert!(sessions.disconnect(&second));
        assert!(!sessions.is_current(&second));
        let third = sessions.connect(id, NonZeroU64::new(8).unwrap()).unwrap();
        assert!(third.generation() > second.generation());
        assert!(sessions.is_current(&third));
    }
    #[test]
    fn reused_transport_invalidates_previous_device_owner() {
        let mut sessions = PhysicalSessions::default();
        let transport = NonZeroU64::new(1).unwrap();
        let old = sessions
            .connect(PhysicalId::new("old").unwrap(), transport)
            .unwrap();
        let new = sessions
            .connect(PhysicalId::new("new").unwrap(), transport)
            .unwrap();
        assert!(!sessions.is_current(&old));
        assert!(!sessions.disconnect(&old));
        assert!(sessions.is_current(&new));
    }
    #[test]
    fn generation_exhaustion_does_not_reuse_an_old_token() {
        let mut sessions = PhysicalSessions {
            last_generation: u64::MAX,
            ..Default::default()
        };
        assert!(
            sessions
                .connect(
                    PhysicalId::new("phone").unwrap(),
                    NonZeroU64::new(1).unwrap()
                )
                .is_err()
        );
        assert!(sessions.active.is_empty());
    }
}
