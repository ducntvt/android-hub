//! USB discovery uses the shared loopback ADB server without restarting it.
//! No AVD, SDK image, Java, shell interpolation or serial-derived paths.
mod setup;
#[cfg(target_os = "macos")]
pub mod stream;
mod wire;
use android_hub_domain::PhysicalId;
use std::{collections::BTreeSet, num::NonZeroU64, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionState {
    Ready,
    Unauthorized,
    Offline,
    Disconnected,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportKind {
    Usb,
    Network,
    Emulator,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdbDevice {
    pub id: PhysicalId,
    pub transport: NonZeroU64,
    pub kind: TransportKind,
    pub state: ConnectionState,
    pub model: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhoneMetadata {
    pub model: String,
    pub android: String,
    pub emulator: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiscoveryError {
    Unavailable,
    Timeout,
    Version,
    Protocol,
    Rejected(String),
    Io(String),
}
impl std::fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for DiscoveryError {}

/// Strictly parse one complete snapshot; never turn malformed output into a
/// successful empty device list. USB must be positively identified by ADB.
pub fn parse_devices(raw: &str) -> Result<Vec<AdbDevice>, DiscoveryError> {
    let mut devices = Vec::new();
    let mut ids = BTreeSet::new();
    let mut transports = BTreeSet::new();
    for line in raw.lines().filter(|line| !line.trim().is_empty()) {
        if devices.len() >= 128 {
            return Err(DiscoveryError::Protocol);
        }
        let mut fields = line.split_whitespace();
        let serial = fields.next().ok_or(DiscoveryError::Protocol)?;
        let id = PhysicalId::new(serial).map_err(|_| DiscoveryError::Protocol)?;
        let state = match fields.next().ok_or(DiscoveryError::Protocol)? {
            "device" => ConnectionState::Ready,
            "unauthorized" => ConnectionState::Unauthorized,
            "offline" => ConnectionState::Offline,
            _ => ConnectionState::Unknown,
        };
        let mut transport = None;
        let mut usb = false;
        let mut model = None;
        for field in fields {
            if let Some(value) = field.strip_prefix("transport_id:") {
                if transport.is_some() {
                    return Err(DiscoveryError::Protocol);
                }
                transport = Some(
                    value
                        .parse::<NonZeroU64>()
                        .map_err(|_| DiscoveryError::Protocol)?,
                );
            } else if let Some(value) = field.strip_prefix("usb:") {
                usb = !value.is_empty();
            } else if let Some(value) = field.strip_prefix("model:") {
                if value.len() > 256 || value.chars().any(char::is_control) {
                    return Err(DiscoveryError::Protocol);
                }
                model = Some(value.replace('_', " "));
            }
        }
        let transport = transport.ok_or(DiscoveryError::Protocol)?;
        if !ids.insert(id.clone()) || !transports.insert(transport) {
            return Err(DiscoveryError::Protocol);
        }
        let kind = if serial.starts_with("emulator-") {
            TransportKind::Emulator
        } else if usb {
            TransportKind::Usb
        } else if serial.contains(':') || serial.ends_with("._adb-tls-connect._tcp") {
            TransportKind::Network
        } else {
            TransportKind::Unknown
        };
        devices.push(AdbDevice {
            id,
            transport,
            kind,
            state,
            model,
        });
    }
    Ok(devices)
}

/// One long-lived tracker per workspace. EOF/errors invalidate all session tokens
/// before retry. Idle connections do not time out merely because no phone changes.
#[derive(Default)]
pub struct AdbDiscovery {
    stream: Option<tokio::net::TcpStream>,
    retry: bool,
}
impl AdbDiscovery {
    pub async fn next(&mut self) -> Result<Vec<AdbDevice>, DiscoveryError> {
        if self.retry {
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
        let result = self.read_snapshot().await;
        if result.is_err() {
            self.stream = None;
        }
        self.retry = result.is_err();
        result
    }
    async fn read_snapshot(&mut self) -> Result<Vec<AdbDevice>, DiscoveryError> {
        let first = self.stream.is_none();
        if first {
            wire::check_version().await?;
            let mut stream = wire::connect().await?;
            wire::request(&mut stream, "host:track-devices-l").await?;
            self.stream = Some(stream);
        }
        let read = wire::tracked_frame(self.stream.as_mut().unwrap());
        let raw = if first {
            tokio::time::timeout(Duration::from_secs(4), read)
                .await
                .map_err(|_| DiscoveryError::Timeout)??
        } else {
            read.await?
        };
        parse_devices(&raw)
    }
}

pub async fn read_phone_metadata(transport: NonZeroU64) -> Result<PhoneMetadata, DiscoveryError> {
    wire::metadata(transport).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn distinguishes_usb_network_external_emulator_and_unknown() {
        let rows = parse_devices("phone device usb:1-1 model:Pixel_8 transport_id:1\nemulator-5554 device transport_id:2\n192.0.2.1:5555 offline transport_id:3\nodd-serial unauthorized transport_id:4\n").unwrap();
        assert_eq!(
            rows.iter().map(|d| d.kind).collect::<Vec<_>>(),
            vec![
                TransportKind::Usb,
                TransportKind::Emulator,
                TransportKind::Network,
                TransportKind::Unknown
            ]
        );
        assert_eq!(rows[0].model.as_deref(), Some("Pixel 8"));
        assert_eq!(rows[2].state, ConnectionState::Offline);
        assert_eq!(rows[3].state, ConnectionState::Unauthorized);
    }
    #[test]
    fn malformed_or_ambiguous_snapshot_is_not_an_empty_success() {
        for raw in [
            "x",
            "x device",
            "x device transport_id:0",
            "../x device transport_id:1",
            "x device transport_id:1 transport_id:2",
            "x device transport_id:1\ny device transport_id:1",
            "x device transport_id:1\nx offline transport_id:2",
        ] {
            assert!(parse_devices(raw).is_err(), "{raw}");
        }
        assert!(parse_devices("").unwrap().is_empty());
        assert_eq!(
            parse_devices("usb-phone recovery usb:1 transport_id:1").unwrap()[0].state,
            ConnectionState::Unknown
        );
    }
}
