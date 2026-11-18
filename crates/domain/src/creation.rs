//! User choices only: identity, ports, paths and image selection belong to the backend.
use crate::{DEFAULT_FPS, Profile};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateRequest {
    pub name: String,
    pub ram_mb: u32,
    pub cpus: u32,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

impl CreateRequest {
    pub fn suggested(profiles: &[Profile]) -> Self {
        let number = (1..)
            .find(|n| {
                let candidate = format!("Android {n}");
                !profiles
                    .iter()
                    .any(|p| p.name.trim().eq_ignore_ascii_case(&candidate))
            })
            .expect("finite profile list");
        Self {
            name: format!("Android {number}"),
            ram_mb: 3072,
            cpus: 2,
            width: 720,
            height: 1280,
            fps: DEFAULT_FPS,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        self.profile("hub-validation".into(), 5584).map(|_| ())
    }

    pub fn profile(&self, id: String, port: u16) -> Result<Profile, String> {
        // Validate the original name before trimming: controls must not disappear
        // during normalization, and the editor and mutation use identical limits.
        let mut profile = Profile {
            id,
            name: self.name.clone(),
            port,
            ram_mb: self.ram_mb,
            cpus: self.cpus,
            width: self.width,
            height: self.height,
            fps: self.fps,
        };
        profile.validate()?;
        profile.name = profile.name.trim().to_owned();
        Ok(profile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggestion_fills_gaps_without_reusing_an_existing_display_name() {
        let profiles = vec![
            Profile::standard("hub-a".into(), " android 1 ".into(), 5584),
            Profile::standard("hub-b".into(), "Android 3".into(), 5586),
        ];
        let request = CreateRequest::suggested(&profiles);
        assert_eq!(request.name, "Android 2");
        assert!(request.validate().is_ok());
    }

    #[test]
    fn custom_creation_preserves_resources_but_rejects_invalid_input() {
        let mut request = CreateRequest::suggested(&[]);
        request.name = "  My game  ".into();
        request.ram_mb = 4096;
        request.cpus = 4;
        request.width = 1920;
        request.height = 1080;
        request.fps = 60;
        let p = request.profile("hub-created".into(), 5660).unwrap();
        assert_eq!(p.name, "My game");
        assert_eq!(
            (p.ram_mb, p.cpus, p.width, p.height, p.fps),
            (4096, 4, 1920, 1080, 60)
        );
        for name in ["", "   ", "game\n", &"a".repeat(49)] {
            let mut invalid = request.clone();
            invalid.name = name.into();
            assert!(invalid.validate().is_err());
        }
        request.ram_mb = 999;
        assert!(request.validate().is_err());
        request.ram_mb = 4096;
        request.width = 1919;
        assert!(request.validate().is_err());
        request.width = 1920;
        for fps in crate::FPS_OPTIONS {
            request.fps = fps;
            assert_eq!(
                request.profile("hub-created".into(), 5660).unwrap().fps,
                fps
            );
        }
        request.fps = 144;
        assert!(request.validate().is_err());
    }
}
