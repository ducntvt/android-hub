//! Host presentation preference. Never changes guest configuration or macOS settings.
use iced::{Color, Theme, theme::Palette};
use std::{path::Path, sync::OnceLock};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Preference {
    #[default]
    System,
    Light,
    Dark,
}
impl Preference {
    pub const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];
    pub fn label(self) -> &'static str {
        match self {
            Self::System => "Hệ thống",
            Self::Light => "Sáng",
            Self::Dark => "Tối",
        }
    }
    fn key(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
    pub fn is_dark(self, system_dark: bool) -> bool {
        match self {
            Self::System => system_dark,
            Self::Light => false,
            Self::Dark => true,
        }
    }
    pub fn load(root: &Path) -> Self {
        match std::fs::read_to_string(root.join("ui-appearance")) {
            Ok(s) if s.trim() == "light" => Self::Light,
            Ok(s) if s.trim() == "dark" => Self::Dark,
            _ => Self::System,
        }
    }
    pub fn save(self, root: &Path) -> std::io::Result<()> {
        let dir = root;
        std::fs::create_dir_all(dir)?;
        let staging = dir.join(format!(".ui-appearance-{}.tmp", std::process::id()));
        std::fs::write(&staging, format!("{}\n", self.key()))?;
        let result = std::fs::rename(&staging, dir.join("ui-appearance"));
        if result.is_err() {
            let _ = std::fs::remove_file(staging);
        }
        result
    }
}

/// Called on Iced's main thread at startup and the existing 3-second refresh.
/// MainThreadMarker keeps AppKit off worker threads (including unit tests).
#[cfg(target_os = "macos")]
pub fn system_dark() -> Option<bool> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSApplication};
    use objc2_foundation::NSArray;
    let main = MainThreadMarker::new()?;
    let appearance = NSApplication::sharedApplication(main).effectiveAppearance();
    // Apple exports immutable NSString constants. No appearance override is set,
    // so application.effectiveAppearance continues to inherit macOS settings.
    unsafe {
        let names = NSArray::from_slice(&[NSAppearanceNameAqua, NSAppearanceNameDarkAqua]);
        appearance
            .bestMatchFromAppearancesWithNames(&names)
            .map(|name| &*name == NSAppearanceNameDarkAqua)
    }
}
#[cfg(not(target_os = "macos"))]
pub fn system_dark() -> Option<bool> {
    None
}

pub fn theme(dark: bool) -> Theme {
    static LIGHT: OnceLock<Theme> = OnceLock::new();
    static DARK: OnceLock<Theme> = OnceLock::new();
    (if dark { &DARK } else { &LIGHT })
        .get_or_init(|| {
            Theme::custom(
                if dark {
                    "Android Hub Dark"
                } else {
                    "Android Hub Light"
                }
                .into(),
                Palette {
                    background: if dark {
                        Color::from_rgb8(24, 27, 33)
                    } else {
                        Color::WHITE
                    },
                    text: if dark {
                        Color::from_rgb8(236, 239, 245)
                    } else {
                        Color::from_rgb8(33, 36, 41)
                    },
                    primary: if dark {
                        Color::from_rgb8(120, 170, 255)
                    } else {
                        Color::from_rgb8(37, 99, 235)
                    },
                    success: if dark {
                        Color::from_rgb8(100, 210, 158)
                    } else {
                        Color::from_rgb8(20, 106, 66)
                    },
                    danger: if dark {
                        Color::from_rgb8(255, 150, 150)
                    } else {
                        Color::from_rgb8(170, 35, 35)
                    },
                },
            )
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preference_round_trip_and_unknown_fallback_are_independent_of_guest_data() {
        let root = std::env::temp_dir().join(format!("hub-appearance-test-{}", std::process::id()));
        assert_eq!(Preference::load(&root), Preference::System);
        for p in Preference::ALL {
            p.save(&root).unwrap();
            assert_eq!(Preference::load(&root), p);
        }
        let file = root.join("ui-appearance");
        std::fs::write(&file, "future-version").unwrap();
        assert_eq!(Preference::load(&root), Preference::System);
        std::fs::write(&file, [0xff]).unwrap();
        assert_eq!(Preference::load(&root), Preference::System);
        assert!(Preference::System.is_dark(true));
        assert!(!Preference::System.is_dark(false));
        assert!(!Preference::Light.is_dark(true));
        assert!(Preference::Dark.is_dark(false));
        std::fs::remove_dir_all(root).unwrap();
    }
}
