mod device;
pub use device::{
    DeviceCapabilities, DeviceId, DeviceKind, DeviceOperation, EmulatorId, PhysicalId,
    PhysicalSession, PhysicalSessions, UnsupportedOperation,
};
use serde::{Deserialize, Serialize};
mod android;
pub use android::{AndroidVersion, ImageIdentity};
mod creation;
pub use creation::CreateRequest;

pub const FPS_OPTIONS: [u32; 5] = [15, 30, 60, 90, 120];
pub const DEFAULT_FPS: u32 = 30;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub port: u16,
    pub ram_mb: u32,
    pub cpus: u32,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    /// None means an unresolved legacy binding, never an implicit Android 16.
    #[serde(default)]
    pub image: Option<ImageIdentity>,
}
impl Profile {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(image) = &self.image {
            image.validate()?;
        }
        validate_profile_id(&self.id)?;
        if self.name.trim().is_empty()
            || self.name.chars().count() > 48
            || self.name.chars().any(char::is_control)
        {
            return Err("Tên máy phải có 1–48 ký tự".into());
        }
        if !(5580..=5680).contains(&self.port) || !self.port.is_multiple_of(2) {
            return Err("Port máy không hợp lệ".into());
        }
        if !(1024..=8192).contains(&self.ram_mb) || !(1..=8).contains(&self.cpus) {
            return Err("RAM: 1024–8192 MiB; CPU: 1–8".into());
        }
        if ![(480, 854), (720, 1280), (1080, 1920)]
            .contains(&(self.width.min(self.height), self.width.max(self.height)))
            || !FPS_OPTIONS.contains(&self.fps)
        {
            return Err("Độ phân giải hoặc FPS không được hỗ trợ".into());
        }
        Ok(())
    }
    pub fn dimensions_for_orientation(&self, landscape: bool) -> (u32, u32) {
        let short = self.width.min(self.height);
        let long = self.width.max(self.height);
        if landscape {
            (long, short)
        } else {
            (short, long)
        }
    }
    pub fn standard(id: String, name: String, port: u16) -> Self {
        Self {
            id,
            name,
            port,
            ram_mb: 3072,
            cpus: 2,
            width: 720,
            height: 1280,
            fps: DEFAULT_FPS,
            image: None,
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct Runtime {
    pub pid: Option<u32>,
    pub cpu_percent: Option<f32>,
    pub rss_kib: Option<u64>,
}

fn validate_profile_id(id: &str) -> Result<(), String> {
    if !id.starts_with("p0-google-") && !id.starts_with("hub-") {
        return Err("Tên kỹ thuật không thuộc Hub".into());
    }
    if id.len() > 48
        || !id
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
    {
        return Err("ID máy không hợp lệ".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_supported_orientations_and_maps_input_from_actual_frame() {
        let mut p = Profile::standard("hub-orientation".into(), "Test".into(), 5584);
        for (short, long) in [(480, 854), (720, 1280), (1080, 1920)] {
            for (w, h) in [(short, long), (long, short)] {
                p.width = w;
                p.height = h;
                assert!(p.validate().is_ok());
                assert_eq!(p.dimensions_for_orientation(true), (long, short));
                assert_eq!(p.dimensions_for_orientation(false), (short, long));
            }
        }
        p.width = 1280;
        p.height = 721;
        assert!(p.validate().is_err());
    }
    #[test]
    fn rejects_unsafe_identity_and_out_of_bounds_settings() {
        let base = Profile::standard("hub-1".into(), "Android".into(), 5584);
        assert!(base.validate().is_ok());
        let mut p = base.clone();
        p.id = "hub-../../other".into();
        assert!(p.validate().is_err());
        let mut p = base.clone();
        p.port = 5585;
        assert!(p.validate().is_err());
        let mut p = base.clone();
        p.ram_mb = 999999;
        assert!(p.validate().is_err());
        let mut p = base;
        p.fps = 0;
        assert!(p.validate().is_err());
    }
}
